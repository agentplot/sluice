//! sluice's identity on GitHub, against a fake of GitHub's API on a port
//! the system picks (C.4 pushed refs). The tests that need git are in
//! the check-tools tier.

#[path = "../support/mod.rs"]
mod support;

#[path = "forge/github.rs"]
mod github;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, TimeDelta, Utc};
use serde_json::json;
use sluice::identity::Identity;
use sluice::profiles::github::check::{check, Finding};
use sluice::profiles::github::{App, Repo};
use sluice::retry::Policy;
use support::http::{json, Answer, Fake, Request};

const KEY: &[u8] = include_bytes!("forge/test-key.pem");
const APP: u64 = 4242;
const INSTALLATION: u64 = 7;

fn repo() -> Repo {
    Repo::from_url("https://github.com/o/r.git").unwrap()
}

/// A clock the test moves.
#[derive(Clone)]
struct Clock(Arc<Mutex<DateTime<Utc>>>);

impl Clock {
    fn new() -> Clock {
        Clock(Arc::new(Mutex::new(
            DateTime::parse_from_rfc3339("2026-10-14T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        )))
    }

    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }

    fn set(&self, t: DateTime<Utc>) {
        *self.0.lock().unwrap() = t;
    }
}

/// What the fake's GitHub holds for the app: its installation's
/// permissions, the repositories a token it gives reaches, and each
/// ruleset's bypass actors on `main`.
#[derive(Clone)]
struct Forge {
    permissions: serde_json::Value,
    reaches: serde_json::Value,
    rulesets: Vec<(u64, serde_json::Value)>,
    clock: Clock,
}

impl Forge {
    fn new(clock: &Clock) -> Forge {
        Forge {
            permissions: json!({
                "contents": "write", "workflows": "write", "checks": "write",
                "pull_requests": "write", "metadata": "read"
            }),
            reaches: json!([{ "full_name": "o/r" }]),
            rulesets: vec![(
                11,
                json!([{ "actor_id": APP, "actor_type": "Integration", "bypass_mode": "always" }]),
            )],
            clock: clock.clone(),
        }
    }

    /// The fake API; each token it gives is `token-<n>`, good for an hour.
    fn start(self) -> Fake {
        let minted = Mutex::new(0);
        Fake::start(move |r: &Request| self.answer(r, &minted))
    }

    /// The fake API's answer to `r`, `minted` the tokens it gave so far.
    fn answer(&self, r: &Request, minted: &Mutex<usize>) -> Answer {
        let path = r.path.as_str();
        if path == "/repos/o/r/installation" && r.method == "GET" {
            return json(
                200,
                json!({ "id": INSTALLATION, "app_id": APP, "permissions": self.permissions }),
            );
        }
        if path == format!("/app/installations/{INSTALLATION}/access_tokens") && r.method == "POST"
        {
            let mut n = minted.lock().unwrap();
            *n += 1;
            let ends = self.clock.now() + TimeDelta::hours(1);
            return json(
                201,
                json!({
                    "token": format!("token-{n}"), "expires_at": ends.to_rfc3339(),
                    "permissions": { "contents": "write", "metadata": "read" },
                    "repository_selection": "selected", "repositories": self.reaches
                }),
            );
        }
        if path == "/app" && r.method == "GET" {
            return json(200, json!({ "id": APP, "slug": "sluice-test" }));
        }
        if path == "/repos/o/r/rules/branches/main" {
            let rules: Vec<_> = self
                .rulesets
                .iter()
                .map(|(id, _)| json!({ "type": "update", "ruleset_id": id }))
                .collect();
            return json(200, json!(rules));
        }
        // Who may bypass a ruleset is shown to its admins alone.
        for (id, actors) in &self.rulesets {
            if path.starts_with(&format!("/repos/o/r/rulesets/{id}")) {
                let mut set = json!({ "id": id, "name": format!("ruleset {id}") });
                if r.header("Authorization") == Some("Bearer operator") {
                    set["bypass_actors"] = actors.clone();
                }
                return json(200, set);
            }
        }
        json(404, json!({ "message": "Not Found" }))
    }
}

/// The retry policy, its pauses skipped.
fn policy() -> Rc<Policy> {
    Rc::new(Policy::command().recording())
}

fn app(fake: &Fake, clock: &Clock) -> App {
    let clock = clock.clone();
    App::new(
        APP,
        KEY,
        fake.url(),
        Box::new(move || clock.now()),
        policy(),
    )
    .unwrap()
}

fn minted(fake: &Fake) -> usize {
    fake.requests()
        .iter()
        .filter(|r| r.path.ends_with("/access_tokens"))
        .count()
}

/// The app's login is its bot user, named after the slug GitHub gives
/// the app's own JWT, read once however often it is asked for.
#[test]
fn the_apps_login_is_read_once() {
    use sluice::profiles::github::forge::Forge as _;
    let clock = Clock::new();
    let fake = Forge::new(&clock).start();
    let app = app(&fake, &clock);
    assert_eq!(app.login().unwrap(), "sluice-test[bot]");
    assert_eq!(app.login().unwrap(), "sluice-test[bot]");
    let reads: Vec<_> = fake
        .requests()
        .into_iter()
        .filter(|r| r.path == "/app")
        .collect();
    assert_eq!(reads.len(), 1);
    let auth = reads[0].header("Authorization").unwrap_or_default();
    assert!(
        auth.starts_with("Bearer ") && !auth.starts_with("Bearer token-"),
        "{auth}"
    );
    assert_eq!(minted(&fake), 0);
}

/// A token is bought once and kept until five minutes before its hour
/// ends: over two hours, one is bought at 0, 56 and 112 minutes.
#[test]
fn a_token_is_bought_once_an_hour() {
    let clock = Clock::new();
    let fake = Forge::new(&clock).start();
    let app = app(&fake, &clock);
    let start = clock.now();
    let mut seen = Vec::new();
    for minute in 0..=120 {
        clock.set(start + TimeDelta::minutes(minute));
        let t = app.token(&repo()).unwrap();
        if seen.last() != Some(&t) {
            seen.push(t);
        }
    }
    assert_eq!(seen, ["token-1", "token-2", "token-3"]);
    assert_eq!(minted(&fake), 3);
    let bearer = fake.requests()[0]
        .header("Authorization")
        .unwrap()
        .to_string();
    assert!(bearer.starts_with("Bearer ey"), "{bearer}");
}

/// A failure GitHub reports is an error that names it.
#[test]
fn a_refused_token_says_why() {
    let fake = Fake::start(|_| {
        json(
            401,
            json!({ "message": "A JSON web token could not be decoded" }),
        )
    });
    let app = app(&fake, &Clock::new());
    let err = format!("{:#}", app.token(&repo()).unwrap_err());
    assert!(
        err.contains("401 A JSON web token could not be decoded"),
        "{err}"
    );
}

/// A clock that moves ten seconds on at each reading.
fn ticking(clock: &Clock) -> sluice::profiles::github::app::Now {
    let clock = clock.clone();
    Box::new(move || {
        let t = clock.now();
        clock.set(t + TimeDelta::seconds(10));
        t
    })
}

/// The credentials for a remote on GitHub say what their token is: the
/// app, the repository, the installation it was bought from, the scope
/// GitHub gave it, and its age; renewed, they are a token bought then,
/// which the next call is given too.
#[test]
fn credentials_say_what_their_token_is_and_renew_buys_one() {
    let clock = Clock::new();
    let fake = Forge::new(&clock).start();
    let app = App::new(APP, KEY, fake.url(), ticking(&clock), policy()).unwrap();
    let url = "https://github.com/o/r.git";
    let c = app.credentials(url).unwrap();
    assert_eq!(c.password, "token-1");
    let scope = "scoped to o/r with contents: write, metadata: read";
    assert!(
        c.about
            .starts_with("app 4242's token for o/r from installation 7"),
        "{}",
        c.about
    );
    assert!(c.about.contains(scope), "{}", c.about);
    assert!(c.about.ends_with("s ago"), "{}", c.about);
    assert!(!c.about.contains("token-1"), "{}", c.about);
    assert_eq!(app.credentials(url).unwrap().password, "token-1");
    let fresh = app.renew(url).unwrap();
    assert_eq!(fresh.password, "token-2");
    assert!(fresh.about.contains(scope), "{}", fresh.about);
    assert_eq!(app.credentials(url).unwrap().password, "token-2");
    assert_eq!(minted(&fake), 2);
    let asked: Vec<_> = fake
        .requests()
        .into_iter()
        .filter(|r| r.path.ends_with("/access_tokens"))
        .map(|r| serde_json::from_str::<serde_json::Value>(&r.body).unwrap())
        .collect();
    let alone = json!({ "repositories": ["r"] });
    assert_eq!(asked, [alone.clone(), alone], "for o/r alone");
}

/// A token GitHub gives that does not reach the repository is refused.
#[test]
fn a_token_for_another_repository_is_refused() {
    let clock = Clock::new();
    let mut forge = Forge::new(&clock);
    forge.reaches = json!([{ "full_name": "o/other" }]);
    let fake = forge.start();
    let err = format!("{:#}", app(&fake, &clock).token(&repo()).unwrap_err());
    assert!(
        err.contains("GitHub gave a token for o/r that reaches o/other, not o/r"),
        "{err}"
    );
}

fn failures(found: &[Finding]) -> Vec<&str> {
    found
        .iter()
        .filter(|f| !f.ok)
        .map(|f| f.what.as_str())
        .collect()
}

/// All set: the app is installed to write contents, workflows, checks and
/// pull requests and is the only bypass of the branch's ruleset.
#[test]
fn forge_check_all_set() {
    let clock = Clock::new();
    let fake = Forge::new(&clock).start();
    let found = check(&app(&fake, &clock), &repo(), "main", Some("operator"));
    assert!(failures(&found).is_empty(), "{found:?}");
    assert_eq!(found.len(), 6, "{found:?}");
}

/// A ruleset that lets the operator's role through beside the app fails,
/// naming the extra bypass.
#[test]
fn forge_check_names_an_extra_bypass() {
    let clock = Clock::new();
    let mut forge = Forge::new(&clock);
    forge.rulesets[0].1 = json!([
        { "actor_id": APP, "actor_type": "Integration", "bypass_mode": "always" },
        { "actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always" },
    ]);
    let fake = forge.start();
    let found = check(&app(&fake, &clock), &repo(), "main", Some("operator"));
    let f = failures(&found);
    assert_eq!(f.len(), 1, "{found:?}");
    assert!(
        f[0].contains("lets more than the app through: RepositoryRole 5"),
        "{}",
        f[0]
    );
}

/// Each other failure is named: a ruleset without the app, contents the
/// app may only read, workflows it may not write, no ruleset at all, and no token of the operator's
/// or one that cannot see the bypass list.
#[test]
fn forge_check_names_each_failure() {
    let clock = Clock::new();
    let one = |forge: Forge, operator: Option<&str>| -> Vec<String> {
        let fake = forge.start();
        let found = check(&app(&fake, &clock), &repo(), "main", operator);
        failures(&found).into_iter().map(String::from).collect()
    };

    let mut f = Forge::new(&clock);
    f.rulesets[0].1 =
        json!([{ "actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always" }]);
    let got = one(f, Some("operator"));
    assert!(got[0].contains("does not let app 4242 through"), "{got:?}");

    let mut f = Forge::new(&clock);
    f.permissions = json!({
        "contents": "read", "workflows": "write", "checks": "write", "pull_requests": "write"
    });
    let got = one(f, Some("operator"));
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        got[0].contains("may not write o/r's contents (contents: read)"),
        "{got:?}"
    );

    let mut f = Forge::new(&clock);
    f.permissions = json!({ "contents": "write", "checks": "write", "pull_requests": "write" });
    let got = one(f, Some("operator"));
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        got[0].contains(
            "may not write o/r's workflows (workflows: none): give it Workflows read and write"
        ),
        "{got:?}"
    );

    // ADR 0044: no permission to post checks.
    let mut f = Forge::new(&clock);
    f.permissions = json!({
        "contents": "write", "workflows": "write", "checks": "read", "pull_requests": "write"
    });
    let got = one(f, Some("operator"));
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        got[0].contains("may not write o/r's checks (checks: read): give it Checks read and write"),
        "{got:?}"
    );

