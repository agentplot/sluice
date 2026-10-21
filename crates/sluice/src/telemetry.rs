//! OpenTelemetry for sluice (ADR 0014): off unless `--otel` or
//! `SLUICE_OTEL` turns it on, then the core's `tracing` spans and metric
//! events exported over OTLP/HTTP to wherever the standard
//! `OTEL_EXPORTER_OTLP_*` variables say. Telemetry decides nothing: an
//! exporter that fails is silent, and nothing reads it back.
//!
//! A run's checks get sluice's `OTEL_*` and, in `OTEL_RESOURCE_ATTRIBUTES`,
//! the trace and span of the round that started them, so what they export
//! is found beside the round; and the W3C `TRACEPARENT`, for a tool that
//! reads it to nest its spans under the start.
//!
//! Logs (ADR 0022): a failed check's output, which the core emits once
//! where it keeps it, and sluice's own warnings and errors, sent as OTLP
//! log records under the span they were emitted in. `SLUICE_OTEL_LOGS`
//! says how much of the output goes: none, a summary naming where it is
//! kept, or its end scrubbed of what reads as a secret. The kept copy on
//! disk is the record (20, 26; rule core.11); a record lost on the way is
//! never sent again.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

use std::time::Duration;

use anyhow::Result;
use opentelemetry::logs::{AnyValue, LogRecord as _};
use opentelemetry::trace::{TraceContextExt, TracerProvider as _};
use opentelemetry::InstrumentationScope;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter};
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::logs::{self, LogProcessor, SdkLogRecord, SdkLoggerProvider};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use sluice_core::OUTPUT;
use sluice_seams::LOG_LIMIT;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetrySpanExt};
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;

static ON: AtomicBool = AtomicBool::new(false);

/// How much of a failed check's output is sent (ADR 0022).
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Logs {
    /// No logs at all.
    Off,
    /// Which task failed and where its output is kept; never the output.
    Summary,
    /// The summary's fields and the output's end, scrubbed.
    Tail,
}

/// The exporters, flushed by [`Telemetry::finish`] before sluice exits.
pub struct Telemetry {
    traces: SdkTracerProvider,
    metrics: SdkMeterProvider,
    logs: Option<SdkLoggerProvider>,
}

impl Telemetry {
    /// Export from here on, as `service`, with `logs` of a failure's
    /// output and at most `limit` bytes of it.
    pub fn start(service: &str, logs: Logs, limit: usize) -> Result<Telemetry> {
        let exporter = match logs {
            Logs::Off => None,
            _ => Some(LogExporter::builder().with_http().build()?),
        };
        let spans = SpanExporter::builder().with_http().build()?;
        Telemetry::start_with(service, logs, limit, spans, exporter)
    }

    /// [`Telemetry::start`], with the spans sent to `spans` and the logs
    /// to `exporter`.
    pub fn start_with<S, E>(
        service: &str,
        logs: Logs,
        limit: usize,
        spans: S,
        exporter: Option<E>,
    ) -> Result<Telemetry>
    where
        S: opentelemetry_sdk::trace::SpanExporter + 'static,
        E: logs::LogExporter + 'static,
    {
        let resource = Resource::builder()
            .with_service_name(service.to_string())
            .with_attribute(opentelemetry::KeyValue::new(
                "service.version",
                crate::VERSION,
            ))
            .build();
        let traces = SdkTracerProvider::builder()
            .with_batch_exporter(spans)
            .with_resource(resource.clone())
            .build();
        let metrics = SdkMeterProvider::builder()
            .with_periodic_exporter(MetricExporter::builder().with_http().build()?)
            .with_resource(resource.clone())
            .build();
        let logs = exporter.filter(|_| logs != Logs::Off).map(|e| {
            SdkLoggerProvider::builder()
                .with_log_processor(Scrub::new(logs, limit, std::env::vars()))
                .with_batch_exporter(e)
                .with_resource(resource)
                .build()
        });
        // Only a failure's output and what sluice warns of become logs:
        // never a metric's event.
        let bridge = logs.as_ref().map(|l| {
            OpenTelemetryTracingBridge::new(l).with_filter(filter_fn(|m| {
                m.is_event() && (m.target() == OUTPUT || *m.level() <= tracing::Level::WARN)
            }))
        });
        let subscriber = tracing_subscriber::registry()
            .with(tracing_opentelemetry::layer().with_tracer(traces.tracer("sluice")))
            .with(MetricsLayer::new(metrics.clone()))
            .with(bridge);
        tracing::subscriber::set_global_default(subscriber)?;
        ON.store(true, Ordering::SeqCst);
        Ok(Telemetry {
            traces,
            metrics,
            logs,
        })
    }

