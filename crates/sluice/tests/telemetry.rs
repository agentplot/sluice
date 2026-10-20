//! What a round reports while telemetry is on (ADRs 0014, 0022): its
//! spans, its metric events and a failure's output, over the fakes. What
//! it decides is the same with telemetry on or off, which the CLI's tests
//! show with it off.

mod support;

use std::cell::Cell;
use std::sync::{Arc, Mutex};

use clap::Parser;
use sluice::{run, Cli, Pause, FAILED, LANDED};
use sluice_core::OUTPUT;
use sluice_fakes::Fakes;
use tracing::field::{Field, Visit};
use tracing::span::Attributes;
use tracing::{Event, Id, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

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

/// Every span's name, every metric event's metric and every output event
/// as `output <task> in <span>`, in order.
#[derive(Clone, Default)]
struct Seen(Arc<Mutex<Vec<String>>>);

struct Names<'a>(&'a mut Vec<String>);

/// An output event's task.
struct Task(String);

impl Visit for Task {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "sluice.task" {
            self.0 = format!("{value:?}");
        }
    }
}

impl Visit for Names<'_> {
    fn record_debug(&mut self, field: &Field, _: &dyn std::fmt::Debug) {
        let name = field.name();
        if ["monotonic_counter.", "histogram.", "gauge."]
            .iter()
            .any(|p| name.starts_with(p))
        {
            self.0.push(name.to_string());
        }
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for Seen {
    fn on_new_span(&self, attrs: &Attributes<'_>, _: &Id, _: Context<'_, S>) {
        self.0
            .lock()
            .unwrap()
            .push(attrs.metadata().name().to_string());
    }

    fn on_event(&self, event: &Event<'_>, cx: Context<'_, S>) {
        if event.metadata().target() == OUTPUT {
            let mut task = Task(String::new());
            event.record(&mut task);
            let span = cx.event_span(event).map(|s| s.name()).unwrap_or("none");
            self.0
                .lock()
                .unwrap()
                .push(format!("output {} in {span}", task.0));
        }
        let mut names = Vec::new();
        event.record(&mut Names(&mut names));
        self.0.lock().unwrap().extend(names);
    }
}

fn submit(f: &Fakes, worktree: &str, agent: &str, changes: &[(&str, Option<&str>)]) -> Vec<String> {
    let head = f.world.work(&f.world.tip(BRANCH), changes);
    f.world.session(worktree, agent, &head);
    let seen = Seen::default();
    let subscriber = tracing_subscriber::registry().with(seen.clone());
    let cli = Cli::try_parse_from(["sluice", "submit", BRANCH, "--worktree", worktree]).unwrap();
    let mut out = Vec::new();
    let code = tracing::subscriber::with_default(subscriber, || {
        run(
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
            &mut out,
        )
        .unwrap()
    });
    let mut seen = seen.0.lock().unwrap().clone();
    seen.push(format!("exit {code}"));
    seen
}

fn setup() -> Fakes {
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
    f.world.task("lib:test", "lib/");
    f
}

fn has(seen: &[String], name: &str) -> bool {
    seen.iter().any(|s| s == name)
}

#[test]
fn a_landing_reports_its_round_verdict_and_landing() {
    let f = setup();
    let seen = submit(&f, "unit/a", "agent-a", &[("lib/a.py", Some("changed"))]);
    assert!(has(&seen, &format!("exit {LANDED}")), "{seen:?}");
    for name in [
        "sluice.round",
        "sluice.recover",
        "sluice.decide",
        "sluice.perform",
        "sluice.form",
        "sluice.start",
        "sluice.verdict",
        "sluice.land",
        "gauge.sluice.queue.depth",
        "monotonic_counter.sluice.verdicts",
        "monotonic_counter.sluice.tasks.ran",
        "monotonic_counter.sluice.landed",
        "histogram.sluice.submission.landed_after",
        "histogram.sluice.candidate.tested_for",
    ] {
        assert!(has(&seen, name), "no {name} in {seen:?}");
    }
    assert!(!has(&seen, "sluice.blame"), "{seen:?}");
    assert!(!seen.iter().any(|s| s.starts_with("output ")), "{seen:?}");
}

#[test]
fn a_failure_reports_its_culprit() {
    let f = setup();
    f.world.fails_with("tests:t-a", "tests/t_a.py", "broken");
    let seen = submit(&f, "unit/b", "agent-b", &[("tests/t_a.py", Some("broken"))]);
    assert!(has(&seen, &format!("exit {FAILED}")), "{seen:?}");
    for name in ["sluice.blame", "monotonic_counter.sluice.culprits"] {
        assert!(has(&seen, name), "no {name} in {seen:?}");
    }
    assert!(!has(&seen, "sluice.land"), "{seen:?}");
    // ADR 0022, 20: the failed task's output, once, in the verdict's span.
    let outputs: Vec<&String> = seen.iter().filter(|s| s.starts_with("output ")).collect();
    assert_eq!(outputs, ["output tests:t-a in sluice.verdict"], "{seen:?}");
}