    let mut f = Forge::new(&clock);
    f.rulesets.clear();
    let got = one(f, Some("operator"));
    assert!(
        got[0].contains("no ruleset on GitHub protects main"),
        "{got:?}"
    );

    let got = one(Forge::new(&clock), None);
    assert!(got[0].contains("set GH_TOKEN"), "{got:?}");

    let got = one(Forge::new(&clock), Some("not-an-admin"));
    assert!(got[0].contains("cannot see who may bypass"), "{got:?}");

    let fake = Fake::start(|_| json(404, json!({ "message": "Not Found" })));
    let found = check(&app(&fake, &clock), &repo(), "main", Some("operator"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].what.contains("makes no token for o/r"),
        "{found:?}"
    );
}

/// A repository whose `main` lives on GitHub as configured, and on a bare
/// repository on this host as git reaches it; the host's config and the
/// fake API are the test's own.
#[cfg(feature = "tools")]
struct OnGitHub {
    repo: support::Repo,
    _bare: support::Scratch,
    config: support::Scratch,
    fake: Fake,
}

#[cfg(feature = "tools")]
impl OnGitHub {
    fn new(name: &str, forge: Forge) -> OnGitHub {
        let repo = support::Repo::new(name);
        repo.write(
            ".config/sluice.toml",
            "[tests]\nrunner = \"command\"\ncommand = \"true\"\n\n[branches.main]\nremote = \"origin\"\n",
        );
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "sluice's policy"]);
        let bare = repo.bare_origin();
        repo.git(&["remote", "set-url", "origin", "https://github.com/o/r.git"]);
        repo.git(&[
            "config",
            &format!("url.{}.insteadOf", bare.path().display()),
            "https://github.com/o/r.git",
        ]);
        OnGitHub {
            repo,
            _bare: bare,
            config: support::Scratch::new(&format!("{name}-config")),
            fake: forge.start(),
        }
    }

    /// The host's `[forge]` names the app, its key with mode `0600`.
    fn give_forge(&self) {
        use std::os::unix::fs::PermissionsExt;
        let dir = self.config.path().join("sluice");
        std::fs::create_dir_all(&dir).unwrap();
        let key = dir.join("app.pem");
        std::fs::write(&key, KEY).unwrap();
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(
            dir.join("config.toml"),
            format!(
                "[forge]\napp_id = {APP}\nkey = {:?}\n",
                key.display().to_string()
            ),
        )
        .unwrap();
    }

    /// `sluice <args>`: its exit and what it said. The operator's token is
    /// the fake's, so nothing asks gh for the user's.
    fn sluice(&self, args: &[&str]) -> (i32, String) {
        let out = support::isolated(&mut std::process::Command::new(env!(
            "CARGO_BIN_EXE_sluice"
        )))
        .current_dir(self.repo.dir())
        .args(args)
        .env("XDG_CONFIG_HOME", self.config.path())
        .env("SLUICE_FORGE_API", self.fake.url())
        .env("GH_TOKEN", "operator")
        .env_remove("GITHUB_TOKEN")
        .env_remove("GIT_DIR")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        (out.status.code().unwrap_or(-1), text)
    }
}

