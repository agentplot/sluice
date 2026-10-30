//! The app's JWT and its installation tokens. A JWT lasts ten minutes; an
//! installation token an hour, bought from the repository's own
//! installation for that repository alone, kept in this process's memory
//! and bought again five minutes before it ends, or at once for a call
//! GitHub refused it on. Nothing is written anywhere.
//!
//! Every call to the API runs under the retry policy (134; rule core.29),
//! classed by what it does ([`Call`]): one GitHub never judged, a
//! connection not made, is tried again; a call GitHub refused the
//! installation's token on (401) is tried again on one bought afresh; a
//! read, or a write that sets a state, is tried again whatever else
//! failed but GitHub's answer; a write that makes something is not made
//! again once GitHub may have taken it, a 5xx among them, and fails
//! [`Unanswered`], for its caller to read whether it was made.
//!
//! GitHub's word that an allowance is spent, a 429, a 403 that says so,
//! GraphQL's `RATE_LIMITED`, or a success that leaves no call, is a pause
//! kept in the host's state for the installation and the allowance, read
//! before every call by every process on the host: while it stands no
//! call is made, and none to find out (134 as amended 2026-10-28; rule
//! core.29; ADR 0071). Every `GET` carries the ETag and last-modified time
//! of the answer the host keeps for its address, and GitHub's 304 is
//! answered from it (136; rule core.32). The calls the app's JWT makes,
//! finding the installation and buying a token, read no pause.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, TimeDelta, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::Value;
use sluice_seams::Unanswered;

use super::kept::{self, Allowance, Kept, Pause};
use crate::instance::Forge;
use crate::retry::{Failed, Judged, Kind, Limited, Policy};

/// GitHub's API.
pub const API: &str = "https://api.github.com";

/// Where the API calls go instead of [`API`]: a fake one, in tests.
pub const API_VAR: &str = "SLUICE_FORGE_API";

/// How long a JWT is good for, the most GitHub takes.
const JWT_LIFE: i64 = 600;
/// How far back a JWT is dated, for a host clock a little ahead.
const JWT_SKEW: i64 = 60;
/// How long before its end a token is bought again.
const REFRESH: TimeDelta = TimeDelta::minutes(5);
/// How old a token bought for a refused call is before it is handed over:
/// GitHub may refuse a token for its first seconds, until all its servers
/// know it (Octokit's auth-app waits as long for the same reason).
const SETTLE: TimeDelta = TimeDelta::seconds(5);
/// How long a pause lasts when GitHub names no time: doubled for each
/// such answer in a row, up to [`LONGEST_PAUSE`], as GitHub asks.
const FIRST_PAUSE: TimeDelta = TimeDelta::minutes(1);
const LONGEST_PAUSE: TimeDelta = TimeDelta::minutes(30);

/// The time now; a test's own clock in tests.
pub type Now = Box<dyn Fn() -> DateTime<Utc>>;

/// A repository on github.com.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Repo {
    pub owner: String,
    pub name: String,
}

impl Repo {
    /// The repository a remote's URL names on github.com, in its HTTPS,
    /// `ssh://` or `git@github.com:` form; `None` for any other host.
    pub fn from_url(url: &str) -> Option<Repo> {
        let path = [
            "https://github.com/",
            "ssh://git@github.com/",
            "git@github.com:",
        ]
        .iter()
        .find_map(|p| url.strip_prefix(p))?;
        let path = path.trim_end_matches('/');
        let path = path.strip_suffix(".git").unwrap_or(path);
        let (owner, name) = path.split_once('/')?;
        if owner.is_empty() || name.is_empty() || name.contains('/') {
            return None;
        }
        Some(Repo {
            owner: owner.into(),
            name: name.into(),
        })
    }

    pub fn https(&self) -> String {
        format!("https://github.com/{}/{}.git", self.owner, self.name)
    }
}

impl std::fmt::Display for Repo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

/// The app's installation on a repository.
pub struct Installation {
    pub id: u64,
    /// Each permission the installation was granted: `contents` to
    /// `write`, and so on.
    pub permissions: BTreeMap<String, String>,
}

