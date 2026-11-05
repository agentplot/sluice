//! 124, 148: a write the protection refused, kept once per cause in the
//! branch's document while it stands and counted there, never from the log
//! (rule core.47). A cause is the host whose ref was tried, the ref, and the
//! command that tried it with each full object name in it read as
//! `<revision>` (design D1). It stands until a day passes with none of its
//! writes refused (design D4). Nothing a round, gate, landing or submission
//! decides reads it (rule core.14): only the hook's write, the write that
//! ends it, status and the overview do.

use chrono::{DateTime, Utc};

use crate::records::RefusedWrite;

/// How long a cause stands after its last refused write: a day.
pub const SHOWN: chrono::Duration = chrono::Duration::seconds(86_400);

/// How many causes one host keeps standing on one branch; a cause past
/// them ends the one whose last refusal is oldest.
pub const STANDING: usize = 20;

/// `by`, the command git ran, with each whole word of 40 or 64
/// hexadecimal digits, no letter, digit or underscore on either side,
/// written as `<revision>`. Nothing else of it changes.
pub fn command_of(by: &str) -> String {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = String::with_capacity(by.len());
    let mut rest = by;
    while let Some(start) = rest.find(word) {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let end = after.find(|c: char| !word(c)).unwrap_or(after.len());
        let w = &after[..end];
        if matches!(w.len(), 40 | 64) && w.bytes().all(|b| b.is_ascii_hexdigit()) {
            out.push_str("<revision>");
        } else {
            out.push_str(w);
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// Whether `cause` stands at `now`: less than a day since its last refused
/// write. A cause whose `last` cannot be read stands no more.
pub fn stands(cause: &RefusedWrite, now: &str) -> bool {
    match (ends(cause), time(now)) {
        (Some(end), Some(now)) => now < end,
        _ => false,
    }
}

/// When `cause` ends: its last refused write and a day; `None` where its
/// `last` cannot be read.
pub fn ends(cause: &RefusedWrite) -> Option<DateTime<Utc>> {
    Some(time(&cause.last)? + SHOWN)
}

fn time(at: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(at)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

impl RefusedWrite {
    /// The cause: two refused writes are one exactly when these are the
    /// same.
    pub fn key(&self) -> (&str, &str, &str) {
        (&self.host, &self.reference, &self.command)
    }

    /// The time its last write was refused, for ordering; `None` sorts
    /// first.
    pub fn last_at(&self) -> Option<DateTime<Utc>> {
        time(&self.last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sluice_seams::Rev;

    const FULL: &str = "8079db3c2b1e9f0a4d5c6b7a8f9e0d1c2b3a4f5e";
    const OTHER: &str = "10e4791a2b3c4d5e6f708192a3b4c5d6e7f80912";

    fn cause(last: &str) -> RefusedWrite {
        RefusedWrite {
            host: "build-1".into(),
            reference: "refs/heads/main".into(),
            command: "git reset --hard origin/main".into(),
            times: 1,
            first: last.into(),
            last: last.into(),
            from: Rev::new("10e4791"),
            to: Some(Rev::new("8079db3")),
        }
    }

    /// 148: a full revision of 40 or 64 digits is `<revision>`, wherever
    /// it stands in the command.
    #[test]
    fn a_full_revision_is_written_as_revision() {
        assert_eq!(
            command_of(&format!(
                "git -C /home/me/app update-ref refs/heads/main {FULL} {OTHER}"
            )),
            "git -C /home/me/app update-ref refs/heads/main <revision> <revision>"
        );
        let sha256 = FULL.repeat(2)[..64].to_string();
        assert_eq!(
            command_of(&format!("git update-ref refs/heads/main {sha256}")),
            "git update-ref refs/heads/main <revision>"
        );
        assert_eq!(
            command_of(&format!("git push origin {FULL}:refs/heads/main")),
            "git push origin <revision>:refs/heads/main"
        );
    }

    /// 148: an abbreviated revision, a word of 41 digits, and digits inside
    /// a longer word are left as written, and nothing else changes.
    #[test]
    fn anything_but_a_whole_full_revision_is_left_as_written() {
        for by in [
            "git branch -f main 8079db3",
            &format!("git branch -f main {FULL}a"),
            &format!("git branch -f main x{FULL}"),
            &format!("git branch -f main {FULL}_x"),
            &format!("git branch -f main _{FULL}"),
            "git commit -m 'views: refresh  anchors\n\tagain' --  ",
            "",
            "  ",
        ] {
            assert_eq!(command_of(by), by);
        }
    }

    /// 148: a cause stands one second before its day, and not at it.
    #[test]
    fn a_cause_stands_until_a_day_after_its_last() {
        let c = cause("2026-11-02T16:01:20Z");
        assert!(stands(&c, "2026-11-02T16:01:20Z"));
        assert!(stands(&c, "2026-11-03T16:01:19Z"));
        assert!(!stands(&c, "2026-11-03T16:01:20Z"));
        assert_eq!(
            ends(&c)
                .unwrap()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "2026-11-03T16:01:20Z"
        );
    }

    /// A cause whose last refusal cannot be read stands no more, and has
    /// no end of its own.
    #[test]
    fn a_cause_whose_last_cannot_be_read_stands_no_more() {
        let c = cause("yesterday");
        assert!(!stands(&c, "2026-11-02T16:01:20Z"));
        assert_eq!(ends(&c), None);
    }
}