/// `sluice status` on a branch whose `remote` is GitHub's: with no
/// `[forge]`, it says sluice has no identity of its own; with one, it
/// names the app, whose token the fetch was made with.
#[cfg(feature = "tools")]
#[test]
fn status_says_who_pushes_the_branch() {
    let s = OnGitHub::new("forge-status", Forge::new(&Clock::new()));
    let (code, without) = s.sluice(&["status", "main"]);
    assert_eq!(code, 0, "{without}");
    assert!(
        without.contains(
            "pushed to origin with this host's own git credentials: \
             sluice has no identity of its own there"
        ),
        "{without}"
    );
    assert_eq!(minted(&s.fake), 0);

    s.give_forge();
    let (code, with) = s.sluice(&["status", "main"]);
    assert_eq!(code, 0, "{with}");
    assert!(
        with.contains(&format!("pushed to origin as GitHub app {APP}")),
        "{with}"
    );
    assert_eq!(
        minted(&s.fake),
        1,
        "the fetch was made with the app's token"
    );
}

/// `sluice forge check main` exits 1 without `[forge]` or with a second
/// bypass, naming it, and 0 when all is set.
#[cfg(feature = "tools")]
#[test]
fn forge_check_exits_by_what_it_finds() {
    let clock = Clock::new();
    let s = OnGitHub::new("forge-check", Forge::new(&clock));
    let (code, said) = s.sluice(&["forge", "check", "main"]);
    assert_eq!(code, 1, "{said}");
    assert!(said.contains("no [forge] table"), "{said}");
    let (code, said) = s.sluice(&["forge", "check", "main", "--json"]);
    assert_eq!(code, 1, "{said}");
    let doc: serde_json::Value = serde_json::from_str(&said).unwrap();
    assert_eq!(
        (&doc["branch"], &doc["ok"]),
        (&json!("main"), &json!(false))
    );
    assert_eq!(doc["checks"][0]["ok"], false);
    assert!(
        doc["checks"][0]["message"]
            .as_str()
            .unwrap()
            .contains("no [forge] table"),
        "{said}"
    );

    s.give_forge();
    let (code, said) = s.sluice(&["forge", "check"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains(&format!("lets only app {APP} through")),
        "{said}"
    );
    let (code, said) = s.sluice(&["forge", "check", "--json"]);
    assert_eq!(code, 0, "{said}");
    let doc: serde_json::Value = serde_json::from_str(&said).unwrap();
    assert_eq!(doc["ok"], true);
    let checks = doc["checks"].as_array().unwrap();
    assert!(checks.iter().all(|c| c["ok"] == true), "{said}");
    assert!(
        checks.iter().any(|c| c["message"]
            .as_str()
            .unwrap()
            .contains(&format!("lets only app {APP} through"))),
        "{said}"
    );

    let mut extra = Forge::new(&clock);
    extra.rulesets[0].1 = json!([
        { "actor_id": APP, "actor_type": "Integration", "bypass_mode": "always" },
        { "actor_id": 1, "actor_type": "OrganizationAdmin", "bypass_mode": "always" },
    ]);
    let s = OnGitHub::new("forge-check-extra", extra);
    s.give_forge();
    let (code, said) = s.sluice(&["forge", "check", "main"]);
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains("lets more than the app through: OrganizationAdmin 1"),
        "{said}"
    );
}

