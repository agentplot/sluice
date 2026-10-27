## Why

An agent that submits with `wt submit <branch> --no-wait` and then waits with `sluice wait` finds its worktree still there after its work lands, because `sluice wait` is the core's and the core never removes a session's worktree (25; rule core.16). Every agent that follows the briefs removes its landed worktree by hand with `wt remove`, which the user read as misuse. Record 0065 decides that the driver's wait runs again instead: `wt submit` run again in the worktree follows the standing submission and removes the worktree when it lands, and `sluice wait` stays the core's wait that removes nothing. The statement's worktrunk row says so as of agentplot/blueprints ccc45c8 (C.5).

## What Changes

- `wt submit <branch>` run in a worktree whose last submission to `<branch>` is of the worktree's head and stands, queued, waiting, under test, passed, landing or landed, submits nothing. It follows that submission as `sluice wait <branch>` does and, when it lands, releases the worktree and removes it as a waiting `wt submit` does today. Run where that submission has already landed, it removes the worktree at once.
- `wt submit` takes `--timeout <duration>`, as `sluice wait` does. It bounds that wait, and the wait ends with `sluice wait`'s exits and document: 0 landed, 1 failed, 2 conflict, 3 could not read, 4 withdrawn, 5 still undecided when the time ran out. An agent runs `wt submit <branch> --timeout 9m --json` from its harness's background again while it exits 5, as it runs `sluice wait` today. `--timeout` given where nothing stands is refused with exit 64 and nothing is submitted.
- A worktree with a new head, or whose last submission failed, conflicted or was withdrawn, is submitted as today: a run again is a submission there (67, 54).
- On a branch that takes pull requests, the pull request stands while it is open, out of draft, at the worktree's head, or once its submission landed at that head. So that `wt submit` can tell, `sluice submit <branch> --status --json` on such a branch gains the pull request's `head` and `draft` beside `pull_request`, `branch` and `state`.
- `sluice wait` removes no worktree, now stated and tested.
- The using-sluice skill, the session-start note, the worktrunk setup reference and the site's worktrunk guide tell an agent under worktrunk to wait with `wt submit <branch> --timeout 9m --json` in rounds and never to remove a landed worktree by hand.

## Capabilities

### New Capabilities

- `worktrunk-submit`: what `wt submit` does beyond `sluice submit`: removing a landed worktree, and following a standing submission when run again, under `--timeout`.

### Modified Capabilities

- `submission-wait`: `sluice wait` removes no worktree and releases no place, whatever it waits on.
- `pull-request-submit`: `--status --json` on a branch that takes pull requests names the pull request's head and whether it is a draft.
- `plugin-usage`: an agent under worktrunk with capped tool calls waits with `wt submit --timeout` in rounds and removes nothing by hand.

## Out of scope

- A `wt submit --no-wait` run again at the same head still submits again, withdrawing the standing submission (67). Whether a no-wait run should instead answer with the standing submission is a decision neither record 0065 nor the statement makes. It waits on the design agent.
- `sluice submit` gains no `--timeout`, so a run that would submit refuses `--timeout`. A first submit that waits under a time limit waits on a design decision.
- crew's briefs and the fleet's agents that run `sluice wait` and then `wt remove` change once this lands. Record 0065 leaves that to crew and the fleet. Nothing in this repository waits on it.

## Impact

- `crates/sluice/src/profiles/worktrunk/`: `wt-submit` reads the worktree's standing submission before forwarding, and strips and validates its own `--timeout`.
- `crates/sluice/src/lib.rs` and `crates/sluice/src/profiles/github/pulls.rs`: a followed pull request carries its head and draft mark, which the pull-request `--status --json` document prints. The timeout parser `sluice wait` uses is shared with `wt-submit`.
- Tests: the worktrunk and pull-request profile suites, the CLI suite's fake pull requests, and the plugin hook test.
- Docs: `plugins/sluice` (skill references, session-start note) and `site/src/content/docs/guides/worktrunk.md`.
- Sources: record 0065; statement C.5, worktrunk row (agentplot/blueprints ccc45c8); clauses 25, 50, 67; rules core.1, core.4, core.16.

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
