//! sluice's identity on GitHub, against a fake of GitHub's API on a port
//! the system picks (C.4 pushed refs). The tests that need git are in
//! the check-tools tier.

#[path = "../support/mod.rs"]
mod support;

use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeDelta, Utc};
use serde_json::json;
use sluice::profiles::github::check::{check, Finding};
use sluice::profiles::github::{App, Repo};
use support::http::{json, Fake, Request};

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
/// permissions and each ruleset's bypass actors on `main`.
#[derive(Clone)]
struct Forge {
    permissions: serde_json::Value,
    rulesets: Vec<(u64, serde_json::Value)>,
    clock: Clock,
}

impl Forge {
    fn new(clock: &Clock) -> Forge {
        Forge {
            permissions: json!({ "contents": "write", "metadata": "read" }),
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
        Fake::start(move |r: &Request| {
            let path = r.path.as_str();
            if path == "/repos/o/r/installation" && r.method == "GET" {
                return json(
                    200,
                    json!({ "id": INSTALLATION, "app_id": APP, "permissions": self.permissions }),
                );
            }
            if path == format!("/app/installations/{INSTALLATION}/access_tokens")
                && r.method == "POST"
            {
                let mut n = minted.lock().unwrap();
                *n += 1;
                let ends = self.clock.now() + TimeDelta::hours(1);
                return json(
                    201,
                    json!({ "token": format!("token-{n}"), "expires_at": ends.to_rfc3339() }),
                );
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
        })
    }
}

fn app(fake: &Fake, clock: &Clock) -> App {
    let clock = clock.clone();
    App::new(APP, KEY, fake.url(), Box::new(move || clock.now())).unwrap()
}

fn minted(fake: &Fake) -> usize {
    fake.requests()
        .iter()
        .filter(|r| r.path.ends_with("/access_tokens"))
        .count()
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

fn failures(found: &[Finding]) -> Vec<&str> {
    found
        .iter()
        .filter(|f| !f.ok)
        .map(|f| f.what.as_str())
        .collect()
}

/// All set: the app is installed to write contents and is the only
/// bypass of the branch's ruleset.
#[test]
fn forge_check_all_set() {
    let clock = Clock::new();
    let fake = Forge::new(&clock).start();
    let found = check(&app(&fake, &clock), &repo(), "main", Some("operator"));
    assert!(failures(&found).is_empty(), "{found:?}");
    assert_eq!(found.len(), 3, "{found:?}");
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
/// app may only read, no ruleset at all, and no token of the operator's
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
    f.permissions = json!({ "contents": "read" });
    let got = one(f, Some("operator"));
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        got[0].contains("may not write o/r's contents (contents: read)"),
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

    s.give_forge();
    let (code, said) = s.sluice(&["forge", "check"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains(&format!("lets only app {APP} through")),
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
