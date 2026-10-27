//! The app's JWT and its installation tokens. A JWT lasts ten minutes; an
//! installation token an hour, bought from the repository's own
//! installation for that repository alone, kept in this process's memory
//! and bought again five minutes before it ends, or at once for a call
//! GitHub refused it on. Nothing is written anywhere.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, TimeDelta, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::Value;
use sluice_seams::Unanswered;

use crate::instance::Forge;

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

pub struct App {
    id: u64,
    key: EncodingKey,
    api: String,
    now: Now,
    http: ureq::Agent,
    tokens: RefCell<BTreeMap<Repo, Token>>,
}

impl App {
    /// The app `id`, signing with the PEM private key `pem`, calling the
    /// API at `api`.
    pub fn new(id: u64, pem: &[u8], api: &str, now: Now) -> Result<App> {
        let key = EncodingKey::from_rsa_pem(pem).context("the app's key is no RSA private key")?;
        let http = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent("sluice")
            .build()
            .into();
        Ok(App {
            id,
            key,
            api: api.trim_end_matches('/').into(),
            now,
            http,
            tokens: RefCell::default(),
        })
    }

    /// The host's `[forge]` app.
    pub fn open(forge: &Forge) -> Result<App> {
        let pem = std::fs::read(&forge.key)
            .with_context(|| format!("reading the app's key {}", forge.key.display()))?;
        let api = std::env::var(API_VAR).unwrap_or_else(|_| API.into());
        App::new(forge.app_id, &pem, &api, Box::new(Utc::now))
            .with_context(|| format!("[forge] key {}", forge.key.display()))
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

    /// The app's installation on `repo`.
    pub fn installation(&self, repo: &Repo) -> Result<Installation> {
        let v = self
            .get(
                &format!("/repos/{}/{}/installation", repo.owner, repo.name),
                &self.jwt()?,
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
        let v = self
            .post_api(
                &format!("/app/installations/{}/access_tokens", installation.id),
                &self.jwt()?,
                &body,
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

    /// `GET` an API path with a bearer token.
    pub fn get(&self, path: &str, bearer: &str) -> Result<Value> {
        self.send("GET", path, bearer, None)
    }

    pub fn post_api(&self, path: &str, bearer: &str, body: &str) -> Result<Value> {
        self.send("POST", path, bearer, Some(body))
    }

    pub(super) fn patch_api(&self, path: &str, bearer: &str, body: &str) -> Result<Value> {
        self.send("PATCH", path, bearer, Some(body))
    }

    fn send(&self, method: &str, path: &str, bearer: &str, body: Option<&str>) -> Result<Value> {
        let (status, v, text) = self.call(method, path, bearer, body)?;
        if !(200..300).contains(&status) {
            return Err(failed(method, path, status, &v, &text));
        }
        Ok(v)
    }

    /// `GET` an API path; `None` when GitHub has nothing there (404).
    pub(super) fn get_found(&self, path: &str, bearer: &str) -> Result<Option<Value>> {
        let (status, v, text) = self.call("GET", path, bearer, None)?;
        match status {
            404 => Ok(None),
            200..=299 => Ok(Some(v)),
            _ => Err(failed("GET", path, status, &v, &text)),
        }
    }

    /// One call: its status, its JSON, and its text.
    fn call(
        &self,
        method: &str,
        path: &str,
        bearer: &str,
        body: Option<&str>,
    ) -> Result<(u16, Value, String)> {
        let url = format!("{}{path}", self.api);
        let auth = format!("Bearer {bearer}");
        let resp = match (method, body) {
            ("GET", _) => self
                .http
                .get(&url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("Authorization", &auth)
                .call(),
            (_, body) => {
                let req = match method {
                    "POST" => self.http.post(&url),
                    "PATCH" => self.http.patch(&url),
                    other => bail!("sluice makes no {other} call"),
                };
                req.header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .header("Authorization", &auth)
                    .header("Content-Type", "application/json")
                    .send(body.unwrap_or(""))
            }
        };
        let mut resp = resp.with_context(|| format!("{method} {path}"))?;
        let status = resp.status().as_u16();
        let text = resp
            .body_mut()
            .read_to_string()
            .with_context(|| format!("{method} {path}"))?;
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        Ok((status, v, text))
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
        let app = App::new(4242, KEY, API, Box::new(move || at)).unwrap();
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
