//! The CLI over the fakes: the walkthrough's output and exits.

mod support;

use std::cell::{Cell, RefCell};

use clap::Parser;
use sluice::{run, Cli, Pause, CONFLICT, FAILED, LANDED};
use sluice_fakes::Fakes;
use sluice_seams::Outcome;

const BRANCH: &str = "bolt/concurrent-units";

const POLICY: &str = "[branches.\"bolt/*\"]\nrequired = [\"#gate\"]\nparallel = 4\ntrust = [\"open\", \"sandbox\", \"container\"]\n";

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
        &(),
        &(),
        &f.clock,
        &Rounds(Cell::new(0)),
        &mut out,
    )
    .unwrap();
    (code, String::from_utf8(out).unwrap())
}

/// What a waiting command says while it waits, a submission's state or
/// its combination's progress (ADR 0042), left out where a test reads
/// the outcome it ends with.
fn outcome(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| {
            !(l.starts_with('#') || l.starts_with("testing ") && !l.contains(" tasks to run, "))
        })
        .collect()
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
    let lines: Vec<&str> = out.lines().collect();
    // ADR 0042: while it waits, its combination and how long it has run.
    assert!(
        lines[1].starts_with(&format!("testing {BRANCH} + #1 · "))
            && lines[1].ends_with(" of 1h00m"),
        "{out}"
    );
    assert_eq!(
        [&lines[..1], &lines[2..]].concat(),
        [
            format!("submission 1 to {BRANCH} · nothing queued ahead"),
            format!("testing {BRANCH} + #1 · 2 tasks to run, 0 replayed"),
            format!("passed · landed as {}", &tip.0[..7]),
        ]
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
    let lines = outcome(&out);
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
    assert_eq!(
        lines[6],
        "the agent working in unit/batch-review fixes it there and submits again"
    );
}

/// Could-not-run-1 115: a check the host could not run is blamed on no
/// one, and its way on names the agent working in the worktree.
#[test]
fn a_check_the_host_could_not_run_is_submitted_again_by_the_agent_in_its_worktree() {
    let f = setup();
    f.world
        .cannot_run("tests:t-batch-cards", Some("no emulator on this host"));
    work(&f, "unit/a", "agent", &[("tests/t_cards.py", Some("x"))]);
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, FAILED, "{out}");
    assert_eq!(
        out.lines().last(),
        Some(
            "the host could not run tests:t-batch-cards: once it can, the agent working in \
             unit/a submits again"
        ),
        "{out}"
    );
}

/// 85, 86: a failure on a system the gate names says which system.
#[test]
fn a_failure_names_its_system() {
    let f = Fakes::new();
    f.world.init(
        BRANCH,
        &[
            (
                ".config/sluice.toml",
                &format!(
                    "[systems]\nlinux-x64 = {{ os = \"linux\", arch = \"x86_64\" }}\n\n{}\
                     systems = [\"linux-x64\"]\n",
                    POLICY.replace(
                        "required = [\"#gate\"]",
                        "required = [\"tests:t-batch-cards\"]"
                    )
                ),
            ),
            ("tests/t_cards.py", "cards"),
        ],
    );
    f.world.task("tests:t-batch-cards", "tests/");
    f.world
        .fails_with("tests:t-batch-cards", "tests/t_cards.py", "broken");
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("tests/t_cards.py", Some("broken"))],
    );
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, FAILED);
    assert!(
        out.contains("failed: tests:t-batch-cards (linux-x64)\n"),
        "{out}"
    );
    assert!(
        out.contains("  log of tests:t-batch-cards (linux-x64): "),
        "{out}"
    );
    let (_, out) = sluice(&f, &["status", BRANCH, "--all"]);
    assert!(out.contains("tests:t-batch-cards (linux-x64) on "), "{out}");
    let (_, out) = sluice(&f, &["status", BRANCH, "--all", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["submissions"][0]["failed"][0]["system"], "linux-x64");
}

/// 19, 20: a test read as failed only because its inputs failed before
/// says it is flaky: that it passed in this run, when and on which
/// combination the same inputs failed, how the branch reads that, and
/// where both runs' logs are kept; and that the same head lands once the
/// operator sets the fail aside, or a real fix changes what the test
/// reads, never by a change to its inputs only to have it tested again.
/// `--json` carries the same, with when and on which host each run was,
/// and so does the overview's combination while it is listed.
#[test]
fn a_flaky_failure_says_so() {
    let f = setup();
    f.world
        .flaky("tests:t-batch-cards", &[Outcome::Fail, Outcome::Pass]);
    work(
        &f,
        "unit/batch-review",
        "crw-1-unit-2",
        &[("tests/t_cards.py", Some("flaky"))],
    );
    let (code, first) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/batch-review"]);
    assert_eq!(code, FAILED, "{first}");
    let line = |out: &str, prefix: &str| -> String {
        out.lines()
            .find_map(|l| l.strip_prefix(prefix))
            .unwrap_or_else(|| panic!("no {prefix:?} in {out}"))
            .to_string()
    };
    let failed_on = line(&first, &format!("  on {BRANCH} + #1 ("));
    let failed_on = failed_on.trim_end_matches(')');
    let failed_log = line(&first, "  log of tests:t-batch-cards: ");

    // The same head again: the test runs, passes, and its key now holds
    // a pass and a fail, which the default policy reads as failed.
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/batch-review"]);
    assert_eq!(code, FAILED, "{out}");
    let (_, json) = sluice(&f, &["status", "2", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let flaky = &v["failed"][0]["flaky"];
    assert_eq!(flaky["here"], "pass", "{json}");
    // When and where each run was, from the lines recorded before the
    // verdict: this run's `ran` line, and the run it contradicts.
    let last_ran = |f: &Fakes| {
        let log = String::from_utf8(f.world.files(BRANCH)["log.jsonl"].clone()).unwrap();
        log.lines()
            .rev()
            .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
            .find(|l| l["event"] == "ran" && l["task"] == "tests:t-batch-cards")
            .unwrap()
    };
    let ran = last_ran(&f);
    assert_eq!(ran["outcome"], "pass", "{ran}");
    assert_eq!(flaky["at"], ran["at"], "{json}");
    assert_eq!(flaky["host"], "fake-host", "{json}");
    assert_eq!(flaky["other"]["host"], "fake-host", "{json}");
    assert_eq!(flaky["other"]["outcome"], "fail", "{json}");
    assert_eq!(flaky["other"]["log"], failed_log.as_str(), "{json}");
    assert!(
        flaky["other"]["combination"]
            .as_str()
            .unwrap()
            .starts_with(failed_on),
        "{json}"
    );
    let (_, status) = sluice(&f, &["status", BRANCH, "--all"]);
    assert!(
        status.contains(&format!("tests:t-batch-cards (flaky) on {failed_on}")),
        "{status}"
    );
    let at = flaky["other"]["at"].as_str().unwrap();
    let this_log = v["failed"][0]["log"].as_str().unwrap();

    let lines = outcome(&out);
    assert_eq!(
        lines[0],
        format!("submission 2 to {BRANCH} · nothing queued ahead")
    );
    // The same combination as #1's: only this test of it is counted.
    assert_eq!(
        lines[1],
        format!("testing {BRANCH} + #2 · 1 tasks to run, 1 replayed")
    );
    assert_eq!(lines[2], "failed: tests:t-batch-cards (flaky)");
    assert!(lines[3].starts_with(&format!("  on {BRANCH} + #2 (")));
    assert_eq!(
        lines[4..],
        [
            format!(
                "  tests:t-batch-cards passed in this run and failed on the same inputs \
                 at {at} on {failed_on}"
            ),
            format!("  log of this run: {this_log}"),
            format!("  log of the run that failed: {failed_log}"),
            format!(
                "  if the host or a time limit caused that fail, the operator can set it aside: \
                 sluice set-aside tests:t-batch-cards --at {at} --reason \"…\""
            ),
            format!(
                "  {BRANCH} reads a test that passed and failed on the same inputs as failed \
                 (flaky = \"fail\")"
            ),
            "the same head lands once the operator sets the other result aside, or once a \
             real fix changes what tests:t-batch-cards reads"
                .to_string(),
        ],
        "{out}"
    );

    // A combination whose failure waits on one ahead of it to be blamed
    // (18) is listed in the overview, with when and where each run was:
    // on a branch whose window has room for both.
    let f = setup();
    f.world
        .flaky("tests:t-batch-cards", &[Outcome::Fail, Outcome::Pass]);
    work(
        &f,
        "unit/batch-review",
        "crw-1-unit-2",
        &[("tests/t_cards.py", Some("flaky"))],
    );
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/batch-review"]);
    assert_eq!(code, FAILED, "{out}");
    f.world.hold_runs(true);
    work(
        &f,
        "unit/ahead",
        "crw-1-unit-1",
        &[("plugin/ahead", Some("a"))],
    );
    sluice(
        &f,
        &["submit", BRANCH, "--worktree", "unit/ahead", "--no-wait"],
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
    sluice(&f, &["step", BRANCH]);
    f.world.complete(&tested(&f, 3));
    sluice(&f, &["step", BRANCH]);
    let (_, ov) = sluice(&f, &["overview", "--json"]);
    let ov: serde_json::Value = serde_json::from_str(&ov).unwrap();
    let c = ov["branches"][0]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["submissions"] == serde_json::json!([2, 3]))
        .unwrap_or_else(|| panic!("{ov:#}"));
    let listed = &c["failed"][0]["flaky"];
    assert_eq!(c["state"], "failed", "{c:#}");
    assert_eq!(listed["here"], "pass", "{c:#}");
    assert_eq!(listed["at"], last_ran(&f)["at"], "{c:#}");
    assert_eq!(listed["host"], "fake-host", "{c:#}");
    assert_eq!(listed["other"]["host"], "fake-host", "{c:#}");
}

/// Time-limit-1 66: a submission whose run its time limit ended names the
/// test the limit cut off, with its log, says nothing was recorded for it
/// and gives the way on; the same head lands once the test ends in time,
/// with nothing under its key to read as flaky.
#[test]
fn a_timed_out_submission_names_the_test_the_limit_ended() {
    let f = setup();
    f.world.hangs("tests:t-batch-cards");
    work(
        &f,
        "unit/batch-review",
        "crw-1-unit-2",
        &[("tests/t_cards.py", Some("slow"))],
    );
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/batch-review"]);
    assert_eq!(code, FAILED, "{out}");
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines.contains(
            &"timed out: tests:t-batch-cards was running when the time limit ended the run"
        ),
        "{out}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("  log of tests:t-batch-cards: ")),
        "{out}"
    );
    assert_eq!(
        lines.last().copied(),
        Some(
            "nothing was recorded for tests:t-batch-cards: if the host was slow, the agent \
             working in unit/batch-review submits the same head again; if tests:t-batch-cards \
             never ends, it fixes it there and submits again"
        ),
        "{out}"
    );
    assert!(!out.contains("not run:"), "{out}");

    f.world.ends_in_time("tests:t-batch-cards");
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/batch-review"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("flaky"), "{out}");
}

/// Could-not-run-1 116: the operator sets aside the fail a host caused;
/// the same head then lands, and asking again for it is refused.
#[test]
fn a_fail_set_aside_lets_the_same_head_land() {
    let f = setup();
    f.world
        .flaky("tests:t-batch-cards", &[Outcome::Fail, Outcome::Pass]);
    work(
        &f,
        "unit/batch-review",
        "crw-1-unit-2",
        &[("tests/t_cards.py", Some("flaky"))],
    );
    let submit = ["submit", BRANCH, "--worktree", "unit/batch-review"];
    assert_eq!(sluice(&f, &submit).0, FAILED);
    assert_eq!(sluice(&f, &submit).0, FAILED, "the key reads flaky");
    let (_, json) = sluice(&f, &["status", "2", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let at = v["failed"][0]["flaky"]["other"]["at"]
        .as_str()
        .unwrap()
        .to_string();

    let aside = [
        "set-aside",
        "tests:t-batch-cards",
        "--at",
        &at,
        "--reason",
        "the engine was down",
    ];
    let (code, out) = sluice(&f, &aside);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("set aside tests:t-batch-cards "), "{out}");
    assert!(
        out.contains(&format!("recorded at {at} on {BRANCH} ")),
        "{out}"
    );
    assert!(
        out.ends_with(
            ": the engine was down\nno gate reads it from now; its line stays in the results\n"
        ),
        "{out}"
    );
    let (code, out) = sluice(&f, &aside);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("already set aside"), "{out}");

    let (code, out) = sluice(&f, &submit);
    assert_eq!(code, LANDED, "{out}");
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
             the agent working in unit/forecast merges or rebases onto {BRANCH} there and \
             submits again\n"
        )
    );
}

/// `--check --head` forecasts a branch no worktree holds, a bolt's, and
/// `--json` says what it was formed from: the head, the branch's revision
/// and the submissions ahead (64, 65, 47a).
#[test]
fn check_forecasts_a_branch_with_no_worktree() {
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
    let tip = f.world.tip(BRANCH);
    let bolt = f.world.work(&tip, &[("plugin/lib/plan.py", Some("two"))]);
    f.world.set_branch("bolt/y", &bolt);

    let (code, out) = sluice(&f, &["submit", BRANCH, "--check", "--head", "bolt/y"]);
    assert_eq!(code, CONFLICT);
    assert_eq!(
        out,
        "would conflict: plugin/lib/plan.py with #1 (crw-1-unit-1, unit/touches)\n"
    );

    let (code, out) = sluice(
        &f,
        &["submit", BRANCH, "--check", "--head", "bolt/y", "--json"],
    );
    assert_eq!(code, CONFLICT);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        v["conflict"]["paths"],
        serde_json::json!(["plugin/lib/plan.py"])
    );
    assert_eq!(v["conflict"]["with"][0]["id"], 1);
    assert_eq!(v["head"], bolt.as_str());
    assert_eq!(v["onto"], tip.as_str());
    assert_eq!(v["ahead"], serde_json::json!([1]));

    let (code, out) = sluice(
        &f,
        &["submit", BRANCH, "--check", "--head", "bolt/none", "--json"],
    );
    assert_eq!(code, CONFLICT);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["error"]["message"], "no branch bolt/none to forecast");
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
         the agent working in unit/a commits or removes them and submits again\n"
    );
}

/// A refusal's way on names the agent working in the worktree under
/// `--json` as in text, and what moves it on is the same command.
#[test]
fn a_refusal_under_json_names_the_agent_and_keeps_its_next() {
    let f = setup();
    work(&f, "unit/a", "agent", &[("plugin/x", Some("x"))]);
    f.world.dirty("unit/a", &["plugin/scratch.py"]);
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--json"]);
    assert_eq!(code, CONFLICT, "{out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        v["error"]["message"],
        "unit/a holds changes beyond its commits:\n  plugin/scratch.py\n\
         the agent working in unit/a commits or removes them and submits again"
    );
    assert_eq!(
        v["error"]["next"],
        serde_json::json!([format!("sluice submit {BRANCH}")])
    );
}

/// ADR 0050: a branch requiring a check the task runner in the worktree
/// keeps out of CI refuses the submission with exit 2, naming the check,
/// the tag that requires it and its line of the policy, and the way on;
/// nothing is queued or published.
#[test]
fn a_submission_whose_gate_requires_a_check_kept_out_of_ci_is_refused_with_exit_two() {
    let f = setup();
    f.world.keeps_out("plugin:test");
    work(&f, "unit/a", "agent", &[("tests/t_cards.py", Some("x"))]);
    f.world.at_path("unit/a", "/work/unit-a");
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, CONFLICT);
    assert_eq!(
        out,
        format!(
            "refused: {BRANCH} requires plugin:test through #gate (.config/sluice.toml line 2), \
             which the task runner in this worktree keeps out of CI, so it is never run and \
             {BRANCH} cannot pass while it is required\n\
             the agent working in unit/a lets it run in CI there, or drops it from {BRANCH}'s \
             policy in the same submission that lets it run, and submits again\n"
        )
    );
    assert!(f.world.published().is_empty());
    let (_, status) = sluice(&f, &["status", BRANCH, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&status).unwrap();
    assert_eq!(v["submissions"], serde_json::json!([]), "{status}");

    let (code, out) = sluice(
        &f,
        &[
            "submit",
            BRANCH,
            "--worktree",
            "unit/a",
            "--no-wait",
            "--json",
        ],
    );
    assert_eq!(code, CONFLICT, "{out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["error"]["kind"], "refused");
    assert!(v["error"]["message"]
        .as_str()
        .unwrap()
        .starts_with(&format!("{BRANCH} requires plugin:test through #gate")));
    assert_eq!(
        v["error"]["next"],
        serde_json::json!([format!("sluice status {BRANCH}")])
    );
}

/// ADR 0050: when the task runner cannot be asked, the submission is
/// taken, with a warning saying why it was not checked; under `--json
/// --no-wait`, the warning is the document's.
#[test]
fn a_submission_the_runner_could_not_check_is_taken_with_a_warning() {
    let f = setup();
    f.world
        .kept_out_fails(Some("[tests] toolchain exited 1: no devenv"));
    work(&f, "unit/a", "agent", &[("tests/t_cards.py", Some("x"))]);
    f.world.at_path("unit/a", "/work/unit-a");
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        out,
        format!(
            "submission 1 to {BRANCH} · nothing queued ahead\n\
             warning: could not check which of {BRANCH}'s required checks the task runner in \
             this worktree keeps out of CI, so the submission was taken unchecked: \
             [tests] toolchain exited 1: no devenv\n"
        )
    );

    work(
        &f,
        "unit/b",
        "agent-b",
        &[("plugin/lib/plan.py", Some("y"))],
    );
    f.world.at_path("unit/b", "/work/unit-b");
    let (code, out) = sluice(
        &f,
        &[
            "submit",
            BRANCH,
            "--worktree",
            "unit/b",
            "--no-wait",
            "--json",
        ],
    );
    assert_eq!(code, 0, "{out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["id"], 2);
    let warnings = v["warnings"].as_array().expect("warnings");
    assert_eq!(warnings.len(), 1, "{out}");
    assert!(warnings[0].as_str().unwrap().contains("no devenv"), "{out}");
}

/// Under `--json`, a refusal is one error document on stdout, naming
/// what moves it on; the exit is a refusal's still.
#[test]
fn a_refusal_under_json_is_an_error_document() {
    let f = setup();
    work(&f, "unit/a", "agent", &[("plugin/x", Some("x"))]);
    f.world.dirty("unit/a", &["plugin/scratch.py"]);
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--json"]);
    assert_eq!(code, CONFLICT, "{out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["error"]["kind"], "refused");
    assert!(v["error"]["message"]
        .as_str()
        .unwrap()
        .starts_with("unit/a holds changes beyond its commits"));
    assert_eq!(
        v["error"]["next"],
        serde_json::json!([format!("sluice submit {BRANCH}")])
    );

    let (code, out) = sluice(
        &f,
        &[
            "submit",
            "bolt/none",
            "--worktree",
            "unit/a",
            "--check",
            "--json",
        ],
    );
    assert_eq!(code, CONFLICT, "{out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["error"]["message"], "no branch bolt/none");
    assert_eq!(
        v["error"]["next"],
        serde_json::json!([
            "sluice branch create bolt/none --from main",
            "sluice overview"
        ])
    );
}

/// What is asked about and does not exist is `not_found` under `--json`.
#[test]
fn nothing_found_under_json_is_an_error_document() {
    let f = setup();
    let (code, out) = sluice(&f, &["status", "9999", "--json"]);
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["error"]["kind"], "not_found");
    assert_eq!(v["error"]["message"], "no submission 9999");
    assert_eq!(v["error"]["next"], serde_json::json!(["sluice overview"]));

    work(&f, "unit/a", "agent", &[("plugin/x", Some("x"))]);
    let (code, out) = sluice(
        &f,
        &[
            "submit",
            BRANCH,
            "--worktree",
            "unit/a",
            "--status",
            "--json",
        ],
    );
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["error"]["kind"], "not_found");

    let (code, out) = exec(&f, &[BRANCH, "--worktree", "nowhere", "--json"]);
    assert_eq!(code, CONFLICT);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["error"]["kind"], "refused");
}