    /// Send what is held so far, and go on.
    pub fn flush(&self) {
        let _ = self.traces.force_flush();
        if let Some(l) = &self.logs {
            let _ = l.force_flush();
        }
    }

    /// Send what is left. A collector that cannot be reached loses it.
    pub fn finish(self) {
        let _ = self.traces.shutdown();
        let _ = self.metrics.shutdown();
        if let Some(l) = self.logs {
            let _ = l.shutdown();
        }
    }
}

/// A line sluice says on stderr, sent as a warning while telemetry sends
/// logs.
pub fn warn(line: &str) {
    eprintln!("{line}");
    tracing::warn!("{line}");
}

/// A line sluice says on stderr, sent as an error while telemetry sends
/// logs.
pub fn error(line: &str) {
    eprintln!("{line}");
    tracing::error!("{line}");
}

/// What a log record's body is made before it is sent: a failure's output
/// as the level says, and every body scrubbed of what reads as a secret.
#[derive(Debug)]
struct Scrub {
    level: Logs,
    limit: usize,
    secrets: crate::scrub::Scrub,
}

impl Scrub {
    fn new(level: Logs, limit: usize, env: impl Iterator<Item = (String, String)>) -> Scrub {
        Scrub {
            level,
            limit: limit.min(LOG_LIMIT),
            secrets: crate::scrub::Scrub::new(env),
        }
    }

    fn scrub(&self, text: &str) -> String {
        self.secrets.scrub(text)
    }

    /// The body a record of `target` with `body` is sent with.
    fn body(&self, target: &str, body: &str, attr: impl Fn(&str) -> Option<String>) -> String {
        if target != OUTPUT {
            return self.scrub(body);
        }
        match self.level {
            Logs::Tail => end(&self.scrub(body), self.limit).to_string(),
            _ => {
                let what = attr("sluice.task")
                    .or_else(|| attr("sluice.preview").map(|p| format!("preview {p}")))
                    .unwrap_or_else(|| "a check".into());
                match attr("sluice.kept").filter(|k| !k.is_empty()) {
                    Some(at) => format!("{what} failed; its output is kept at {at}"),
                    None => format!("{what} failed"),
                }
            }
        }
    }
}

impl LogProcessor for Scrub {
    fn emit(&self, record: &mut SdkLogRecord, _: &InstrumentationScope) {
        let Some(AnyValue::String(body)) = record.body() else {
            return;
        };
        let target = record.target().map(|t| t.to_string()).unwrap_or_default();
        let attr = |key: &str| {
            record
                .attributes_iter()
                .find(|(k, _)| k.as_str() == key)
                .map(|(_, v)| match v {
                    AnyValue::String(s) => s.as_str().to_string(),
                    other => format!("{other:?}"),
                })
        };
        let body = self.body(&target, body.as_str(), attr);
        record.set_body(AnyValue::String(body.into()));
    }

    fn force_flush(&self) -> OTelSdkResult {
        Ok(())
    }

    fn shutdown_with_timeout(&self, _: Duration) -> OTelSdkResult {
        Ok(())
    }
}

use crate::scrub::end;

/// Whether this process exports.
pub fn on() -> bool {
    ON.load(Ordering::SeqCst)
}

/// What a run's environment takes of telemetry: nothing when it is off;
/// otherwise sluice's own `OTEL_*`, with `attributes` and the current
/// span's trace and span ids added to `OTEL_RESOURCE_ATTRIBUTES`, and the
/// current span as `TRACEPARENT`.
pub fn run_env(attributes: &[(&str, &str)]) -> BTreeMap<String, String> {
    if !on() {
        return BTreeMap::new();
    }
    let mut env: BTreeMap<String, String> = std::env::vars()
        .filter(|(k, _)| k.starts_with("OTEL_"))
        .collect();
    let cx = tracing::Span::current().context();
    let span = cx.span();
    let ids = span.span_context();
    let (trace, span) = (ids.trace_id().to_string(), ids.span_id().to_string());
    let mut pairs: Vec<String> = env
        .remove("OTEL_RESOURCE_ATTRIBUTES")
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    if ids.is_valid() {
        pairs.push(format!("sluice.trace_id={trace}"));
        pairs.push(format!("sluice.span_id={span}"));
        env.insert(
            "TRACEPARENT".into(),
            format!("00-{trace}-{span}-{:02x}", ids.trace_flags().to_u8()),
        );
    }
    for (k, v) in attributes {
        pairs.push(format!("{k}={}", encode(v)));
    }
    env.insert("OTEL_RESOURCE_ATTRIBUTES".into(), pairs.join(","));
    env
}