#[derive(Clone)]
struct Token {
    secret: String,
    ends: DateTime<Utc>,
    bought: DateTime<Utc>,
    installation: u64,
    /// What GitHub said the token reaches, in its words.
    scope: String,
}

/// An installation token, and what it is for a report of its refusal.
pub struct Issued {
    pub secret: String,
    /// The app, the repository, the installation it was bought from, its
    /// scope and its age; never the secret.
    pub about: String,
}

#[derive(Serialize)]
struct Claims {
    iat: i64,
    exp: i64,
    iss: String,
}

/// What a call to the API does, which decides how it is tried again (134;
/// rule core.29).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    /// Changes nothing: a `GET`, a GraphQL query, a token bought.
    Read,
    /// Sets a state, which set twice is as set once: tried again as a
    /// read is.
    Set,
    /// Makes or starts something: tried again only while GitHub never
    /// took it.
    Make,
}

impl Call {
    /// The policy's kind of call this is.
    fn kind(self) -> Kind {
        match self {
            Call::Read | Call::Set => Kind::Read,
            Call::Make => Kind::Write,
        }
    }
}

/// What an API call authenticates as.
#[derive(Clone, Copy)]
pub enum Bearer<'a> {
    /// The app itself, with a JWT.
    App,
    /// The app's installation on a repository, with its token: one bought
    /// afresh for the next try when GitHub refuses it.
    Repo(&'a Repo),
    /// A token given, the operator's own.
    Token(&'a str),
}

pub struct App {
    id: u64,
    key: EncodingKey,
    api: String,
    now: Now,
    http: ureq::Agent,
    tokens: RefCell<BTreeMap<Repo, Token>>,
    /// The app's slug, once read.
    slug: RefCell<Option<String>>,
    policy: Rc<Policy>,
    /// The host's answers and pauses; none, nothing is kept.
    kept: Option<Kept>,
    /// The end of the last pause this process said it waits out.
    told: Cell<Option<DateTime<Utc>>>,
}

/// What GitHub said of its allowance with an answer, and of the answer's
/// version.
#[derive(Default)]
struct Heard {
    remaining: Option<u64>,
    reset: Option<DateTime<Utc>>,
    resource: Option<String>,
    retry_after: Option<u64>,
    etag: Option<String>,
    last_modified: Option<String>,
}

/// One call's answer: its status, its JSON, its text and what GitHub said
/// with it.
struct Reply {
    status: u16,
    v: Value,
    text: String,
    heard: Heard,
}

impl App {
    /// The app `id`, signing with the PEM private key `pem`, calling the
    /// API at `api`, each call under `policy`.
    pub fn new(id: u64, pem: &[u8], api: &str, now: Now, policy: Rc<Policy>) -> Result<App> {
        let key = EncodingKey::from_rsa_pem(pem).context("the app's key is no RSA private key")?;
        let http = http_config().into();
        Ok(App {
            id,
            key,
            api: api.trim_end_matches('/').into(),
            now,
            http,
            tokens: RefCell::default(),
            slug: RefCell::default(),
            policy,
            kept: None,
            told: Cell::new(None),
        })
    }

    /// The host's `[forge]` app, each call under the command's `policy`,
    /// keeping its answers and pauses in the host's state.
    pub fn open(forge: &Forge, policy: Rc<Policy>) -> Result<App> {
        let pem = std::fs::read(&forge.key)
            .with_context(|| format!("reading the app's key {}", forge.key.display()))?;
        let api = std::env::var(API_VAR).unwrap_or_else(|_| API.into());
        let app = App::new(forge.app_id, &pem, &api, Box::new(Utc::now), policy)
            .with_context(|| format!("[forge] key {}", forge.key.display()))?;
        Ok(app.keeping(Kept::new(&crate::instance::state_dir()?)))
    }

    /// `self`, keeping its answers and pauses in `kept`.
    pub fn keeping(mut self, kept: Kept) -> App {
        self.kept = Some(kept);
        self
    }

    /// `self`, finding the address of each call's host with `resolver`
    /// instead of the host's own lookup: a test's, which can give an
    /// address no connection is ever made to.
    pub fn resolving(mut self, resolver: impl ureq::unversioned::resolver::Resolver) -> App {
        self.http = ureq::Agent::with_parts(
            http_config(),
            ureq::unversioned::transport::DefaultConnector::default(),
            resolver,
        );
        self
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    /// A JWT that authenticates as the app for ten minutes.
    pub fn jwt(&self) -> Result<String> {
        let now = (self.now)().timestamp();
        let claims = Claims {
            iat: now - JWT_SKEW,
            exp: now - JWT_SKEW + JWT_LIFE,
            iss: self.id.to_string(),
        };
        Ok(jsonwebtoken::encode(
            &Header::new(Algorithm::RS256),
            &claims,
            &self.key,
        )?)
    }

    /// The app's slug, which GitHub names its bot user after: read with
    /// its JWT once in this process, and kept.
    pub fn slug(&self) -> Result<String> {
        if let Some(slug) = self.slug.borrow().as_ref() {
            return Ok(slug.clone());
        }
        let v = self
            .get("/app", Bearer::App)
            .with_context(|| format!("reading app {}", self.id))?;
        let slug = v["slug"]
            .as_str()
            .ok_or_else(|| anyhow!("GitHub gave app {} no slug", self.id))?
            .to_string();
        *self.slug.borrow_mut() = Some(slug.clone());
        Ok(slug)
    }

    /// The app's installation on `repo`.
    pub fn installation(&self, repo: &Repo) -> Result<Installation> {
        let v = self
            .get(
                &format!("/repos/{}/{}/installation", repo.owner, repo.name),
                Bearer::App,
            )
            .with_context(|| format!("finding app {}'s installation on {repo}", self.id))?;
        let id = v["id"]
            .as_u64()
            .ok_or_else(|| anyhow!("the installation on {repo} has no id"))?;
        let permissions = v["permissions"]
            .as_object()
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        Ok(Installation { id, permissions })
    }

    /// An installation token for `repo`, bought when this process has
    /// none that lasts five minutes more.
    pub fn token(&self, repo: &Repo) -> Result<String> {
        Ok(self.held(repo)?.secret)
    }

    /// A token for `repo` as [`App::token`] gives it, or, `fresh`, one
    /// bought now whatever this process holds, which replaces it: for a
    /// call GitHub refused the last one on. A fresh token is handed over
    /// once it is [`SETTLE`] old.
    pub fn issue(&self, repo: &Repo, fresh: bool) -> Result<Issued> {
        let t = if fresh {
            let t = self.buy(repo)?;
            if let Ok(wait) = (SETTLE - ((self.now)() - t.bought)).to_std() {
                std::thread::sleep(wait);
            }
            t
        } else {
            self.held(repo)?
        };
        let age = ((self.now)() - t.bought).num_seconds().max(0);
        Ok(Issued {
            about: format!(
                "app {}'s token for {repo} from installation {}, scoped to {}, bought {age}s ago",
                self.id, t.installation, t.scope
            ),
            secret: t.secret,
        })
    }

    /// The token this process holds for `repo` while it lasts five
    /// minutes more, or one bought now.
    fn held(&self, repo: &Repo) -> Result<Token> {
        let now = (self.now)();
        if let Some(t) = self.tokens.borrow().get(repo) {
            if t.ends - now > REFRESH {
                return Ok(t.clone());
            }
        }
        self.buy(repo)
    }

    /// A token bought now from `repo`'s installation for `repo` alone,
    /// kept for the next call on `repo`.
    fn buy(&self, repo: &Repo) -> Result<Token> {
        let installation = self.installation(repo)?;
        let body = serde_json::json!({ "repositories": [repo.name] }).to_string();
        // Buying a token changes nothing the repository holds: a read.
        let v = self
            .send(
                "POST",
                Call::Read,
                &format!("/app/installations/{}/access_tokens", installation.id),
                Bearer::App,
                Some(&body),
            )
            .with_context(|| format!("making a token for {repo}"))?;
        let bought = (self.now)();
        let secret = v["token"]
            .as_str()
            .ok_or_else(|| anyhow!("GitHub gave no token for {repo}"))?
            .to_string();
        let ends = v["expires_at"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .ok_or_else(|| anyhow!("GitHub gave a token for {repo} with no end"))?
            .with_timezone(&Utc);
        let t = Token {
            secret,
            ends,
            bought,
            installation: installation.id,
            scope: scope(repo, &v)?,
        };
        self.tokens.borrow_mut().insert(repo.clone(), t.clone());
        Ok(t)
    }

    /// `GET` an API path as `bearer`.
    pub fn get(&self, path: &str, bearer: Bearer) -> Result<Value> {
        self.send("GET", Call::Read, path, bearer, None)
    }

    /// `POST` to an API path as `bearer` a call that does what `call`
    /// says.
    pub fn post_api(&self, call: Call, path: &str, bearer: Bearer, body: &str) -> Result<Value> {
        self.send("POST", call, path, bearer, Some(body))
    }

    /// `PATCH` an API path as `bearer`: every `PATCH` sluice makes sets a
    /// state.
    pub(super) fn patch_api(&self, path: &str, bearer: Bearer, body: &str) -> Result<Value> {
        self.send("PATCH", Call::Set, path, bearer, Some(body))
    }

    fn send(
        &self,
        method: &str,
        call: Call,
        path: &str,
        bearer: Bearer,
        body: Option<&str>,
    ) -> Result<Value> {
        let (status, v, text, _) = self.answer(method, call, path, bearer, body)?;
        if !(200..300).contains(&status) {
            return Err(failed(method, path, status, &v, &text));
        }
        Ok(v)
    }

    /// `GET` an API path as `bearer`: its JSON, and whether GitHub sent it
    /// afresh, `false` when it answered that the answer the host keeps for
    /// the path stands (a 304; 136).
    pub(super) fn get_fresh(&self, path: &str, bearer: Bearer) -> Result<(Value, bool)> {
        let (status, v, text, fresh) = self.answer("GET", Call::Read, path, bearer, None)?;
        if !(200..300).contains(&status) {
            return Err(failed("GET", path, status, &v, &text));
        }
        Ok((v, fresh))
    }

    /// `GET` an API path; `None` when GitHub has nothing there (404).
    pub(super) fn get_found(&self, path: &str, bearer: Bearer) -> Result<Option<Value>> {
        let (status, v, text, _) = self.answer("GET", Call::Read, path, bearer, None)?;
        match status {
            404 => Ok(None),
            200..=299 => Ok(Some(v)),
            _ => Err(failed("GET", path, status, &v, &text)),
        }
    }

    /// GitHub's answer to a `call`, under the retry policy: its status,
    /// its JSON, its text and whether GitHub sent it afresh rather than
    /// answering a `GET` from what the host keeps, for a success or any
    /// status GitHub would answer again. A 5xx is tried again, but on a call that makes
    /// something, which GitHub may have taken; a 401 on the installation's
    /// token is tried again on one bought afresh; a spent allowance is
    /// the policy's to wait out or stop on, no call made while its pause
    /// stands. The last try's failure is the call's when the policy stops;
    /// a call that makes something stopped with its answer not read fails
    /// [`Unanswered`], saying so.
    fn answer(
        &self,
        method: &str,
        call: Call,
        path: &str,
        bearer: Bearer,
        body: Option<&str>,
    ) -> Result<(u16, Value, String, bool)> {
        // The last failure's words, for a token that could not be bought
        // afresh after it.
        let mut said = String::new();
        self.policy
            .run(call.kind(), |t| {
                let auth = match bearer {
                    Bearer::App => self.jwt(),
                    Bearer::Repo(repo) if t.renew() => self
                        .issue(repo, true)
                        .map(|i| i.secret)
                        .map_err(|e| anyhow!("{said}; and no token could be bought afresh: {e:#}")),
                    Bearer::Repo(repo) => self.token(repo),
                    Bearer::Token(token) => Ok(token.to_string()),
                };
                let auth = auth.map_err(|e| match call {
                    // Nothing was sent: no answer to it went unread.
                    Call::Make => Failed::new(Judged::Answered, anyhow!("{e:#}")),
                    _ => Failed::new(Judged::Answered, e),
                })?;
                let allowance = self.allowance(bearer, path);
                let paused = allowance
                    .as_ref()
                    .and_then(|a| self.kept.as_ref()?.pause(a));
                if let Some(p) = paused.as_ref().filter(|p| p.until > self.policy.wall()) {
                    return Err(self.limited(p.until, &p.words));
                }
                let url = format!("{}{path}", self.api);
                let kept = match (&self.kept, method) {
                    (Some(k), "GET") => k.answer(&url),
                    _ => None,
                };
                let r = self.call(method, path, &auth, body, kept.as_ref())?;
                if let Some(words) = spent(path, &r) {
                    return Err(self.pause(allowance.as_ref(), paused, &r.heard, words));
                }
                let judged = match r.status {
                    500..=599 if call == Call::Make => Judged::Unknown,
                    500..=599 => Judged::Never,
                    401 if matches!(bearer, Bearer::Repo(_)) => Judged::Credential,
                    _ => {
                        self.heard(allowance.as_ref(), paused, &r.heard);
                        return Ok(self.kept_answer(&url, method, kept, r));
                    }
                };
                let e = failed(method, path, r.status, &r.v, &r.text);
                said = format!("{e:#}");
                Err(Failed::new(judged, e))
            })
            .map_err(|s| match (call, s.judged) {
                (Call::Make, Judged::Unknown) => {
                    let words = format!("{:#}", s.error);
                    let called = format!("{method} {path}: ");
                    let words = words.strip_prefix(&called).unwrap_or(&words);
                    Unanswered(format!(
                        "GitHub's answer to {method} {path} was not read: {words}"
                    ))
                    .into()
                }
                _ => s.into_error(),
            })
    }

    /// The allowance a call as `bearer` to `path` is counted against:
    /// the installation's, `graphql` or `core`; `None` for the app's own
    /// calls and the operator's token, or before a token was bought.
    fn allowance(&self, bearer: Bearer, path: &str) -> Option<Allowance> {
        let Bearer::Repo(repo) = bearer else {
            return None;
        };
        let installation = self.tokens.borrow().get(repo)?.installation;
        Some(Allowance {
            app: self.id,
            installation,
            resource: resource(path).into(),
        })
    }

    /// A try not made, or refused, for an allowance spent until `until`;
    /// said once on stderr when the policy waits it out.
    fn limited(&self, until: DateTime<Utc>, words: &str) -> Failed<anyhow::Error> {
        if self.policy.waits() && self.told.get() != Some(until) {
            self.told.set(Some(until));
            eprintln!(
                "sluice: GitHub's allowance of calls is spent until {}; waiting: {words}",
                crate::retry::clock(until)
            );
        }
        let e = Limited {
            app: self.id,
            until,
            words: words.into(),
        };
        Failed::new(Judged::Limited { until }, e.into())
    }

    /// GitHub said in `words` that an allowance is spent: the pause, until
    /// `retry-after`, else the reset when no call remains, else a minute
    /// doubled for each such answer in a row; kept for `allowance`, the
    /// resource GitHub named winning, later than any kept already.
    fn pause(
        &self,
        allowance: Option<&Allowance>,
        paused: Option<Pause>,
        heard: &Heard,
        words: String,
    ) -> Failed<anyhow::Error> {
        let now = self.policy.wall();
        let streak = paused.map_or(0, |p| p.streak);
        let named = match (heard.retry_after, heard.remaining, heard.reset) {
            (Some(secs), _, _) => Some(now + TimeDelta::seconds(secs as i64)),
            (None, Some(0), Some(reset)) => Some(reset),
            _ => None,
        };
        let until = named.unwrap_or_else(|| {
            now + (FIRST_PAUSE * 2i32.saturating_pow(streak.min(5))).min(LONGEST_PAUSE)
        });
        let pause = Pause {
            until,
            words,
            streak: streak + 1,
        };
        let pause = match (&self.kept, allowance) {
            (Some(k), Some(a)) => {
                let a = Allowance {
                    resource: heard.resource.clone().unwrap_or_else(|| a.resource.clone()),
                    ..a.clone()
                };
                match k.put_pause(&a, pause.clone()) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!("keeping GitHub's pause: {e:#}");
                        pause
                    }
                }
            }
            _ => pause,
        };
        self.limited(pause.until, &pause.words)
    }

    /// GitHub answered: an answer that leaves no call of the allowance
    /// starts its pause until the reset, and any other ends the run of
    /// spent answers a pause kept.
    fn heard(&self, allowance: Option<&Allowance>, paused: Option<Pause>, heard: &Heard) {
        let (Some(k), Some(a)) = (&self.kept, allowance) else {
            return;
        };
        let a = Allowance {
            resource: heard.resource.clone().unwrap_or_else(|| a.resource.clone()),
            ..a.clone()
        };
        let pause = match (heard.remaining, heard.reset) {
            (Some(0), Some(until)) => Pause {
                until,
                words: "x-ratelimit-remaining: 0".into(),
                streak: 0,
            },
            _ => match paused {
                Some(p) if p.streak > 0 => Pause { streak: 0, ..p },
                _ => return,
            },
        };
        if let Err(e) = k.put_pause(&a, pause) {
            tracing::warn!("keeping GitHub's pause: {e:#}");
        }
    }

    /// The answer to a call GitHub judged, and whether GitHub sent it
    /// afresh: a `GET`'s 304 answered from `kept`, which it did not, its
    /// 200 kept for `url` when it has a version, its 404 forgetting what
    /// was kept.
    fn kept_answer(
        &self,
        url: &str,
        method: &str,
        kept: Option<kept::Answer>,
        r: Reply,
    ) -> (u16, Value, String, bool) {
        let Some(k) = self.kept.as_ref().filter(|_| method == "GET") else {
            return (r.status, r.v, r.text, true);
        };
        match (r.status, kept) {
            (304, Some(a)) => {
                k.used(url);
                let v = serde_json::from_str(&a.body).unwrap_or(Value::Null);
                (200, v, a.body, false)
            }
            (200, _) if r.heard.etag.is_some() || r.heard.last_modified.is_some() => {
                let a = kept::Answer {
                    url: url.into(),
                    etag: r.heard.etag,
                    last_modified: r.heard.last_modified,
                    body: r.text.clone(),
                    at: (self.now)(),
                };
                if let Err(e) = k.keep(&a) {
                    tracing::warn!("keeping GitHub's answer to {url}: {e:#}");
                }
                (r.status, r.v, r.text, true)
            }
            (404, _) => {
                k.forget(url);
                (r.status, r.v, r.text, true)
            }
            _ => (r.status, r.v, r.text, true),
        }
    }

    /// One call, conditional on `kept` when it is given: its answer; a
    /// call that got no answer read, how it stands.
    fn call(
        &self,
        method: &str,
        path: &str,
        bearer: &str,
        body: Option<&str>,
        kept: Option<&kept::Answer>,
    ) -> Result<Reply, Failed<anyhow::Error>> {
        let url = format!("{}{path}", self.api);
        let auth = format!("Bearer {bearer}");
        let resp = match (method, body) {
            ("GET", _) => {
                let mut req = self
                    .http
                    .get(&url)
                    .header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .header("Authorization", &auth);
                if let Some(etag) = kept.and_then(|k| k.etag.as_deref()) {
                    req = req.header("If-None-Match", etag);
                }
                if let Some(at) = kept.and_then(|k| k.last_modified.as_deref()) {
                    req = req.header("If-Modified-Since", at);
                }
                req.call()
            }
            (_, body) => {
                let req = match method {
                    "POST" => self.http.post(&url),
                    "PATCH" => self.http.patch(&url),
                    other => {
                        let e = anyhow!("sluice makes no {other} call");
                        return Err(Failed::new(Judged::Answered, e));
                    }
                };
                req.header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .header("Authorization", &auth)
                    .header("Content-Type", "application/json")
                    .send(body.unwrap_or(""))
            }
        };
        let unread = |judged: Judged, e: anyhow::Error| {
            Failed::new(judged, e.context(format!("{method} {path}")))
        };
        let mut resp = resp.map_err(|e| unread(sent(&e), e.into()))?;
        let status = resp.status().as_u16();
        let header = |name: &str| {
            resp.headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        let number = |name: &str| header(name).and_then(|v| v.trim().parse::<u64>().ok());
        let heard = Heard {
            remaining: number("x-ratelimit-remaining"),
            reset: number("x-ratelimit-reset").and_then(|t| DateTime::from_timestamp(t as i64, 0)),
            resource: header("x-ratelimit-resource"),
            retry_after: number("retry-after"),
            etag: header("etag"),
            last_modified: header("last-modified"),
        };
        let text = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| unread(Judged::Unknown, e.into()))?;
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        Ok(Reply {
            status,
            v,
            text,
            heard,
        })
    }
}

/// How every call is made: a status GitHub answers is read, not an error,
/// and no call takes longer than thirty seconds.
fn http_config() -> ureq::config::Config {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(30)))
        .user_agent("sluice")
        .build()
}

