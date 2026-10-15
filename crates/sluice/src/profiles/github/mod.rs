//! sluice's identity on GitHub (C.4 pushed refs: the forge's rules, which
//! only sluice's identity bypasses): a GitHub App whose key is on the
//! host. A JWT signed with the key finds the app's installation on a
//! repository and buys an installation token for it, which only sluice's
//! own git calls are given (`docs/adr/0019`).

pub mod app;
pub mod check;
pub mod fake;
pub mod forge;
pub mod pulls;
mod rest;

pub use app::{App, Repo};
pub use forge::Forge;

use anyhow::Result;

use crate::identity::{Credentials, Identity};

/// The username git gives GitHub with an installation token.
const TOKEN_USER: &str = "x-access-token";

impl Identity for App {
    fn name(&self) -> String {
        format!("GitHub app {}", self.id())
    }

    /// A remote on github.com, over HTTPS whatever its URL, since the
    /// app's token is no SSH key.
    fn url(&self, remote_url: &str) -> Option<String> {
        Repo::from_url(remote_url).map(|r| r.https())
    }

    fn credentials(&self, url: &str) -> Result<Credentials> {
        let repo = Repo::from_url(url)
            .ok_or_else(|| anyhow::anyhow!("{url} is no repository on GitHub"))?;
        Ok(Credentials {
            username: TOKEN_USER.into(),
            password: self.token(&repo)?,
        })
    }
}
