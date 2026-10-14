//! The CLI over the fakes: the walkthrough's output and exits.

mod support;

use std::cell::Cell;

use clap::Parser;
use sluice::{run, Cli, Pause, CONFLICT, FAILED, LANDED};
use sluice_fakes::Fakes;

const BRANCH: &str = "bolt/concurrent-units";

const POLICY: &str = "[branches.\"bolt/*\"]\nrequired = [\"#gate\"]\nparallel = 4\ntrust = [\"open\", \"sandbox\", \"container\", \"cache\"]\n";

struct Rounds(Cell<usize>);

impl Pause for Rounds {
    fn pause(&self) -> bool {
        self.0.set(self.0.get() + 1);
        assert!(self.0.get() < 50, "waited too long");
        true
    }
}

fn setup() -> Fakes {
    let f = Fakes::new();
    f.world.init(
        BRANCH,
        &[
            (".config/sluice.toml", POLICY),
            ("plugin/lib/plan.py", "plan"),
            ("tests/t_cards.py", "cards"),
        ],
    );
    f.world.task("tests:t-batch-cards", "tests/");
    f.world.task("plugin:test", "plugin/");
    f
}

fn sluice(f: &Fakes, args: &[&str]) -> (i32, String) {
    let cli = Cli::try_parse_from(std::iter::once("sluice").chain(args.iter().copied())).unwrap();
    let mut out = Vec::new();
    let code = run(
        cli,
        f.seams(),
        &support::FakeHost {
            world: f.world.clone(),
            reserved: 0,
        },
        &f.clock,
        &Rounds(Cell::new(0)),
        &mut out,
    )
    .unwrap();
    (code, String::from_utf8(out).unwrap())
}

fn work(f: &Fakes, worktree: &str, agent: &str, changes: &[(&str, Option<&str>)]) {
    let head = f.world.work(&f.world.tip(BRANCH), changes);
    f.world.session(worktree, agent, &head);
}

#[test]
fn submit_waits_and_lands() {
    let f = setup();
    work(
        &f,
        "unit/touches",
        "crw-1-unit-1",
        &[("plugin/lib/plan.py", Some("touched"))],
    );
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/touches"]);
    let tip = f.world.tip(BRANCH);
    assert_eq!(code, LANDED);
    assert_eq!(
        out,
        format!(
            "submission 1 to {BRANCH} · nothing queued ahead\n\
             testing {BRANCH} + #1 · 2 tasks to run, 0 replayed\n\
             passed · landed as {}\n",
            &tip.0[..7]
        )
    );
}

#[test]
fn submit_fails_and_names_the_combination() {
    let f = setup();
    f.world
        .fails_with("tests:t-batch-cards", "tests/t_cards.py", "broken");
    work(
        &f,
        "unit/touches",
        "crw-1-unit-1",
        &[("plugin/lib/plan.py", Some("touched"))],
    );
    // Behind #1, so tested on top of it.
    work(
        &f,
        "unit/batch-review",
        "crw-1-unit-2",
        &[
            ("tests/t_cards.py", Some("broken")),
            ("plugin/notes", Some("n")),
        ],
    );
    let (_, _) = sluice(
        &f,
        &["submit", BRANCH, "--worktree", "unit/touches", "--no-wait"],
    );
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/batch-review"]);
    assert_eq!(code, FAILED);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0],
        format!("submission 2 to {BRANCH} · 1 queued ahead (#1)")
    );
    assert!(lines[1].starts_with(&format!("testing {BRANCH} + #1 + #2 · ")));
    assert_eq!(lines[2], "failed: tests:t-batch-cards");
    assert!(lines[3].starts_with(&format!("  on {BRANCH} + #1 + #2 (")));
    assert_eq!(
        lines[4],
        "  #1 passed on its own, so the failure is from this submission"
    );
    // 20: where the failed task's output is kept, and what it holds.
    let at = lines[5]
        .strip_prefix("  log of tests:t-batch-cards: ")
        .unwrap_or_else(|| panic!("{}", lines[5]));
    assert!(f
        .world
        .kept_log(at)
        .unwrap()
        .contains("tests:t-batch-cards failed"));
    assert_eq!(lines[6], "fix it in your worktree and submit again");
}

