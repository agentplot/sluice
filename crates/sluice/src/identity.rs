//! sluice's own identity on a forge (C.4 pushed refs: the forge's rules,
//! which only sluice's identity bypasses). A profile of the forge gives
//! credentials for a remote; the pushed refs profile hands them to its own
//! git calls through `sluice credential`, a helper named on that call's
//! command line and given the secret in that call's environment alone.
//! Nothing is written to a git config, so an agent's git never finds the
//! helper and never holds the secret.

use std::io::{BufRead, Write};

use anyhow::Result;

/// What git is given to authenticate one call.
pub struct Credentials {
    pub username: String,
    pub password: String,
    /// What they are, never the secret: whose, for what, and how old, for
    /// a report of the remote's refusing them.
    pub about: String,
}

/// An identity sluice has on a forge.
pub trait Identity {
    /// What it is called, for `sluice status`.
    fn name(&self) -> String;

    /// The URL sluice's git reaches `remote_url` by as this identity;
    /// `None` when the remote is on no forge it has an identity on.
    fn url(&self, remote_url: &str) -> Option<String>;

    /// Credentials for `url`, one [`Identity::url`] gave.
    fn credentials(&self, url: &str) -> Result<Credentials>;

    /// Credentials for `url` made afresh, never ones given before: for a
    /// call the remote refused the last ones on.
    fn renew(&self, url: &str) -> Result<Credentials>;
}

/// The variables a git call of sluice's is given its credentials in, read
/// by the helper that call runs.
pub const USERNAME_VAR: &str = "SLUICE_CREDENTIAL_USERNAME";
pub const PASSWORD_VAR: &str = "SLUICE_CREDENTIAL_PASSWORD";

/// `sluice credential <operation>`, git's credential helper protocol: a
/// `get` answers with the credentials in its environment, when it has
/// any; `store` and `erase` keep and remove nothing. What git sends is
/// read to its end either way.
pub fn helper(operation: &str, input: &mut dyn BufRead, out: &mut dyn Write) -> Result<()> {
    let mut line = String::new();
    while input.read_line(&mut line)? > 0 && line.trim_end() != "" {
        line.clear();
    }
    if operation != "get" {
        return Ok(());
    }
    if let (Ok(username), Ok(password)) = (std::env::var(USERNAME_VAR), std::env::var(PASSWORD_VAR))
    {
        write!(out, "username={username}\npassword={password}\n")?;
    }
    Ok(())
}
