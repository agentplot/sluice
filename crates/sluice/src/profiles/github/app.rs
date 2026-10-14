//! The app's JWT and its installation tokens. A JWT lasts ten minutes; an
//! installation token an hour, kept in this process's memory and bought
//! again five minutes before it ends. Nothing is written anywhere.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, TimeDelta, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::Value;

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

struct Token {
    secret: String,
    ends: DateTime<Utc>,
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
        let now = (self.now)();
        if let Some(t) = self.tokens.borrow().get(repo) {
            if t.ends - now > REFRESH {
                return Ok(t.secret.clone());
            }
        }
        let installation = self.installation(repo)?;
        let body = serde_json::json!({ "repositories": [repo.name] }).to_string();
        let v = self
            .post(
                &format!("/app/installations/{}/access_tokens", installation.id),
                &self.jwt()?,
                &body,
            )
            .with_context(|| format!("making a token for {repo}"))?;
        let secret = v["token"]
            .as_str()
            .ok_or_else(|| anyhow!("GitHub gave no token for {repo}"))?
            .to_string();
        let ends = v["expires_at"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .ok_or_else(|| anyhow!("GitHub gave a token for {repo} with no end"))?
            .with_timezone(&Utc);
        self.tokens.borrow_mut().insert(
            repo.clone(),
            Token {
                secret: secret.clone(),
                ends,
            },
        );
        Ok(secret)
    }

    /// `GET` an API path with a bearer token.
    pub fn get(&self, path: &str, bearer: &str) -> Result<Value> {
        let resp = self
            .http
            .get(format!("{}{path}", self.api))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Authorization", format!("Bearer {bearer}"))
            .call();
        answer("GET", path, resp)
    }

    fn post(&self, path: &str, bearer: &str, body: &str) -> Result<Value> {
        let resp = self
            .http
            .post(format!("{}{path}", self.api))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Authorization", format!("Bearer {bearer}"))
            .header("Content-Type", "application/json")
            .send(body);
        answer("POST", path, resp)
    }
}

/// The JSON of a successful answer; any other is an error with GitHub's
/// message.
fn answer(
    method: &str,
    path: &str,
    resp: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<Value> {
    let mut resp = resp.with_context(|| format!("{method} {path}"))?;
    let status = resp.status().as_u16();
    let text = resp
        .body_mut()
        .read_to_string()
        .with_context(|| format!("{method} {path}"))?;
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if !(200..300).contains(&status) {
        let why = v["message"].as_str().unwrap_or(text.trim());
        bail!("{method} {path}: {status} {why}");
    }
    Ok(v)
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
