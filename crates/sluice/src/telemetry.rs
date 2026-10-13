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

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use opentelemetry::trace::{TraceContextExt, TracerProvider as _};
use opentelemetry_otlp::{MetricExporter, SpanExporter};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetrySpanExt};
use tracing_subscriber::layer::SubscriberExt;

static ON: AtomicBool = AtomicBool::new(false);

/// The exporters, flushed by [`Telemetry::finish`] before sluice exits.
pub struct Telemetry {
    traces: SdkTracerProvider,
    metrics: SdkMeterProvider,
}

impl Telemetry {
    /// Export from here on, as `service`.
    pub fn start(service: &str) -> Result<Telemetry> {
        let resource = Resource::builder()
            .with_service_name(service.to_string())
            .with_attribute(opentelemetry::KeyValue::new(
                "service.version",
                crate::VERSION,
            ))
            .build();
        let traces = SdkTracerProvider::builder()
            .with_batch_exporter(SpanExporter::builder().with_http().build()?)
            .with_resource(resource.clone())
            .build();
        let metrics = SdkMeterProvider::builder()
            .with_periodic_exporter(MetricExporter::builder().with_http().build()?)
            .with_resource(resource)
            .build();
        let subscriber = tracing_subscriber::registry()
            .with(tracing_opentelemetry::layer().with_tracer(traces.tracer("sluice")))
            .with(MetricsLayer::new(metrics.clone()));
        tracing::subscriber::set_global_default(subscriber)?;
        ON.store(true, Ordering::SeqCst);
        Ok(Telemetry { traces, metrics })
    }

    /// Send what is left. A collector that cannot be reached loses it.
    pub fn finish(self) {
        let _ = self.traces.shutdown();
        let _ = self.metrics.shutdown();
    }
}

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

    #[test]
    fn a_run_gets_nothing_while_telemetry_is_off() {
        assert!(run_env(&[("sluice.place", "p")]).is_empty());
    }
}
