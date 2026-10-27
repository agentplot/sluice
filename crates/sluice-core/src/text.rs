//! What a landing commit says (23; ADR 0006; walkthrough section 8).

use crate::records::{Candidate, Submission};

/// The landing commit's message: the policy's subject, with the submission
/// filled in, and the verdict it landed on as `Sluice-*` trailers.
/// `{from}` is what lands: the submit's subject or, where it gave none,
/// the branch submitted, which `Sluice-From` names either way (23). The
/// subject is filled in one pass, so a submitter's words are carried as
/// written, whatever braces they hold.
pub fn landing_message(subject: &str, to: &str, sub: &Submission, verdict: &Candidate) -> String {
    let id = sub.id.to_string();
    let from = sub.subject.as_deref().unwrap_or(&sub.from.worktree);
    let fills = [
        ("{id}", id.as_str()),
        ("{from}", from),
        ("{agent}", sub.from.agent.as_str()),
        ("{to}", to),
    ];
    let mut first = String::new();
    let mut rest = subject;
    while let Some(i) = rest.find('{') {
        first.push_str(&rest[..i]);
        rest = &rest[i..];
        let (name, value) = fills
            .iter()
            .find(|(name, _)| rest.starts_with(name))
            .copied()
            .unwrap_or(("{", "{"));
        first.push_str(value);
        rest = &rest[name.len()..];
    }
    first.push_str(rest);
    format!(
        "{first}\n\
         \n\
         Sluice-Submission: {id}\n\
         Sluice-From: {agent} ({worktree})\n\
         Sluice-Tested: {tested}\n\
         Sluice-Result: {result} ({ran} run, {replayed} replayed)\n",
        id = sub.id,
        worktree = sub.from.worktree,
        agent = sub.from.agent,
        tested = verdict.revision,
        result = verdict.result_word(),
        ran = verdict.ran,
        replayed = verdict.replayed,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(subject: Option<&str>) -> Submission {
        let mut json = serde_json::json!({
            "schema": crate::records::SUBMISSION_SCHEMA, "id": 7, "branch": "bolt/x",
            "head": "h", "from": { "agent": "crw-1-unit-1", "worktree": "unit/review" },
            "submitted": "2026-10-27T10:00:00Z", "after": [], "priority": 0,
            "attempt": 1, "state": "landing",
        });
        if let Some(s) = subject {
            json["subject"] = s.into();
        }
        serde_json::from_value(json).unwrap()
    }

    fn passed() -> Candidate {
        serde_json::from_value(serde_json::json!({
            "id": "c1", "on": "base", "submissions": [7], "revision": "c1c1",
            "state": "passed", "ran": 3,
        }))
        .unwrap()
    }

    fn first_line(subject: &str, s: &Submission) -> String {
        let m = landing_message(subject, "bolt/x", s, &passed());
        m.lines().next().unwrap().to_string()
    }

    /// 23: the default subject names the work by the submit's subject, and
    /// `Sluice-From` still names the agent and the branch.
    #[test]
    fn the_default_subject_names_the_work_by_its_subject() {
        let s = sub(Some("Batch review cards"));
        let m = landing_message(crate::policy::LANDING_SUBJECT, "bolt/x", &s, &passed());
        assert_eq!(
            m,
            "chore(land): #7 Batch review cards\n\nSluice-Submission: 7\n\
             Sluice-From: crw-1-unit-1 (unit/review)\nSluice-Tested: c1c1\n\
             Sluice-Result: passed (3 run, 0 replayed)\n"
        );
    }

    /// 23: with no subject, the work is named by its branch, as before.
    #[test]
    fn with_no_subject_the_work_is_named_by_its_branch() {
        assert_eq!(
            first_line(crate::policy::LANDING_SUBJECT, &sub(None)),
            "chore(land): #7 unit/review"
        );
    }

    /// 23: a policy's own landing subject takes the subject as `{from}`.
    #[test]
    fn a_policys_own_subject_takes_it_as_from() {
        let s = sub(Some("Batch review cards"));
        assert_eq!(
            first_line("land {from} on {to}", &s),
            "land Batch review cards on bolt/x"
        );
        assert_eq!(
            first_line("land {from} on {to}", &sub(None)),
            "land unit/review on bolt/x"
        );
    }

    /// A subject is carried as written: what looks like a placeholder in
    /// the submitter's words is not filled.
    #[test]
    fn a_subject_is_carried_as_written() {
        let s = sub(Some("Fill {to} and {id} in {"));
        assert_eq!(
            first_line("{agent}: {from} on {to}", &s),
            "crw-1-unit-1: Fill {to} and {id} in { on bolt/x"
        );
    }
}
