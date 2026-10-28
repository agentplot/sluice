//! The session a submit runs in, as herdr names it, and the outcome
//! pushed into it (50, 51; C.5 worktrunk; ADR 0068). A pane herdr runs
//! has `HERDR_PANE_ID` and `HERDR_SOCKET_PATH`; the session is
//! `herdr:<pane>@<socket>`, which the core shows and never reads into
//! (rule core.1). The outcome is one line of sluice's words.

use sluice_seams::Notification;

const PANE: &str = "HERDR_PANE_ID";
const SOCKET: &str = "HERDR_SOCKET_PATH";
const PREFIX: &str = "herdr:";

/// 51: the session this process runs in, or `None` outside any herdr
/// pane.
pub fn session_here() -> Option<String> {
    session_of(|k| std::env::var(k).ok())
}

/// The session the variables `var` reads name: `herdr:<pane>@<socket>`,
/// or `None` where either is unset or empty.
pub fn session_of(var: impl Fn(&str) -> Option<String>) -> Option<String> {
    let given = |k| var(k).filter(|v| !v.is_empty());
    Some(format!("{PREFIX}{}@{}", given(PANE)?, given(SOCKET)?))
}

/// A session this profile named: its pane and the socket of the herdr
/// server the pane is on. A pane's ID holds no `@`.
pub fn pane_of(session: &str) -> Option<(&str, &str)> {
    let (pane, socket) = session.strip_prefix(PREFIX)?.split_once('@')?;
    (!pane.is_empty() && !socket.is_empty()).then_some((pane, socket))
}

/// 50: the outcome in one line of sluice's words, the same as `sluice
/// untold` tells it (51).
pub fn words(n: &Notification) -> String {
    crate::words::told_line(n.submission, &n.branch, &n.told)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 51: a pane is named by its ID and its server's socket, and only
    /// where both are set.
    #[test]
    fn a_session_is_its_pane_on_its_server() {
        let env = |pane: Option<&str>, socket: Option<&str>| {
            session_of(|k| match k {
                PANE => pane.map(str::to_string),
                SOCKET => socket.map(str::to_string),
                _ => None,
            })
        };
        let named = env(Some("w1:p2"), Some("/run/herdr.sock"));
        assert_eq!(named.as_deref(), Some("herdr:w1:p2@/run/herdr.sock"));
        assert_eq!(
            pane_of(named.as_deref().unwrap()),
            Some(("w1:p2", "/run/herdr.sock"))
        );
        assert_eq!(
            pane_of("herdr:w1:p2@/home/a@b/herdr.sock"),
            Some(("w1:p2", "/home/a@b/herdr.sock"))
        );
        assert_eq!(env(Some("w1:p2"), None), None);
        assert_eq!(env(None, Some("/run/herdr.sock")), None);
        assert_eq!(env(Some(""), Some("/run/herdr.sock")), None);
        assert_eq!(env(Some("w1:p2"), Some("")), None);
        assert_eq!(pane_of("tmux:%1"), None);
        assert_eq!(pane_of("herdr:@/run/herdr.sock"), None);
    }
}
