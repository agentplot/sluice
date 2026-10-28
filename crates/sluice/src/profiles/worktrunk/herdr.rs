//! The session a submit runs in, as herdr names it, and the outcome
//! pushed into it (50, 51; C.5 worktrunk; ADR 0068). A pane herdr runs
//! has `HERDR_PANE_ID` and `HERDR_SOCKET_PATH`; the session is
//! `herdr:<pane>@<socket>`, which the core shows and never reads into
//! (rule core.1). The outcome is one line of sluice's words.

use sluice_seams::{Notification, Told};

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

/// 50: the outcome in one line, opening `[sluice]` so the agent and a
/// reader of its transcript never take it for the user's typing.
pub fn words(n: &Notification) -> String {
    let (id, branch) = (n.submission, &n.branch);
    match &n.told {
        Told::Landed { landed_as } => {
            format!(
                "[sluice] #{id} landed on {branch} as {}.",
                landed_as.short()
            )
        }
        Told::Failed {
            failed, timed_out, ..
        } => {
            let what = if *timed_out {
                ": timed out".to_string()
            } else if failed.is_empty() {
                String::new()
            } else {
                format!(": {}", failed.join(", "))
            };
            format!("[sluice] #{id} failed on {branch}{what}. sluice wait {id} shows why.")
        }
        Told::Conflict { paths, with } => {
            let mut text = format!("[sluice] #{id} conflicts on {branch}");
            if !with.is_empty() {
                let ids: Vec<String> = with.iter().map(|c| format!("#{}", c.id)).collect();
                text.push_str(&format!(" with {}", ids.join(", ")));
            }
            if !paths.is_empty() {
                text.push_str(&format!(" in {}", paths.join(", ")));
            }
            text.push_str(&format!(". sluice wait {id} shows it."));
            text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sluice_seams::{Contested, Rev, Submitter};

    fn told(told: Told) -> Notification {
        Notification {
            id: "12:landed".into(),
            submission: 12,
            branch: "main".into(),
            to: Submitter {
                agent: "a".into(),
                worktree: "unit/a".into(),
                path: None,
                head: None,
                host: None,
                session: None,
            },
            told,
            report: None,
        }
    }

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

    /// 50: each outcome in one line of sluice's words.
    #[test]
    fn an_outcome_is_one_line() {
        let landed = told(Told::Landed {
            landed_as: Rev::new("abc1234def5678"),
        });
        assert_eq!(words(&landed), "[sluice] #12 landed on main as abc1234.");

        let failed = |timed_out| {
            told(Told::Failed {
                failed: vec!["cli:test".into(), "core:test".into()],
                combination: Rev::new("c"),
                submissions: vec![12],
                timed_out,
                logs: Vec::new(),
            })
        };
        assert_eq!(
            words(&failed(false)),
            "[sluice] #12 failed on main: cli:test, core:test. sluice wait 12 shows why."
        );
        assert_eq!(
            words(&failed(true)),
            "[sluice] #12 failed on main: timed out. sluice wait 12 shows why."
        );

        let conflict = told(Told::Conflict {
            paths: vec!["src/lib.rs".into()],
            with: vec![Contested {
                id: 11,
                from: landed.to.clone(),
            }],
        });
        assert_eq!(
            words(&conflict),
            "[sluice] #12 conflicts on main with #11 in src/lib.rs. sluice wait 12 shows it."
        );
    }
}
