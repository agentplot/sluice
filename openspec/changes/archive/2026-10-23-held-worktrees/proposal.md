# Proposal

## Why

`session-runs` gave an agent's worktree ports and a release, and left
open a worktree whose agent is gone and never released it. Its place
keeps a block of ports and what its runs started, a stack of containers
above all, until something runs `sluice release`; a worktree removed by
a driver that knows nothing of sluice, or by hand, leaves its place
behind for good, and nothing lists the places a repository holds. The
blueprint answers it as clause 97 of
`design/sluice/proposals/session-runs-1.md` in agentplot/blueprints,
section "The abandoned place": a session's place whose working copy no
longer exists is no session's, and the next session run or round on the
host releases it (95); one whose working copy exists is held until its
session or driver releases it, however long since its last run, and
sluice lists every session place it holds, with the time of its last
run.

## What Changes

- The next `sluice exec` on the host, and the next round, release every
  session place whose worktree directory no longer exists, as `sluice
  release` does: the policy's release command, read from the branch's
  current revision, run under the executor with the place's ports in
  its scratch directory, its runs' process groups ended, the scratch
  removed, the block of ports freed and the record removed (95, 97).
  The release is logged as `released` in the branch's log, naming the
  worktree, and reported in the output of the command that ran it.
- A session place whose worktree exists is never released by sluice,
  however old its last run (97, 25).
- A session place's record keeps when its last run started.
- `sluice status` with no argument, and with `--json`, lists every
  session place the repository holds: worktree, branch, ports and last
  run, marking one whose worktree is gone.
- Rule core.16 becomes: the core releases only places it made, and a
  session's only when its working copy is gone. ADR 0020 records why.
- Docs: the commands page (`sluice status`, held worktrees), the state
  page (the record's last run, the `released` event's worktree), the
  worktrunk and walkthrough guides' sentence on removing a worktree.

Out of scope: a lapse after a time without runs, which the proposal
rules out; listing or releasing places held on another host, which
waits on the many-hosts work (`many-hosts-2.md`, C.8 deployments); a
place whose worktree still exists but is no longer a git worktree,
which counts as existing.

## Capabilities

### New Capabilities
- `held-worktrees`: the session places a repository holds, listed for a
  person or a crew, and released by the next run or round once their
  worktree no longer exists. It extends `worktree-release` of the
  change `session-runs`, which is not yet archived into the specs.

### Modified Capabilities
<!-- none: session-runs' capabilities are not in openspec/specs yet -->

## Impact

- The place seam: `Checkout::session_places`, `SessionPlace`, and
  `adopt` taking the run's start; the places profile's record gains
  `last_run`; a release command whose worktree is gone runs in the
  place's scratch directory.
- The core: `Sluice::release_abandoned`, `release_policy` and `round`
  (which `step` wraps); the log's `Released` event gains `worktree`.
- The CLI: `exec`, `step`, `run` and a waiting `submit` report what they
  released; `status` takes no argument.
- The fakes and the conformance suite: a removed worktree, and a shared
  scenario on fakes and real profiles.
- Docs: ADR 0020, `docs/architecture/core.md` and the site's rules page,
  the commands, state, worktrunk and walkthrough pages.
