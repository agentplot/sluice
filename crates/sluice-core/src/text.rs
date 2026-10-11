//! What a landing commit says (23; ADR 0006; walkthrough section 8).

use crate::records::{Candidate, Submission};

/// The landing commit's message: the submission and its branch as the
/// subject, and the verdict it landed on as `Sluice-*` trailers.
pub fn landing_message(sub: &Submission, verdict: &Candidate) -> String {
    format!(
        "Land #{id}: {worktree}\n\
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
