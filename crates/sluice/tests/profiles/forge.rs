//! sluice's identity on GitHub, against a fake of GitHub's API on a port
//! the system picks (C.4 pushed refs). The tests that need git are in
//! the check-tools tier.

#[path = "../support/mod.rs"]
mod support;

use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeDelta, Utc};
use serde_json::json;
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
            for (id, actors) in &self.rulesets {
                if path.starts_with(&format!("/repos/o/r/rulesets/{id}")) {
                    return json(
                        200,
                        json!({ "id": id, "name": format!("ruleset {id}"), "bypass_actors": actors }),
                    );
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
