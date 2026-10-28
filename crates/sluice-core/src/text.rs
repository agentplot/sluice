//! What a landing commit says (23; ADR 0006; walkthrough section 8), and
//! how a subject is shown on a person's line (26).

use crate::records::{Candidate, Submission};

/// The landing commit's message: the policy's subject, with the submission
/// filled in, and the verdict it landed on as `Sluice-*` trailers.
pub fn landing_message(subject: &str, to: &str, sub: &Submission, verdict: &Candidate) -> String {
    let subject = subject
        .replace("{id}", &sub.id.to_string())
        .replace("{from}", &sub.from.worktree)
        .replace("{agent}", &sub.from.agent)
        .replace("{to}", to);
    format!(
        "{subject}\n\
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

/// A submission's name as a line shows it (26): each run of line breaks
/// or other control characters one space, so the name stays on its line
/// and paints nothing, and the ends trimmed. Never what a record or a
/// JSON document holds, which keeps the submitter's words as written.
pub fn shown(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut run = false;
    for c in name.chars() {
        if c.is_control() {
            if !run {
                out.push(' ');
            }
            run = true;
        } else {
            out.push(c);
            run = false;
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_subject_is_shown_on_one_line() {
        assert_eq!(shown("Batch review\ncards"), "Batch review cards");
        assert_eq!(shown("Batch review\r\n\ncards\n"), "Batch review cards");
        assert_eq!(shown("Batch\treview cards"), "Batch review cards");
        assert_eq!(shown("\u{1b}[31mBatch\u{1b}[0m"), "[31mBatch [0m");
        assert_eq!(
            shown("Fix the forecast's dates"),
            "Fix the forecast's dates"
        );
    }
}