fn home(f: &Fakes, worktree: &str) -> serde_json::Value {
    let (code, out) = sluice(f, &["--worktree", worktree, "--json"]);
    assert_eq!(code, 0, "{out}");
    serde_json::from_str(&out).unwrap()
}

/// With no command: the worktree's own submission, found by the path it
/// was submitted from, the branches' counts and what to run next.
#[test]
fn home_shows_the_worktrees_submission_by_its_path() {
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    f.world.at_path("unit/a", "/w/a");
    let v = home(&f, "unit/a");
    assert_eq!(v["schema"], "sluice/home@1");
    assert_eq!(v["worktree"]["path"], "/w/a");
    assert_eq!(v["submission"], serde_json::Value::Null);

    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let v = home(&f, "unit/a");
    assert_eq!(v["submission"]["id"], 1);
    assert_eq!(v["submission"]["state"], "queued");
    assert_eq!(
        v["branches"],
        serde_json::json!([{"branch": BRANCH, "depth": 1, "testing": 0, "parallel": null}])
    );
    assert_eq!(
        v["next"],
        serde_json::json!(["sluice status 1", "sluice withdraw 1 --as agent-a"])
    );

    // A submission with no path is no worktree's, whatever its branch.
    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/b", "--no-wait"]);
    assert_eq!(home(&f, "unit/b")["submission"], serde_json::Value::Null);
}

/// Once out of the queue, a submission is still the worktree's, found
/// through the branch's recent lines: landed, or failed with its log.
#[test]
fn home_finds_a_landed_or_failed_submission_through_recent() {
    let f = setup();
    f.world
        .fails_with("tests:t-batch-cards", "tests/t_cards.py", "broken");
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    f.world.at_path("unit/a", "/w/a");
    assert_eq!(
        sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]).0,
        LANDED
    );
    let v = home(&f, "unit/a");
    assert_eq!(v["submission"]["state"], "landed");
    assert_eq!(v["submission"]["landed_as"], f.world.tip(BRANCH).0);
    assert_eq!(v["next"][0], format!("sluice submit {BRANCH}"));

    work(
        &f,
        "unit/b",
        "agent-b",
        &[("tests/t_cards.py", Some("broken"))],
    );
    f.world.at_path("unit/b", "/w/b");
    assert_eq!(
        sluice(&f, &["submit", BRANCH, "--worktree", "unit/b"]).0,
        FAILED
    );
    let v = home(&f, "unit/b");
    assert_eq!(v["submission"]["state"], "failed");
    assert_eq!(v["submission"]["failed"][0]["task"], "tests:t-batch-cards");
    assert!(v["submission"]["failed"][0]["log"].is_string());
    assert_eq!(
        v["next"],
        serde_json::json!([format!("sluice submit {BRANCH}")])
    );

    let (code, out) = sluice(&f, &["--worktree", "unit/b"]);
    assert_eq!(code, 0);
    assert!(out.contains(&format!("#2 to {BRANCH} · failed\n")), "{out}");
    assert!(out.contains("  failed tests:t-batch-cards: "), "{out}");
}

/// A pull request's submission names no worktree's path, but the branch it
/// was opened from: the worktree on that branch is shown it (ADR 0025).
#[test]
fn home_shows_a_pull_requests_submission_to_the_worktree_it_was_opened_from() {
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    f.world.at_path("unit/a", "/w/a");
    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    f.world.at_path("unit/b", "/w/b");
    work(&f, "pull/9", "agent-a", &[("plugin/a", Some("a"))]);
    f.world.opened_from("pull/9", "unit/a");
    sluice(&f, &["submit", BRANCH, "--worktree", "pull/9", "--no-wait"]);

    let v = home(&f, "unit/a");
    assert_eq!(v["submission"]["id"], 1);
    assert_eq!(v["submission"]["state"], "queued");
    assert_eq!(v["next"][0], "sluice status 1");
    assert_eq!(home(&f, "unit/b")["submission"], serde_json::Value::Null);
}