/// The app's pull request calls against the fake API, answered from the
/// file-backed fake: what a person does there reads back through the app
/// as GitHub would show it, and what the app posts lands there.
#[test]
fn the_apps_pull_request_calls() {
    use sluice::profiles::github::forge::{approvals, Forge as _, Posting, Standing, Status};
    let dir = support::Scratch::new("forge-pulls");
    let gh = github::GitHub::start(&github::file(dir.path()), None);
    let app = App::new(APP, KEY, gh.url(), Box::new(Utc::now), policy()).unwrap();
    let f = &gh.forge;
    let r = repo();
    let a = f
        .open_by("ann", "main", "unit/a", "aaa", false, None)
        .unwrap();
    let d = f
        .open_by("dan", "main", "unit/d", "ddd", true, Some("dan/r"))
        .unwrap();
    f.open_by("cat", "bolt/x", "unit/c", "ccc", false, None)
        .unwrap();
    let pulls = app.pulls(&r, "main").unwrap();
    assert_eq!(pulls.iter().map(|p| p.number).collect::<Vec<_>>(), [a, d]);
    assert_eq!(pulls[0].author, "ann");
    assert_eq!(pulls[0].head, "aaa");
    assert!(pulls[0].from_repo(&r) && !pulls[1].from_repo(&r) && pulls[1].draft);
    assert_eq!(app.pull(&r, 99).unwrap(), None);

    f.review(a, "bob", "APPROVED", "old").unwrap();
    f.review(a, "cat", "APPROVED", "aaa").unwrap();
    assert_eq!(approvals(&app.reviews(&r, a).unwrap(), "aaa"), 1);
    f.check("aaa", "qa/approved", Standing::Passed).unwrap();
    f.check("aaa", "ci", Standing::Pending).unwrap();
    let checks = app.checks(&r, "aaa").unwrap();
    assert!(checks
        .iter()
        .any(|c| c.name == "qa/approved" && c.standing == Standing::Passed));
    assert!(checks
        .iter()
        .any(|c| c.name == "ci" && c.standing == Standing::Pending));
    f.set_draft(d, false).unwrap();
    assert_eq!(app.readied(&r, d).unwrap().len(), 1);

    // sluice's own check run, added then updated in place, and never
    // counted among other systems' checks.
    assert_eq!(app.posted(&r, "aaa").unwrap(), None);
    let waiting = Posting {
        status: Status::Queued,
        title: "waiting".into(),
        summary: "needs qa/approved".into(),
        text: None,
    };
    app.post(&r, "aaa", None, &waiting).unwrap();
    let first = app.posted(&r, "aaa").unwrap().unwrap();
    assert_eq!(first.posting, waiting);
    let landed = Posting {
        status: Status::Success,
        title: "landed".into(),
        summary: String::new(),
        text: Some("## sluice: landed\n| | Check |".into()),
    };
    app.post(&r, "aaa", Some(first.id), &landed).unwrap();
    assert_eq!(app.posted(&r, "aaa").unwrap().unwrap().posting, landed);
    assert_eq!(f.held().unwrap().posts.len(), 2);
    assert!(app
        .checks(&r, "aaa")
        .unwrap()
        .iter()
        .all(|c| c.name != "sluice"));

    app.comment(&r, a, "blamed").unwrap();
    assert_eq!(app.comments(&r, a).unwrap(), ["blamed"]);
    // ADR 0044: the check's text reads back, a check per check is sluice's
    // and no other system's, and a comment is edited in place.
    app.post_named(&r, "aaa", "sluice / a:test (linux-x64)", None, &waiting)
        .unwrap();
    let names: Vec<String> = app
        .posted_all(&r, "aaa")
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert!(
        names.contains(&"sluice / a:test (linux-x64)".to_string()),
        "{names:?}"
    );
    assert!(app
        .checks(&r, "aaa")
        .unwrap()
        .iter()
        .all(|c| !c.name.starts_with("sluice")));
    let c = app.comment_list(&r, a).unwrap().remove(0);
    assert!(
        c.url.contains(&format!("issuecomment-{}", c.id)),
        "{}",
        c.url
    );
    app.edit_comment(&r, c.id, "edited").unwrap();
    assert_eq!(app.comments(&r, a).unwrap(), ["edited"]);
    let opened = app.open(&r, "main", "unit/e", "E", "body", false).unwrap();
    assert!(opened.open && !opened.draft && opened.title == "E");
    let drafted = app.open(&r, "main", "unit/f", "F", "", true).unwrap();
    assert!(drafted.open && drafted.draft);
    app.edit(&r, opened.number, None, "new body").unwrap();
    let edited = app.pull(&r, opened.number).unwrap().unwrap();
    assert_eq!(
        (edited.title.as_str(), edited.body.as_str()),
        ("E", "new body")
    );
    app.edit(&r, opened.number, Some("E, retitled"), "new body")
        .unwrap();
    assert_eq!(
        app.pull(&r, opened.number).unwrap().unwrap().title,
        "E, retitled"
    );
}

