## Context

`wt-submit` (`crates/sluice/src/profiles/worktrunk/wt_submit.rs`) forwards its arguments to `sluice submit`. When a waiting submit exits landed, it asks sluice whether the worktree's submission landed (`landed()`, which reads `sluice submit <branch> --status --json --worktree <w>`). If so, it runs `sluice release --worktree <w>`, then `wt remove --foreground`, unless `--no-remove` is given or worktrunk's `[merge] remove` is false. `Submit::parse` reads the command line as sluice reads it.

A same-head `sluice submit` is a new submission: the core's `admit` withdraws the worktree's standing entry "by resubmission" (67) and queues the same head again at the back. `sluice wait <branch> --worktree <w>` already follows the worktree's last submission, and on a branch that takes pull requests the open pull request, taken in or not; it has `--timeout` and exits 0 to 5. `sluice submit --status --json` prints the last submission's record or, on a pull-request branch with an open request, `{pull_request, branch, state}`; where there is nothing, `{"error":{"kind":"not_found",…}}` with exit 1.

What a submit sends is the core's to decide: `Sluice::submitted_head` (`crates/sluice-core/src/ops.rs`) gives the worktree's head, or, from a worktree that has checked out a branch sluice lands on, the remote's revision of that branch (124). It reads and writes nothing (core.4).

Record 0065 puts the resumed wait in the driver, never in the core (core.1, core.16, 25). The statement's worktrunk row at agentplot/blueprints a048b60 adds that a `--no-wait` run again answers with the standing entry, that `--timeout` serves a first submit too, and that a standing submission is read at the revision a submit from the place would send.

## Goals / Non-Goals

**Goals:**

- The resumed wait is built from the core's existing commands, so a round, its lock and its records are the core's alone (core.4, core.14). `wt-submit` adds only the choice between submitting, following and answering, and the removal it already does.
- A run that cannot tell whether a submission stands submits nothing, so it can never withdraw a standing entry by mistake.
- The driver never decides which revision a worktree sends. It reads that from the core (core.1, core.27).

**Non-Goals:**

- No change to `sluice submit`'s own handling of a same-head submit, which every driver shares (67), and no `--timeout` on `sluice submit`.
- No read of what stands for a submit of several worktrees (125).

## Decisions

**The core names what a submit would send.** `sluice submit <branch> --status --json --worktree <w>` adds `sends`, the revision `submitted_head` gives for the worktree's session now, to the document it prints: to the submission's record, and to the open pull request's document. Reading it reads the branch as a submit does, the remote's revision on a branch kept on a remote, under the same retry policy (core.29); a read that fails fails the status read as any read of the branch does. The human `--status` is unchanged. *Alternative:* the driver reads the worktree's branch and the remote's tracking ref itself. Rejected: it would repeat the core's routing (core.27) in a profile, and read a revision no submit sends.

**What stands is read in `wt-submit`, from that document.** Before a single-worktree run that would submit, waiting or `--no-wait`, `wt-submit` reads `sluice submit <branch> --status --json --worktree <w>`. A submission stands when:

- the document is a submission's record whose `head` is its `sends` and whose `state` is `queued`, `waiting`, `testing`, `passed`, `landing` or `landed`; or
- the document is an open pull request (it has `pull_request` and no `id`, and `state` is not `closed`) whose `head` is its `sends` and whose `draft` is `false`.

A `not_found` error, or any other record, means nothing stands. Any other answer, such as an exit other than 0 or 1, a document that does not parse, a document with no `sends`, or another error kind, makes `wt submit` say sluice's words and exit 3 with nothing submitted. Treating an unread state as "nothing stands" would resubmit and withdraw the standing entry. `landed()` reads the same answer, so a worktree is removed only on the landing of the revision it would send. *Alternative:* teach the core's `sluice submit` to keep a same-head entry. Rejected: it changes 67 for every driver, and the removal would still need the driver.

**A run where a submission stands:**