#[test]
fn a_conflict_exits_two_and_check_says_so_first() {
    let f = setup();
    work(
        &f,
        "unit/touches",
        "crw-1-unit-1",
        &[("plugin/lib/plan.py", Some("one"))],
    );
    sluice(
        &f,
        &["submit", BRANCH, "--worktree", "unit/touches", "--no-wait"],
    );
    work(
        &f,
        "unit/forecast",
        "crw-1-unit-3",
        &[("plugin/lib/plan.py", Some("two"))],
    );

    let (code, out) = sluice(
        &f,
        &["submit", "--check", BRANCH, "--worktree", "unit/forecast"],
    );
    assert_eq!(code, CONFLICT);
    assert_eq!(
        out,
        "would conflict: plugin/lib/plan.py with #1 (crw-1-unit-1, unit/touches)\n"
    );

    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/forecast"]);
    assert_eq!(code, CONFLICT);
    assert_eq!(
        out,
        format!(
            "submission 2 to {BRANCH} · 1 queued ahead (#1)\n\
             conflict: plugin/lib/plan.py\n  conflicts with #1 (crw-1-unit-1, unit/touches)\n\
             merge or rebase onto {BRANCH} in your worktree and submit again\n"
        )
    );
}

#[test]
fn a_dirty_place_is_refused_with_exit_two() {
    let f = setup();
    work(&f, "unit/a", "agent", &[("plugin/x", Some("x"))]);
    f.world.dirty("unit/a", &["plugin/scratch.py"]);
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, CONFLICT);
    assert_eq!(
        out,
        "refused: unit/a holds changes beyond its commits:\n  plugin/scratch.py\n\
         commit or remove them and submit again\n"
    );
}