/// How a call that got no answer stands: never judged when no connection
/// was made, and not known otherwise, since GitHub may have taken it.
fn sent(e: &ureq::Error) -> Judged {
    use std::io::ErrorKind;
    match e {
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => Judged::Never,
        ureq::Error::Timeout(ureq::Timeout::Resolve | ureq::Timeout::Connect) => Judged::Never,
        ureq::Error::Io(io)
            if matches!(
                io.kind(),
                ErrorKind::ConnectionRefused
                    | ErrorKind::AddrNotAvailable
                    | ErrorKind::HostUnreachable
                    | ErrorKind::NetworkUnreachable
            ) =>
        {
            Judged::Never
        }
        _ => Judged::Unknown,
    }
}

/// GitHub's name for the allowance a call to `path` is counted against.
fn resource(path: &str) -> &'static str {
    if path == "/graphql" {
        "graphql"
    } else {
        "core"
    }
}

/// GitHub's words, when `r` says an allowance is spent: a 429; a 403 that
/// leaves no call, names a time to call again or says `rate limit`; a
/// GraphQL answer whose errors say `RATE_LIMITED`.
fn spent(path: &str, r: &Reply) -> Option<String> {
    let message = || {
        r.v["message"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| r.text.trim().to_string())
    };
    match r.status {
        429 => Some(message()),
        403 if r.heard.remaining == Some(0)
            || r.heard.retry_after.is_some()
            || message().to_lowercase().contains("rate limit") =>
        {
            Some(message())
        }
        200 if path == "/graphql" => r.v["errors"]
            .as_array()?
            .iter()
            .find(|e| e["type"] == "RATE_LIMITED")
            .map(|e| e["message"].as_str().unwrap_or("RATE_LIMITED").to_string()),
        _ => None,
    }
}

