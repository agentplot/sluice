# Proposal

## Why

The herdr plugin is to show `checked 4m · ready` on an agent's pane when
the last `sluice exec` in that worktree passed on the worktree's head,
and the Claude Code board wants the same. Both read `sluice overview
--json`, whose `sessions[]` gives a worktree's branch, ports and the time
its last run started, but not the head and outcome session-runs-1
clause 97 lists, nor whether its results were recorded under their keys
(92). Nothing durable holds that today: the results log keeps
only clean runs of the checks and skips a line equal to one it has, so a
fail after a pass on the same key leaves the pass as the newest line; a
command's run and a run in a worktree holding more than its head leave
nothing at all.

## What Changes

- A session run that ends, of the checks or of a command, clean or not,
  leaves its result on its worktree's held state: when it ended, the
  head it ran on, whether it ran the checks or a command, how it ended
  (`pass`, `fail`, `timed_out`, `lost`, `stopped`) and whether its
  results were recorded under their keys (97). The next run replaces it;
  the worktree's release removes it (95). A command's run and an unclean
  run are listed with their outcome and never recorded under keys (92).
- `sluice status --json`'s `worktrees[]` and `sluice overview --json`'s
  `sessions[]` gain `last_result`, that result, on every worktree that
  has one: `{at, head, kind, outcome, recorded}`. Fields are only
  added; `last_run` stays as it is.
- `recorded` is true only for a run of the checks in a worktree that
  held exactly its head and did not hit its time limit (92; rule
  core.18). A command's run and a run in a worktree holding more always
  say `recorded: false`, whatever their outcome.
- `sluice status`'s text lists the result beside the last run.
- No round, gate, submission or replay reads it: it decides nothing,
  as a preview's state decides nothing (rule core.19's pattern).
- Docs: the commands page (the worktrees list and `sessions[]`), the
  state page (the place's record).

Out of scope: whether the worktree's head or files have moved since the
run, which the consumer reads from the worktree itself (see design);
`sluice/home@1`'s `place`, which waits on the CLI review change that
owns the home view; a run under way, which waits on the herdr plugin's
own design for a running token; remote session runs (96), which wait on
that clause's change.

## Capabilities

### New Capabilities
- `worktree-last-result`: how the last session run in a held worktree
  ended, kept until the worktree's release and listed by `sluice status`
  and `sluice overview`.

### Modified Capabilities
<!-- none -->

## Impact

- The seams: `Checkout` gains `ran`, keeping a session place's last
  result; `SessionPlace` gains `last_result`. `LastResult`, `RunKind` and
  `RunOutcome` are new seam types.
- The core: `Sluice::finish_session` writes the result after any
  results are recorded; `Sluice::session_ended` writes it for a run that
  was lost or stopped.
- The place profile keeps it on the session place's record
  (`<common>/sluice/.places/<flat>.json`); the fakes keep it in memory.
- The CLI: `exec` calls the core on every way a run ends;
  `worktree_json` and the worktrees text add it.
- The conformance suite gains the place seam's scenario.
- Docs: the commands and state reference pages; an ADR for keeping it on
  the place rather than deriving it from the results log.