#[test]
fn status_lists_the_queue() {
    let f = setup();
    f.world
        .fails_with("tests:t-batch-cards", "tests/t_cards.py", "broken");
    work(
        &f,
        "unit/touches",
        "crw-1-unit-1",
        &[("plugin/lib/plan.py", Some("touched"))],
    );
    work(
        &f,
        "unit/batch-review",
        "crw-1-unit-2",
        &[("tests/t_cards.py", Some("broken"))],
    );
    sluice(
        &f,
        &["submit", BRANCH, "--worktree", "unit/touches", "--no-wait"],
    );
    sluice(
        &f,
        &[
            "submit",
            BRANCH,
            "--worktree",
            "unit/batch-review",
            "--no-wait",
        ],
    );
    sluice(&f, &["run", "--rounds", "6"]);
    f.world.hold_runs(true);
    work(
        &f,
        "unit/forecast",
        "crw-1-unit-3",
        &[("plugin/other.py", Some("x"))],
    );
    sluice(
        &f,
        &["submit", BRANCH, "--worktree", "unit/forecast", "--no-wait"],
    );
    sluice(&f, &["step", BRANCH]);

    let (code, out) = sluice(&f, &["status", BRANCH]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().collect();
    let tip = f.world.tip(BRANCH);
    assert_eq!(
        lines[0],
        format!("{BRANCH} at {} · testing 1 at once (up to 4)", &tip.0[..7])
    );
    assert_eq!(lines[1], "  #1  landed    crw-1-unit-1  unit/touches");
    assert!(lines[2]
        .starts_with("  #2  failed    crw-1-unit-2  unit/batch-review   tests:t-batch-cards on "));
    assert_eq!(
        lines[3],
        format!("  #3  testing   crw-1-unit-3  unit/forecast       {BRANCH} + #3")
    );

    let (_, json) = sluice(&f, &["status", BRANCH, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["submissions"].as_array().unwrap().len(), 3);
    assert_eq!(v["submissions"][1]["state"], "failed");
}

#[test]
fn amend_and_withdraw() {
    let f = setup();
    f.world.hold_runs(true);
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/b", "--no-wait"]);
    assert_eq!(
        sluice(&f, &["amend", "2", "--priority", "1"]),
        (0, "amended #2\n".into())
    );
    sluice(&f, &["step", BRANCH]);
    assert_eq!(
        sluice(&f, &["amend", "2", "--after", "1"]),
        (2, "refused: #2 is under test; its order is fixed\n".into())
    );
    assert_eq!(
        sluice(&f, &["withdraw", "1", "--as", "agent-b"]),
        (2, "refused: #1 is not agent-b's to withdraw\n".into())
    );
    assert_eq!(
        sluice(&f, &["withdraw", "1", "--as", "agent-a"]),
        (0, "withdrew #1\n".into())
    );
}

// ---------------------------------------------------------------------------
// sluice exec (session-runs-1 clauses 89 to 93)
// ---------------------------------------------------------------------------

/// A pause that runs `then` on its first call, and stops the wait when
/// `stop` says so.
struct Then<F: Fn(usize) -> bool>(Cell<usize>, F);

impl<F: Fn(usize) -> bool> Pause for Then<F> {
    fn pause(&self) -> bool {
        self.0.set(self.0.get() + 1);
        assert!(self.0.get() < 50, "waited too long");
        (self.1)(self.0.get())
    }
}

fn exec_with(f: &Fakes, reserved: usize, pause: &dyn Pause, args: &[&str]) -> (i32, String) {
    let cli =
        Cli::try_parse_from(["sluice", "exec"].into_iter().chain(args.iter().copied())).unwrap();
    let mut out = Vec::new();
    let host = support::FakeHost {
        world: f.world.clone(),
        reserved,
    };
    let code = run(cli, f.seams(), &host, &f.clock, pause, &mut out).unwrap();
    (code, String::from_utf8(out).unwrap())
}

fn exec(f: &Fakes, args: &[&str]) -> (i32, String) {
    exec_with(f, 0, &Rounds(Cell::new(0)), args)
}

/// The arguments: a branch, and a command after `--`.
#[test]
fn exec_takes_a_branch_and_a_command() {
    assert!(Cli::try_parse_from(["sluice", "exec"]).is_err());
    let cli =
        Cli::try_parse_from(["sluice", "exec", BRANCH, "--", "moon", "run", ":test"]).unwrap();
    match cli.command {
        sluice::Command::Exec {
            branch,
            command,
            worktree,
            json,
            release,
        } => {
            assert!(!release);
            assert_eq!(branch, BRANCH);
            assert_eq!(command, ["moon", "run", ":test"]);
            assert_eq!(worktree, ".");
            assert!(!json);
        }
        other => panic!("{other:?}"),
    }
}

/// A place that is no worktree is refused, and nothing runs.
#[test]
fn exec_outside_a_worktree_is_refused() {
    let f = setup();
    let (code, out) = exec(&f, &[BRANCH, "--worktree", "nowhere"]);
    assert_eq!(code, CONFLICT);
    assert!(
        out.starts_with("refused: nowhere is not a worktree"),
        "{out}"
    );
    assert!(f.world.started().is_empty());
    let (code, out) = exec(&f, &["bolt/none", "--worktree", "nowhere"]);
    assert_eq!(code, CONFLICT, "{out}");
}

/// 89: the checks the head affects against the branch run in the
/// worktree's adopted place, take a slot only while they run, and the
/// exit is theirs.
#[test]
fn exec_runs_the_affected_checks_against_the_branch() {
    let f = setup();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    let base = f.world.tip(BRANCH);
    let (code, out) = exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    let req = f.world.requests().pop().unwrap();
    assert_eq!(req.place.name, "session/unit/a");
    assert_eq!(req.since, Some(base.clone()));
    assert!(!req.replay);
    assert_eq!(f.world.observed().len(), 1, "only the affected check ran");
    assert_eq!(f.world.adopted_places(), ["session/unit/a"]);
    assert_eq!(
        sluice_seams::Tests::sessions(&f.tests).unwrap(),
        0,
        "the slot is let go"
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0],
        format!(
            "running the checks {} affects against {BRANCH} ({})",
            sluice_seams::Driver::session(&f.driver, "unit/a")
                .unwrap()
                .head
                .short(),
            base.short()
        )
    );
    let n = lines.len();
    assert_eq!(lines[n - 2], "passed: 1 task");
    assert!(lines[n - 1].starts_with("recorded 1 result under its key for "));

    f.world
        .fails_with("plugin:test", "plugin/lib/plan.py", "p2");
    let (code, out) = exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, FAILED);
    assert!(out.contains("failed: plugin:test\n"), "{out}");
}