/// The same calls against GitHub itself, as a scratch app on a scratch
/// repository: opt in with `SLUICE_GITHUB_TEST_REPO` (`owner/name`),
/// `SLUICE_GITHUB_TEST_APP` (the app's id), `SLUICE_GITHUB_TEST_KEY` (its
/// key) and `SLUICE_GITHUB_TEST_HEAD` (a branch of the repository with a
/// commit its default branch `SLUICE_GITHUB_TEST_BASE`, `main` if unset,
/// lacks). It opens a pull request from that branch, or uses the one
/// open, posts sluice's check run on its head twice and comments once.
/// Skipped when unset.
#[test]
fn live_pull_request_calls() {
    use sluice::profiles::github::forge::{Forge as _, Posting, Status};
    let Ok(name) = std::env::var("SLUICE_GITHUB_TEST_REPO") else {
        return;
    };
    let var = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("{k} is unset"));
    let r = Repo::from_url(&format!("https://github.com/{name}.git")).unwrap();
    let key = std::fs::read(var("SLUICE_GITHUB_TEST_KEY")).unwrap();
    let app = App::new(
        var("SLUICE_GITHUB_TEST_APP").parse().unwrap(),
        &key,
        sluice::profiles::github::app::API,
        Box::new(Utc::now),
        Rc::new(Policy::command()),
    )
    .unwrap();
    let base = std::env::var("SLUICE_GITHUB_TEST_BASE").unwrap_or_else(|_| "main".into());
    let head = var("SLUICE_GITHUB_TEST_HEAD");
    let pull = match app
        .pulls(&r, &base)
        .unwrap()
        .into_iter()
        .find(|p| p.head_ref == head)
    {
        Some(p) => p,
        None => app
            .open(
                &r,
                &base,
                &head,
                "sluice live test",
                "opened by sluice's live test",
                false,
            )
            .unwrap(),
    };
    app.reviews(&r, pull.number).unwrap();
    app.checks(&r, &pull.head).unwrap();
    app.readied(&r, pull.number).unwrap();
    let existing = app.posted(&r, &pull.head).unwrap().map(|p| p.id);
    let waiting = Posting {
        status: Status::Queued,
        title: "waiting".into(),
        summary: "sluice's live test".into(),
        text: None,
    };
    app.post(&r, &pull.head, existing, &waiting).unwrap();
    let posted = app.posted(&r, &pull.head).unwrap().unwrap();
    assert_eq!(posted.posting, waiting);
    let landed = Posting {
        status: Status::Success,
        title: "landed".into(),
        summary: String::new(),
        text: Some("## sluice: landed\n| | Check |".into()),
    };
    app.post(&r, &pull.head, Some(posted.id), &landed).unwrap();
    assert_eq!(app.posted(&r, &pull.head).unwrap().unwrap().posting, landed);
    app.comment(&r, pull.number, "sluice's live test").unwrap();
    assert!(app
        .comments(&r, pull.number)
        .unwrap()
        .iter()
        .any(|c| c == "sluice's live test"));
    app.edit(&r, pull.number, None, "edited by sluice's live test")
        .unwrap();
}

