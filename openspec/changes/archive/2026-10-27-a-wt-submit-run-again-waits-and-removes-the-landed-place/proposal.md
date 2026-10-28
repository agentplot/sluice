## Why

An agent that submits with `wt submit <branch> --no-wait` and then waits with `sluice wait` finds its worktree still there after its work lands, because `sluice wait` is the core's and the core never removes a session's worktree (25; rule core.16). Every agent that follows the briefs removes its landed worktree by hand with `wt remove`, which the user read as misuse. Record 0065 decides that the driver's wait runs again instead: `wt submit` run again in the worktree follows the standing submission and removes the worktree when it lands, and `sluice wait` stays the core's wait that removes nothing. The statement's worktrunk row says so, and as of agentplot/blueprints a048b60 it also says how `--no-wait` and `--timeout` behave on a run again and at which revision a submission stands (C.5, with 124).

## What Changes

- A submission stands when it is the worktree's last submission to the branch, it is at the revision a submit from that worktree would send now, and it is queued, waiting, under test, passed, landing or landed. That revision is the worktree's head, or, in a worktree that has checked out a branch sluice lands on, the remote's revision of that branch (124). So that `wt submit` can tell, `sluice submit <branch> --status --json` gains `sends`, the revision a submit from the worktree would send now.
- `wt submit <branch>` run where a submission stands submits nothing and withdraws nothing. It follows that submission as `sluice wait <branch>` does and, when it lands, releases the worktree and removes it as a waiting `wt submit` does today. Run where that submission has already landed, it removes the worktree at once.
- `wt submit <branch> --no-wait` run where a submission stands answers with it at once, as `wt submit <branch> --status` prints it, and exits 0. It submits nothing, withdraws nothing and removes nothing.
- `wt submit` takes `--timeout <duration>`, as `sluice wait` does, wherever it waits. Where a submission stands, it bounds the follow. Where none stands, it submits with `sluice submit <branch> --no-wait` and then waits as `sluice wait` does under that limit. Either way it ends with `sluice wait`'s exits and document: 0 landed, 1 failed, 2 conflict, 3 could not read, 4 withdrawn, 5 still undecided when the time ran out. One command, `wt submit <branch> --timeout 9m --json` run from the harness's background again while it exits 5, takes an agent from its submit to its landing.
- A new head, or a submission that failed, conflicted or was withdrawn, is submitted again as today (54, 67). A run naming `--after` or `--priority` where a submission stands is refused with exit 64 and names `sluice amend`, since only a new head submits again.
- On a branch that takes pull requests, the pull request stands while it is open, out of draft and at the revision the worktree would send, or once its submission landed there. `sluice submit <branch> --status --json` on such a branch gains the pull request's `head` and `draft` beside `pull_request`, `branch` and `state`, and `sends` with them.
- `sluice wait` removes no worktree, now stated and tested.
- The using-sluice skill, the session-start note, the worktrunk setup reference and the site's worktrunk guide tell an agent under worktrunk to submit and wait with `wt submit <branch> --timeout 9m --json` in rounds, and never to remove a landed worktree by hand.

## Capabilities

### New Capabilities

- `worktrunk-submit`: what `wt submit` does beyond `sluice submit`: removing a landed worktree, following or answering with a standing submission when run again, and waiting under its own `--timeout`.

### Modified Capabilities

- `submission-wait`: `sluice wait` removes no worktree and releases no place, whatever it waits on.
- `remote-branch`: `sluice submit --status --json` names `sends`, the revision a submit from the worktree would send now, the remote's on a branch sluice lands on (124).
- `pull-request-submit`: `--status --json` on a branch that takes pull requests names the pull request's head, whether it is a draft, and the revision the worktree would send.
- `plugin-usage`: an agent under worktrunk with capped tool calls submits and waits with `wt submit --timeout` in rounds and removes nothing by hand.

## Out of scope

- `sluice submit` itself changes nothing: it takes no `--timeout`, and a same-head `sluice submit` still withdraws the entry standing (67). Only the worktrunk driver reads what stands.
- A submit of several worktrees as one act (125) is not read for what stands: `wt submit <branch> --worktree a --worktree b` submits as today, and `--timeout` with it is refused.
- crew's briefs and the fleet's agents that run `sluice wait` and then `wt remove` change once this lands. Record 0065 leaves that to crew and the fleet. Nothing in this repository waits on it.

## Impact

- `crates/sluice/src/profiles/worktrunk/`: `wt-submit` reads what stands before it submits, follows or answers with it, submits and then waits under its own `--timeout`, and strips and checks that flag.
- `crates/sluice/src/lib.rs` and `crates/sluice/src/profiles/github/pulls.rs`: the `--status --json` document names `sends`, and a followed pull request carries its head and draft mark. The timeout parser `sluice wait` uses is shared with `wt-submit`.
- Tests: the worktrunk and pull-request profile suites, the CLI suite and its fake pull requests, and the plugin hook test.
- Docs: `plugins/sluice` (skill references, session-start note) and `site/src/content/docs/guides/worktrunk.md`.
- Sources: record 0065; statement C.5, worktrunk row, and clause 124 (agentplot/blueprints a048b60); clauses 25, 50, 54, 67; rules core.1, core.4, core.16, core.27.

## Touches

```
crates/sluice/src/profiles/worktrunk/*
crates/sluice/src/lib.rs
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/tests/profiles/worktrunk.rs
crates/sluice/tests/profiles/pull_requests.rs
crates/sluice/tests/cli.rs
plugins/sluice/scripts/session-context.sh
plugins/sluice/tests/hooks.sh
plugins/sluice/skills/using-sluice/**
plugins/sluice/skills/setting-up-sluice/references/profiles/worktrunk.md
site/src/content/docs/guides/worktrunk.md
```