/// A resource attribute's value, percent-encoded where the variable's
/// own syntax would read it (W3C baggage octets).
fn encode(v: &str) -> String {
    let mut s = String::new();
    for b in v.bytes() {
        match b {
            b',' | b'=' | b';' | b'%' | b'"' | b'\\' | 0..=0x20 | 0x7f.. => {
                s.push_str(&format!("%{b:02X}"))
            }
            _ => s.push(b as char),
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_encoded_where_the_syntax_would_read_them() {
        assert_eq!(encode("sluice/try/bolt/x/3"), "sluice/try/bolt/x/3");
        assert_eq!(encode("a,b=c d"), "a%2Cb%3Dc%20d");
    }

    fn scrub(level: Logs, limit: usize, env: &[(&str, &str)]) -> Scrub {
        let env = env.iter().map(|(k, v)| (k.to_string(), v.to_string()));
        Scrub::new(level, limit, env.collect::<Vec<_>>().into_iter())
    }

    #[test]
    fn what_reads_as_a_secret_is_redacted() {
        let s = scrub(Logs::Tail, 4096, &[]);
        for secret in [
            "ghp_0123456789abcdefghijABCDEFGHIJ012345",
            "github_pat_11ABCDEFG0123456789_abcdefghijkl",
            "AKIAABCDEFGHIJKLMNOP",
            "xoxb-1234567890-abcdef",
            "sk-ant-api03-abcdefghijkl",
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.c2lnbmF0dXJl",
            "Bearer abcdefgh.ijkl",
            "-----BEGIN RSA PRIVATE KEY-----\nMIIE\n-----END RSA PRIVATE KEY-----",
        ] {
            let out = s.body(OUTPUT, &format!("before {secret} after"), |_| None);
            assert_eq!(out, "before [redacted] after", "{secret}");
        }
        assert_eq!(s.body(OUTPUT, "ok: 3 passed", |_| None), "ok: 3 passed");
    }

    #[test]
    fn a_secret_variable_s_value_is_redacted_wherever_it_appears() {
        let s = scrub(
            Logs::Tail,
            4096,
            &[
                ("NPM_TOKEN", "s3cr3t-value-1"),
                ("OTEL_EXPORTER_OTLP_HEADERS", "x-api=abcdefgh"),
                ("HOME", "/home/someone"),
                ("API_KEY", "short"),
            ],
        );
        let out = s.body(
            OUTPUT,
            "a s3cr3t-value-1 b x-api=abcdefgh c /home/someone short",
            |_| None,
        );
        assert_eq!(out, "a [redacted] b [redacted] c /home/someone short");
    }

    #[test]
    fn a_summary_sends_where_the_output_is_kept_never_the_output() {
        let s = scrub(Logs::Summary, 4096, &[]);
        let attr = |k: &str| match k {
            "sluice.task" => Some("a:test".to_string()),
            "sluice.kept" => Some("/x/.logs/a.log".to_string()),
            _ => None,
        };
        assert_eq!(
            s.body(OUTPUT, "the output", attr),
            "a:test failed; its output is kept at /x/.logs/a.log"
        );
        // What sluice warns of is never summarised, only scrubbed.
        assert_eq!(
            s.body(
                "sluice::lib",
                "a warning ghp_0123456789abcdefghijABCDEF",
                |_| None
            ),
            "a warning [redacted]"
        );
    }

    #[test]
    fn a_tail_keeps_the_end_from_a_character_s_start() {
        // é is two bytes: the last two of "abcdéf" begin inside it.
        let s = scrub(Logs::Tail, 2, &[]);
        assert_eq!(s.body(OUTPUT, "abcdéf", |_| None), "f");
        assert_eq!(s.body(OUTPUT, "abcdefgh", |_| None), "gh");
        assert_eq!(scrub(Logs::Tail, 1 << 30, &[]).limit, LOG_LIMIT);
    }

    #[test]
    fn a_run_gets_nothing_while_telemetry_is_off() {
        assert!(run_env(&[("sluice.place", "p")]).is_empty());
    }
}