/// 88, 107, 111: GitHub Actions as a pool, against the fake API: a run is
/// one dispatch of the pool's workflow on the signer's branch, with the
/// runner of its system and its claim; a job is alive while a queued or
/// running run of the workflow bears its claim in its title; a wake is a
/// dispatch to advance the branch.
#[test]
fn the_pools_calls() {
    use sluice::profiles::checks::Launcher;
    use sluice::profiles::github::actions::Actions;
    use sluice_seams::{Launch, Rev, Setting};
    use std::collections::BTreeMap;

    let clock = Clock::new();
    let tokens = Forge::new(&clock);
    let fake = Fake::start(move |r: &Request| {
        let path = r.path.as_str();
        if path == "/repos/o/r/installation" || path.ends_with("/access_tokens") {
            return if path.ends_with("/access_tokens") {
                json(
                    201,
                    json!({ "token": "token-1", "expires_at": (tokens.clock.now() + TimeDelta::hours(1)).to_rfc3339() }),
                )
            } else {
                json(
                    200,
                    json!({ "id": INSTALLATION, "app_id": APP, "permissions": tokens.permissions }),
                )
            };
        }
        if path == "/repos/o/r/actions/workflows/sluice.yml/dispatches" && r.method == "POST" {
            return (204, vec![], String::new());
        }
        if path.starts_with("/repos/o/r/actions/workflows/sluice.yml/runs?status=in_progress") {
            return json(
                200,
                json!({ "workflow_runs": [{ "display_title": "sluice pool/forge/7" }] }),
            );
        }
        if path.starts_with("/repos/o/r/actions/workflows/sluice.yml/runs") {
            return json(200, json!({ "workflow_runs": [] }));
        }
        json(404, json!({ "message": "Not Found" }))
    });
    let dir = support::Scratch::new("pool-repo");
    for args in [
        &["init", "-q", "."][..],
        &["remote", "add", "origin", "https://github.com/o/r.git"][..],
    ] {
        let out = support::git_command()
            .current_dir(dir.path())
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success());
    }
    let table = |pairs: &[(&str, &str)]| -> (String, Setting) {
        let t: BTreeMap<String, Setting> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), Setting::Text(v.to_string())))
            .collect();
        ("github".into(), Setting::Tables(vec![t]))
    };
    let kind = table(&[("workflow", ".github/workflows/sluice.yml")]);
    let pool = Actions::new(Rc::new(app(&fake, &clock)), dir.path());
    pool.launch(&Launch {
        pool: "forge".into(),
        kind: kind.clone(),
        signer: table(&[
            ("workflow", ".github/workflows/sluice.yml"),
            ("ref", "refs/heads/main"),
        ]),
        runner: "macos-15".into(),
        remote: Some("origin".into()),
        branch: "main".into(),
        candidate: "c3".into(),
        combination: Rev::new("c0ffee"),
        system: Some("macos-arm64".into()),
        claim: "pool/forge/7".into(),
    })
    .unwrap();
    let sent = fake
        .requests()
        .into_iter()
        .find(|r| r.path.ends_with("/dispatches"))
        .unwrap();
    let body: serde_json::Value = serde_json::from_str(&sent.body).unwrap();
    assert_eq!(body["ref"], "main");
    assert_eq!(body["inputs"]["mode"], "run");
    assert_eq!(body["inputs"]["runner"], "macos-15");
    assert_eq!(body["inputs"]["claim"], "pool/forge/7");
    assert_eq!(body["inputs"]["combination"], "c0ffee");
    assert!(pool
        .job_alive(&kind, "pool/forge/7", Some("origin"))
        .unwrap());
    assert!(!pool
        .job_alive(&kind, "pool/forge/8", Some("origin"))
        .unwrap());
    pool.wake(&kind, "main", Some("origin")).unwrap();
    let wake = fake
        .requests()
        .into_iter()
        .rfind(|r| r.path.ends_with("/dispatches"))
        .unwrap();
    let body: serde_json::Value = serde_json::from_str(&wake.body).unwrap();
    assert_eq!(body["inputs"]["mode"], "round");
    assert_eq!(body["inputs"]["branch"], "main");
}

// 134; rule core.29: every call to GitHub's API under the retry policy.

/// A policy whose pauses are kept and skipped, its clock moved on by each.
fn pausing() -> (Rc<Policy>, Rc<RefCell<Vec<Duration>>>) {
    let pauses = Rc::new(RefCell::new(Vec::new()));
    let clock = Rc::new(Cell::new(Instant::now()));
    let (kept, moved) = (pauses.clone(), clock.clone());
    let policy = Policy::command().pausing(
        move |d| {
            kept.borrow_mut().push(d);
            moved.set(moved.get() + d);
            true
        },
        move || clock.get(),
    );
    (Rc::new(policy), pauses)
}

/// The fake GitHub holding one open pull request from `unit/a` at `aaa`,
/// its number, and the app calling it under `policy`.
fn on_github(name: &str, policy: Rc<Policy>) -> (support::Scratch, github::GitHub, App, u64) {
    let dir = support::Scratch::new(name);
    let gh = github::GitHub::start(&github::file(dir.path()), None);
    let app = App::new(APP, KEY, gh.url(), Box::new(Utc::now), policy).unwrap();
    let n = gh
        .forge
        .open_by("ann", "main", "unit/a", "aaa", false, None)
        .unwrap();
    (dir, gh, app, n)
}

fn waiting() -> sluice::profiles::github::forge::Posting {
    use sluice::profiles::github::forge::{Posting, Status};
    Posting {
        status: Status::Queued,
        title: "waiting".into(),
        summary: "needs qa/approved".into(),
        text: None,
    }
}

/// The remote-retry spec's "A pull request read answered 503": a read
/// GitHub answers 503 three times is made a fourth, each after a longer
/// pause; one answered 404 is made once and reads as absent.
#[test]
fn a_read_github_could_not_answer_is_made_again() {
    use sluice::profiles::github::forge::Forge as _;
    let (policy, pauses) = pausing();
    let (_dir, gh, app, n) = on_github("forge-503", policy);
    gh.unavailable(3);
    let p = app.pull(&repo(), n).unwrap().unwrap();
    assert_eq!(p.number, n);
    assert_eq!(gh.calls("GET", &format!("/repos/o/r/pulls/{n}")), 4);
    let pauses = pauses.borrow().clone();
    assert_eq!(pauses.len(), 3, "{pauses:?}");
    assert!(pauses.windows(2).all(|w| w[1] > w[0]), "{pauses:?}");

    assert_eq!(app.pull(&repo(), 99).unwrap(), None);
    assert_eq!(gh.calls("GET", "/repos/o/r/pulls/99"), 1);
}

