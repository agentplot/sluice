//! `sluice preview`, `sluice previews` and `sluice release <preview>` over
//! the fakes (previews-1 clauses 98 to 103).

mod support;

use std::cell::Cell;

use clap::Parser;
use sluice::{run, Cli, Pause, CONFLICT, FAILED, LANDED, STOPPED};
use sluice_fakes::Fakes;

const BRANCH: &str = "bolt/concurrent-units";

const POLICY: &str = "[checkout]\nports = [\"web\", \"api\"]\n\n\
[branches.\"bolt/*\"]\nrequired = [\"#gate\"]\nparallel = 4\n\n\
[preview]\ncommand = [\"moon\", \"run\", \"app:preview\"]\n\
addresses = { web = \"https://{name.web}.localhost\", api = \"http://localhost:{port.api}/docs\" }\n";

struct Then<F: Fn(usize) -> bool>(Cell<usize>, F);

impl<F: Fn(usize) -> bool> Pause for Then<F> {
    fn pause(&self) -> bool {
        self.0.set(self.0.get() + 1);
        assert!(self.0.get() < 50, "waited too long");
        (self.1)(self.0.get())
    }
}

fn setup() -> Fakes {
    let f = Fakes::new();
    f.world.init(
        BRANCH,
        &[
            (".config/sluice.toml", POLICY),
            ("plugin/lib/plan.py", "plan"),
        ],
    );
    f.world.task("plugin:test", "plugin/");
    f
}

fn sluice_with(f: &Fakes, reserved: usize, pause: &dyn Pause, args: &[&str]) -> (i32, String) {
    let cli = Cli::try_parse_from(std::iter::once("sluice").chain(args.iter().copied())).unwrap();
    let mut out = Vec::new();
    let host = support::FakeHost {
        world: f.world.clone(),
        reserved,
    };
    let code = run(cli, f.seams(), &host, &(), &(), &f.clock, pause, &mut out).unwrap();
    (code, String::from_utf8(out).unwrap())
}

fn sluice(f: &Fakes, args: &[&str]) -> (i32, String) {
    sluice_with(f, 0, &Then(Cell::new(0), |_| true), args)
}

/// The arguments: a branch and a revision; `previews`; `release` with a
/// preview or none.
#[test]
fn the_arguments() {
    assert!(Cli::try_parse_from(["sluice", "preview", BRANCH]).is_err());
    assert!(Cli::try_parse_from(["sluice", "preview", BRANCH, "abc", "--wait", "--json"]).is_ok());
    assert!(Cli::try_parse_from(["sluice", "previews", "--json"]).is_ok());
    assert!(Cli::try_parse_from(["sluice", "release", "preview/abc"]).is_ok());
    assert!(Cli::try_parse_from(["sluice", "release", "--worktree", "w"]).is_ok());
}

/// 98, 99, S30: the addresses are printed when `sluice preview` returns,
/// with the command started and not yet ended.
#[test]
fn a_preview_returns_its_addresses_once_its_command_starts() {
    let f = setup();
    f.world.hold_runs(true);
    let rev = f.world.tip(BRANCH);
    let (code, out) = sluice(&f, &["preview", BRANCH, rev.as_str()]);
    assert_eq!(code, LANDED, "{out}");
    let name = format!("preview/{}", &rev.as_str()[..12]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0],
        format!("{name} of {BRANCH} at {}: starting", rev.short())
    );
    let p = &f.world.live_previews()[0];
    assert_eq!(
        lines[1],
        format!(
            "  api  http://localhost:{}/docs",
            p.ports["SLUICE_PORT_API"]
        )
    );
    assert_eq!(
        lines[2],
        format!("  web  https://{}.localhost", p.names["SLUICE_NAME_WEB"])
    );
    assert!(lines[3].starts_with("  ports: SLUICE_PORT_API="), "{out}");
    let req = f.world.requests().pop().unwrap();
    assert_eq!(req.place.name, name);
    assert_eq!(
        req.tests["command"],
        sluice_seams::Setting::Text("moon run app:preview".into())
    );
    assert_eq!(req.place.env["SLUICE_NAME_WEB"], p.names["SLUICE_NAME_WEB"]);
    // 102: counted while it runs.
    assert_eq!(sluice_seams::Tests::sessions(&f.tests).unwrap(), 1);
    f.world.complete(&rev);
    let (_, out) = sluice(&f, &["previews"]);
    assert!(
        out.starts_with(&format!("{name} of {BRANCH} at {}: up\n", rev.short())),
        "{out}"
    );
    assert_eq!(
        sluice_seams::Tests::sessions(&f.tests).unwrap(),
        0,
        "an ended command counts against nothing"
    );
}

/// S35: a revision the branch has not landed is refused, and nothing is
/// made.
#[test]
fn an_unlanded_revision_is_refused() {
    let f = setup();
    let head = f
        .world
        .work(&f.world.tip(BRANCH), &[("plugin/x", Some("x"))]);
    let (code, out) = sluice(&f, &["preview", BRANCH, head.as_str()]);
    assert_eq!(code, CONFLICT);
    assert_eq!(
        out,
        format!(
            "refused: {BRANCH} has not landed {}: only what a branch has landed is previewed\n",
            head.short()
        )
    );
    assert!(f.world.live_previews().is_empty() && f.world.requests().is_empty());
    let (code, out) = sluice(&f, &["preview", "bolt/none", head.as_str()]);
    assert_eq!(
        (code, out.as_str()),
        (CONFLICT, "refused: no branch bolt/none\n")
    );
}

