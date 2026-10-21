//! A failure's output as an OTLP log record (ADR 0022), through the
//! exporter sluice sets up: under the span of the round that read the
//! verdict, scrubbed, and no record from a metric's event. Its own
//! process, since the exporter's subscriber is the process's.

mod support;

use std::cell::Cell;

use clap::Parser;
use opentelemetry::logs::{AnyValue, Severity};
use opentelemetry_sdk::logs::InMemoryLogExporter;
use opentelemetry_sdk::trace::InMemorySpanExporter;
use sluice::telemetry::{Logs, Telemetry};
use sluice::{run, Cli, Pause, FAILED};
use sluice_core::OUTPUT;
use sluice_fakes::Fakes;

const BRANCH: &str = "bolt/telemetry";

const POLICY: &str = "[branches.\"bolt/*\"]\nrequired = [\"#gate\"]\nparallel = 4\ntrust = [\"open\", \"sandbox\", \"container\"]\n";

struct Rounds(Cell<usize>);

impl Pause for Rounds {
    fn pause(&self) -> bool {
        self.0.set(self.0.get() + 1);
        assert!(self.0.get() < 50, "waited too long");
        true
    }
}

#[test]
fn a_failure_s_output_is_sent_under_its_verdict_scrubbed() {
    // What the fake check prints begins so: as a secret's value, it is
    // never sent.
    std::env::set_var("SLUICE_TEST_TOKEN", "tests:t-a failed");
    let spans = InMemorySpanExporter::default();
    let logs = InMemoryLogExporter::default();
    let telemetry = Telemetry::start_with(
        "sluice",
        Logs::Tail,
        8192,
        spans.clone(),
        Some(logs.clone()),
    )
    .unwrap();

    let f = Fakes::new();
    f.world.init(
        BRANCH,
        &[
            (".config/sluice.toml", POLICY),
            ("lib/a.py", "a"),
            ("tests/t_a.py", "t"),
        ],
    );
    f.world.task("tests:t-a", "tests/");
    f.world.fails_with("tests:t-a", "tests/t_a.py", "broken");
    let head = f
        .world
        .work(&f.world.tip(BRANCH), &[("tests/t_a.py", Some("broken"))]);
    f.world.session("unit/a", "agent-a", &head);
    let cli = Cli::try_parse_from(["sluice", "submit", BRANCH, "--worktree", "unit/a"]).unwrap();
    let code = run(
        cli,
        f.seams(),
        &support::FakeHost {
            world: f.world.clone(),
            reserved: 0,
        },
        &(),
        &(),
        &f.clock,
        &Rounds(Cell::new(0)),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(code, FAILED);
    telemetry.flush();

    let records = logs.get_emitted_logs().unwrap();
    // Only a failure's output and warnings: never a metric's event.
    for r in &records {
        let r = &r.record;
        assert!(
            r.target().map(|t| t.as_ref()) == Some(OUTPUT)
                || r.severity_number().is_some_and(|s| s >= Severity::Warn),
            "{r:?}"
        );
    }
    let outputs: Vec<_> = records
        .iter()
        .map(|r| &r.record)
        .filter(|r| r.target().map(|t| t.as_ref()) == Some(OUTPUT))
        .collect();
    assert_eq!(outputs.len(), 1, "{records:?}");
    let out = outputs[0];
    let Some(AnyValue::String(body)) = out.body() else {
        panic!("no body: {out:?}");
    };
    assert!(body.as_str().starts_with("[redacted] in "), "{body:?}");
    let task = out
        .attributes_iter()
        .find(|(k, _)| k.as_str() == "sluice.task")
        .map(|(_, v)| v.clone());
    assert_eq!(task, Some(AnyValue::String("tests:t-a".into())));

    let verdict = spans
        .get_finished_spans()
        .unwrap()
        .into_iter()
        .find(|s| s.name == "sluice.verdict")
        .expect("a verdict span");
    let ids = out.trace_context().expect("the record's trace context");
    assert_eq!(ids.trace_id, verdict.span_context.trace_id());
    assert_eq!(ids.span_id, verdict.span_context.span_id());
    telemetry.finish();
}