/// What a token GitHub gave for `repo` reaches, in GitHub's words: its
/// repositories, then each permission. A token whose repositories leave
/// out `repo` is refused, since it is used for `repo` alone.
fn scope(repo: &Repo, v: &Value) -> Result<String> {
    let names: Vec<&str> = v["repositories"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r["full_name"].as_str())
        .collect();
    let full = repo.to_string();
    if v["repositories"].is_array() && !names.iter().any(|n| n.eq_ignore_ascii_case(&full)) {
        bail!(
            "GitHub gave a token for {repo} that reaches {}, not {repo}",
            if names.is_empty() {
                "no repository".to_string()
            } else {
                names.join(", ")
            }
        );
    }
    let reaches = if names.is_empty() {
        match v["repository_selection"].as_str() {
            Some(s) => format!("{s} repositories"),
            None => "repositories GitHub did not list".to_string(),
        }
    } else {
        names.join(", ")
    };
    let granted: Vec<String> = v["permissions"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| Some(format!("{k}: {}", v.as_str()?)))
        .collect();
    Ok(if granted.is_empty() {
        reaches
    } else {
        format!("{reaches} with {}", granted.join(", "))
    })
}

/// A call GitHub answered with `status`, not a success, in its words: a
/// 5xx is [`Unanswered`], GitHub unable to answer just then.
fn failed(method: &str, path: &str, status: u16, v: &Value, text: &str) -> anyhow::Error {
    let why = v["message"].as_str().unwrap_or(text.trim());
    let said = format!("{method} {path}: {status} {why}");
    if (500..600).contains(&status) {
        Unanswered(said).into()
    } else {
        anyhow!(said)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{DecodingKey, Validation};

    const KEY: &[u8] = include_bytes!("../../../tests/profiles/forge/test-key.pem");
    const PUBLIC: &[u8] = include_bytes!("../../../tests/profiles/forge/test-key.pub.pem");

    /// A JWT signed with the app's key verifies with its public half and
    /// carries the app's id and ten minutes of life.
    #[test]
    fn a_jwt_verifies_with_the_public_half() {
        let at = Utc::now();
        let policy = Rc::new(Policy::command());
        let app = App::new(4242, KEY, API, Box::new(move || at), policy).unwrap();
        let jwt = app.jwt().unwrap();
        let mut v = Validation::new(Algorithm::RS256);
        v.set_required_spec_claims(&["exp", "iat", "iss"]);
        v.set_issuer(&["4242"]);
        let data = jsonwebtoken::decode::<serde_json::Value>(
            &jwt,
            &DecodingKey::from_rsa_pem(PUBLIC).unwrap(),
            &v,
        )
        .unwrap();
        let c = data.claims;
        assert_eq!(c["iss"], "4242");
        let (iat, exp) = (c["iat"].as_i64().unwrap(), c["exp"].as_i64().unwrap());
        assert_eq!(exp - iat, 600);
        assert!(iat <= at.timestamp() && exp > at.timestamp());
    }

    /// A 5xx is GitHub unable to answer just then, told apart from any
    /// other failure; the words are GitHub's either way.
    #[test]
    fn a_5xx_is_unanswered_and_a_4xx_is_not() {
        let path = "/repos/o/r/pulls/7";
        let e = failed("GET", path, 503, &Value::Null, "Service Unavailable\n");
        assert!(sluice_seams::unanswered(&e));
        assert!(sluice_seams::unanswered(&e.context("following #7")));
        let e = failed("GET", path, 503, &Value::Null, "Service Unavailable\n");
        assert_eq!(
            e.to_string(),
            format!("GET {path}: 503 Service Unavailable")
        );
        let e = failed(
            "GET",
            path,
            404,
            &serde_json::json!({"message": "Not Found"}),
            "",
        );
        assert!(!sluice_seams::unanswered(&e));
        assert_eq!(e.to_string(), format!("GET {path}: 404 Not Found"));
    }

    #[test]
    fn a_remote_on_github_and_others() {
        let r = Repo {
            owner: "o".into(),
            name: "r".into(),
        };
        for url in [
            "https://github.com/o/r.git",
            "https://github.com/o/r",
            "git@github.com:o/r.git",
            "ssh://git@github.com/o/r.git",
        ] {
            assert_eq!(Repo::from_url(url), Some(r.clone()), "{url}");
        }
        assert_eq!(r.https(), "https://github.com/o/r.git");
        for url in [
            "https://gitlab.com/o/r.git",
            "/srv/git/r.git",
            "https://github.com/o",
            "https://github.com/o/r/x",
        ] {
            assert_eq!(Repo::from_url(url), None, "{url}");
        }
    }
}
