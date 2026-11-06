## Why

sluice 0.66.0 gave every checkout it makes or adopts a scratch under a
short directory of the host user's own, 45 bytes on macOS, so a tool's
sockets in a folder under `SLUICE_TMP` fit the 104 bytes macOS allows
(35 as amended at agentplot/blueprints f15896b; record 0063). A
worktree's checkout an older sluice had adopted kept its scratch under
the repository's `sluice/.tmp/` until `sluice release`, and a worktree's
checkout lives as long as its worktree. On deneb on 2026-11-05, with
sluice 0.67.0 installed, two of another repository's worktrees were still
given its 73-byte scratch under `sluice exec main`. Ten of that
repository's tests start a terminal multiplexer's named session, whose
`herdr-client.sock` lies in `<home>/herdr/sessions/<name>/` with the home
in `SLUICE_TMP`. They came to 117 to 122 bytes and failed before any code
under test ran. The fail, held under the check's key, then failed a later
landing as flaky. That repository put an exit its policy reads as not run
on its own branch meanwhile.

The user's words, 2026-11-05: "send sluice the short scratch folder as a
fix so the tests run again". Clause 35 on agentplot/blueprints main
allows a place's scratch under no repository. Record 0063 and rule
core.30 are amended with this change's first commit.

## What Changes

- **A worktree's checkout made before takes the short scratch at its next
  run.** At a run, a worktree's checkout whose record names a scratch
  under the repository's `sluice/.tmp/`, or names none, is given a
  scratch under the user's directory once none of its runs is going. Its
  record names the new path first. Then its former scratch is renamed to
  it, so what its checks keep between runs goes with it. Where the system
  cannot rename it, as from one file system to another, the new path is
  made empty and the former removed.
- **A run going keeps the former scratch.** While a run of the worktree
  is going, a run beside it is given the former scratch still, since the
  going run's sandbox writes that path alone. The first run with none
  going moves it.
- **A candidate's or a preview's checkout made before keeps its scratch
  until it is released.** Each is made for one run or one command, and a
  preview's stack may keep what it handed off in its scratch.
- **A landing's check binds a socket under its scratch.** A test of
  sluice's own lands a submission whose check, under the sandbox, binds a
  socket at `$SLUICE_TMP/x/sessions/work/x-client.sock` and passes only
  where its path is under the user's directory and shorter than 104
  bytes. `sluice exec` already has the same test.
- The state reference page says a worktree's checkout made before moves
  its scratch at its next run.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `place-scratch`: a worktree's checkout made before takes the short
  scratch at its next run; a candidate's checkout made before is released
  where its record names; a landing's check binds a socket in a folder
  under its scratch.

## Out of scope

- **A preview made before** keeps its scratch until it is released, as
  now. Its command runs once, and a stack it handed off may keep its
  state there.
- **A copy where the system cannot rename.** The former scratch is
  removed and the new one made empty. A check keeps nothing in its
  scratch it cannot make again (record 0063), as a system that empties
  `/tmp` already asks of it.
- **The other repository's exit for a test with no room for its socket.**
  It is that repository's to drop once its hosts carry this change. Its
  orchestrator is told.

## Impact

- `crates/sluice/src/profiles/places.rs`: `adopt` gives a worktree's
  checkout made before the short scratch and moves its former one; the
  making of the directories above a scratch is split from the making of
  the scratch, so a move makes them first; the doc comments that say a
  checkout made before keeps its scratch.
- `crates/sluice/src/runs.rs`: `Runs::any_going`, whether a supervisor
  holds any run of a worktree's checkout.
- Tests: `crates/sluice/tests/profiles/places.rs`,
  `crates/sluice/tests/exec.rs`.
- Docs: `site/src/content/docs/reference/state.md`.
- Records and rules: record 0063 and rule core.30 as amended 2026-11-05.
- Sources: clauses 34, 35 (as amended at agentplot/blueprints f15896b,
  on main at ebfbc0d), 123, C.4 sandbox row; record 0063 as amended;
  rules core.10, core.16, core.30.

## Touches

```
crates/sluice/src/profiles/places.rs
crates/sluice/src/runs.rs
crates/sluice/tests/profiles/places.rs
crates/sluice/tests/exec.rs
site/src/content/docs/reference/state.md
docs/adr/0063-a-places-scratch-is-at-a-short-path-named-by-its-record.md
docs/architecture/core.md
openspec/changes/a-places-scratch-fits-a-socket/**
```