/// The remote-retry spec's "The policy's time is set by what called it":
/// a read GitHub answers 503 until the policy's thirty seconds are spent
/// fails as GitHub could not answer, saying how long it was tried.
#[test]
fn a_read_answered_503_until_the_bound_fails_unanswered() {
    use sluice::profiles::github::forge::Forge as _;
    let (policy, pauses) = pausing();
    let (_dir, gh, app, n) = on_github("forge-spent", policy);
    gh.unavailable(usize::MAX);
    let e = app.pull(&repo(), n).unwrap_err();
    assert!(sluice_seams::unanswered(&e), "{e:#}");
    let slept: Duration = pauses.borrow().iter().sum();
    assert_eq!(sluice::retry::lasted(&e), slept);
    assert!(
        slept > Duration::ZERO && slept <= Duration::from_secs(30),
        "{slept:?}"
    );
    let tries = pauses.borrow().len() + 1;
    assert_eq!(gh.calls("GET", &format!("/repos/o/r/pulls/{n}")), tries);
    let words = format!("{e:#}");
    assert!(
        words.contains(&format!("tried {tries} times over")),
        "{words}"
    );
    assert!(words.ends_with("503 Service Unavailable"), "{words}");
}

/// The remote-retry spec's "The forge's API refuses a token": a call
/// answered 401 on the token held is made again on one bought afresh,
/// which sluice holds after.
#[test]
fn a_call_refused_its_token_is_made_on_a_fresh_one() {
    use sluice::profiles::github::forge::Forge as _;
    let clock = Clock::new();
    let forge = Forge::new(&clock);
    let sold = Mutex::new(0);
    let fake = Fake::start(move |r: &Request| {
        if r.path != "/repos/o/r/pulls/7" {
            return forge.answer(r, &sold);
        }
        if r.header("Authorization") == Some("Bearer token-1") {
            return json(401, json!({ "message": "Bad credentials" }));
        }
        json(
            200,
            json!({ "number": 7, "state": "open", "head": { "ref": "unit/a" }, "base": { "ref": "main" } }),
        )
    });
    // A clock that moves on at each reading, so the fresh token is
    // settled at once.
    let app = App::new(APP, KEY, fake.url(), ticking(&clock), policy()).unwrap();
    assert_eq!(app.pull(&repo(), 7).unwrap().unwrap().number, 7);
    let asked: Vec<_> = fake
        .requests()
        .into_iter()
        .filter(|r| r.path == "/repos/o/r/pulls/7")
        .map(|r| r.header("Authorization").unwrap().to_string())
        .collect();
    assert_eq!(asked, ["Bearer token-1", "Bearer token-2"]);
    assert_eq!(app.token(&repo()).unwrap(), "token-2");
    assert_eq!(minted(&fake), 2);
}

/// The remote-retry spec's "A draft mark answered 502" and "A check run's
/// update cut after it was sent": a write that sets a state is made again.
#[test]
fn a_write_that_sets_a_state_is_made_again() {
    use github::Trouble;
    use sluice::profiles::github::forge::{Forge as _, Posting, Status};
    let (_dir, gh, app, n) = on_github("forge-set", policy());
    let r = repo();
    gh.trouble("POST", "/graphql", 1, Trouble::Status(502));
    app.set_draft(&r, n, true).unwrap();
    assert_eq!(gh.calls("POST", "/graphql"), 2);
    assert!(app.pull(&r, n).unwrap().unwrap().draft);

    app.post(&r, "aaa", None, &waiting()).unwrap();
    let id = app.posted(&r, "aaa").unwrap().unwrap().id;
    let landed = Posting {
        status: Status::Success,
        title: "landed".into(),
        summary: String::new(),
        text: None,
    };
    gh.trouble(
        "PATCH",
        "/repos/o/r/check-runs/",
        1,
        Trouble::Cut { taken: false },
    );
    app.post(&r, "aaa", Some(id), &landed).unwrap();
    assert_eq!(gh.calls("PATCH", "/repos/o/r/check-runs/"), 2);
    assert_eq!(app.posted(&r, "aaa").unwrap().unwrap().posting, landed);
}

/// The remote-retry spec's "A comment posted, its answer lost", "A
/// comment not taken" and "A check run made, its answer lost": a write
/// that makes something whose answer was not read is read back on GitHub,
/// carried on when GitHub holds it and made once more when it does not.
#[test]
fn a_write_that_makes_something_is_read_back_before_it_is_made_again() {
    use github::Trouble;
    use sluice::profiles::github::forge::Forge as _;
    let (_dir, gh, app, n) = on_github("forge-make", policy());
    let r = repo();
    let comments = format!("/repos/o/r/issues/{n}/comments");

    gh.trouble("POST", &comments, 1, Trouble::Taken(502));
    app.comment(&r, n, "taken").unwrap();
    assert_eq!(gh.calls("POST", &comments), 1);
    assert_eq!(app.comments(&r, n).unwrap(), ["taken"]);

    gh.trouble("POST", &comments, 1, Trouble::Status(502));
    app.comment(&r, n, "not taken").unwrap();
    assert_eq!(gh.calls("POST", &comments), 3);
    assert_eq!(app.comments(&r, n).unwrap(), ["taken", "not taken"]);

    gh.trouble(
        "POST",
        "/repos/o/r/check-runs",
        1,
        Trouble::Cut { taken: true },
    );
    app.post(&r, "aaa", None, &waiting()).unwrap();
    assert_eq!(gh.calls("POST", "/repos/o/r/check-runs"), 1);
    let names: Vec<String> = app
        .posted_all(&r, "aaa")
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, ["sluice"]);

    gh.trouble("POST", "/repos/o/r/pulls", 1, Trouble::Taken(502));
    let opened = app.open(&r, "main", "unit/e", "E", "body", false).unwrap();
    assert_eq!(
        (opened.head_ref.as_str(), opened.title.as_str()),
        ("unit/e", "E")
    );
    assert_eq!(gh.calls("POST", "/repos/o/r/pulls"), 1);
    assert_eq!(app.pulls(&r, "main").unwrap().len(), 2);
}

