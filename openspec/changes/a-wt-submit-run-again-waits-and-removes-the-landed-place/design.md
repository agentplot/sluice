## Context

`wt-submit` (`crates/sluice/src/profiles/worktrunk/wt_submit.rs`) forwards its arguments to `sluice submit`. When a waiting submit exits landed, it asks sluice whether the worktree's head landed (`landed()`, which reads `sluice submit <branch> --status --json --worktree <w>`). If so, it runs `sluice release --worktree <w>`, then `wt remove --foreground`, unless `--no-remove` is given or worktrunk's `[merge] remove` is false. `Submit::parse` decides whether a run is a waiting one. Only a single-worktree submit without `--no-wait`, `--check` or `--status` is.

A run again at the same head today is a new submission. The core's `admit` withdraws the worktree's standing entry "by resubmission" (67) and queues the same head again at the back. `sluice wait <branch> --worktree <w>` already follows the worktree's last submission: on a branch that takes pull requests it follows the open pull request, taken in or not. It already has `--timeout` and exits 0 to 5. `sluice submit --status --json` prints the last submission's record (`id`, `state`, `head`, …) or, on a pull-request branch with an open request, `{pull_request, branch, state}`. Where there is nothing, it prints `{"error":{"kind":"not_found",…}}` and exits 1.

Record 0065 puts the resumed wait in the driver, never in the core (core.1, core.16, 25). See proposal.md for why.

## Goals / Non-Goals

**Goals:**

- The resumed wait is built from the core's existing commands, so a round, its lock and its records are the core's alone (core.4, core.14). `wt-submit` adds only the choice between submitting and following, and the removal it already does.
- A run that cannot tell whether a submission stands submits nothing, so it can never withdraw a standing entry by mistake.

**Non-Goals:**

- No change to `sluice submit`'s own handling of a same-head submit, which every driver shares (67).
- No `--timeout` on `sluice submit`, and no new behaviour for `wt submit --no-wait` run again (proposal, Out of scope).

## Decisions

**The choice is made in `wt-submit`, from `sluice submit --status --json`.** A waiting run first reads `sluice submit <branch> --status --json --worktree <w>`, as `landed()` does, and the worktree's `HEAD`, as `landed()` reads it. It follows rather than submits when:

- the document is a submission's record whose `head` is the worktree's `HEAD` and whose `state` is `queued`, `waiting`, `testing`, `passed`, `landing` or `landed`; or
- the document is an open pull request (it has `pull_request` and no `id`, and `state` is not `closed`) whose `head` is the worktree's `HEAD` and whose `draft` is `false`.

A `not_found` error means nothing stands, and the run submits. Any other failure to read, such as an exit other than 0 or 1, a document that does not parse, or another error kind, makes `wt submit` say sluice's words and exit 3 with nothing submitted. Treating an unread state as "nothing stands" would resubmit and withdraw the standing entry. *Alternative:* teach the core's `sluice submit` to wait on a same-head entry. Rejected: it changes 67 for every driver, and the removal would still need the driver.

**Following is `sluice wait <branch> --worktree <w>`, run as a child.** It takes `--timeout <d>` and `--json` when given. Its stdout, stderr and exit pass through unchanged, so the resumed wait's output is `sluice wait`'s exactly, a pull request not yet taken in included. On exit 0 the existing landed → release → remove step runs unchanged, which also covers "landed and not yet removed": `sluice wait` reports a decided submission at once with no round. *Alternative:* `sluice wait <id>`. Rejected because a pull request not yet taken in has no id, and `<branch>` follows both kinds as the statement's row says ("as `sluice wait <branch>` follows").

**`--timeout` is `wt submit`'s own flag.** It is stripped from the arguments as `--no-remove` is, in both `--timeout <d>` and `--timeout=<d>` forms, and checked with the same parser `sluice wait` uses. `fn timeout` in `crates/sluice/src/lib.rs` is made `pub` for this. A bad value exits 64 as `sluice wait` does. `--timeout` given with `--no-wait`, `--check`, `--status` or several `--worktree`s is refused with exit 64 before anything runs. Given on a waiting run where nothing stands, it is refused with exit 64 after the status read, naming `wt submit <branch> --no-wait`. A time limit on a first submit would need `sluice submit` to take one, which no source gives it.

**A run naming `--after` or `--priority` still submits.** These change a submission's place in the queue, which only a submission can set. A run that names them is read as asking for one, as today. *Alternative:* follow and ignore them. Rejected because it would drop what the caller said without a word.

**The resumed run says what it is doing.** Before following, `wt submit` prints one line on stderr, so stdout stays `sluice wait`'s document: `wt submit: following submission <id>, already submitted at this head`, or `wt submit: following pull request #<n>, already open at this head`.

**The pull request's status carries `head` and `draft`.** `Followed::Open` in `crates/sluice/src/lib.rs` gains the request's head and draft mark. The GitHub profile's `follow` fills them from the `Pull` it already reads (`pull.head`, `pull.draft`), and `submit_pull` in `PullMode::Status` adds them to its JSON document. The `--no-wait` and wait documents stay as the pull-request-submit and submission-wait specs fix them. The fake `PullRequests` in `crates/sluice/tests/cli.rs` fills the new fields. A request whose pull the forge did not return has no head, and prints `head` `null`, which never matches a worktree's head.

**The briefs follow.** The session-start note (`plugins/sluice/scripts/session-context.sh`), the using-sluice skill's submit reference and SKILL.md, the worktrunk setup reference and the site's worktrunk guide say: under worktrunk, wait with `wt submit <branch> --timeout 9m --json` in the background again while it exits 5, and never remove a landed worktree by hand. They also say `sluice wait` removes nothing. The words follow core.26.

## Risks / Trade-offs

- [The submission is decided between the status read and the wait.] → `sluice wait` reports a decided submission at once. A withdrawal in that gap exits 4, the agent sees it, and nothing is resubmitted.
- [A worktree on a branch sluice lands on submits the remote's revision, not its own `HEAD` (124).] → The comparison misses, and the run submits as today. This matches `landed()`, which already compares `HEAD`, and the bolt's own submit is outside this change.
- [An agent running `wt submit <branch> --timeout 9m` after its submission failed gets exit 64, not a resubmission.] → That is deliberate: a failed submission needs a fix and a deliberate `--no-wait` submit, and the message names it.
- [A forge read that fails on a pull-request branch would block the resumed wait.] → It exits 3 with the forge's words and submits nothing, as `sluice wait` does when it cannot read.

## Migration Plan

Nothing is stored differently. Once `wt-submit` is installed, an agent's existing `--no-wait` submissions are followed by a run again. Agents still on `sluice wait` keep working and keep their worktrees, as today.