/// 100, S32: asking again gives the same preview and starts nothing.
#[test]
fn asking_again_gives_the_same_preview() {
    let f = setup();
    let rev = f.world.tip(BRANCH);
    let (_, first) = sluice(&f, &["preview", BRANCH, rev.as_str(), "--json"]);
    let (code, again) = sluice(&f, &["preview", BRANCH, rev.as_str(), "--json"]);
    assert_eq!(code, LANDED);
    let a: serde_json::Value = serde_json::from_str(&first).unwrap();
    let b: serde_json::Value = serde_json::from_str(&again).unwrap();
    assert_eq!(a, b);
    assert_eq!(a["state"], "up");
    assert_eq!(a["exit"], 0);
    assert_eq!(f.world.requests().len(), 1);
}

/// 102: with the bound full, the preview waits and starts when a run
/// ends; it never takes the reserve.
#[test]
fn a_preview_waits_for_room_and_never_takes_the_reserve() {
    let f = setup();
    f.world.host_limit(2);
    f.world.session_run("/w/other", true);
    let rev = f.world.tip(BRANCH);
    let world = f.world.clone();
    let pause = Then(Cell::new(0), move |n| {
        if n == 1 {
            assert!(world.started().is_empty(), "started into the reserve");
            world.kill_session("/w/other");
        }
        true
    });
    let (code, out) = sluice_with(&f, 1, &pause, &["preview", BRANCH, rev.as_str()]);
    assert_eq!(code, LANDED, "{out}");
    assert!(
        out.starts_with("waiting: the host is at its bound\n"),
        "{out}"
    );
    assert_eq!(f.world.started().len(), 1);
}

/// 102: stopped while it waits, nothing ran; asked again, the same
/// preview is taken up.
#[test]
fn a_preview_stopped_while_waiting_is_taken_up_again() {
    let f = setup();
    f.world.host_limit(1);
    f.world.session_run("/w/other", true);
    let rev = f.world.tip(BRANCH);
    let (code, out) = sluice_with(
        &f,
        0,
        &Then(Cell::new(0), |_| false),
        &["preview", BRANCH, rev.as_str()],
    );
    assert_eq!(code, STOPPED, "{out}");
    assert!(f.world.started().is_empty());
    let (_, out) = sluice(&f, &["previews"]);
    assert!(out.contains(": waiting\n"), "{out}");
    let names = f.world.live_previews()[0].names.clone();
    f.world.kill_session("/w/other");
    let (code, out) = sluice(&f, &["preview", BRANCH, rev.as_str()]);
    assert_eq!(code, LANDED, "{out}");
    assert_eq!(f.world.started().len(), 1);
    assert_eq!(f.world.live_previews()[0].names, names);
}

/// 101, 103, S33: a failed command is reported with what it printed and
/// released; `--wait` exits 1; nothing is recorded; it stays listed until
/// released.
#[test]
fn a_failed_preview_is_reported_and_released() {
    let f = setup();
    f.world.preview_fails(true);
    let rev = f.world.tip(BRANCH);
    let (code, out) = sluice(&f, &["preview", BRANCH, rev.as_str(), "--wait"]);
    assert_eq!(code, FAILED, "{out}");
    let name = format!("preview/{}", &rev.as_str()[..12]);
    assert!(
        out.starts_with(&format!(
            "released {name}: its command failed; `sluice previews` shows what it printed\n"
        )),
        "{out}"
    );
    assert!(
        out.contains(": failed\n") && out.contains("\n  | preview failed"),
        "{out}"
    );
    assert_eq!(f.world.results_log(), "");
    assert_eq!(f.world.preview_releases(), vec![(name.clone(), true)]);
    let (_, listed) = sluice(&f, &["previews", "--json"]);
    let docs: serde_json::Value = serde_json::from_str(&listed).unwrap();
    assert_eq!(docs[0]["state"], "failed");
    assert!(docs[0]["output"].as_str().unwrap().contains("failed"));
    let (code, out) = sluice(&f, &["release", &name]);
    assert_eq!(code, LANDED);
    assert!(out.starts_with(&format!("released {name}: ")), "{out}");
    assert_eq!(sluice(&f, &["previews"]).1, "no previews\n");
    assert_eq!(sluice(&f, &["release", &name]).1, "nothing to release\n");
}

/// 101: released by its revision; a preview that is up is never released
/// by a step.
#[test]
fn a_preview_is_released_only_when_asked() {
    let f = setup();
    let rev = f.world.tip(BRANCH);
    sluice(&f, &["preview", BRANCH, rev.as_str()]);
    let (code, _) = sluice(&f, &["step", BRANCH]);
    assert_eq!(code, 0);
    assert!(f.world.preview_releases().is_empty());
    let (_, out) = sluice(&f, &["release", rev.as_str()]);
    assert!(out.starts_with("released preview/"), "{out}");
    assert!(f.world.live_previews().is_empty());
}