/// 89, 71: with no slot free, exec waits, and starts when one frees.
#[test]
fn exec_waits_for_a_slot() {
    let f = setup();
    f.world.host_limit(1);
    f.world.session_run("/w/other", true);
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    let world = f.world.clone();
    let pause = Then(Cell::new(0), move |n| {
        if n == 1 {
            assert!(world.started().is_empty(), "started into a held slot");
            world.kill_session("/w/other");
        }
        true
    });
    let (code, out) = exec_with(&f, 0, &pause, &[BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    assert!(out.contains("waiting for a slot\n"), "{out}");
    assert_eq!(f.world.started().len(), 1);
}

/// 91: a session run never takes a slot the reserve keeps; stopped while
/// waiting, nothing ran and nothing is recorded.
#[test]
fn exec_never_takes_the_reserve() {
    let f = setup();
    f.world.host_limit(2);
    f.world.session_run("/w/other", true);
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    let pause = Then(Cell::new(0), |_| false);
    let (code, out) = exec_with(&f, 1, &pause, &[BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, sluice::STOPPED);
    assert!(
        out.ends_with("waiting for a slot\nstopped: nothing ran\n"),
        "{out}"
    );
    assert!(f.world.started().is_empty());
    assert_eq!(f.world.results_log(), "");
    // Without the reserve, the free slot is taken.
    let (code, _) = exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED);
}

/// 66, 92: a run past the branch's time limit is reported as timed out,
/// and recorded as neither a pass nor a fail.
#[test]
fn exec_past_its_time_limit_is_timed_out_and_unrecorded() {
    let f = Fakes::new();
    f.world.init(
        BRANCH,
        &[
            (
                ".config/sluice.toml",
                &format!("{POLICY}timeout = \"1m\"\n"),
            ),
            ("plugin/lib/plan.py", "plan"),
        ],
    );
    f.world.task("plugin:test", "plugin/");
    f.world.hangs("plugin:test");
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    let (code, out) = exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, FAILED);
    assert!(out.ends_with("timed out\n"), "{out}");
    assert_eq!(f.world.results_log(), "");
}

/// 89, 92: a command runs in the adopted place under the command runner
/// with its words quoted for `sh -c`, gives its exit, and records
/// nothing, clean worktree or not.
#[test]
fn exec_runs_a_command_and_records_nothing() {
    let f = setup();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    let (code, out) = exec(
        &f,
        &[BRANCH, "--worktree", "unit/a", "--", "moon", "run", "a b's"],
    );
    assert_eq!(code, LANDED, "{out}");
    assert_eq!(out, "running moon run 'a b'\\''s'\n");
    let req = f.world.requests().pop().unwrap();
    assert!(req.required.is_empty());
    assert_eq!(
        req.tests.get("runner"),
        Some(&sluice_seams::Setting::Text("command".into()))
    );
    assert_eq!(
        req.tests.get("command"),
        Some(&sluice_seams::Setting::Text("moon run 'a b'\\''s'".into()))
    );
    assert_eq!(f.world.results_log(), "");
}

fn result_lines(f: &Fakes) -> Vec<sluice_core::records::ResultLine> {
    sluice_core::records::parse_lines(f.world.results_log().as_bytes()).unwrap()
}

/// 92, 16: a clean worktree's run of the checks writes one record per
/// check it ran, with its executor and host, on its head from its
/// worktree; a dirty worktree's run writes none and says so.
#[test]
fn exec_records_a_clean_run_and_not_a_dirty_one() {
    let f = setup();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[
            ("plugin/lib/plan.py", Some("p2")),
            ("tests/t_cards.py", Some("c2")),
        ],
    );
    let head = sluice_seams::Driver::session(&f.driver, "unit/a")
        .unwrap()
        .head;
    let (code, out) = exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    assert!(
        out.contains(&format!(
            "recorded 2 results under their keys for {}\n",
            head.short()
        )),
        "{out}"
    );
    let lines = result_lines(&f);
    assert_eq!(lines.len(), 2);
    for l in &lines {
        assert_eq!(l.kind, sluice_core::records::ResultKind::Ran);
        assert_eq!(l.combination, head);
        assert_eq!(l.branch, BRANCH);
        assert_eq!(l.worktree.as_deref(), Some("unit/a"));
        assert_eq!(l.host.as_deref(), Some("fake-host"));
        let e = l.executor.as_ref().unwrap();
        assert_eq!((e.kind.as_str(), e.confined), ("open", false));
    }

    f.world.dirty("unit/a", &["notes.txt"]);
    let (code, out) = exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    assert!(
        out.contains("not recorded: worktree is not clean\n"),
        "{out}"
    );
    assert_eq!(result_lines(&f).len(), 2);
}

/// `--json`: one document on stdout, in `sluice status --json`'s field
/// names, with each check's key, outcome and whether it was recorded; the
/// human output goes to stderr.
#[test]
fn exec_json_is_one_document() {
    let f = setup();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    let (code, out) = exec(&f, &[BRANCH, "--worktree", "unit/a", "--json"]);
    assert_eq!(code, LANDED);
    let doc: serde_json::Value =
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("{e}: {out}"));
    assert_eq!(out.lines().count(), 1, "{out}");
    assert_eq!(doc["branch"], BRANCH);
    assert_eq!(doc["exit"], 0);
    assert_eq!(doc["timed_out"], false);
    assert_eq!(doc["ports"], serde_json::json!({}));
    let tasks = doc["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["task"], "plugin:test");
    assert_eq!(tasks[0]["outcome"], "pass");
    assert_eq!(tasks[0]["recorded"], true);
    assert!(tasks[0]["hash"].is_string());

    f.world.dirty("unit/a", &["notes.txt"]);
    let (_, out) = exec(&f, &[BRANCH, "--worktree", "unit/a", "--json"]);
    let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(doc["tasks"][0]["recorded"], false);
}