/// Outside any worktree sluice can submit from, it says so and who
/// submits, shows the queues and points at the overview.
#[test]
fn home_outside_a_worktree_says_so() {
    let f = setup();
    let v = home(&f, "nowhere");
    assert_eq!(v["worktree"], serde_json::Value::Null);
    assert_eq!(v["next"], serde_json::json!(["sluice overview"]));
    let (_, out) = sluice(&f, &["--worktree", "nowhere"]);
    assert!(
        out.starts_with(
            "not in a worktree sluice can submit from: the agent working in a worktree submits \
             its work from there\n"
        ),
        "{out}"
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

    // By default only the queue, with its count and what is left out.
    let (code, out) = sluice(&f, &["status", BRANCH]);
    assert_eq!(code, 0);
    let tip = f.world.tip(BRANCH);
    let header = format!(
        "{BRANCH} at {} · 1 in the queue · testing 1 at once (up to 4)",
        &tip.0[..7]
    );
    // ADR 0042: the combination under test, with how long it has run.
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], header);
    assert!(
        lines[1].starts_with(&format!("  testing {BRANCH} + #3 · ")),
        "{out}"
    );
    assert_eq!(
        lines[2..],
        [
            format!("  #3  testing   crw-1-unit-3  unit/forecast   {BRANCH} + #3"),
            format!("2 no longer in the queue: sluice status {BRANCH} --all"),
        ]
    );

    let (code, out) = sluice(&f, &["status", BRANCH, "--all"]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], header);
    assert!(lines[1].starts_with(&format!("  testing {BRANCH} + #3 · ")));
    assert_eq!(lines[2], "  #1  landed    crw-1-unit-1  unit/touches");
    assert!(lines[3]
        .starts_with("  #2  failed    crw-1-unit-2  unit/batch-review   tests:t-batch-cards on "));
    assert_eq!(
        lines[4],
        format!("  #3  testing   crw-1-unit-3  unit/forecast       {BRANCH} + #3")
    );

    // `--json` lists the queue, `--all` the history, the landed #1 read
    // from the finished records (131).
    let (_, json) = sluice(&f, &["status", BRANCH, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["submissions"].as_array().unwrap().len(), 1);
    assert_eq!(v["depth"], 1);
    let (_, json) = sluice(&f, &["status", BRANCH, "--json", "--all"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["submissions"].as_array().unwrap().len(), 3);
    assert_eq!(v["submissions"][0]["state"], "landed");
    assert_eq!(v["submissions"][1]["state"], "failed");
    assert!(f.world.files(BRANCH).contains_key(".finished/1.json"));
    assert!(!f.world.files(BRANCH).contains_key("submissions/1.json"));
}

/// `status <branch>` reads the branch at the revision its records last
/// saw, as the overview does, and asks the branch itself nothing: a
/// commit made since is shown by the next round (ADR 0046).
#[test]
fn status_reads_the_revision_the_records_saw() {
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    sluice(&f, &["step", BRANCH]);
    let seen = f.world.tip(BRANCH);
    f.world.hand_commit(BRANCH, &[("plugin/b", Some("b"))]);
    let (_, out) = sluice(&f, &["status", BRANCH, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["at"], seen.0);
}

/// 133, record 0061: while this host's rounds of the branch are refused,
/// status says what could not be read, from where, since when, for how
/// many rounds and what git said, on as many lines as git said it; once a
/// round reads the remote again it says nothing of it. A state read
/// answered from what was last fetched still says so (80).
#[test]
fn status_shows_this_hosts_refusal_while_it_stands() {
    let f = setup();
    sluice(&f, &["step", BRANCH]);
    let said = "remote: Repository not found.\n\
                fatal: repository 'https://github.com/acme/app.git/' not found";
    f.world.refuse_reads(BRANCH, Some(said));
    for _ in 0..3 {
        assert!(f.sluice().step(BRANCH).is_err());
    }
    let kept: serde_json::Value =
        serde_json::from_str(&f.world.refusal(BRANCH).expect("a refusal kept")).unwrap();
    let since = kept["since"].as_str().unwrap();
    let (code, out) = sluice(&f, &["status", BRANCH]);
    assert_eq!(code, 0);
    assert!(
        out.contains(&format!(
            "this host could not read refs/heads/{BRANCH} from origin since {since}, 3 rounds: \
             remote: Repository not found.\n  \
             fatal: repository 'https://github.com/acme/app.git/' not found\n"
        )),
        "{out}"
    );
    let (_, out) = sluice(&f, &["status", BRANCH, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        v["refused"],
        serde_json::json!({
            "remote": "origin",
            "asked": format!("refs/heads/{BRANCH}"),
            "said": said,
            "since": since,
            "rounds": 3,
        })
    );

    f.world.refuse_reads(BRANCH, None);
    f.sluice().step(BRANCH).unwrap();
    let (_, out) = sluice(&f, &["status", BRANCH]);
    assert!(!out.contains("could not read"), "{out}");
    let (_, out) = sluice(&f, &["status", BRANCH, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("refused").is_none(), "{v}");

    f.world
        .refuse_state_reads(BRANCH, Some("error: RPC failed; HTTP 503"));
    let (_, out) = sluice(&f, &["status", BRANCH]);
    assert!(
        out.contains("origin could not be reached: this is the state last fetched from it"),
        "{out}"
    );
    let (_, out) = sluice(&f, &["status", BRANCH, "--json"]);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&out).unwrap()["stale"],
        "origin"
    );
}

/// Clauses 130, 131: a landed submission leaves the live records in the
/// write that tells its outcome, is read by its number, still counts as
/// landed for an `after`, still counts toward its worktree's attempts,
/// and is refused a change as before.
#[test]
fn a_landed_submission_moves_to_the_finished_records() {
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    assert_eq!(
        sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]).0,
        LANDED
    );
    let files = f.world.files(BRANCH);
    assert!(files.contains_key(".finished/1.json"), "{:?}", files.keys());
    assert!(!files.contains_key("submissions/1.json"));
    assert_eq!(
        std::str::from_utf8(&files["finished.jsonl"]).unwrap(),
        "{\"id\":1,\"state\":\"landed\",\"worktree\":\"unit/a\"}\n"
    );
    let (_, out) = sluice(&f, &["status", "1", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["state"], "landed");

    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    assert_eq!(
        sluice(
            &f,
            &["submit", BRANCH, "--worktree", "unit/b", "--after", "1"]
        )
        .0,
        LANDED
    );
    work(&f, "unit/a", "agent-a", &[("plugin/c", Some("c"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let (_, out) = sluice(&f, &["status", "3", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["attempt"], 2);

    assert_eq!(
        sluice(&f, &["withdraw", "1", "--as", "agent-b"]),
        (
            2,
            "refused: #1 is not agent-b's to withdraw: ask --as agent-a, its agent, \
             or --as operator\n"
                .into()
        )
    );
    assert_eq!(
        sluice(&f, &["amend", "1", "--priority", "2"]),
        (
            2,
            "refused: #1 is landed, past changing: sluice status 1\n".into()
        )
    );
    let (_, out) = sluice(&f, &["status", BRANCH]);
    assert!(
        out.contains(&format!(
            "2 no longer in the queue: sluice status {BRANCH} --all"
        )),
        "{out}"
    );
}

/// A branch that does not exist and has no records is said to be none,
/// with the branches that have records, and exits 1.
#[test]
fn status_of_an_unknown_branch_says_so() {
    let f = setup();
    let (code, out) = sluice(&f, &["status", "nosuch"]);
    assert_eq!(code, FAILED);
    assert_eq!(out, "no branch nosuch, and no branch has records here\n");

    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let (code, out) = sluice(&f, &["status", "nosuch", "--json"]);
    assert_eq!(code, FAILED);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["error"]["kind"], "not_found");
    assert_eq!(
        v["error"]["message"],
        format!("no branch nosuch; branches with records: {BRANCH}")
    );
    assert_eq!(
        v["error"]["next"],
        serde_json::json!([format!("sluice status {BRANCH}"), "sluice overview"])
    );

    let (code, out) = sluice(&f, &["status", BRANCH]);
    assert_eq!(code, 0);
    assert!(!out.contains("no longer in the queue"), "{out}");
}

/// 108: `cache` in `trust` is read as absent, and `sluice status` says so.
#[test]
fn status_warns_of_cache_in_trust() {
    let f = Fakes::new();
    f.world.init(
        BRANCH,
        &[
            (
                ".config/sluice.toml",
                &POLICY.replace("\"container\"]", "\"container\", \"cache\"]"),
            ),
            ("plugin/lib/plan.py", "plan"),
        ],
    );
    let (code, out) = sluice(&f, &["status", BRANCH]);
    assert_eq!(code, 0);
    assert!(
        out.contains(
            "warning: [branches.\"bolt/*\"] trust lists \"cache\", which no longer counts"
        ),
        "{out}"
    );
    let (_, out) = sluice(&f, &["status", BRANCH, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["warnings"].as_array().unwrap().len(), 1);
    let (_, out) = sluice(&setup(), &["status", BRANCH, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["warnings"], serde_json::json!([]));
}

/// 35, 71; ADR 0049: a policy that still declares a checkout cache is
/// refused when `sluice status` reads it, an error the binary prints and
/// exits non-zero on, naming what replaced it.
#[test]
fn status_refuses_a_checkout_cache() {
    let f = Fakes::new();
    f.world.init(
        BRANCH,
        &[
            (
                ".config/sluice.toml",
                &format!(
                    "{POLICY}\n[[checkout.cache]]\nkey = [\"Cargo.lock\"]\npaths = [\"target\"]\n"
                ),
            ),
            ("plugin/lib/plan.py", "plan"),
        ],
    );
    let cli = Cli::try_parse_from(["sluice", "status", BRANCH]).unwrap();
    let mut out = Vec::new();
    let host = support::FakeHost {
        world: f.world.clone(),
        reserved: 0,
    };
    let e = run(
        cli,
        f.seams(),
        &host,
        &(),
        &(),
        &f.clock,
        &Rounds(Cell::new(0)),
        &mut out,
    )
    .unwrap_err();
    let e = format!("{e:#}");
    assert!(
        e.contains("[checkout] declares a cache")
            && e.contains("ADR 0049")
            && e.contains("[[caches]]"),
        "{e}"
    );
}

/// A landed #1, a blamed #2 and #3 under test, as `status_lists_the_queue`
/// leaves them.
fn three_submissions(f: &Fakes) {
    f.world
        .fails_with("tests:t-batch-cards", "tests/t_cards.py", "broken");
    work(
        f,
        "unit/touches",
        "crw-1-unit-1",
        &[("plugin/lib/plan.py", Some("touched"))],
    );
    work(
        f,
        "unit/batch-review",
        "crw-1-unit-2",
        &[("tests/t_cards.py", Some("broken"))],
    );
    sluice(
        f,
        &["submit", BRANCH, "--worktree", "unit/touches", "--no-wait"],
    );
    sluice(
        f,
        &[
            "submit",
            BRANCH,
            "--worktree",
            "unit/batch-review",
            "--no-wait",
        ],
    );
    sluice(f, &["run", "--rounds", "6"]);
    f.world.hold_runs(true);
    work(
        f,
        "unit/forecast",
        "crw-1-unit-3",
        &[("plugin/other.py", Some("x"))],
    );
    sluice(
        f,
        &["submit", BRANCH, "--worktree", "unit/forecast", "--no-wait"],
    );
    sluice(f, &["step", BRANCH]);
}

#[test]
fn overview_shows_every_branch_at_a_glance() {
    let f = setup();
    three_submissions(&f);

    let (code, json) = sluice(&f, &["overview", "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["schema"], "sluice/overview@1");
    assert_eq!(v["host"], "fake-host");
    assert_eq!(v["repository"], "/fake/repo.git");
    assert!(v["at"].is_string());
    assert_eq!(v["sessions"], serde_json::json!([]));
    assert_eq!(v["previews"], serde_json::json!([]));
    // ADR 0035: the one combination under test counts against the bound.
    assert_eq!(v["room"]["queue"], 1);
    assert_eq!(v["room"]["sessions"], 0);
    let branches = v["branches"].as_array().unwrap();
    assert_eq!(branches.len(), 1);
    let b = &branches[0];
    assert_eq!(b["branch"], BRANCH);
    assert_eq!(b["at"], f.world.tip(BRANCH).0);
    assert_eq!(b["parallel"], 4);
    // C.4, C.7: a branch whose policy names no remote names none.
    assert!(b.get("remote").is_none(), "{json}");
    assert_eq!(b["depth"], 1);
    assert!(b["version"].as_u64().unwrap() > 0);
    // Only the submission still in play, as `status <id> --json` has it.
    let subs = b["submissions"].as_array().unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0]["id"], 3);
    assert_eq!(subs[0]["state"], "testing");
    assert_eq!(subs[0]["from"]["worktree"], "unit/forecast");
    let testing: Vec<&serde_json::Value> = b["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["state"] == "testing")
        .collect();
    assert_eq!(testing.len(), 1);
    assert_eq!(testing[0]["submissions"], serde_json::json!([3]));
    assert_eq!(testing[0]["targets"], serde_json::json!(["#gate"]));
    assert!(testing[0]["started"].is_string());
    // The log's own lines: #1 landed, #2 blamed.
    let recent = b["recent"].as_array().unwrap();
    let events: Vec<&str> = recent
        .iter()
        .map(|r| r["event"].as_str().unwrap())
        .collect();
    assert!(events.contains(&"landed"));
    assert!(events.contains(&"blamed"));
    let blamed = recent.iter().find(|r| r["event"] == "blamed").unwrap();
    assert_eq!(blamed["id"], 2);
    assert_eq!(blamed["schema"], "sluice/log@1");

    let (code, text) = sluice(&f, &["overview"]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = text.lines().collect();
    let tip = f.world.tip(BRANCH);
    assert_eq!(
        lines[0],
        format!(
            "{BRANCH} at {} · 1 in the queue · testing 1 at once (up to 4)",
            &tip.0[..7]
        )
    );
    assert!(lines[1].starts_with(&format!("  testing {BRANCH} + #3 · ")));
}

/// 26, 49, 61: the overview names the worktree each submission its recent
/// lines name came from, the landed #1 read from the finished records.
#[test]
fn overview_names_where_recent_submissions_came_from() {
    let f = setup();
    three_submissions(&f);
    assert!(f.world.files(BRANCH).contains_key(".finished/1.json"));

    let (_, json) = sluice(&f, &["overview", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let b = &v["branches"][0];
    let mut named: Vec<u64> = b["recent"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| {
            r["ids"]
                .as_array()
                .cloned()
                .unwrap_or_else(|| vec![r["id"].clone()])
        })
        .map(|id| id.as_u64().unwrap())
        .collect();
    named.sort_unstable();
    assert_eq!(named, [1, 2], "{json}");
    assert_eq!(
        b["recent_from"],
        serde_json::json!({"1": "unit/touches", "2": "unit/batch-review"}),
        "{json}"
    );
}

/// ADR 0028: a combination under test carries its run's tasks, as a
/// round kept them, and its text line says how many have ended.
#[test]
fn overview_shows_a_combinations_tasks() {
    use sluice_seams::{Rev, TaskProgress, TaskState};
    let f = setup();
    three_submissions(&f);
    let (_, json) = sluice(&f, &["overview", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let c = &v["branches"][0]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["state"] == "testing")
        .unwrap()
        .clone();
    assert!(c.get("tasks").is_none());
    let task = |target: &str, state: TaskState, exit: Option<i32>| TaskProgress {
        target: target.into(),
        state,
        started: exit.map(|_| "2026-10-18T12:00:00Z".into()),
        ended: exit.map(|_| "2026-10-18T12:01:48Z".into()),
        exit,
    };
    f.world.says_tasks(
        &Rev::new(c["revision"].as_str().unwrap()),
        &[
            task("tests:t-cards", TaskState::Passed, Some(0)),
            task("tests:t-batch-cards", TaskState::Running, None),
            task("plugin:test", TaskState::Waiting, None),
        ],
    );
    sluice(&f, &["step", BRANCH]);

    let (_, json) = sluice(&f, &["overview", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let c = v["branches"][0]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["state"] == "testing")
        .unwrap()
        .clone();
    assert_eq!(
        c["tasks"],
        serde_json::json!([
            {"target": "tests:t-cards", "state": "passed",
             "started": "2026-10-18T12:00:00Z", "ended": "2026-10-18T12:01:48Z", "exit": 0},
            {"target": "tests:t-batch-cards", "state": "running"},
            {"target": "plugin:test", "state": "waiting"},
        ])
    );
    let (_, text) = sluice(&f, &["overview"]);
    assert!(
        text.lines().any(|l| l.starts_with(&format!(
            "  testing {BRANCH} + #3 · 1/3 tasks · running tests:t-batch-cards · "
        ))),
        "{text}"
    );
}

/// ADR 0042: `status` says how far a combination under test has got, a
/// line under the branch's counts, the rows as they were.
#[test]
fn status_shows_a_combinations_progress() {
    use sluice_core::Clock;
    use sluice_seams::{Rev, TaskProgress, TaskState};
    let f = setup();
    three_submissions(&f);
    let (_, before) = sluice(&f, &["status", BRANCH]);
    assert!(
        before
            .lines()
            .any(|l| l == format!("  testing {BRANCH} + #3").as_str()
                || l.starts_with(&format!("  testing {BRANCH} + #3 · "))),
        "{before}"
    );
    let (_, json) = sluice(&f, &["status", "3", "--json"]);
    let rev = Rev::new(
        serde_json::from_str::<serde_json::Value>(&json).unwrap()["tested"]["combination"]
            .as_str()
            .unwrap(),
    );
    let started = (f.clock.now() - chrono::Duration::minutes(4))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let task = |target: &str, state: TaskState, started: Option<&str>| TaskProgress {
        target: target.into(),
        state,
        started: started.map(String::from),
        ended: None,
        exit: (state == TaskState::Passed).then_some(0),
    };
    f.world.says_tasks(
        &rev,
        &[
            task("tests:t-cards", TaskState::Passed, None),
            task("tests:t-batch-cards", TaskState::Running, Some(&started)),
            task("plugin:test", TaskState::Waiting, None),
        ],
    );
    sluice(&f, &["step", BRANCH]);

    let head = format!("  testing {BRANCH} + #3 · 1/3 tasks · running tests:t-batch-cards 4m");
    let (_, text) = sluice(&f, &["status", BRANCH]);
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.starts_with(&head))
        .unwrap_or_else(|| panic!("{text}"));
    assert!(lines[at].contains(" · "), "{text}");
    assert!(
        lines[at + 1..]
            .iter()
            .any(|l| l.starts_with("  #3  testing ") && l.ends_with(&format!("{BRANCH} + #3"))),
        "{text}"
    );
    let (_, text) = sluice(&f, &["status", "3"]);
    assert!(text.lines().any(|l| l.starts_with(&head)), "{text}");
    // A submission in no combination under test has no line.
    let (_, text) = sluice(&f, &["status", "1"]);
    assert!(!text.contains("  testing "), "{text}");
}

/// ADR 0036: the branches in the order the default branch's policy writes
/// their tables, read at its own revision when it has no records; every
/// other branch is still gated by its own policy (rule core.15).
#[test]
fn overview_orders_branches_as_the_policy_names_them() {
    let f = Fakes::new();
    // The repository's HEAD names release before bolt/*; the bolt branches'
    // own policies name them the other way round.
    f.world.init(
        "main",
        &[(
            ".config/sluice.toml",
            "[branches.release]\nrequired = []\n[branches.\"bolt/*\"]\nrequired = []\n",
        )],
    );
    let theirs = "[branches.\"bolt/*\"]\nrequired = []\n[branches.release]\nrequired = []\n";
    for b in ["bolt/b", "release", "bolt/a"] {
        f.world.init(b, &[(".config/sluice.toml", theirs)]);
        let head = f.world.work(&f.world.tip(b), &[("work", Some(b))]);
        let worktree = format!("w/{b}");
        f.world.session(&worktree, "agent", &head);
        let (code, _) = sluice(&f, &["submit", b, "--worktree", &worktree, "--no-wait"]);
        assert_eq!(code, 0, "submitting to {b}");
    }

    let (_, json) = sluice(&f, &["overview", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let order: Vec<&str> = v["branches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["branch"].as_str().unwrap())
        .collect();
    assert_eq!(order, ["release", "bolt/a", "bolt/b"]);
    let (_, text) = sluice(&f, &["overview"]);
    let heads: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with(' '))
        .map(|l| l.split(' ').next().unwrap())
        .collect();
    assert_eq!(heads, ["release", "bolt/a", "bolt/b"]);
}

#[test]
fn overview_of_a_repository_with_no_records_says_so() {
    let f = setup();
    let (code, text) = sluice(&f, &["overview"]);
    assert_eq!(code, 0);
    assert_eq!(text, "no branch has records here\n");
    let (_, json) = sluice(&f, &["overview", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["branches"], serde_json::json!([]));
}

/// A pause that does `between(n)` before the `n`th wait, and stops after
/// `rounds` of them.
struct Between<'a> {
    n: Cell<usize>,
    rounds: usize,
    between: &'a dyn Fn(usize),
}

impl Pause for Between<'_> {
    fn pause(&self) -> bool {
        let n = self.n.get() + 1;
        self.n.set(n);
        if n > self.rounds {
            return false;
        }
        (self.between)(n);
        true
    }
}

#[test]
fn overview_watch_prints_a_line_at_start_and_on_each_change() {
    let f = setup();
    work(
        &f,
        "unit/touches",
        "crw-1-unit-1",
        &[("plugin/lib/plan.py", Some("touched"))],
    );
    let between = |n: usize| {
        if n == 2 {
            sluice(
                &f,
                &["submit", BRANCH, "--worktree", "unit/touches", "--no-wait"],
            );
        }
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 4,
        between: &between,
    };
    let cli = Cli::try_parse_from(["sluice", "overview", "--json", "--watch"]).unwrap();
    let mut out = Vec::new();
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
        &pause,
        &mut out,
    )
    .unwrap();
    assert_eq!(code, sluice::STOPPED);
    let out = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    // At the start, then once for the submission; the fake clock moves on
    // at every read, and the time alone prints nothing.
    assert_eq!(lines.len(), 2, "{out}");
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(first["branches"], serde_json::json!([]));
    assert_eq!(second["branches"][0]["depth"], 1);
}

#[test]
fn watch_needs_json() {
    assert!(Cli::try_parse_from(["sluice", "overview", "--watch"]).is_err());
}

// ---------------------------------------------------------------------------
// One reader of the repository's queues on the host (129; ADR 0056)
// ---------------------------------------------------------------------------

/// The repository the fakes' host names.
const REPOSITORY: &str = "/fake/repo.git";

/// One of `repository`'s reader's files on the fakes' host.
fn readers_file(repository: &str, ext: &str) -> std::path::PathBuf {
    let name = sluice::runs::flat_name(repository);
    support::readers().join(format!("{name}.{ext}"))
}

/// `repository`'s reader's document, as it stands.
fn readers_doc_of(repository: &str) -> Option<serde_json::Value> {
    let bytes = std::fs::read(readers_file(repository, "json")).ok()?;
    Some(serde_json::from_slice(&bytes).unwrap())
}

fn readers_doc() -> Option<serde_json::Value> {
    readers_doc_of(REPOSITORY)
}

/// A live reader of `repository`, the test's own, until dropped; its
/// document `doc`.
fn reader_of(repository: &str, doc: &serde_json::Value) -> sluice::reader::Lead {
    match sluice::reader::Reader::take(&support::readers(), repository).unwrap() {
        sluice::reader::Reader::Lead(mut lead) => {
            lead.write(doc).unwrap();
            lead
        }
        sluice::reader::Reader::Follow(_) => panic!("a reader of {repository} already"),
    }
}

/// Replace the reader's document with `doc`, as it is, time and all.
fn rewrite_readers_doc(doc: &serde_json::Value) {
    std::fs::write(readers_file(REPOSITORY, "json"), doc.to_string()).unwrap();
}

/// The overview as a reader read it of queues no test's records hold:
/// `#7` under test on the branch.
fn seven_testing() -> serde_json::Value {
    let g = setup();
    g.world.hold_runs(true);
    work(&g, "unit/r", "agent-r", &[("plugin/r", Some("r"))]);
    sluice(&g, &["submit", BRANCH, "--worktree", "unit/r", "--no-wait"]);
    sluice(&g, &["step", BRANCH]);
    let (_, out) = sluice(&g, &["overview", "--json"]);
    let mut doc = json(&out);
    let b = &mut doc["branches"][0];
    assert_eq!(b["submissions"][0]["state"], "testing", "{b}");
    b["submissions"][0]["id"] = 7.into();
    b["submissions"][0]["tested"]["submissions"] = serde_json::json!([7]);
    b["candidates"][0]["submissions"] = serde_json::json!([7]);
    doc
}

/// 129: a watch that finds no reader becomes it, and writes each document
/// it prints, whole: at its start, and once the submission is queued.
#[test]
fn watch_with_no_reader_writes_each_document() {
    let f = setup();
    work(
        &f,
        "unit/touches",
        "crw-1-unit-1",
        &[("plugin/lib/plan.py", Some("touched"))],
    );
    let written = RefCell::new(Vec::new());
    let between = |n: usize| {
        written.borrow_mut().push(readers_doc());
        if n == 2 {
            sluice(
                &f,
                &["submit", BRANCH, "--worktree", "unit/touches", "--no-wait"],
            );
        }
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 4,
        between: &between,
    };
    let (code, out) = sluice_with(&f, &pause, &["overview", "--json", "--watch"]);
    assert_eq!(code, sluice::STOPPED);
    let lines: Vec<serde_json::Value> = out.lines().map(json).collect();
    assert_eq!(lines.len(), 2, "{out}");
    let written = written.into_inner();
    assert_eq!(written[0].as_ref(), Some(&lines[0]));
    assert_eq!(written[2].as_ref(), Some(&lines[1]));
    assert_eq!(lines[1]["schema"], "sluice/overview@1");
    assert_eq!(lines[1]["branches"][0]["depth"], 1);
}

/// 129: a watch that finds a live reader follows its document: what it
/// prints is what the reader wrote, which no records here hold.
#[test]
fn watch_follows_the_readers_document() {
    let f = setup();
    let doc = seven_testing();
    let _reader = reader_of(REPOSITORY, &doc);
    let pause = Between {
        n: Cell::new(0),
        rounds: 2,
        between: &|_| {},
    };
    let (code, out) = sluice_with(&f, &pause, &["overview", "--json", "--watch"]);
    assert_eq!(code, sluice::STOPPED);
    let lines: Vec<serde_json::Value> = out.lines().map(json).collect();
    assert_eq!(lines, [doc], "{out}");
}

/// 129: a follower prints the reader's document again each time it
/// changes, and never for its time alone.
#[test]
fn watch_follows_each_change_once() {
    let f = setup();
    let testing = seven_testing();
    let mut landed = testing.clone();
    let b = &mut landed["branches"][0];
    b["depth"] = 0.into();
    b["submissions"] = serde_json::json!([]);
    b["candidates"] = serde_json::json!([]);
    b["recent"] = serde_json::json!([{
        "at": "2026-10-11T09:05:00Z", "event": "landed", "ids": [7], "to": "f00d",
    }]);
    let mut later = landed.clone();
    later["at"] = "2026-10-11T09:10:00Z".into();
    let _reader = reader_of(REPOSITORY, &testing);
    let between = |n: usize| match n {
        1 => rewrite_readers_doc(&landed),
        2 => rewrite_readers_doc(&later),
        _ => {}
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 3,
        between: &between,
    };
    let (_, out) = sluice_with(&f, &pause, &["overview", "--json", "--watch"]);
    let lines: Vec<serde_json::Value> = out.lines().map(json).collect();
    assert_eq!(lines, [testing, landed], "{out}");
}

/// 129: once the reader stops, a follower is the reader within one pause,
/// and the document it writes is the one it read from the records.
#[test]
fn watch_leads_once_the_reader_stops() {
    let f = setup();
    let doc = seven_testing();
    let reader = RefCell::new(Some(reader_of(REPOSITORY, &doc)));
    let written = RefCell::new(None);
    let between = |n: usize| match n {
        1 => drop(reader.borrow_mut().take()),
        2 => *written.borrow_mut() = readers_doc(),
        _ => {}
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 3,
        between: &between,
    };
    let (_, out) = sluice_with(&f, &pause, &["overview", "--json", "--watch"]);
    let lines: Vec<serde_json::Value> = out.lines().map(json).collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert_eq!(lines[0], doc);
    assert_eq!(lines[1]["branches"], serde_json::json!([]));
    assert_eq!(written.into_inner().as_ref(), Some(&lines[1]));
}

/// 129: a reader serves its own repository alone: a watch of another
/// repository on the host is that one's reader.
#[test]
fn watch_of_another_repository_leads() {
    let f = setup();
    let other = "/fake/other.git";
    let doc = seven_testing();
    let _reader = reader_of(other, &doc);
    let pause = Between {
        n: Cell::new(0),
        rounds: 1,
        between: &|_| {},
    };
    let (_, out) = sluice_with(&f, &pause, &["overview", "--json", "--watch"]);
    let lines: Vec<serde_json::Value> = out.lines().map(json).collect();
    assert_eq!(lines.len(), 1, "{out}");
    assert_eq!(lines[0]["branches"], serde_json::json!([]));
    assert_eq!(readers_doc().as_ref(), Some(&lines[0]));
    assert_eq!(readers_doc_of(other), Some(doc));
}

/// 129: a one-shot overview reads the records itself, beside a live
/// reader or with none, and never takes the reader's place.
#[test]
fn one_shot_overview_reads_the_records_beside_a_reader() {
    let f = setup();
    sluice(&f, &["overview", "--json"]);
    assert!(!readers_file(REPOSITORY, "lock").exists());
    let doc = seven_testing();
    let _reader = reader_of(REPOSITORY, &doc);
    let (code, out) = sluice(&f, &["overview", "--json"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(json(&out)["branches"], serde_json::json!([]));
    let (_, text) = sluice(&f, &["overview"]);
    assert_eq!(text, "no branch has records here\n");
    assert_eq!(readers_doc(), Some(doc));
}

/// The overview as a reader read it of queues no test's records hold:
/// `#1` under test on the branch from a head of its own, `done` of its
/// five tasks ended.
fn one_testing(done: usize) -> serde_json::Value {
    use sluice_seams::{TaskProgress, TaskState};
    let g = setup();
    g.world.hold_runs(true);
    work(&g, "unit/r", "agent-r", &[("plugin/r", Some("r"))]);
    sluice(&g, &["submit", BRANCH, "--worktree", "unit/r", "--no-wait"]);
    sluice(&g, &["step", BRANCH]);
    let tasks: Vec<TaskProgress> = (0..5)
        .map(|i| {
            let state = match i {
                i if i < done => TaskState::Passed,
                i if i == done => TaskState::Running,
                _ => TaskState::Waiting,
            };
            TaskProgress {
                target: format!("t{i}:test"),
                state,
                started: None,
                ended: None,
                exit: state.ended().then_some(0),
            }
        })
        .collect();
    g.world.says_tasks(&tested(&g, 1), &tasks);
    // A round keeps what the runner says beside the records.
    sluice(&g, &["step", BRANCH]);
    let (_, out) = sluice(&g, &["overview", "--json"]);
    json(&out)
}

/// 129: a wait that finds no reader becomes it, and writes the whole
/// overview, every branch's, not only its own.
#[test]
fn wait_with_no_reader_writes_the_whole_overview() {
    let f = setup();
    let other = "bolt/other";
    f.world.init(other, &[(".config/sluice.toml", POLICY)]);
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    let head = f
        .world
        .work(&f.world.tip(other), &[("plugin/b", Some("b"))]);
    f.world.session("unit/b", "agent-b", &head);
    f.world.hold_runs(true);
    sluice(&f, &["submit", other, "--worktree", "unit/b", "--no-wait"]);
    f.world.hold_runs(false);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let (code, out) = sluice(&f, &["wait", "2"]);
    assert_eq!(code, LANDED, "{out}");
    let doc = readers_doc().expect("the reader's document");
    assert_eq!(doc["schema"], "sluice/overview@1");
    let mut branches: Vec<&str> = doc["branches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["branch"].as_str().unwrap())
        .collect();
    branches.sort();
    assert_eq!(branches, [BRANCH, other], "{doc}");
}

/// 129: a wait that finds a live reader says its combination's progress
/// from the reader's document, which no records here hold.
#[test]
fn wait_follows_the_readers_progress() {
    let f = setup();
    f.world.hold_runs(true);
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let _reader = reader_of(REPOSITORY, &one_testing(3));
    let pause = Between {
        n: Cell::new(0),
        rounds: 2,
        between: &|_| {},
    };
    let (code, out) = sluice_with(&f, &pause, &["wait", "1"]);
    assert_eq!(code, sluice::STOPPED, "{out}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert!(
        lines[0].starts_with(&format!(
            "testing {BRANCH} + #1 · 3/5 tasks · running t3:test"
        )),
        "{out}"
    );
    assert_eq!(lines[1], "still waiting: sluice wait 1");
}

/// 129: out of time, a follower's `--json` is its record as the reader's
/// document holds it.
#[test]
fn wait_out_of_time_prints_the_readers_record() {
    let f = setup();
    f.world.hold_runs(true);
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let doc = one_testing(3);
    let _reader = reader_of(REPOSITORY, &doc);
    let (code, out) = sluice(&f, &["wait", "1", "--timeout", "2s", "--json"]);
    assert_eq!(code, sluice::WAITING, "{out}");
    let held = &doc["branches"][0]["submissions"][0];
    assert_eq!(json(&out), *held);
    assert_eq!(held["state"], "testing");
    let (_, out) = sluice(&f, &["status", "1", "--json"]);
    assert_ne!(json(&out)["head"], held["head"]);
}

/// 129: once the reader's document no longer holds a follower's
/// submission, the wait reads its record once and says how it ended.
#[test]
fn followed_wait_reports_its_landing() {
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let (_, queued) = sluice(&f, &["overview", "--json"]);
    let reader = RefCell::new(reader_of(REPOSITORY, &json(&queued)));
    let between = |n: usize| {
        if n == 3 {
            let (_, now) = sluice(&f, &["overview", "--json"]);
            let now = json(&now);
            assert_eq!(now["branches"][0]["submissions"], serde_json::json!([]));
            reader.borrow_mut().write(&now).unwrap();
        }
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 10,
        between: &between,
    };
    let (code, out) = sluice_with(&f, &pause, &["wait", "1"]);
    assert_eq!(code, LANDED, "{out}");
    // It waited on the document, whatever its own rounds had done.
    assert_eq!(pause.n.get(), 3, "{out}");
    let tip = f.world.tip(BRANCH);
    assert!(
        out.ends_with(&format!("passed · landed as {}\n", &tip.0[..7])),
        "{out}"
    );
}

/// 129: a follower's submission withdrawn ends its wait as a wait that
/// read the records would.
#[test]
fn followed_wait_reports_its_withdrawal() {
    let f = setup();
    f.world.hold_runs(true);
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let (_, queued) = sluice(&f, &["overview", "--json"]);
    let reader = RefCell::new(reader_of(REPOSITORY, &json(&queued)));
    let between = |n: usize| match n {
        1 => {
            sluice(&f, &["withdraw", "1"]);
        }
        2 => {
            let (_, now) = sluice(&f, &["overview", "--json"]);
            reader.borrow_mut().write(&json(&now)).unwrap();
        }
        _ => {}
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 10,
        between: &between,
    };
    let (code, out) = sluice_with(&f, &pause, &["wait", "1", "--json"]);
    assert_eq!(code, sluice::WITHDRAWN, "{out}");
    assert_eq!(json(&out)["state"], "withdrawn");
    assert_eq!(pause.n.get(), 2, "{out}");
}

/// 129: when the wait that is the reader ends, a watch that followed it
/// is the reader within one pause, and printed nothing before but the
/// documents the wait wrote.
#[test]
fn watch_leads_once_the_readers_wait_ends() {
    use std::sync::mpsc;

    /// A pause that says it has paused and waits for the next tick; none
    /// to come stops it.
    struct Lockstep(mpsc::Sender<()>, mpsc::Receiver<()>);

    impl Pause for Lockstep {
        fn pause(&self) -> bool {
            let _ = self.0.send(());
            self.1.recv().is_ok()
        }
    }

    let readers = support::readers();
    let (tick, ticks) = mpsc::channel();
    let (paused, pauses) = mpsc::channel();
    // Another process on the host, with fakes of its own: it reads them
    // only once it is the reader.
    let watch = std::thread::spawn(move || {
        support::share_readers(readers);
        ticks.recv().unwrap();
        let g = setup();
        sluice_with(
            &g,
            &Lockstep(paused, ticks),
            &["overview", "--json", "--watch"],
        )
        .1
    });

    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let wrote = RefCell::new(Vec::new());
    let step = Then(Cell::new(0), |_| {
        wrote
            .borrow_mut()
            .push(readers_doc().expect("the wait's document"));
        tick.send(()).unwrap();
        pauses.recv().unwrap();
        true
    });
    let (code, out) = sluice_with(&f, &step, &["wait", "1"]);
    assert_eq!(code, LANDED, "{out}");
    // One tick once the wait has ended, then none.
    tick.send(()).unwrap();
    pauses.recv().unwrap();
    drop(tick);
    let out = watch.join().unwrap();
    let lines: Vec<serde_json::Value> = out.lines().map(json).collect();
    let (led, followed) = lines.split_last().expect("a line");
    assert!(!followed.is_empty(), "{out}");
    let wrote = wrote.into_inner();
    for l in followed {
        assert!(wrote.contains(l), "not the wait's: {l}");
    }
    assert_eq!(led["branches"], serde_json::json!([]), "{out}");
    assert_eq!(readers_doc().as_ref(), Some(led));
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
        (
            2,
            "refused: #2 is under test; its order is fixed: sluice status 2 follows it, \
             or sluice withdraw 2 takes it out\n"
                .into()
        )
    );
    assert_eq!(
        sluice(&f, &["withdraw", "1", "--as", "agent-b"]),
        (
            2,
            "refused: #1 is not agent-b's to withdraw: ask --as agent-a, its agent, \
             or --as operator\n"
                .into()
        )
    );
    assert_eq!(
        sluice(&f, &["withdraw", "1", "--as", "agent-a"]),
        (0, "withdrew #1\n".into())
    );
    // 5, 48: it left the queue by withdrawal, so asking again is no
    // change; still not another's to ask.
    assert_eq!(
        sluice(&f, &["withdraw", "1", "--as", "agent-a"]),
        (0, "#1 was already withdrawn\n".into())
    );
    assert_eq!(sluice(&f, &["withdraw", "1", "--as", "agent-b"]).0, 2);
    assert_eq!(
        sluice(&f, &["amend", "1", "--priority", "2"]),
        (
            2,
            "refused: #1 is withdrawn, past changing: sluice status 1\n".into()
        )
    );
    assert_eq!(
        sluice(&f, &["withdraw", "99"]),
        (
            2,
            "refused: no submission 99: sluice overview lists the queues\n".into()
        )
    );
    work(&f, "unit/c", "agent-c", &[("plugin/c", Some("c"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/c", "--no-wait"]);
    assert_eq!(
        sluice(&f, &["amend", "3", "--after", "7"]).1,
        format!("refused: --after names no queued submission: #7; sluice status {BRANCH} lists the queue\n")
    );
}

fn json(out: &str) -> serde_json::Value {
    serde_json::from_str(out).unwrap_or_else(|e| panic!("{e}: {out}"))
}

/// `--json` on amend and withdraw: one document each, and a refusal is
/// the error document with the commands its text names.
#[test]
fn amend_and_withdraw_under_json() {
    let f = setup();
    f.world.hold_runs(true);
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/b", "--no-wait"]);
    let (code, out) = sluice(
        &f,
        &["amend", "2", "--priority", "3", "--after", "1", "--json"],
    );
    assert_eq!(code, 0);
    assert_eq!(
        json(&out),
        serde_json::json!({"id": 2, "after": [1], "priority": 3})
    );

    let (code, out) = sluice(
        &f,
        &["amend", "1", "--priority", "1", "--as", "agent-b", "--json"],
    );
    assert_eq!(code, 2);
    assert_eq!(
        json(&out),
        serde_json::json!({"error": {
            "kind": "refused",
            "message": "#1 is not agent-b's to change: ask --as agent-a, its agent, or --as operator",
            "next": ["sluice amend 1 --as agent-a", "sluice amend 1 --as operator"],
        }})
    );
    let (code, out) = sluice(&f, &["amend", "99", "--json"]);
    assert_eq!(code, 2);
    assert_eq!(
        json(&out)["error"]["next"],
        serde_json::json!(["sluice overview"])
    );

    sluice(&f, &["step", BRANCH]);
    let (code, out) = sluice(&f, &["amend", "1", "--priority", "1", "--json"]);
    assert_eq!(code, 2);
    assert_eq!(
        json(&out)["error"]["next"],
        serde_json::json!(["sluice status 1", "sluice withdraw 1"])
    );

    let (code, out) = sluice(&f, &["withdraw", "1", "--as", "agent-b", "--json"]);
    assert_eq!(code, 2);
    assert_eq!(
        json(&out)["error"]["next"],
        serde_json::json!([
            "sluice withdraw 1 --as agent-a",
            "sluice withdraw 1 --as operator"
        ])
    );
    let (code, out) = sluice(&f, &["withdraw", "1", "--as", "agent-a", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(
        json(&out),
        serde_json::json!({"id": 1, "state": "withdrawn"})
    );
    let (code, out) = sluice(&f, &["withdraw", "1", "--as", "agent-a", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(
        json(&out),
        serde_json::json!({"id": 1, "state": "withdrawn", "already": true})
    );
    // Once a round has moved it to the finished records, the same.
    sluice(&f, &["step", BRANCH]);
    let (code, out) = sluice(&f, &["withdraw", "1", "--as", "agent-a", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(json(&out)["already"], true);
    let (code, out) = sluice(&f, &["withdraw", "1", "--as", "agent-b", "--json"]);
    assert_eq!(code, 2);
    assert_eq!(json(&out)["error"]["kind"], "refused");
    let (code, out) = sluice(&f, &["withdraw", "99", "--json"]);
    assert_eq!(code, 2);
    assert_eq!(
        json(&out)["error"]["message"],
        "no submission 99: sluice overview lists the queues"
    );
}

/// 5: a landed submission left the queue by landing, so it is never
/// withdrawn, however often it is asked.
#[test]
fn a_landed_submission_is_not_withdrawn() {
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    assert_eq!(
        sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]).0,
        LANDED
    );
    assert_eq!(
        sluice(&f, &["withdraw", "1"]),
        (
            2,
            "refused: #1 is landed, out of the queue already: sluice status 1\n".into()
        )
    );
    let (code, out) = sluice(&f, &["withdraw", "1", "--json"]);
    assert_eq!(code, 2);
    assert_eq!(
        json(&out),
        serde_json::json!({"error": {
            "kind": "refused",
            "message": "#1 is landed, out of the queue already: sluice status 1",
            "next": ["sluice status 1"],
        }})
    );
}

/// `sluice step --json`: the round's outcome as one document.
#[test]
fn step_under_json() {
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let (code, out) = sluice(&f, &["step", BRANCH, "--json"]);
    assert_eq!(code, 0);
    assert_eq!(
        json(&out),
        serde_json::json!({
            "branch": BRANCH, "outcome": "stepped", "released": [], "previews_released": [],
            "releases": [],
        })
    );
    assert_eq!(
        sluice(&f, &["step", BRANCH]),
        (0, format!("{BRANCH}: stepped\n"))
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
    let code = run(cli, f.seams(), &host, &(), &(), &f.clock, pause, &mut out).unwrap();
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
        Some(sluice::Command::Exec {
            branch,
            command,
            worktree,
            json,
            release,
        }) => {
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
/// worktree's adopted place, count against the bound only while they run, and the
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
        sluice_seams::Tests::running(&f.tests).unwrap().sessions,
        0,
        "the run is no longer counted"
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

/// 89, 71: with the bound full, exec waits, and starts when a run ends.
#[test]
fn exec_waits_for_room() {
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
            assert!(world.started().is_empty(), "started into a full bound");
            world.kill_session("/w/other");
        }
        true
    });
    let (code, out) = exec_with(&f, 0, &pause, &[BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    assert!(out.contains("waiting: the host is at its bound\n"), "{out}");
    assert_eq!(f.world.started().len(), 1);
}

/// 71: a start the check seam refuses at the host's bound, another
/// repository's run having taken the last one after the count was read,
/// waits as one the count refused, says so once, and starts on the next
/// try.
#[test]
fn exec_waits_on_a_start_refused_at_the_bound() {
    let f = setup();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    f.world.refuse_next_start();
    let (code, out) = exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    assert_eq!(
        out.matches("waiting: the host is at its bound\n").count(),
        1,
        "{out}"
    );
    assert_eq!(f.world.started().len(), 1);
}

/// 91: a session run never uses what the reserve keeps; stopped while
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
        out.ends_with("waiting: the host is at its bound\nstopped: nothing ran\n"),
        "{out}"
    );
    assert!(f.world.started().is_empty());
    assert_eq!(f.world.results_log(), "");
    // Without the reserve, the room is taken.
    let (code, _) = exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED);
}

/// ADR 0035: with the host full and an agent's check holding a run, a
/// branch with a submission queued and room in its window says so on its
/// counts line, in the overview, `sluice status` and the home view; once
/// the check ends, none of them does.
#[test]
fn the_counts_say_when_agents_checks_hold_the_queue_back() {
    let f = setup();
    f.world.host_limit(2);
    f.world.session_run("/w/agent", true);
    work(
        &f,
        "unit/touches",
        "crw-1-unit-1",
        &[("plugin/lib/plan.py", Some("touched"))],
    );
    work(
        &f,
        "unit/cards",
        "crw-1-unit-2",
        &[("tests/t_cards.py", Some("cards 2"))],
    );
    f.world.hold_runs(true);
    for w in ["unit/touches", "unit/cards"] {
        sluice(&f, &["submit", BRANCH, "--worktree", w, "--no-wait"]);
    }
    sluice(&f, &["run", "--rounds", "2"]);
    let clause = "testing 1 at once (up to 4) · 1 slot to an agent's checks";

    let (_, text) = sluice(&f, &["overview"]);
    assert!(text.lines().next().unwrap().ends_with(clause), "{text}");
    assert!(
        text.contains("\nhost: 2 of 2 runs · 1 queue · 1 agent\n"),
        "{text}"
    );
    let (_, text) = sluice(&f, &["status", BRANCH]);
    assert!(text.lines().next().unwrap().ends_with(clause), "{text}");
    let (_, text) = sluice(&f, &[]);
    assert!(
        text.contains(&format!("{BRANCH}: 2 in the queue · {clause}\n")),
        "{text}"
    );

    f.world.kill_session("/w/agent");
    for args in [&["overview"][..], &["status", BRANCH], &[]] {
        let (_, text) = sluice(&f, args);
        assert!(!text.contains("checks"), "{text}");
    }
}

/// The overview's document, as a fake host with `reserved` reads it.
fn overview_doc(f: &Fakes, reserved: usize) -> serde_json::Value {
    let cli = Cli::try_parse_from(["sluice", "overview", "--json"]).unwrap();
    let mut out = Vec::new();
    let host = support::FakeHost {
        world: f.world.clone(),
        reserved,
    };
    let pause = Rounds(Cell::new(0));
    run(cli, f.seams(), &host, &(), &(), &f.clock, &pause, &mut out).unwrap();
    serde_json::from_slice(&out).unwrap()
}

/// ADR 0035: while an agent's check runs, its worktree's entry carries the
/// tasks its `sluice exec` keeps, and `room` counts it against the bound;
/// keeping them moves no branch's version. Once it ends the run is gone,
/// and its result lists its tasks and says it is reused.
#[test]
fn overview_shows_an_agents_check_and_the_hosts_room() {
    use sluice_seams::{TaskProgress, TaskState};
    let f = setup();
    f.world.host_limit(3);
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    let head = sluice_seams::Driver::session(&f.driver, "unit/a")
        .unwrap()
        .head;
    f.world.hold_runs(true);
    let task = |state| TaskProgress {
        target: "plugin:test".into(),
        state,
        started: None,
        ended: None,
        exit: None,
    };
    let seen = RefCell::new(None);
    let text = RefCell::new(String::new());
    let version = f.world.version(BRANCH);
    let pause = Then(Cell::new(0), |n| {
        match n {
            1 => f.world.says_tasks(&head, &[task(TaskState::Running)]),
            2 => {
                *seen.borrow_mut() = Some(overview_doc(&f, 1));
                *text.borrow_mut() = sluice(&f, &["overview"]).1;
                f.world.complete(&head);
            }
            _ => {}
        }
        true
    });
    let (code, out) = exec_with(&f, 0, &pause, &[BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");

    let during = seen.into_inner().expect("read while the check ran");
    let w = &during["sessions"][0];
    assert_eq!(w["worktree"], "unit/a");
    assert!(w["run"]["started"].is_string());
    assert!(w["run"].get("command").is_none());
    assert_eq!(w["run"]["tasks"][0]["target"], "plugin:test");
    assert_eq!(w["run"]["tasks"][0]["state"], "running");
    assert_eq!(
        during["room"],
        serde_json::json!({"bound": 3, "queue": 0, "sessions": 1, "reserve": 1})
    );
    assert_eq!(
        f.world.version(BRANCH),
        version,
        "no branch's version moved"
    );
    let text = text.into_inner();
    assert!(
        text.contains(&format!("\nchecking unit/a → {BRANCH} 0/1 since ")),
        "{text}"
    );
    assert!(!text.contains("testing unit/a"), "{text}");

    let after = overview_doc(&f, 0);
    let w = &after["sessions"][0];
    assert!(w.get("run").is_none());
    assert_eq!(after["room"]["sessions"], 0);
    let last = &w["last_result"];
    assert_eq!(last["recorded"], true);
    assert_eq!(last["reused"], true);
    assert_eq!(last["tasks"][0]["target"], "plugin:test");
    assert_eq!(last["tasks"][0]["state"], "passed");
}

/// ADR 0035: a run of a command names its command and lists no tasks; a
/// session run in another repository on the host counts only in `room`.
#[test]
fn overview_names_a_commands_check_and_counts_another_repositorys() {
    let f = setup();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    f.world.session_run("/elsewhere/agent", true);
    f.world.hold_runs(true);
    let seen = RefCell::new(None);
    let pause = Then(Cell::new(0), |n| {
        if n == 1 {
            *seen.borrow_mut() = Some(overview_doc(&f, 0));
            f.world.complete_all();
        }
        true
    });
    exec_with(
        &f,
        0,
        &pause,
        &[BRANCH, "--worktree", "unit/a", "--", "make", "e2e"],
    );
    let during = seen.into_inner().unwrap();
    let sessions = during["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1, "{sessions:?}");
    assert_eq!(sessions[0]["run"]["command"], "make e2e");
    assert!(sessions[0]["run"].get("tasks").is_none());
    assert_eq!(during["room"]["sessions"], 2);
    // A command's result says nothing of what it is worth.
    let after = overview_doc(&f, 0);
    let last = &after["sessions"][0]["last_result"];
    assert_eq!(last["kind"], "command");
    assert!(last.get("reused").is_none() && last.get("tasks").is_none());
}

/// ADR 0035, 93: a check's passes are reused only when recorded under an
/// executor the gate trusts; one with uncommitted changes is not kept.
#[test]
fn a_finished_check_says_whether_it_is_reused() {
    let f = setup();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    f.world.dirty("unit/a", &["scratch"]);
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    let last = &overview_doc(&f, 0)["sessions"][0]["last_result"];
    assert_eq!(last["outcome"], "pass");
    assert_eq!(last["recorded"], false);
    assert_eq!(last["reused"], false);
    assert_eq!(last["tasks"].as_array().unwrap().len(), 1);

    // A gate that trusts only confined runs takes none of the session's.
    let f = Fakes::new();
    f.world.init(
        BRANCH,
        &[
            (
                ".config/sluice.toml",
                "[branches.\"bolt/*\"]\nrequired = [\"#gate\"]\ntrust = [\"confined\"]\n",
            ),
            ("plugin/lib/plan.py", "plan"),
        ],
    );
    f.world.task("plugin:test", "plugin/");
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    let last = &overview_doc(&f, 0)["sessions"][0]["last_result"];
    assert_eq!(last["recorded"], true);
    assert_eq!(last["reused"], false);
}

/// ADR 0035: the watch prints again when a check starts and when one of
/// its tasks moves, and not when nothing of it does.
#[test]
fn overview_watch_prints_when_a_checks_task_moves() {
    use sluice_seams::{HostRun, TaskProgress, TaskState};
    let f = setup();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    let running = [TaskProgress {
        target: "plugin:test".into(),
        state: TaskState::Running,
        started: None,
        ended: None,
        exit: None,
    }];
    let between = |n: usize| match n {
        1 => f.world.begin_host_run(HostRun {
            worktree: "unit/a".into(),
            branch: BRANCH.into(),
            started: "2026-10-20T09:00:00Z".into(),
            command: None,
            tasks: Vec::new(),
        }),
        2 | 3 => f.world.session_tasks("unit/a", &running),
        _ => {}
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 3,
        between: &between,
    };
    let cli = Cli::try_parse_from(["sluice", "overview", "--json", "--watch"]).unwrap();
    let mut out = Vec::new();
    let host = support::FakeHost {
        world: f.world.clone(),
        reserved: 0,
    };
    run(cli, f.seams(), &host, &(), &(), &f.clock, &pause, &mut out).unwrap();
    let out = String::from_utf8(out).unwrap();
    let docs: Vec<serde_json::Value> = out
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(docs.len(), 3, "{out}");
    assert!(docs[0]["sessions"][0].get("run").is_none());
    assert!(docs[1]["sessions"][0]["run"].get("tasks").is_none());
    assert_eq!(
        docs[2]["sessions"][0]["run"]["tasks"][0]["state"],
        "running"
    );
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
        &(),
        &(),
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
    let (code, out) = release(&f, &["--worktree", "unit/a", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(
        json(&out),
        serde_json::json!({"worktree": "unit/a", "released": false})
    );
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

/// `sluice release --json`: the worktree, and that it was released; what
/// it says beside goes to stderr.
#[test]
fn release_under_json() {
    let f = setup_releasing();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    let (code, out) = release(&f, &["--worktree", "unit/a", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(
        json(&out),
        serde_json::json!({"worktree": "unit/a", "released": true})
    );
    assert_eq!(f.world.session_releases().len(), 1);
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

// ---------------------------------------------------------------------------
// A worktree removed unreleased, and the worktrees held (session-runs-1
// clause 97)
// ---------------------------------------------------------------------------

const GONE: &str = "freed unit/a: it no longer exists; its release ran once from its scratch directory; its runs ended, its ports free";

/// 97, 123: the next `sluice exec` on the host runs the release a removed
/// worktree's runs stored, once, frees its place, says so first, and logs
/// it `gone` in the branch's records with its release run; no release is
/// built from the policy, and the worktree it runs in is left alone.
#[test]
fn the_next_exec_frees_a_removed_worktrees_place() {
    let f = setup_releasing();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    work(
        &f,
        "unit/b",
        "agent-b",
        &[("plugin/lib/plan.py", Some("p3"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    f.world.remove_worktree("unit/a");

    let (code, out) = exec(&f, &[BRANCH, "--worktree", "unit/b"]);
    assert_eq!(code, LANDED, "{out}");
    assert_eq!(out.lines().next(), Some(GONE), "{out}");
    assert!(f.world.session_releases().is_empty(), "a release was built");
    assert_eq!(f.world.gone_releases(), ["session/unit/a"]);
    assert_eq!(f.world.freed_places(), ["session/unit/a"]);
    assert_eq!(f.world.adopted_places(), ["session/unit/b"]);
    let snap = sluice_seams::State::read(&f.state, BRANCH).unwrap();
    let log = sluice_core::line::Line::read_log(&snap).unwrap();
    assert!(
        log.iter().any(|l| matches!(
            &l.event,
            sluice_core::records::Event::Gone { place, worktree: Some(w), ran: true, failed: None }
                if place == "session/unit/a" && w == "unit/a"
        )),
        "{log:?}"
    );

    // Freed once: the next run names it no more, and runs nothing of it.
    let (_, out) = exec(&f, &[BRANCH, "--worktree", "unit/b"]);
    assert!(!out.contains("unit/a"), "{out}");
    assert_eq!(f.world.freed_places().len(), 1);
    assert_eq!(f.world.gone_releases().len(), 1);
}

/// 97, 123: the next round, `sluice step` or a pass of `sluice run`,
/// frees it as well and says so.
#[test]
fn the_next_round_frees_a_removed_worktrees_place() {
    for args in [&["step", BRANCH][..], &["run", "--rounds", "1"][..]] {
        let f = setup_releasing();
        work(
            &f,
            "unit/a",
            "agent-a",
            &[("plugin/lib/plan.py", Some("p2"))],
        );
        exec(&f, &[BRANCH, "--worktree", "unit/a"]);
        f.world.remove_worktree("unit/a");
        let (code, out) = sluice(&f, args);
        assert_eq!(code, 0, "{out}");
        assert_eq!(out.lines().next(), Some(GONE), "{args:?}: {out}");
        assert!(f.world.adopted_places().is_empty(), "{args:?}");
    }
}

/// 97, 25, rule core.16: a worktree that still exists is never released by
/// a round or a run, however long since its last run.
#[test]
fn a_worktree_that_exists_is_held_however_old_its_last_run() {
    let f = setup_releasing();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    f.world.advance(chrono::Duration::days(90));
    sluice(&f, &["step", BRANCH]);
    sluice(&f, &["run", "--rounds", "3"]);
    work(
        &f,
        "unit/b",
        "agent-b",
        &[("plugin/lib/plan.py", Some("p3"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/b"]);
    assert!(f.world.session_releases().is_empty());
    assert_eq!(
        f.world.adopted_places(),
        ["session/unit/a", "session/unit/b"]
    );
}

/// 97: `sluice status` with no argument lists every worktree sluice holds,
/// with its branch, its ports and the time of its last run; `--json` is
/// one document of the same.
#[test]
fn status_lists_the_worktrees_held() {
    let f = setup_releasing();
    let (code, out) = sluice(&f, &["status"]);
    assert_eq!(code, 0);
    assert_eq!(out, "no worktree holds ports or runs here\n");

    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    let (code, out) = sluice(&f, &["status"]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "worktrees held until released:", "{out}");
    assert!(
        lines[1].starts_with("  unit/a  bolt/concurrent-units  last run 2026-10-11T09:"),
        "{out}"
    );
    assert_eq!(
        lines[2],
        "release one with: sluice release --worktree <worktree>"
    );

    // A removed worktree is listed as such until a run or round frees it.
    f.world.remove_worktree("unit/a");
    let (_, out) = sluice(&f, &["status"]);
    assert!(
        out.contains("(removed: the next run or round runs its stored release once and frees it)"),
        "{out}"
    );
    assert!(f.world.session_releases().is_empty(), "status released");

    let (code, out) = sluice(&f, &["status", "--json"]);
    assert_eq!(code, 0);
    let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
    let w = &doc["worktrees"][0];
    assert_eq!(w["worktree"], "unit/a");
    assert_eq!(w["branch"], BRANCH);
    assert!(w["ports"].is_object());
    assert!(w["last_run"]
        .as_str()
        .unwrap()
        .starts_with("2026-10-11T09:"));
    assert_eq!(w["removed"], true);
}

/// Session-runs-1 clauses 92, 97, rule core.18: `sluice status --json`'s
/// `worktrees[]` and `sluice overview --json`'s `sessions[]` carry the
/// same `last_result`, beside the fields they had; `sluice status` says
/// it on the worktree's line; a watch prints again when a run ends.
#[test]
fn status_and_overview_carry_a_worktrees_last_result() {
    let f = setup_releasing();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    let head = sluice_seams::Driver::session(&f.driver, "unit/a")
        .unwrap()
        .head;
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    let (_, out) = sluice(&f, &["status", "--json"]);
    let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
    let w = doc["worktrees"][0].clone();
    let last = &w["last_result"];
    assert_eq!(last["head"], head.0, "{w}");
    assert_eq!(last["kind"], "checks", "{w}");
    assert_eq!(last["outcome"], "pass", "{w}");
    assert_eq!(last["recorded"], true, "{w}");
    assert!(last["at"].is_string(), "{w}");
    assert!(w["last_run"].is_string() && w["ports"].is_object(), "{w}");
    assert_eq!(w["removed"], false, "{w}");
    let (_, out) = sluice(&f, &["overview", "--json"]);
    let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(doc["sessions"][0], w, "{doc}");
    let (_, out) = sluice(&f, &["status"]);
    assert!(
        out.contains(&format!("last result passed on {}, recorded", head.short())),
        "{out}"
    );

    // A watch prints again when a run ends; this one unclean.
    let between = |n: usize| {
        if n == 2 {
            f.world.dirty("unit/a", &["scratch"]);
            exec(&f, &[BRANCH, "--worktree", "unit/a"]);
        }
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 4,
        between: &between,
    };
    let cli = Cli::try_parse_from(["sluice", "overview", "--json", "--watch"]).unwrap();
    let mut out = Vec::new();
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
        &pause,
        &mut out,
    )
    .unwrap();
    let out = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(
        second["sessions"][0]["last_result"]["recorded"], false,
        "{second}"
    );
    let (_, out) = sluice(&f, &["status"]);
    assert!(out.contains("last result passed on"), "{out}");
    assert!(out.contains(", not recorded"), "{out}");
}

/// Session-runs-1 clause 97: a result that cannot be kept on the worktree
/// changes neither the run's exit nor what it says; the warning is on
/// stderr, and the worktree lists no result.
#[test]
fn a_result_not_kept_changes_nothing_of_the_run() {
    let run = |keeping_fails: bool| {
        let f = setup_releasing();
        work(
            &f,
            "unit/a",
            "agent-a",
            &[("plugin/lib/plan.py", Some("p2"))],
        );
        f.world.keeping_fails(keeping_fails);
        let ran = exec(&f, &[BRANCH, "--worktree", "unit/a"]);
        let (_, out) = sluice(&f, &["status", "--json"]);
        let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
        (ran, doc["worktrees"][0]["last_result"].clone())
    };
    let (kept, listed) = run(false);
    assert_eq!(kept.0, 0, "{}", kept.1);
    assert!(listed.is_object(), "{listed}");
    let (unkept, listed) = run(true);
    assert_eq!(unkept, kept, "the run's exit and output");
    assert!(listed.is_null(), "{listed}");
}

/// ADR 0024: a worktree that has checked out a branch sluice lands on, by
/// its records or by a gate its policy gives it, is named in overview's
/// `sessions[]`, in `sluice status` and by `sluice protect`'s output; a
/// detached one and one on a branch of its own are not, and the landing
/// goes through.
#[test]
fn a_worktree_holding_a_branch_sluice_lands_on_is_warned_of() {
    let f = setup();
    let other = "bolt/no-records";
    f.world.init(other, &[(".config/sluice.toml", POLICY)]);
    f.world.check_out("/w/main-like", Some(BRANCH));
    f.world.check_out("/w/bolt", Some(other));
    f.world.check_out("/w/view", Some(BRANCH));
    f.world.check_out("/w/view", None);
    work(
        &f,
        "unit/mine",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    f.world.init("unit/mine", &[("plugin/lib/plan.py", "p2")]);
    f.world.check_out("/w/unit", Some("unit/mine"));

    let (code, _) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/mine"]);
    assert_eq!(code, LANDED, "a held branch never stops a landing");

    let (_, json) = sluice(&f, &["overview", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let held: Vec<(&str, &str)> = v["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|w| Some((w["worktree"].as_str()?, w["holds_queued_branch"].as_str()?)))
        .collect();
    assert_eq!(held, vec![("/w/bolt", other), ("/w/main-like", BRANCH)]);
    assert!(v["sessions"][0].get("ports").is_none());

    let (_, text) = sluice(&f, &["overview"]);
    assert!(
        text.contains(&format!("warning: /w/main-like has {BRANCH} checked out")),
        "{text}"
    );
    assert!(!text.contains("worktrees held"), "{text}");

    let (code, out) = sluice(&f, &["status"]);
    assert_eq!(code, 0);
    assert!(
        out.contains(&format!(
            "warning: /w/main-like has {BRANCH} checked out: sluice moves {BRANCH} under it"
        )),
        "{out}"
    );
    let (_, out) = sluice(&f, &["status", "--json"]);
    let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(doc["worktrees"].as_array().unwrap().len(), 2);
}

/// A session's place whose worktree holds a branch sluice lands on carries
/// `holds_queued_branch` on its own entry, not a second one.
#[test]
fn a_held_place_is_one_entry() {
    let f = setup_releasing();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    f.world.check_out("unit/a", Some(BRANCH));
    let (_, out) = sluice(&f, &["status", "--json"]);
    let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
    let w = doc["worktrees"].as_array().unwrap();
    assert_eq!(w.len(), 1, "{out}");
    assert_eq!(w[0]["holds_queued_branch"], BRANCH);
    assert!(w[0]["ports"].is_object());
}

/// `sluice` with a pause of the test's own.
fn sluice_with(f: &Fakes, pause: &dyn Pause, args: &[&str]) -> (i32, String) {
    let cli = Cli::try_parse_from(std::iter::once("sluice").chain(args.iter().copied())).unwrap();
    let mut out = Vec::new();
    let host = support::FakeHost {
        world: f.world.clone(),
        reserved: 0,
    };
    let code = run(cli, f.seams(), &host, &(), &(), &f.clock, pause, &mut out).unwrap();
    (code, String::from_utf8(out).unwrap())
}

/// The combination submission `id` is under test in.
fn tested(f: &Fakes, id: u64) -> sluice_seams::Rev {
    let (_, out) = sluice(f, &["status", &id.to_string(), "--json"]);
    sluice_seams::Rev::new(json(&out)["tested"]["combination"].as_str().unwrap())
}

/// ADR 0041: `sluice wait` runs rounds to the outcome, as a waiting
/// submit does; a submission already decided returns at once, and no
/// round runs.
#[test]
fn wait_runs_to_the_outcome() {
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let (code, out) = sluice(&f, &["wait", "1"]);
    assert_eq!(code, LANDED, "{out}");
    let tip = f.world.tip(BRANCH);
    assert_eq!(out.lines().next(), Some("#1 queued"), "{out}");
    assert!(
        out.ends_with(&format!("passed · landed as {}\n", &tip.0[..7])),
        "{out}"
    );

    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/b", "--no-wait"]);
    let (code, out) = sluice(&f, &["wait", "1", "--json"]);
    assert_eq!(code, LANDED);
    assert_eq!(json(&out)["state"], "landed");
    // No round ran: #2 is still queued.
    let (_, out) = sluice(&f, &["status", "2", "--json"]);
    assert_eq!(json(&out)["state"], "queued");
}

/// A report waited on from another worktree names the agent working in
/// the one the submission came from as who moves it on, and sends its
/// reader into no worktree; a step that is the operator's, setting a fail
/// aside, still names the operator.
#[test]
fn a_report_waited_on_elsewhere_names_the_agent_in_the_worktree_it_came_from() {
    let f = setup();
    f.world
        .fails_with("tests:t-batch-cards", "tests/t_cards.py", "broken");
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("tests/t_cards.py", Some("broken"))],
    );
    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let (code, out) = sluice(&f, &["wait", "1", "--worktree", "unit/b"]);
    assert_eq!(code, FAILED, "{out}");
    assert_eq!(
        out.lines().last(),
        Some("the agent working in unit/a fixes it there and submits again"),
        "{out}"
    );
    for word in ["your worktree", "inside"] {
        assert!(!out.contains(word), "{word}: {out}");
    }

    let f = setup();
    f.world
        .flaky("tests:t-batch-cards", &[Outcome::Fail, Outcome::Pass]);
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("tests/t_cards.py", Some("flaky"))],
    );
    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    let submit = ["submit", BRANCH, "--worktree", "unit/a", "--no-wait"];
    sluice(&f, &submit);
    assert_eq!(sluice(&f, &["wait", "1", "--worktree", "unit/b"]).0, FAILED);
    sluice(&f, &submit);
    let (code, out) = sluice(&f, &["wait", "2", "--worktree", "unit/b"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("the operator can set it aside: sluice set-aside tests:t-batch-cards --at "),
        "{out}"
    );
    for word in ["your worktree", "inside"] {
        assert!(!out.contains(word), "{word}: {out}");
    }
}

/// ADR 0041: the exits tell a blame, a conflict and a withdrawal apart,
/// for `wait` and a waiting submit alike.
#[test]
fn wait_and_submit_tell_the_outcomes_apart() {
    let f = setup();
    f.world
        .fails_with("tests:t-batch-cards", "tests/t_cards.py", "broken");
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("tests/t_cards.py", Some("broken"))],
    );
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let (code, out) = sluice(&f, &["wait", "1"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("failed: tests:t-batch-cards"), "{out}");

    work(
        &f,
        "unit/b",
        "agent-b",
        &[("plugin/lib/plan.py", Some("one"))],
    );
    work(
        &f,
        "unit/c",
        "agent-c",
        &[("plugin/lib/plan.py", Some("two"))],
    );
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/b", "--no-wait"]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/c", "--no-wait"]);
    let (code, out) = sluice(&f, &["wait", "3", "--json"]);
    assert_eq!(code, CONFLICT, "{out}");
    assert_eq!(json(&out)["state"], "conflict");

    f.world.hold_runs(true);
    work(&f, "unit/d", "agent-d", &[("plugin/d", Some("d"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/d", "--no-wait"]);
    let withdraw = |n: usize| {
        if n == 1 {
            sluice(&f, &["withdraw", "4"]);
        }
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 5,
        between: &withdraw,
    };
    let (code, out) = sluice_with(&f, &pause, &["wait", "4", "--json"]);
    assert_eq!(code, sluice::WITHDRAWN, "{out}");
    assert_eq!(json(&out)["state"], "withdrawn");

    work(&f, "unit/e", "agent-e", &[("plugin/e", Some("e"))]);
    let withdraw = |n: usize| {
        if n == 1 {
            sluice(&f, &["withdraw", "5"]);
        }
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 5,
        between: &withdraw,
    };
    let (code, out) = sluice_with(&f, &pause, &["submit", BRANCH, "--worktree", "unit/e"]);
    assert_eq!(code, sluice::WITHDRAWN, "{out}");
}

/// ADR 0041: at its timeout, `wait` returns the record as it stands and
/// exits 5, and the submission stays in the queue; stopped with Ctrl-C,
/// one document still.
#[test]
fn wait_returns_at_its_timeout_with_the_record() {
    let f = setup();
    f.world.hold_runs(true);
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let (code, out) = sluice(&f, &["wait", "1", "--timeout", "10s", "--json"]);
    assert_eq!(code, sluice::WAITING, "{out}");
    let v = json(&out);
    assert_eq!(
        (v["id"].as_u64(), v["state"].as_str()),
        (Some(1), Some("testing"))
    );
    let (_, out) = sluice(&f, &["status", "1", "--json"]);
    assert_eq!(json(&out)["state"], "testing");

    let (code, out) = sluice(&f, &["wait", "1", "--timeout", "10s"]);
    assert_eq!(code, sluice::WAITING, "{out}");
    assert!(out.contains(&format!("testing {BRANCH} + #1 · ")), "{out}");
    assert!(out.ends_with("still waiting: sluice wait 1\n"), "{out}");

    let stop = Then(Cell::new(0), |_| false);
    let (code, out) = sluice_with(&f, &stop, &["wait", "1", "--json"]);
    assert_eq!(code, sluice::STOPPED);
    assert_eq!(json(&out)["state"], "testing");
}

/// ADR 0041: a branch names this worktree's last submission to it;
/// nothing submitted, or no such submission, is not found.
#[test]
fn wait_finds_what_it_waits_on() {
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    work(&f, "unit/c", "agent-c", &[("plugin/c", Some("c"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/b", "--no-wait"]);
    let (code, out) = sluice(&f, &["wait", BRANCH, "--worktree", "unit/a", "--json"]);
    assert_eq!(code, LANDED, "{out}");
    assert_eq!(json(&out)["id"], 1);

    let (code, out) = sluice(&f, &["wait", BRANCH, "--worktree", "unit/c", "--json"]);
    assert_eq!(code, FAILED);
    assert_eq!(json(&out)["error"]["kind"], "not_found");
    assert_eq!(
        json(&out)["error"]["message"],
        format!("nothing submitted from unit/c to {BRANCH}")
    );

    let (code, out) = sluice(&f, &["wait", "9999", "--json"]);
    assert_eq!(code, FAILED);
    assert_eq!(json(&out)["error"]["message"], "no submission 9999");
}

/// A pause that moves the fakes' clock on a minute at each wait, for a
/// wait that runs past [`sluice::UNANSWERED_FOR`].
struct Minutes<'a>(&'a Fakes, Cell<usize>);

impl Pause for Minutes<'_> {
    fn pause(&self) -> bool {
        self.1.set(self.1.get() + 1);
        assert!(self.1.get() < 50, "waited too long");
        self.0.world.advance(chrono::Duration::minutes(1));
        true
    }
}

/// A round the forge answered 5xx is run again: `sluice wait` and a
/// waiting submit wait through a few such answers to the outcome.
#[test]
fn a_wait_reads_again_through_a_forges_5xx() {
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    f.world.unanswer_starts(3);
    let (code, out) = sluice(&f, &["wait", "1"]);
    assert_eq!(code, LANDED, "{out}");
    let tip = f.world.tip(BRANCH);
    assert!(
        out.ends_with(&format!("passed · landed as {}\n", &tip.0[..7])),
        "{out}"
    );

    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    f.world.unanswer_starts(3);
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/b", "--json"]);
    assert_eq!(code, LANDED, "{out}");
    assert_eq!(json(&out)["state"], "landed");
}

/// Past five minutes of a forge's 5xx answers, `sluice wait` says it could
/// not read, on stderr with the forge's words and on stdout with how to
/// wait again, and exits 3: no outcome. Nothing was decided of the
/// submission, which lands once the forge answers.
#[test]
fn a_wait_past_five_minutes_of_5xx_says_it_could_not_read() {
    if let Some(err) = stderr_of("a_wait_past_five_minutes_of_5xx_says_it_could_not_read") {
        let said: Vec<&str> = err
            .lines()
            .filter(|l| l.starts_with("sluice: could not read #1 for 5m"))
            .collect();
        assert_eq!(said.len(), 2, "{err}");
        for line in said {
            assert!(
                line.contains(", so whether it landed is not known: starting a run in ")
                    && line.ends_with(": 503 Service Unavailable"),
                "{err}"
            );
        }
        return;
    }
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    f.world.unanswer_starts(usize::MAX);
    let (code, out) = sluice_with(&f, &Minutes(&f, Cell::new(0)), &["wait", "1"]);
    assert_eq!(code, sluice::INTERNAL, "{out}");
    assert!(out.ends_with("could not read #1: sluice wait 1\n"), "{out}");
    assert!(
        !out.contains("landed as") && !out.contains("failed"),
        "{out}"
    );

    let (code, out) = sluice_with(&f, &Minutes(&f, Cell::new(0)), &["wait", "1", "--json"]);
    assert_eq!(code, sluice::INTERNAL, "{out}");
    let v = json(&out);
    assert_eq!(v["error"]["kind"], "internal");
    assert_eq!(v["error"]["next"], serde_json::json!(["sluice wait 1"]));
    let message = v["error"]["message"].as_str().unwrap();
    assert!(message.starts_with("could not read #1 for 5m"), "{message}");
    assert!(message.ends_with(": 503 Service Unavailable"), "{message}");
    let (_, out) = sluice(&f, &["status", "1", "--json"]);
    assert_ne!(json(&out)["state"], "failed");

    f.world.unanswer_starts(0);
    assert_eq!(sluice(&f, &["wait", "1"]).0, LANDED);
}

/// Pull request 7 into the branch, from any worktree, waiting; its forge
/// answers 5xx to as many finds and follows as each count holds.
struct Pull7 {
    finds: Cell<usize>,
    follows: Cell<usize>,
}

impl Pull7 {
    fn new(finds: usize, follows: usize) -> Pull7 {
        Pull7 {
            finds: Cell::new(finds),
            follows: Cell::new(follows),
        }
    }

    /// A read of the pull request, answered 5xx while `left` holds any.
    fn read(left: &Cell<usize>) -> anyhow::Result<()> {
        if left.get() == 0 {
            return Ok(());
        }
        left.set(left.get() - 1);
        Err(
            sluice_seams::Unanswered("GET /repos/o/r/pulls/7: 503 Service Unavailable".into())
                .into(),
        )
    }
}

impl sluice::PullRequests for Pull7 {
    fn branches(&self, _: sluice_core::Seams<'_>) -> anyhow::Result<Vec<String>> {
        Ok(Vec::new())
    }

    fn intake(
        &self,
        _: sluice_core::Seams<'_>,
        _: &dyn sluice_core::Clock,
        _: &str,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn open(
        &self,
        _: sluice_core::Seams<'_>,
        _: &dyn sluice_core::Clock,
        _: &str,
        _: &str,
    ) -> anyhow::Result<Result<sluice::Opened, sluice_core::SubmitRefusal>> {
        Ok(Ok(sluice::Opened {
            number: 7,
            said: "opened #7".into(),
            copy: None,
            submission: None,
        }))
    }

    fn find(
        &self,
        _: sluice_core::Seams<'_>,
        _: &dyn sluice_core::Clock,
        _: &str,
        _: &str,
    ) -> anyhow::Result<Option<u64>> {
        Pull7::read(&self.finds)?;
        Ok(Some(7))
    }

    fn follow(
        &self,
        _: sluice_core::Seams<'_>,
        _: &dyn sluice_core::Clock,
        _: &str,
        _: u64,
    ) -> anyhow::Result<sluice::Followed> {
        Pull7::read(&self.follows)?;
        Ok(sluice::Followed::Open {
            title: "waiting".into(),
            lines: Vec::new(),
            submission: None,
            names: Default::default(),
        })
    }
}

/// The branch, taking pull requests.
fn pulls_setup() -> Fakes {
    let f = Fakes::new();
    let policy = POLICY.replace(
        "[branches.\"bolt/*\"]\n",
        "[branches.\"bolt/*\"]\nremote = \"origin\"\n",
    ) + "\n[branches.\"bolt/*\".pull_requests]\n";
    f.world.init(
        BRANCH,
        &[(".config/sluice.toml", &policy), ("plugin/a", "a")],
    );
    f.world.task("plugin:test", "plugin/");
    f
}

/// `sluice <args>` over the fakes, its pull requests from `pulls`.
fn sluice_pulls(
    f: &Fakes,
    pulls: &dyn sluice::PullRequests,
    pause: &dyn Pause,
    args: &[&str],
) -> (i32, String) {
    let cli = Cli::try_parse_from(std::iter::once("sluice").chain(args.iter().copied())).unwrap();
    let mut out = Vec::new();
    let host = support::FakeHost {
        world: f.world.clone(),
        reserved: 0,
    };
    let code = run(cli, f.seams(), &host, &(), pulls, &f.clock, pause, &mut out).unwrap();
    (code, String::from_utf8(out).unwrap())
}

/// Following a pull request, a find or a follow the forge answered 5xx is
/// made again; past five minutes of such answers, `sluice wait` and a
/// waiting submit say they could not read it and exit 3, never that it
/// closed or failed.
#[test]
fn following_a_pull_request_reads_again_through_a_forges_5xx() {
    let f = pulls_setup();
    let stop = Then(Cell::new(0), |n| n < 6);
    let (code, out) = sluice_pulls(&f, &Pull7::new(2, 2), &stop, &["wait", BRANCH]);
    assert_eq!(code, sluice::STOPPED, "{out}");
    assert_eq!(
        out,
        format!("waiting\nstill open: #7; sluice wait {BRANCH}\n")
    );

    let pulls = Pull7::new(0, usize::MAX);
    let (code, out) = sluice_pulls(&f, &pulls, &Minutes(&f, Cell::new(0)), &["wait", BRANCH]);
    assert_eq!(code, sluice::INTERNAL, "{out}");
    assert_eq!(out, format!("could not read #7: sluice wait {BRANCH}\n"));
    let (code, out) = sluice_pulls(
        &f,
        &pulls,
        &Minutes(&f, Cell::new(0)),
        &["wait", BRANCH, "--json"],
    );
    assert_eq!(code, sluice::INTERNAL, "{out}");
    let v = json(&out);
    assert_eq!(v["error"]["kind"], "internal");
    assert_eq!(
        v["error"]["next"],
        serde_json::json!([format!("sluice wait {BRANCH}")])
    );
    let message = v["error"]["message"].as_str().unwrap();
    assert!(message.starts_with("could not read #7 for 5m"), "{message}");
    assert!(
        message.ends_with(
            ", so whether it landed is not known: GET /repos/o/r/pulls/7: 503 Service Unavailable"
        ),
        "{message}"
    );

    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("b"))]);
    let (code, out) = sluice_pulls(
        &f,
        &pulls,
        &Minutes(&f, Cell::new(0)),
        &["submit", BRANCH, "--worktree", "unit/a"],
    );
    assert_eq!(code, sluice::INTERNAL, "{out}");
    assert_eq!(
        out,
        format!("opened #7\ncould not read #7: sluice wait {BRANCH}\n")
    );

    let pulls = Pull7::new(usize::MAX, 0);
    let (code, out) = sluice_pulls(
        &f,
        &pulls,
        &Minutes(&f, Cell::new(0)),
        &["wait", BRANCH, "--worktree", "unit/a"],
    );
    assert_eq!(code, sluice::INTERNAL, "{out}");
    assert_eq!(
        out,
        format!(
            "could not read the pull request from unit/a into {BRANCH}: sluice wait {BRANCH}\n"
        )
    );
}

/// ADR 0042: a waiting command says its combination's progress each time
/// the count ended or the tasks running move, never at every round.
#[test]
fn wait_says_progress_when_it_moves() {
    use sluice_seams::{TaskProgress, TaskState};
    let f = setup();
    f.world.hold_runs(true);
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    let task = |target: &str, state: TaskState| TaskProgress {
        target: target.into(),
        state,
        started: None,
        ended: None,
        exit: state.ended().then_some(0),
    };
    let between = |n: usize| match n {
        1 => f.world.says_tasks(
            &tested(&f, 1),
            &[
                task("a:test", TaskState::Running),
                task("b:test", TaskState::Waiting),
            ],
        ),
        3 => f.world.says_tasks(
            &tested(&f, 1),
            &[
                task("a:test", TaskState::Passed),
                task("b:test", TaskState::Running),
            ],
        ),
        5 => {
            f.world.hold_runs(false);
            f.world.complete_all();
        }
        _ => {}
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 10,
        between: &between,
    };
    let (code, out) = sluice_with(&f, &pause, &["wait", "1"]);
    assert_eq!(code, LANDED, "{out}");
    let said: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with(&format!("testing {BRANCH} + #1 · ")))
        .filter(|l| !l.contains(" tasks to run, "))
        .collect();
    assert_eq!(said.len(), 3, "{out}");
    assert!(
        said[1].contains(" · 0/2 tasks · running a:test · "),
        "{out}"
    );
    assert!(
        said[2].contains(" · 1/2 tasks · running b:test · "),
        "{out}"
    );
}

// ---------------------------------------------------------------------------
// A checkout held because its release failed (123)
// ---------------------------------------------------------------------------

const RELEASE_FAILS: &str = "usage: scripts/preview up";

/// A submission landed while every release fails, and rounds until its
/// checkout is held; the checkout's name.
fn held_checkout(f: &Fakes) -> String {
    f.world.release_fails(Some(RELEASE_FAILS));
    work(
        f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    let (code, out) = sluice(f, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    for _ in 0..4 {
        sluice(f, &["step", BRANCH]);
    }
    let place = format!("sluice/try/{BRANCH}/1");
    assert!(f.world.live_places().contains(&place));
    place
}

/// 123: `sluice release <name>` runs a held checkout's release once
/// more: failing again, it exits 1 saying what failed and the checkout
/// stays; once it succeeds, the checkout is gone.
#[test]
fn release_by_name_releases_a_held_checkout() {
    let f = setup();
    let place = held_checkout(&f);
    let tries = f.world.release_attempts().len();

    let (code, out) = sluice(&f, &["release", &place]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.starts_with(&format!("could not release {place}: ")) && out.contains(RELEASE_FAILS),
        "{out}"
    );
    assert!(out.contains("--accept-failure"), "{out}");
    assert_eq!(f.world.release_attempts().len(), tries + 1);
    assert!(f.world.live_places().contains(&place));

    f.world.release_fails(None);
    let (code, out) = sluice(&f, &["release", &place, "--json"]);
    assert_eq!(code, LANDED, "{out}");
    assert_eq!(
        json(&out),
        serde_json::json!({"place": place, "released": true, "accepted": false, "failed": null})
    );
    assert!(!f.world.live_places().contains(&place));
}

/// 123: `--accept-failure` frees a held checkout with no release run.
#[test]
fn release_accepting_the_failure_frees_a_held_checkout() {
    let f = setup();
    let place = held_checkout(&f);
    let tries = f.world.release_attempts().len();
    let (code, out) = sluice(&f, &["release", &place, "--accept-failure"]);
    assert_eq!(code, LANDED, "{out}");
    assert_eq!(
        out,
        format!(
            "freed {place} without its release command: its runs ended, its checkout removed, its ports free\n"
        )
    );
    assert_eq!(f.world.release_attempts().len(), tries, "its release ran");
    assert_eq!(f.world.freed_places(), std::slice::from_ref(&place));
    assert!(!f.world.live_places().contains(&place));
}

/// 123: a name that names no checkout here is nothing to release.
#[test]
fn release_of_an_unknown_checkout_is_nothing() {
    let f = setup();
    for name in [
        format!("sluice/try/{BRANCH}/7"),
        "sluice/try/nothing".to_string(),
    ] {
        let (code, out) = sluice(&f, &["release", &name]);
        assert_eq!((code, out.as_str()), (LANDED, "nothing to release\n"));
    }
}

/// 123: a worktree whose release fails exits 1 with what failed and keeps
/// its place; `--accept-failure` then frees the place with no release run
/// and leaves the worktree.
#[test]
fn release_accepting_the_failure_frees_a_worktrees_place() {
    let f = setup_releasing();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    f.world.release_fails(Some(RELEASE_FAILS));
    let (code, out) = release(&f, &["--worktree", "unit/a"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.starts_with("could not release unit/a: ") && out.contains(RELEASE_FAILS),
        "{out}"
    );
    assert_eq!(f.world.adopted_places(), ["session/unit/a"]);

    let (code, out) = release(&f, &["--worktree", "unit/a", "--accept-failure", "--json"]);
    assert_eq!(code, LANDED, "{out}");
    assert_eq!(
        json(&out),
        serde_json::json!({"worktree": "unit/a", "released": true, "accepted": true})
    );
    assert!(f.world.adopted_places().is_empty());
    assert_eq!(f.world.freed_places(), ["session/unit/a"]);
    assert!(f.world.session_releases().is_empty());
    assert!(sluice_seams::Driver::session(&f.driver, "unit/a").is_ok());
    let snap = sluice_seams::State::read(&f.state, BRANCH).unwrap();
    let log = sluice_core::line::Line::read_log(&snap).unwrap();
    assert!(
        log.iter().any(|l| matches!(
            &l.event,
            sluice_core::records::Event::Released { place, accepted: Some(a), .. }
                if place == "session/unit/a" && a.contains(RELEASE_FAILS)
        )),
        "{log:?}"
    );
}

/// The fakes' setup with `release_retries = 1`.
fn setup_one_retry() -> Fakes {
    let f = Fakes::new();
    f.world.init(
        BRANCH,
        &[
            (
                ".config/sluice.toml",
                &format!("{POLICY}release_retries = 1\n"),
            ),
            ("plugin/lib/plan.py", "plan"),
        ],
    );
    f.world.task("plugin:test", "plugin/");
    f
}

/// 123: a waiting submit says each failed try of a checkout's release
/// and the hold, once, on the round that held it; the waits after say
/// nothing of it. `sluice status <branch>` lists it held with what
/// failed and its tries, and how to release it; `--json` as
/// `held_checkouts`.
#[test]
fn a_waiting_submit_says_a_held_checkout_once() {
    let f = setup_one_retry();
    let first = format!("sluice/try/{BRANCH}/1");
    f.world.place_release_fails(&first, Some(RELEASE_FAILS));
    let mut said = Vec::new();
    for (unit, file) in [("unit/a", "p2"), ("unit/b", "p3"), ("unit/c", "p4")] {
        work(&f, unit, unit, &[("plugin/lib/plan.py", Some(file))]);
        let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", unit]);
        assert_eq!(code, LANDED, "{out}");
        said.push(out);
    }
    let tried = |out: &str| {
        out.lines()
            .filter(|l| l.starts_with(&format!("could not release {first} (try 1 of 2): ")))
            .count()
    };
    let held = |out: &str| {
        out.lines()
            .filter(|l| l.starts_with(&format!("{first} is held: its release failed 2 times: ")))
            .count()
    };
    assert_eq!(tried(&said[0]), 1, "{}", said[0]);
    assert!(said[0].contains(RELEASE_FAILS), "{}", said[0]);
    assert_eq!(held(&said[1]), 1, "{}", said[1]);
    assert!(
        said[1].contains(&format!(
            "`sluice release {first} --accept-failure` frees it without"
        )),
        "{}",
        said[1]
    );
    assert!(!said[2].contains(&first), "{}", said[2]);
    assert_eq!(
        f.world
            .release_attempts()
            .iter()
            .filter(|p| **p == first)
            .count(),
        2
    );

    let (code, out) = sluice(&f, &["status", BRANCH]);
    assert_eq!(code, 0);
    assert!(
        out.contains("held checkouts, whose release failed:\n"),
        "{out}"
    );
    assert!(
        out.contains(&format!("  {first}  2 tries, held, last at ")),
        "{out}"
    );
    assert!(
        out.contains(&format!(
            "    | the release command of {first} ended exit status: 2: {RELEASE_FAILS}"
        )),
        "{out}"
    );
    assert!(
        out.contains("sluice release <checkout> --accept-failure"),
        "{out}"
    );
    let (_, out) = sluice(&f, &["status", BRANCH, "--json"]);
    let doc = json(&out);
    let held = doc["held_checkouts"]
        .as_array()
        .expect("held checkouts are listed");
    assert_eq!(held.len(), 1, "{doc}");
    assert_eq!(held[0]["place"], first);
    assert_eq!(held[0]["tries"], 2);
    assert_eq!(held[0]["stopped"], true);
    assert!(held[0]["failed"].as_str().unwrap().contains(RELEASE_FAILS));
    assert!(held[0]["at"].is_string());
}

/// 97, 123: a waiting submit says once that a removed worktree was freed
/// and that its stored release ran, or what that release printed when it
/// failed; the waits after say nothing of it and run nothing of it.
#[test]
fn a_waiting_submit_says_a_removed_worktree_was_freed_once() {
    for fails in [false, true] {
        let f = setup_releasing();
        work(
            &f,
            "unit/a",
            "agent-a",
            &[("plugin/lib/plan.py", Some("p2"))],
        );
        exec(&f, &[BRANCH, "--worktree", "unit/a"]);
        f.world.remove_worktree("unit/a");
        if fails {
            f.world
                .place_release_fails("session/unit/a", Some(RELEASE_FAILS));
        }
        work(
            &f,
            "unit/b",
            "agent-b",
            &[("plugin/lib/plan.py", Some("p3"))],
        );
        let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/b"]);
        assert_eq!(code, LANDED, "{out}");
        let said = match fails {
            false => GONE.to_string(),
            true => format!(
                "freed unit/a: it no longer exists; its release failed once from its scratch directory: \
                 the release command of session/unit/a ended exit status: 2: {RELEASE_FAILS}; \
                 its runs ended, its ports free"
            ),
        };
        assert_eq!(out.lines().filter(|l| *l == said).count(), 1, "{out}");
        work(
            &f,
            "unit/c",
            "agent-c",
            &[("plugin/lib/plan.py", Some("p4"))],
        );
        let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/c"]);
        assert_eq!(code, LANDED, "{out}");
        assert!(!out.contains("unit/a"), "{out}");
        assert!(f.world.session_releases().is_empty());
        assert_eq!(f.world.gone_releases(), ["session/unit/a"]);
    }
}

/// 123: `sluice status` marks a worktree whose release failed, with what
/// failed, until it is released or its failure accepted.
#[test]
fn status_marks_a_worktree_whose_release_failed() {
    let f = setup_releasing();
    work(
        &f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    exec(&f, &[BRANCH, "--worktree", "unit/a"]);
    f.world.release_fails(Some(RELEASE_FAILS));
    release(&f, &["--worktree", "unit/a"]);
    let (_, out) = sluice(&f, &["status"]);
    assert!(
        out.contains("    its release failed (1 try): sluice release runs it again"),
        "{out}"
    );
    assert!(out.contains(RELEASE_FAILS), "{out}");
    let (_, out) = sluice(&f, &["status", "--json"]);
    let doc = json(&out);
    let failed = &doc["worktrees"][0]["release_failed"];
    assert_eq!(failed["tries"], 1, "{doc}");
    assert_eq!(failed["stopped"], true, "{doc}");
}

/// The lines of `text` from the one that starts with `head` on.
fn from_line<'a>(text: &'a str, head: &str) -> Vec<&'a str> {
    text.lines().skip_while(|l| !l.starts_with(head)).collect()
}

/// 123, 26: `sluice overview` says nothing of a checkout its next round
/// tries again, which `--json` lists with `stopped` false; once its tries
/// are spent it names it under its branch's line, with the last line of
/// what failed and the two commands, and `--json` lists it held.
#[test]
fn overview_names_a_held_checkout_under_its_branch() {
    let f = setup_one_retry();
    let first = format!("sluice/try/{BRANCH}/1");
    f.world.place_release_fails(&first, Some(RELEASE_FAILS));
    work(
        &f,
        "unit/a",
        "unit/a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");

    let (_, text) = sluice(&f, &["overview"]);
    assert!(!text.contains(&first), "{text}");
    let (_, out) = sluice(&f, &["status", BRANCH]);
    assert!(out.contains("tried again by the next round"), "{out}");
    let (_, out) = sluice(&f, &["overview", "--json"]);
    let held = &json(&out)["branches"][0]["held_checkouts"];
    assert_eq!(held[0]["place"], first, "{held}");
    assert_eq!(held[0]["stopped"], false, "{held}");
    assert_eq!(held[0]["tries"], 1, "{held}");

    work(
        &f,
        "unit/b",
        "unit/b",
        &[("plugin/lib/plan.py", Some("p3"))],
    );
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/b"]);
    assert_eq!(code, LANDED, "{out}");
    let (_, text) = sluice(&f, &["overview"]);
    let lines = from_line(&text, &format!("{BRANCH} at "));
    assert!(
        lines[1].starts_with(&format!("  {first} is held: its release failed 2 times: "))
            && lines[1].ends_with(RELEASE_FAILS),
        "{text}"
    );
    assert_eq!(
        lines[2],
        format!(
            "    sluice release {first} runs it again; \
             sluice release {first} --accept-failure frees it without"
        ),
        "{text}"
    );
    let (_, out) = sluice(&f, &["overview", "--json"]);
    let doc = json(&out);
    let held = doc["branches"][0]["held_checkouts"]
        .as_array()
        .expect("held checkouts are listed");
    assert_eq!(held.len(), 1, "{doc}");
    assert_eq!(held[0]["place"], first);
    assert_eq!(held[0]["tries"], 2);
    assert_eq!(held[0]["stopped"], true);
    assert!(held[0]["failed"].as_str().unwrap().contains(RELEASE_FAILS));
    assert!(held[0]["at"].is_string());
    assert!(held[0].get("host").is_none(), "{doc}");
}

/// A worktree `unit/a` of `BRANCH` whose `sluice release` failed once,
/// which holds it, and its path as the overview names it.
fn held_worktree(f: &Fakes) -> String {
    work(
        f,
        "unit/a",
        "agent-a",
        &[("plugin/lib/plan.py", Some("p2"))],
    );
    exec(f, &[BRANCH, "--worktree", "unit/a"]);
    f.world.release_fails(Some(RELEASE_FAILS));
    release(f, &["--worktree", "unit/a"]);
    f.world.release_fails(None);
    let (_, out) = sluice(f, &["overview", "--json"]);
    let doc = json(&out);
    assert_eq!(
        doc["sessions"][0]["release_failed"]["stopped"], true,
        "{doc}"
    );
    doc["sessions"][0]["worktree"].as_str().unwrap().to_string()
}

/// 123, 26: a held worktree is named under its branch's line, with what
/// failed and `sluice release --worktree <path>`.
#[test]
fn overview_names_a_held_worktree_under_its_branch() {
    let f = setup_releasing();
    work(
        &f,
        "unit/b",
        "agent-b",
        &[("plugin/lib/plan.py", Some("p3"))],
    );
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/b"]);
    assert_eq!(code, LANDED, "{out}");
    let path = held_worktree(&f);
    let (_, text) = sluice(&f, &["overview"]);
    let lines = from_line(&text, &format!("{BRANCH} at "));
    assert!(
        lines[1].starts_with(&format!(
            "  worktree {path} is held: its release failed once: "
        )) && lines[1].ends_with(RELEASE_FAILS),
        "{text}"
    );
    assert_eq!(
        lines[2],
        format!(
            "    sluice release --worktree {path} runs it again; \
             sluice release --worktree {path} --accept-failure frees it without"
        ),
        "{text}"
    );
    assert!(!text.contains("no records"), "{text}");
}

/// 123: a held worktree of a branch with no records is named under a
/// line of its own for that branch.
#[test]
fn overview_names_a_held_worktree_of_a_branch_with_no_records() {
    let f = setup_releasing();
    let path = held_worktree(&f);
    let (code, text) = sluice(&f, &["overview"]);
    assert_eq!(code, 0);
    let lines = from_line(&text, &format!("{BRANCH} · no records"));
    assert!(lines.len() > 2, "{text}");
    assert!(
        lines[1].starts_with(&format!("  worktree {path} is held: ")),
        "{text}"
    );
    assert!(
        lines[2].starts_with(&format!(
            "    sluice release --worktree {path} runs it again"
        )),
        "{text}"
    );
}

// ---------------------------------------------------------------------------
// A branch read without its last landing is held (121)
// ---------------------------------------------------------------------------

/// Land one submission on `BRANCH`, then have the branch read at the
/// revision it was made at: that revision, and the landing it lacks.
fn read_behind(f: &Fakes) -> (String, String) {
    let made = f.world.tip(BRANCH);
    work(f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    let (code, out) = sluice(f, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    let landing = f.world.tip(BRANCH);
    f.world.set_branch(BRANCH, &made);
    (made.0, landing.0)
}

/// C.5 branch: `sluice branch accept` exits 0 when recorded, 1 for no
/// such branch and 2 on a refusal, naming it, as create and delete do.
#[test]
fn branch_accept_exits_as_create_and_delete() {
    let f = setup();
    let (made, landing) = read_behind(&f);
    f.world
        .set_branch(BRANCH, &sluice_seams::Rev::new(landing.clone()));
    assert_eq!(
        sluice(&f, &["branch", "accept", BRANCH, &landing]),
        (
            CONFLICT,
            format!(
                "refused: {BRANCH} is not held; it reads {landing}, which holds its last landing\n"
            )
        )
    );
    f.world
        .set_branch(BRANCH, &sluice_seams::Rev::new(made.clone()));
    sluice(&f, &["step", BRANCH]);
    assert_eq!(
        sluice(&f, &["branch", "accept", BRANCH, &landing]),
        (
            CONFLICT,
            format!("refused: {BRANCH} is held at {made}, not at {landing}\n")
        )
    );
    let (code, out) = sluice(&f, &["branch", "accept", BRANCH, &landing, "--json"]);
    assert_eq!(code, CONFLICT);
    assert_eq!(
        json(&out),
        serde_json::json!({"error": {
            "kind": "refused",
            "message": format!("{BRANCH} is held at {made}, not at {landing}"),
            "next": [format!("sluice status {BRANCH}")],
        }})
    );
    assert_eq!(
        sluice(&f, &["branch", "accept", "bolt/none", &made]),
        (FAILED, "no branch bolt/none\n".to_string())
    );

    let (code, out) = sluice(&f, &["branch", "accept", BRANCH, &made[..7], "--json"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        json(&out),
        serde_json::json!({ "branch": BRANCH, "accepted": made, "lacked": landing })
    );
    assert_eq!(
        sluice(&f, &["step", BRANCH]),
        (0, format!("{BRANCH}: stepped\n"))
    );
}

#[test]
fn branch_accept_says_what_it_recorded() {
    let f = setup();
    let (made, landing) = read_behind(&f);
    sluice(&f, &["step", BRANCH]);
    assert_eq!(
        sluice(&f, &["branch", "accept", BRANCH, &made]),
        (
            0,
            format!(
                "accepted {BRANCH} at {made}, which lacks the landing {landing}; \
                 the next round forms the queue on it\n"
            )
        )
    );
}

/// The line a held branch is said with.
fn held_text(read: &str, lacks: &str) -> String {
    format!(
        "held: {BRANCH} reads {}, which lacks the landing {}; put the landing back on \
         {BRANCH}, or run sluice branch accept {BRANCH} {}",
        &read[..7],
        &lacks[..7],
        &read[..7]
    )
}

/// 121: `sluice step` says the branch is held, with both revisions and
/// the two ways out; under `--json`, the outcome `held` and the hold.
#[test]
fn step_says_a_branch_is_held() {
    let f = setup();
    let (made, landing) = read_behind(&f);
    assert_eq!(
        sluice(&f, &["step", BRANCH]),
        (0, format!("{}\n", held_text(&made, &landing)))
    );
    let (code, out) = sluice(&f, &["step", BRANCH, "--json"]);
    assert_eq!(code, 0);
    let v = json(&out);
    assert_eq!(v["outcome"], "held");
    assert_eq!(
        (&v["held"]["read"], &v["held"]["lacks"]),
        (&serde_json::json!(made), &serde_json::json!(landing))
    );
    assert!(v["held"]["since"].is_string());
}

/// 121: `sluice run` says a hold once, not on every pass, and again for
/// another revision read lacking the landing.
#[test]
fn run_says_a_hold_once() {
    let f = setup();
    let (made, landing) = read_behind(&f);
    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/b", "--no-wait"]);
    // Read at another revision lacking it after the second pass.
    let beside = f.world.work(
        &sluice_seams::Rev::new(made.clone()),
        &[("plugin/c", Some("c"))],
    );
    let world = f.world.clone();
    let at = beside.clone();
    let pause = Then(Cell::new(0), move |n| {
        if n == 2 {
            world.set_branch(BRANCH, &at);
        }
        true
    });
    let (code, out) = sluice_with(&f, &pause, &["run", "--rounds", "5"]);
    assert_eq!(code, 0);
    let held: Vec<&str> = out.lines().filter(|l| l.starts_with("held: ")).collect();
    assert_eq!(
        held,
        [held_text(&made, &landing), held_text(&beside.0, &landing)],
        "{out}"
    );
}

/// 121: `sluice status <branch>` and `sluice overview` show the hold
/// above the queue, and under `held` in JSON, for as long as it lasts.
#[test]
fn status_and_overview_show_a_hold_while_it_lasts() {
    let f = setup();
    let (made, landing) = read_behind(&f);
    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/b", "--no-wait"]);
    sluice(&f, &["step", BRANCH]);
    let held = held_text(&made, &landing);

    let (code, out) = sluice(&f, &["status", BRANCH]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines,
        [
            format!(
                "{BRANCH} at {} · 1 in the queue · testing 0 at once (up to 4)",
                &landing[..7]
            ),
            held.clone(),
            "  #2  queued    agent-b  unit/b".to_string(),
            format!("1 no longer in the queue: sluice status {BRANCH} --all"),
        ],
        "{out}"
    );
    let v = json(&sluice(&f, &["status", BRANCH, "--json"]).1);
    assert_eq!(
        (&v["held"]["read"], &v["held"]["lacks"]),
        (&serde_json::json!(made), &serde_json::json!(landing))
    );

    let (_, out) = sluice(&f, &["overview"]);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines[0].starts_with(&format!("{BRANCH} at ")), "{out}");
    assert_eq!(lines[1], format!("  {held}"), "{out}");
    let v = json(&sluice(&f, &["overview", "--json"]).1);
    assert_eq!(v["branches"][0]["held"]["read"], serde_json::json!(made));
    assert_eq!(
        v["branches"][0]["held"]["lacks"],
        serde_json::json!(landing)
    );

    // The landing is put back: the next round ends the hold.
    f.world.set_branch(BRANCH, &sluice_seams::Rev::new(landing));
    sluice(&f, &["step", BRANCH]);
    let (_, out) = sluice(&f, &["status", BRANCH]);
    assert!(!out.contains("held: "), "{out}");
    let v = json(&sluice(&f, &["status", BRANCH, "--json"]).1);
    assert!(v["held"].is_null(), "{v}");
    let v = json(&sluice(&f, &["overview", "--json"]).1);
    assert!(v["branches"][0].get("held").is_none(), "{v}");
}

/// 121: a waiting `sluice wait` and `sluice submit` say a hold once in
/// their text; their `--json` record is the submission's, as before.
#[test]
fn a_waiting_submission_says_its_branchs_hold_once() {
    let f = setup();
    let (made, landing) = read_behind(&f);
    let held = held_text(&made, &landing);
    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/b", "--no-wait"]);

    let (code, out) = sluice(&f, &["wait", "2", "--timeout", "10s"]);
    assert_eq!(code, sluice::WAITING, "{out}");
    assert_eq!(out.lines().filter(|l| *l == held).count(), 1, "{out}");
    assert!(out.ends_with("still waiting: sluice wait 2\n"), "{out}");
    let (code, out) = sluice(&f, &["wait", "2", "--timeout", "10s", "--json"]);
    assert_eq!(code, sluice::WAITING, "{out}");
    let v = json(&out);
    assert_eq!(
        (v["id"].as_u64(), v["state"].as_str()),
        (Some(2), Some("queued"))
    );
    assert!(v.get("held").is_none(), "{v}");

    // A waiting submit, stopped after a few rounds.
    work(&f, "unit/c", "agent-c", &[("plugin/c", Some("c"))]);
    let stop = Then(Cell::new(0), |n| n < 4);
    let (code, out) = sluice_with(&f, &stop, &["submit", BRANCH, "--worktree", "unit/c"]);
    assert_eq!(code, sluice::STOPPED, "{out}");
    assert_eq!(out.lines().filter(|l| *l == held).count(), 1, "{out}");
    work(&f, "unit/d", "agent-d", &[("plugin/d", Some("d"))]);
    let stop = Then(Cell::new(0), |n| n < 4);
    let (code, out) = sluice_with(
        &f,
        &stop,
        &["submit", BRANCH, "--worktree", "unit/d", "--json"],
    );
    assert_eq!(code, sluice::STOPPED, "{out}");
    assert_eq!(json(&out)["state"], "queued");
}

/// What `test` says on stderr, which the harness keeps from the test
/// itself: it runs alone in a child of this binary, which must pass.
/// `None` in that child, which runs the test's body.
fn stderr_of(test: &str) -> Option<String> {
    if std::env::var("SLUICE_TEST_ALONE").as_deref() == Ok(test) {
        return None;
    }
    let out = support::isolated(&mut std::process::Command::new(
        std::env::current_exe().unwrap(),
    ))
    .args(["--exact", test, "--nocapture"])
    .env("SLUICE_TEST_ALONE", test)
    .output()
    .unwrap();
    let said = String::from_utf8_lossy(&out.stdout);
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        out.status.success() && said.contains(&format!("test {test} ... ok")),
        "{test}, alone:\n{said}{err}"
    );
    Some(err)
}

/// The lines of `err` saying what failed after a landing (122).
fn after_the_landing(err: &str) -> Vec<&str> {
    err.lines()
        .filter(|l| l.starts_with("sluice: after the landing: "))
        .collect()
}

/// 122, 123: a release that fails after the landing, in the round a
/// waiting submit ran, leaves the submission landed: the round goes on,
/// so nothing fails after the landing; the failed try is said, then the
/// outcome as for any landing, exit 0. The next round releases the place.
#[test]
fn a_release_failing_after_the_landing_leaves_it_landed() {
    if let Some(err) = stderr_of("a_release_failing_after_the_landing_leaves_it_landed") {
        assert!(after_the_landing(&err).is_empty(), "{err}");
        return;
    }
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    f.world.fail_next_release();
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    let tip = f.world.tip(BRANCH);
    assert_eq!(
        outcome(&out).last().copied(),
        Some(format!("passed · landed as {}", &tip.0[..7]).as_str()),
        "{out}"
    );
    let place = format!("sluice/try/{BRANCH}/1");
    assert!(
        out.contains(&format!(
            "could not release {place} (try 1 of 4): the release command of {place} failed\n"
        )),
        "{out}"
    );
    assert_eq!(f.world.live_places().len(), 1);
    sluice(&f, &["step", BRANCH]);
    assert!(f.world.live_places().is_empty());
}

/// 122: telling the outcome fails after the landing, in the round
/// `sluice wait` ran: landed, exit 0, and the telling's failure on stderr.
#[test]
fn telling_failing_after_the_landing_leaves_it_landed() {
    if let Some(err) = stderr_of("telling_failing_after_the_landing_leaves_it_landed") {
        assert_eq!(
            after_the_landing(&err),
            ["sluice: after the landing: crashed after telling 1:landed"],
            "{err}"
        );
        return;
    }
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    f.world.crash_after_next_notify();
    let (code, out) = sluice(&f, &["wait", "1"]);
    assert_eq!(code, LANDED, "{out}");
    let tip = f.world.tip(BRANCH);
    assert!(
        out.ends_with(&format!("passed · landed as {}\n", &tip.0[..7])),
        "{out}"
    );
}

/// 122, 123: under `--json`, a landing whose release then fails is one
/// document, the submission's record, landed; the failed try is on
/// stderr, and nothing failed the round.
#[test]
fn a_landing_whose_release_fails_is_one_document_under_json() {
    if let Some(err) = stderr_of("a_landing_whose_release_fails_is_one_document_under_json") {
        assert!(after_the_landing(&err).is_empty(), "{err}");
        let place = format!("sluice/try/{BRANCH}/1");
        assert!(
            err.contains(&format!("could not release {place} (try 1 of 4): ")),
            "{err}"
        );
        return;
    }
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/a", "--no-wait"]);
    f.world.fail_next_release();
    let (code, out) = sluice(&f, &["wait", "1", "--json"]);
    assert_eq!(code, LANDED, "{out}");
    let v = json(&out);
    assert_eq!(v["state"], "landed");
    assert_eq!(v["landed_as"], f.world.tip(BRANCH).0);
}

/// 122, 24: the round a waiting submit ran moves the branch and fails
/// writing that it landed: the submission is reported landed as the
/// landing's commit, and the next round records it so.
#[test]
fn a_landing_the_round_did_not_record_is_reported_landed() {
    if let Some(err) = stderr_of("a_landing_the_round_did_not_record_is_reported_landed") {
        assert_eq!(after_the_landing(&err).len(), 1, "{err}");
        return;
    }
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    // Between the submit's rounds, another process records the landing
    // and stops before the move; the submit's next round makes the move,
    // and its write confirming it fails.
    let between = |n: usize| {
        if n == 1 {
            f.world.crash(sluice_fakes::Crash::BeforeMove);
            assert!(f.sluice().step(BRANCH).is_err());
            f.world.crash_next_write();
        }
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 5,
        between: &between,
    };
    let (code, out) = sluice_with(&f, &pause, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    let tip = f.world.tip(BRANCH);
    assert!(
        out.ends_with(&format!("passed · landed as {}\n", &tip.0[..7])),
        "{out}"
    );
    let (_, out) = sluice(&f, &["status", "1", "--json"]);
    assert_eq!(json(&out)["state"], "landing");
    sluice(&f, &["step", BRANCH]);
    let (_, out) = sluice(&f, &["status", "1", "--json"]);
    assert_eq!(
        (
            json(&out)["state"].as_str(),
            json(&out)["landed_as"].as_str()
        ),
        (Some("landed"), Some(tip.0.as_str()))
    );
}

/// 122: a round that fails before the branch moved is sluice failing
/// itself, as before: the submission is not reported landed.
#[test]
fn a_round_failing_before_the_landing_is_an_error() {
    let f = setup();
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    let between = |n: usize| {
        if n == 1 {
            f.world.crash(sluice_fakes::Crash::BeforeMove);
        }
    };
    let pause = Between {
        n: Cell::new(0),
        rounds: 5,
        between: &between,
    };
    let cli = Cli::try_parse_from(["sluice", "submit", BRANCH, "--worktree", "unit/a"]).unwrap();
    let mut out = Vec::new();
    let host = support::FakeHost {
        world: f.world.clone(),
        reserved: 0,
    };
    let e = run(cli, f.seams(), &host, &(), &(), &f.clock, &pause, &mut out).unwrap_err();
    assert!(format!("{e:#}").contains("crashed before moving"), "{e:#}");
    let out = String::from_utf8(out).unwrap();
    assert!(!out.contains("landed as"), "{out}");
    let (_, out) = sluice(&f, &["status", "1", "--json"]);
    assert_eq!(json(&out)["state"], "landing");
}

// ---------------------------------------------------------------------------
// group-submit (125)
// ---------------------------------------------------------------------------

/// The setup's branch, testing only the deepest combination (14).
fn setup_batched() -> Fakes {
    let f = setup();
    let policy = POLICY.replace("parallel = 4\n", "parallel = 4\nbatch = true\n");
    let tip = f.world.tip(BRANCH);
    let at = f
        .world
        .work(&tip, &[(".config/sluice.toml", Some(&policy))]);
    f.world.set_branch(BRANCH, &at);
    f
}

/// `sluice submit <BRANCH> --worktree <w>...` for `worktrees`, then `rest`.
fn submit_group(f: &Fakes, worktrees: &[&str], rest: &[&str]) -> (i32, String) {
    let mut args = vec!["submit", BRANCH];
    for w in worktrees {
        args.extend(["--worktree", w]);
    }
    args.extend(rest);
    sluice(f, &args)
}

/// The combinations runs were started on, as lists of submissions.
fn tested_combinations(f: &Fakes) -> Vec<Vec<u64>> {
    let snapshot = sluice_seams::State::read(&f.state, BRANCH).unwrap();
    sluice_core::line::Line::read_log(&snapshot)
        .unwrap()
        .into_iter()
        .filter_map(|l| match l.event {
            sluice_core::records::Event::Testing { submissions, .. } => Some(submissions),
            _ => None,
        })
        .collect()
}

/// The submissions in the branch's records, as `status --json` lists them.
fn queued(f: &Fakes) -> Vec<serde_json::Value> {
    let (_, out) = sluice(f, &["status", BRANCH, "--json", "--all"]);
    json(&out)["submissions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// Three worktrees: three submissions numbered in the order given, each
/// named with its worktree, and nothing run; one worktree answers as a
/// submit always has; under `--json`, one document of them all.
#[test]
fn a_group_is_numbered_in_the_order_given() {
    let f = setup();
    work(&f, "unit/b1", "agent-1", &[("plugin/one", Some("1"))]);
    work(&f, "unit/b2", "agent-2", &[("plugin/two", Some("2"))]);
    work(&f, "unit/b3", "agent-3", &[("tests/three", Some("3"))]);
    let (code, out) = submit_group(&f, &["unit/b1", "unit/b2", "unit/b3"], &["--no-wait"]);
    assert_eq!(code, LANDED, "{out}");
    assert_eq!(
        out,
        format!(
            "submission 1 to {BRANCH} from unit/b1 · nothing queued ahead\n\
             submission 2 to {BRANCH} from unit/b2 · 1 queued ahead (#1)\n\
             submission 3 to {BRANCH} from unit/b3 · 2 queued ahead (#1, #2)\n"
        )
    );
    assert!(f.world.started().is_empty(), "submitting runs nothing");
    for (s, after) in queued(&f).iter().zip([[0u64; 0]; 3]) {
        assert_eq!(s["after"], serde_json::json!(after), "{s}");
    }

    work(&f, "unit/b4", "agent-4", &[("plugin/four", Some("4"))]);
    let (code, out) = submit_group(&f, &["unit/b4"], &["--no-wait"]);
    assert_eq!(code, LANDED);
    assert_eq!(
        out,
        format!("submission 4 to {BRANCH} · 3 queued ahead (#1, #2, #3)\n")
    );

    work(&f, "unit/b5", "agent-5", &[("plugin/five", Some("5"))]);
    work(&f, "unit/b6", "agent-6", &[("plugin/six", Some("6"))]);
    let (code, out) = submit_group(&f, &["unit/b5", "unit/b6"], &["--no-wait", "--json"]);
    assert_eq!(code, LANDED, "{out}");
    assert_eq!(
        json(&out),
        serde_json::json!({
            "branch": BRANCH,
            "submissions": [
                { "id": 5, "worktree": "unit/b5", "ahead": [1, 2, 3, 4] },
                { "id": 6, "worktree": "unit/b6", "ahead": [1, 2, 3, 4, 5] },
            ],
        })
    );
}

/// All or none: one worktree holding more than its commits, or one named
/// twice, refuses the act, naming it, and queues nothing for any; under
/// `--json`, the refused document naming each refused worktree.
#[test]
fn a_group_is_queued_whole_or_not_at_all() {
    let f = setup();
    work(&f, "unit/b1", "agent-1", &[("plugin/one", Some("1"))]);
    work(&f, "unit/b2", "agent-2", &[("plugin/two", Some("2"))]);
    f.world.dirty("unit/b2", &["plugin/scratch.py"]);
    let (code, out) = submit_group(&f, &["unit/b1", "unit/b2"], &[]);
    assert_eq!(code, CONFLICT, "{out}");
    assert_eq!(
        out,
        "refused: unit/b2 holds changes beyond its commits:\n  plugin/scratch.py\n\
         the agent working in unit/b2 commits or removes them and submits again\n\
         nothing submitted: the 2 worktrees are submitted together or not at all\n"
    );
    assert!(queued(&f).is_empty());

    let (code, out) = submit_group(&f, &["unit/b1", "unit/b2"], &["--json"]);
    assert_eq!(code, CONFLICT, "{out}");
    let v = json(&out);
    assert_eq!(v["error"]["kind"], "refused");
    assert_eq!(v["error"]["worktrees"], serde_json::json!(["unit/b2"]));
    assert!(v["error"]["message"]
        .as_str()
        .unwrap()
        .starts_with("unit/b2 holds changes beyond its commits"));
    assert_eq!(
        v["error"]["next"],
        serde_json::json!([format!(
            "sluice submit {BRANCH} --worktree unit/b1 --worktree unit/b2"
        )])
    );

    let (code, out) = submit_group(&f, &["unit/b1", "unit/b1"], &[]);
    assert_eq!(code, CONFLICT, "{out}");
    assert!(
        out.starts_with("refused: unit/b1 is named twice\n"),
        "{out}"
    );
    assert!(queued(&f).is_empty());
}

/// Several worktrees take neither `--check` nor `--status`: a command line
/// sluice cannot read, which writes nothing.
#[test]
fn several_worktrees_take_neither_check_nor_status() {
    let f = setup();
    work(&f, "unit/b1", "agent-1", &[("plugin/one", Some("1"))]);
    work(&f, "unit/b2", "agent-2", &[("plugin/two", Some("2"))]);
    for flag in ["--check", "--status"] {
        let (code, out) = submit_group(&f, &["unit/b1", "unit/b2"], &[flag]);
        assert_eq!(code, sluice::USAGE, "{out}");
        assert!(out.contains(&format!("{flag} takes one worktree")), "{out}");
    }
    assert!(queued(&f).is_empty());
}

/// S59: on a branch that batches, a waiting act of three is tested once as
/// one combination of all three and lands in one move, in the order given;
/// it reports each one's outcome in that order and exits 0. Under
/// `--json`, one document of their records.
#[test]
fn a_waiting_group_lands_together_and_reports_each_in_order() {
    let f = setup_batched();
    work(&f, "unit/b1", "agent-1", &[("plugin/one", Some("1"))]);
    work(&f, "unit/b2", "agent-2", &[("plugin/two", Some("2"))]);
    work(&f, "unit/b3", "agent-3", &[("tests/three", Some("3"))]);
    let (code, out) = submit_group(&f, &["unit/b1", "unit/b2", "unit/b3"], &[]);
    assert_eq!(code, LANDED, "{out}");
    assert_eq!(tested_combinations(&f), vec![vec![1, 2, 3]]);
    assert_eq!(f.world.moves().len(), 1, "one move");
    let lines = outcome(&out);
    let at = |header: &str| {
        lines
            .iter()
            .position(|l| *l == header)
            .unwrap_or_else(|| panic!("{header:?} in {out}"))
    };
    let heads = [
        at("submission 1 from unit/b1:"),
        at("submission 2 from unit/b2:"),
        at("submission 3 from unit/b3:"),
    ];
    assert!(heads.windows(2).all(|w| w[0] < w[1]), "{out}");
    let history = f.world.first_parents(BRANCH);
    for (k, h) in heads.iter().enumerate() {
        let end = heads.get(k + 1).copied().unwrap_or(lines.len());
        let landed = format!("passed · landed as {}", &history[2 - k].0[..7]);
        assert!(lines[*h..end].contains(&landed.as_str()), "{landed}: {out}");
    }

    work(&f, "unit/b4", "agent-4", &[("plugin/four", Some("4"))]);
    work(&f, "unit/b5", "agent-5", &[("plugin/five", Some("5"))]);
    let (code, out) = submit_group(&f, &["unit/b4", "unit/b5"], &["--json"]);
    assert_eq!(code, LANDED, "{out}");
    let v = json(&out);
    assert_eq!(v["branch"], BRANCH);
    let subs = v["submissions"].as_array().unwrap();
    assert_eq!(subs.len(), 2);
    for (s, (id, w)) in subs.iter().zip([(4, "unit/b4"), (5, "unit/b5")]) {
        assert_eq!(
            (s["id"].as_u64(), s["worktree"].as_str()),
            (Some(id), Some(w))
        );
        assert_eq!(s["state"], "landed");
    }
}

/// S59: the middle one conflicting with the first is set aside alone,
/// reported with its file and the submission it is contested with; the
/// others land, and the act exits 2, the middle one's exit. The middle
/// one blamed alone exits 1.
#[test]
fn a_waiting_group_exits_with_the_first_that_did_not_land() {
    let f = setup();
    work(
        &f,
        "unit/b1",
        "agent-1",
        &[("plugin/lib/plan.py", Some("one"))],
    );
    work(
        &f,
        "unit/b2",
        "agent-2",
        &[("plugin/lib/plan.py", Some("two"))],
    );
    work(&f, "unit/b3", "agent-3", &[("tests/three", Some("3"))]);
    let (code, out) = submit_group(&f, &["unit/b1", "unit/b2", "unit/b3"], &[]);
    assert_eq!(code, CONFLICT, "{out}");
    let lines = outcome(&out);
    let two = lines
        .iter()
        .position(|l| *l == "submission 2 from unit/b2:")
        .unwrap_or_else(|| panic!("{out}"));
    assert_eq!(
        lines[two..two + 5],
        [
            "submission 2 from unit/b2:",
            "conflict: plugin/lib/plan.py",
            "  conflicts with #1 (agent-1, unit/b1)",
            &format!(
                "the agent working in unit/b2 merges or rebases onto {BRANCH} there and submits \
                 again"
            ),
            "submission 3 from unit/b3:",
        ],
        "{out}"
    );
    assert!(
        lines[..two]
            .iter()
            .any(|l| l.starts_with("passed · landed as ")),
        "{out}"
    );
    assert!(
        lines[two..]
            .iter()
            .any(|l| l.starts_with("passed · landed as ")),
        "{out}"
    );
    assert!(tested_combinations(&f).iter().all(|c| !c.contains(&2)));

    f.world
        .fails_with("tests:t-batch-cards", "tests/t_cards.py", "broken");
    work(&f, "unit/b4", "agent-4", &[("plugin/four", Some("4"))]);
    work(
        &f,
        "unit/b5",
        "agent-5",
        &[("tests/t_cards.py", Some("broken"))],
    );
    work(&f, "unit/b6", "agent-6", &[("plugin/six", Some("6"))]);
    let (code, out) = submit_group(&f, &["unit/b4", "unit/b5", "unit/b6"], &["--json"]);
    assert_eq!(code, FAILED, "{out}");
    let states: Vec<String> = json(&out)["submissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["state"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(states, ["landed", "failed", "landed"]);
}

/// Each is its own submission once queued: one withdrawn leaves alone,
/// the other lands without it, and `sluice wait` on either reports that
/// one alone.
#[test]
fn each_of_a_group_is_withdrawn_and_waited_on_alone() {
    let f = setup();
    work(&f, "unit/b1", "agent-1", &[("plugin/one", Some("1"))]);
    work(&f, "unit/b2", "agent-2", &[("plugin/two", Some("2"))]);
    submit_group(&f, &["unit/b1", "unit/b2"], &["--no-wait"]);
    assert_eq!(sluice(&f, &["withdraw", "1", "--as", "agent-1"]).0, 0);
    let (code, out) = sluice(&f, &["wait", "2"]);
    assert_eq!(code, LANDED, "{out}");
    assert!(!out.contains("#1"), "{out}");
    assert!(tested_combinations(&f).iter().all(|c| !c.contains(&1)));
    let (code, out) = sluice(&f, &["wait", "1", "--json"]);
    assert_eq!(code, sluice::WITHDRAWN, "{out}");
    assert_eq!(json(&out)["id"], 1);
}

/// 6: a group's `--after` is every one's, so both wait for it and come in
/// the order given; a later submission of higher priority may come ahead
/// of a group, whose submissions keep their order.
#[test]
fn a_group_keeps_its_order_behind_after_and_priority() {
    let f = setup();
    f.world.hold_runs(true);
    work(&f, "unit/x", "agent-x", &[("plugin/x", Some("x"))]);
    sluice(&f, &["submit", BRANCH, "--worktree", "unit/x", "--no-wait"]);
    work(&f, "unit/b1", "agent-1", &[("plugin/one", Some("1"))]);
    work(&f, "unit/b2", "agent-2", &[("plugin/two", Some("2"))]);
    submit_group(&f, &["unit/b1", "unit/b2"], &["--no-wait", "--after", "1"]);
    for s in &queued(&f)[1..] {
        assert_eq!(s["after"], serde_json::json!([1]), "{s}");
    }
    work(&f, "unit/c", "agent-c", &[("tests/c", Some("c"))]);
    sluice(
        &f,
        &[
            "submit",
            BRANCH,
            "--worktree",
            "unit/c",
            "--no-wait",
            "--priority",
            "5",
        ],
    );
    sluice(&f, &["step", BRANCH]);
    assert_eq!(
        tested_combinations(&f),
        vec![vec![4], vec![4, 1], vec![4, 1, 2], vec![4, 1, 2, 3]]
    );
}

/// 125: rounds run beside the act, between its numbering and its write,
/// form nothing of it; the first combination formed holds all three, in
/// the order given.
#[test]
fn a_round_beside_a_group_forms_none_of_it_alone() {
    let f = setup_batched();
    work(&f, "unit/b1", "agent-1", &[("plugin/one", Some("1"))]);
    work(&f, "unit/b2", "agent-2", &[("plugin/two", Some("2"))]);
    work(&f, "unit/b3", "agent-3", &[("tests/three", Some("3"))]);
    f.world.after_allocating(|world, _| {
        let beside = Fakes::of(world.clone());
        assert_eq!(sluice(&beside, &["step", BRANCH]).0, 0);
    });
    let (code, out) = submit_group(&f, &["unit/b1", "unit/b2", "unit/b3"], &["--no-wait"]);
    assert_eq!(code, LANDED, "{out}");
    assert!(tested_combinations(&f).is_empty());
    sluice(&f, &["step", BRANCH]);
    assert_eq!(tested_combinations(&f), vec![vec![1, 2, 3]]);
}

/// Stopped with Ctrl-C, a waiting act exits 130 with its document as it
/// stands: every submission still queued.
#[test]
fn a_waiting_group_stopped_says_where_each_stands() {
    struct Stop;
    impl Pause for Stop {
        fn pause(&self) -> bool {
            false
        }
    }
    let f = setup();
    f.world.hold_runs(true);
    work(&f, "unit/b1", "agent-1", &[("plugin/one", Some("1"))]);
    work(&f, "unit/b2", "agent-2", &[("plugin/two", Some("2"))]);
    let args = [
        "submit",
        BRANCH,
        "--worktree",
        "unit/b1",
        "--worktree",
        "unit/b2",
        "--json",
    ];
    let (code, out) = sluice_with(&f, &Stop, &args);
    assert_eq!(code, sluice::STOPPED, "{out}");
    let v = json(&out);
    let ids: Vec<u64> = v["submissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_u64().unwrap())
        .collect();
    assert_eq!(ids, [1, 2]);
    assert!(v["submissions"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["state"] != "landed"));
    let (code, out) = sluice_with(
        &f,
        &Stop,
        &[
            "submit",
            BRANCH,
            "--worktree",
            "unit/b1",
            "--worktree",
            "unit/b2",
        ],
    );
    assert_eq!(code, sluice::STOPPED, "{out}");
    assert!(out.contains("still waiting: sluice wait 3"), "{out}");
}

/// 124, S64: `sluice status <branch>` shows this host's copy left behind
/// its landing above the queue, and under `copy` in JSON, until a round
/// levels it; then nothing of it.
#[test]
fn status_shows_this_hosts_copy_behind_while_it_lasts() {
    let f = setup();
    f.world.on_remote(BRANCH);
    let made = f.world.tip(BRANCH);
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    let landing = f.world.tip(BRANCH);
    let lock = format!("git holds the lock on refs/heads/{BRANCH}");
    f.world.set_copy(BRANCH, Some(&made));
    f.world.fail_levels(1, &lock);
    sluice(&f, &["step", BRANCH]);

    let (code, out) = sluice(&f, &["status", BRANCH]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[..2],
        [
            format!(
                "{BRANCH} at {} · 0 in the queue · testing 0 at once (up to 4)",
                &landing.0[..7]
            ),
            format!(
                "this host's {BRANCH} is at {}, behind the landing {}: {lock}",
                &made.0[..7],
                &landing.0[..7]
            ),
        ],
        "{out}"
    );
    let v = json(&sluice(&f, &["status", BRANCH, "--json"]).1);
    assert_eq!(
        (&v["copy"]["at"], &v["copy"]["lacks"], &v["copy"]["why"]),
        (
            &serde_json::json!(made),
            &serde_json::json!(landing),
            &serde_json::json!(lock)
        ),
        "{v}"
    );
    assert!(v["copy"]["since"].is_string(), "{v}");

    sluice(&f, &["step", BRANCH]);
    assert_eq!(f.world.copy_of(BRANCH), Some(landing));
    let (_, out) = sluice(&f, &["status", BRANCH]);
    assert!(!out.contains("this host's"), "{out}");
    let v = json(&sluice(&f, &["status", BRANCH, "--json"]).1);
    assert!(v["copy"].is_null(), "{v}");
}

/// 124, S58: a submit from a worktree that has checked out a bolt kept on
/// a remote submits the remote's revision and says how this host's copy
/// stands to it, in text and under `copy` in JSON; level, it says
/// nothing of it.
#[test]
fn submit_from_a_bolt_says_how_its_copy_stands() {
    let f = Fakes::new();
    let on_origin = POLICY.replace("parallel = 4\n", "parallel = 4\nremote = \"origin\"\n");
    f.world.init(
        BRANCH,
        &[
            (".config/sluice.toml", on_origin.as_str()),
            ("plugin/lib/plan.py", "plan"),
        ],
    );
    f.world.task("plugin:test", "plugin/");
    f.world.on_remote(BRANCH);
    f.world.init(
        "main",
        &[(
            ".config/sluice.toml",
            "[branches.main]\nrequired = [\"#gate\"]\nremote = \"origin\"\n",
        )],
    );
    let made = f.world.tip(BRANCH);
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    let landing = f.world.tip(BRANCH);
    f.world.set_copy(BRANCH, Some(&made));
    f.world.session(BRANCH, "ops", &made);

    let (code, out) = sluice(&f, &["submit", "main", "--worktree", BRANCH, "--no-wait"]);
    assert_eq!(code, 0, "{out}");
    let (m, l) = (&made.0[..7], &landing.0[..7]);
    assert_eq!(
        out,
        format!(
            "submission 2 to main · nothing queued ahead\n\
             this host's {BRANCH} is at {m}, behind origin's {l}; submitted {l}\n"
        )
    );
    let (_, out) = sluice(
        &f,
        &[
            "submit",
            "main",
            "--worktree",
            BRANCH,
            "--no-wait",
            "--json",
        ],
    );
    let v = json(&out);
    assert_eq!(
        v["copy"],
        serde_json::json!({
            "branch": BRANCH, "copy": made, "remote": "origin", "at": landing, "how": "behind",
        }),
        "{v}"
    );
    assert_eq!(f.world.copy_of(BRANCH), Some(made));

    f.world.session(BRANCH, "ops", &landing);
    let (_, out) = sluice(&f, &["submit", "main", "--worktree", BRANCH, "--no-wait"]);
    assert_eq!(out.lines().count(), 1, "{out}");
    let (_, out) = sluice(
        &f,
        &[
            "submit",
            "main",
            "--worktree",
            BRANCH,
            "--no-wait",
            "--json",
        ],
    );
    assert!(json(&out).get("copy").is_none(), "{out}");
}

/// 124: a waiting submit under `--json` ends with the submission's record,
/// which carries how this host's copy differed from what it submitted;
/// level, the record alone.
#[test]
fn a_waiting_submit_from_a_bolt_carries_its_copy_under_json() {
    let f = Fakes::new();
    let on_origin = POLICY.replace("parallel = 4\n", "parallel = 4\nremote = \"origin\"\n");
    f.world.init(
        BRANCH,
        &[
            (".config/sluice.toml", on_origin.as_str()),
            ("plugin/lib/plan.py", "plan"),
        ],
    );
    f.world.task("plugin:test", "plugin/");
    f.world.on_remote(BRANCH);
    f.world.init(
        "main",
        &[(
            ".config/sluice.toml",
            "[branches.main]\nrequired = [\"#gate\"]\nremote = \"origin\"\n",
        )],
    );
    let made = f.world.tip(BRANCH);
    work(&f, "unit/a", "agent-a", &[("plugin/a", Some("a"))]);
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/a"]);
    assert_eq!(code, LANDED, "{out}");
    let landing = f.world.tip(BRANCH);
    f.world.set_copy(BRANCH, Some(&made));
    f.world.session(BRANCH, "ops", &made);

    let (_, out) = sluice(&f, &["submit", "main", "--worktree", BRANCH, "--json"]);
    let v = json(&out);
    assert_eq!(
        (v["id"].as_u64(), &v["head"]),
        (Some(2), &serde_json::json!(landing)),
        "{v}"
    );
    assert_eq!(
        v["copy"],
        serde_json::json!({
            "branch": BRANCH, "copy": made, "remote": "origin", "at": landing, "how": "behind",
        }),
        "{v}"
    );
    // A group's document carries it on the record of the place it is of.
    let own = f.world.work(&f.world.tip("main"), &[("m/a", Some("a"))]);
    f.world.session("unit/m", "agent-m", &own);
    let args = [
        "submit",
        "main",
        "--worktree",
        BRANCH,
        "--worktree",
        "unit/m",
    ];
    let (_, out) = sluice(&f, &[&args[..], &["--json"]].concat());
    let v = json(&out);
    let subs = v["submissions"].as_array().expect("a group's records");
    assert_eq!(subs[0]["copy"]["how"], "behind", "{v}");
    assert!(subs[1].get("copy").is_none(), "{v}");

    work(&f, "unit/b", "agent-b", &[("plugin/b", Some("b"))]);
    let (code, out) = sluice(&f, &["submit", BRANCH, "--worktree", "unit/b"]);
    assert_eq!(code, LANDED, "{out}");
    let level = f.world.tip(BRANCH);
    f.world.session(BRANCH, "ops", &level);
    let (_, out) = sluice(&f, &["submit", "main", "--worktree", BRANCH, "--json"]);
    let v = json(&out);
    assert_eq!(v["head"], serde_json::json!(level), "{v}");
    assert!(v.get("copy").is_none(), "{v}");
}