- A waiting run follows it with `sluice wait <branch> --worktree <w>`, run as a child with `--timeout <d>` and `--json` when given. Its stdout, stderr and exit pass through unchanged. Before it, `wt submit` prints one line on stderr: `wt submit: following submission <id>, already submitted at this revision`, or `wt submit: following pull request #<n>, already open at this revision`. On exit 0 the existing landed → release → remove step runs, which also covers "landed and not yet removed": `sluice wait` reports a decided submission at once with no round. *Alternative:* `sluice wait <id>`. Rejected because a pull request not yet taken in has no id, and the row says "as `sluice wait <branch>`".
- A `--no-wait` run answers with it: it runs `sluice submit <branch> --status --worktree <w>`, with `--json` when given, and passes its output and exit through. It removes nothing, even where the submission landed: only a run that waits removes, as today.
- A run naming `--after` or `--priority` is refused with exit 64, nothing submitted: `wt submit: submission <id> stands at this revision; change its place with sluice amend <id>`, or for a pull request not yet taken in, that it stands and only a new head submits again. *Alternative:* submit and withdraw the standing one. Rejected: the row has a new head alone submit again. *Alternative:* follow and ignore the flags. Rejected: it drops what the caller said without a word.

**A run where nothing stands** submits. Without `--timeout` it forwards to `sluice submit` as today, waiting or not. With `--timeout` it runs `sluice submit <branch> --no-wait` with the run's other arguments, `--json` included. If that exits other than 0, its stdout, stderr and exit are the run's, and nothing waits. If it exits 0, its stdout goes to the run's stderr, so stdout is left to one document, and the run then follows as above, under the limit. The followed wait's output and exits are the run's, and its landing removes as above.

**`--timeout` is `wt submit`'s own flag.** It is stripped from the arguments as `--no-remove` is, in both `--timeout <d>` and `--timeout=<d>` forms, and checked with the parser `sluice wait` uses (`fn timeout` in `crates/sluice/src/lib.rs`, made `pub`). A bad value exits 64 as `sluice wait` does. `--timeout` with `--no-wait`, `--check`, `--status` or several `--worktree`s is refused with exit 64 before anything runs, since none of those waits.

**The pull request's status carries `head` and `draft`.** `Followed::Open` in `crates/sluice/src/lib.rs` carries the request's head and draft mark. The GitHub profile's `follow` fills them from the `Pull` it reads, and `submit_pull` in `PullMode::Status` adds them, with `sends`, to its JSON document. The `--no-wait` and wait documents stay as the pull-request-submit and submission-wait specs fix them. The fake `PullRequests` in `crates/sluice/tests/cli.rs` fills the new fields. A request whose pull the forge did not return has `head` `null`, which never matches `sends`.

**The briefs follow.** The session-start note (`plugins/sluice/scripts/session-context.sh`), the using-sluice skill's submit reference and SKILL.md, the worktrunk setup reference and the site's worktrunk guide say: under worktrunk, submit and wait with `wt submit <branch> --timeout 9m --json` as a background command, again while it exits 5, which removes the worktree once it lands; `wt submit <branch> --no-wait --json` answers with what stands; never remove a landed worktree by hand; `sluice wait` removes nothing. The words follow core.26.

## Risks / Trade-offs

- [The submission is decided between the status read and the wait.] → `sluice wait` reports a decided submission at once. A withdrawal in that gap exits 4, the agent sees it, and nothing is resubmitted.
- [The status read now reads the branch's revision, which on a branch kept on a remote is a fetch.] → It is the read a submit makes, under the same retry policy, and a failure exits 3 with nothing submitted.
- [A first `--timeout` run whose follow fails to read after the submit succeeded exits 3 with a submission made.] → The submission stands, so the next run follows it rather than submitting again.
- [A forge read that fails on a pull-request branch would block the resumed wait.] → It exits 3 with the forge's words and submits nothing, as `sluice wait` does when it cannot read.

## Migration Plan

Nothing is stored differently. Once `wt-submit` is installed, an agent's existing `--no-wait` submissions are followed by a run again. Agents still on `sluice wait` keep working and keep their worktrees, as today.