/// The remote-retry spec's "A connection never made": a comment whose
/// first try cannot connect is made again, and posted once.
#[test]
fn a_comment_whose_first_try_cannot_connect_is_posted_once() {
    use sluice::profiles::github::forge::Forge as _;
    use std::net::TcpListener;
    let dir = support::Scratch::new("forge-unconnected");
    let file = github::file(dir.path());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    // GitHub sells the app its token, then goes away: the comment's
    // first try finds nothing on the port, and the pause brings GitHub
    // back on it.
    let selling = support::http::serve(listener, 2, |r: &Request| {
        if r.path.ends_with("/access_tokens") {
            let ends = Utc::now() + TimeDelta::hours(1);
            return json(
                201,
                json!({ "token": "token", "expires_at": ends.to_rfc3339() }),
            );
        }
        json(
            200,
            json!({ "id": INSTALLATION, "app_id": APP, "permissions": {} }),
        )
    });
    let back: Rc<RefCell<Option<github::GitHub>>> = Rc::default();
    let (up, at) = (back.clone(), file.clone());
    let policy = Policy::command().pausing(
        move |_| {
            if up.borrow().is_none() {
                let listener = TcpListener::bind(("127.0.0.1", port)).unwrap();
                *up.borrow_mut() = Some(github::GitHub::start_on(listener, &at, None));
            }
            true
        },
        Instant::now,
    );
    let url = format!("http://127.0.0.1:{port}");
    let app = App::new(APP, KEY, &url, Box::new(Utc::now), Rc::new(policy)).unwrap();
    app.token(&repo()).unwrap();
    assert_eq!(selling.join().unwrap().len(), 2);
    let n = sluice::profiles::github::fake::FileForge::new(&file, &repo())
        .open_by("ann", "main", "unit/a", "aaa", false, None)
        .unwrap();
    app.comment(&repo(), n, "posted").unwrap();
    let gh = back.borrow();
    let gh = gh.as_ref().expect("GitHub back after the first try");
    assert_eq!(
        gh.calls("POST", &format!("/repos/o/r/issues/{n}/comments")),
        1
    );
    assert_eq!(app.comments(&repo(), n).unwrap(), ["posted"]);
    // Never judged, the comment was made again blind: nothing read
    // whether GitHub held it, but the read just made.
    assert_eq!(
        gh.calls("GET", &format!("/repos/o/r/issues/{n}/comments")),
        1
    );
}

/// The remote-retry spec's "A run's dispatch answered 502": a dispatch
/// GitHub answers 502 is made once, and fails saying its answer was not
/// read.
#[test]
fn a_dispatch_answered_502_is_made_once() {
    use sluice::profiles::checks::Launcher;
    use sluice::profiles::github::actions::Actions;
    use sluice_seams::{Launch, Rev, Setting};
    use std::collections::BTreeMap;

    let clock = Clock::new();
    let forge = Forge::new(&clock);
    let minted = Mutex::new(0);
    let fake = Fake::start(move |r: &Request| {
        if r.path.ends_with("/dispatches") {
            return json(502, json!({ "message": "Bad Gateway" }));
        }
        forge.answer(r, &minted)
    });
    let dir = support::Scratch::new("pool-unanswered");
    for args in [
        &["init", "-q", "."][..],
        &["remote", "add", "origin", "https://github.com/o/r.git"][..],
    ] {
        let out = support::git_command()
            .current_dir(dir.path())
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success());
    }
    let workflow = |pairs: &[(&str, &str)]| -> (String, Setting) {
        let t: BTreeMap<String, Setting> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), Setting::Text(v.to_string())))
            .collect();
        ("github".into(), Setting::Tables(vec![t]))
    };
    let kind = workflow(&[("workflow", ".github/workflows/sluice.yml")]);
    let pool = Actions::new(Rc::new(app(&fake, &clock)), dir.path());
    let e = pool
        .launch(&Launch {
            pool: "forge".into(),
            kind: kind.clone(),
            signer: kind,
            runner: "macos-15".into(),
            remote: Some("origin".into()),
            branch: "main".into(),
            candidate: "c3".into(),
            combination: Rev::new("c0ffee"),
            system: Some("macos-arm64".into()),
            claim: "pool/forge/7".into(),
        })
        .unwrap_err();
    assert!(sluice_seams::unanswered(&e), "{e:#}");
    let words = format!("{e:#}");
    assert!(
        words.contains(
            "GitHub's answer to POST /repos/o/r/actions/workflows/sluice.yml/dispatches \
             was not read: 502 Bad Gateway"
        ),
        "{words}"
    );
    let dispatched = fake
        .requests()
        .iter()
        .filter(|r| r.path.ends_with("/dispatches"))
        .count();
    assert_eq!(dispatched, 1);
}