// ---------------------------------------------------------------------------
// sluice release (session-runs-1 clauses 94, 95)
// ---------------------------------------------------------------------------

const RELEASING: &str = "[tests]\nrelease = \"sh release.sh\"\n\n";

fn setup_releasing() -> Fakes {
    let f = Fakes::new();
    f.world.init(
        BRANCH,
        &[
            (".config/sluice.toml", &format!("{RELEASING}{POLICY}")),
            ("plugin/lib/plan.py", "plan"),
        ],
    );
    f.world.task("plugin:test", "plugin/");
    f
}

fn release(f: &Fakes, args: &[&str]) -> (i32, String) {
    let cli = Cli::try_parse_from(
        ["sluice", "release"]
            .into_iter()
            .chain(args.iter().copied()),
    )
    .unwrap();
    let mut out = Vec::new();
    let host = support::FakeHost {
        world: f.world.clone(),
        reserved: 0,
    };
    let code = run(
        cli,
        f.seams(),
        &host,
        &f.clock,
        &Rounds(Cell::new(0)),
        &mut out,
    )
    .unwrap();
    (code, String::from_utf8(out).unwrap())
}

/// 94, 95: the worktree's runs leave its place equipped; `sluice release`
/// releases it under the release command its branch's policy states now,
/// and a second release finds nothing.
#[test]
fn release_runs_the_policys_release_command_once() {
    let f = setup_releasing();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    assert!(
        f.world.session_releases().is_empty(),
        "released between runs"
    );
    assert_eq!(f.world.adopted_places(), ["session/unit/a"]);

    // The policy's release command changed after the last run: the new
    // one is the one that runs (56).
    f.world.hand_commit(
        BRANCH,
        &[(
            ".config/sluice.toml",
            Some(&format!(
                "[tests]\nrelease = \"sh teardown.sh\"\n\n{POLICY}"
            )),
        )],
    );
    let (code, out) = release(&f, &["--worktree", "unit/a"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "released unit/a: its runs ended, the release command run, its ports free\n"
    );
    assert_eq!(
        f.world.session_releases(),
        [(
            "session/unit/a".to_string(),
            Some("sh teardown.sh".to_string())
        )]
    );
    assert!(f.world.adopted_places().is_empty());

    let (code, out) = release(&f, &["--worktree", "unit/a"]);
    assert_eq!((code, out.as_str()), (0, "nothing to release\n"));
    assert_eq!(f.world.session_releases().len(), 1);
    // A worktree that never ran has nothing to release.
    let (code, out) = release(&f, &["--worktree", "unit/none"]);
    assert_eq!((code, out.as_str()), (0, "nothing to release\n"));
}

/// 95: with the branch gone, the release command the last run carried
/// runs, and the output says so.
#[test]
fn release_falls_back_to_the_last_runs_command_and_says_so() {
    let f = setup_releasing();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    f.world.delete_branch(BRANCH);
    let (code, out) = release(&f, &["--worktree", "unit/a"]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0],
        format!(
            "{BRANCH} cannot be read (no branch {BRANCH}): running the release command saved with the last run"
        )
    );
    assert_eq!(
        f.world.session_releases(),
        [("session/unit/a".to_string(), None)]
    );
}

/// 95: `sluice exec --release` releases once its run ends, and the exit
/// is the run's.
#[test]
fn exec_release_releases_after_the_run() {
    let f = setup_releasing();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    let (code, out) = exec(&f, &[BRANCH, "--worktree", "unit/a", "--release"]);
    assert_eq!(code, LANDED, "{out}");
    assert!(
        out.ends_with("released unit/a: its runs ended, the release command run, its ports free\n"),
        "{out}"
    );
    assert_eq!(
        f.world.session_releases(),
        [(
            "session/unit/a".to_string(),
            Some("sh release.sh".to_string())
        )]
    );
}

/// 25, rule core.16: a submission from a worktree that ran `sluice exec`
/// lands, and no round releases the worktree's place.
#[test]
fn a_landing_leaves_a_sessions_place_alone() {
    let f = setup_releasing();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    let (code, _) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED);
    assert_eq!(f.world.adopted_places(), ["session/unit/a"]);
    assert!(f.world.session_releases().is_empty());
}
