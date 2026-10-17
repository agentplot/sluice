# Proposal

## Why

sluice keeps a checkout's record, its directory, its scratch, its runs
and a worktree's entry in `sessions/` under one file name each, made
from the checkout's whole name, a worktree's path included, with every
`/` written as three characters. macOS and Linux allow a file name of
at most 255 bytes, so `sluice exec main` from a worktree at a
247-character path exits 3 with `File name too long (os error 63)`,
while 207 characters works. It bites sluice's own tests run in a
worktree's run, whose `TMPDIR` is already such a name and is written
again inside it. A worktree that cannot run cannot be held to its
branch's policy (session-runs-1 clause 89), and its state cannot be
written at all, let alone read (26).

## What Changes

- A checkout's name that, written as a file name, would be longer than
  246 bytes is written instead as the first part of that file name, a
  mark no shorter name can hold, and 16 hex digits of the SHA-1 of the
  whole name. Every file name sluice makes from a checkout's name, a
  worktree's path or a task's name goes through the same rule: the
  records and checkouts in `.places/` and `.previews/`, `.tmp/`,
  `.runs/`, `.logs/`, and `sessions/` in the host's state.
- A name that fits is written exactly as today, so every record an
  older sluice wrote, its ports and its runs are found where they are,
  with nothing moved.
- Nothing reads a checkout's name back out of a file name: each record
  already carries the worktree's whole path, or the preview's revision,
  and that stays so.
- Docs: the state page says how a long name is shortened.

Out of scope: a whole path longer than the system's limit on paths
(1024 bytes on macOS, 4096 on Linux), which no file name rule can fix
and which waits on a change that moves state out from under the
worktree's own path; filesystems with a lower limit on names than 255
bytes, such as eCryptfs, which wait on a report from one.

## Capabilities

### New Capabilities
- `state-file-names`: how a checkout's name, a worktree's path above
  all, becomes the one file or directory name sluice keeps its state
  under, short enough for macOS and Linux, distinct for every name, and
  unchanged for every name that already fit.

### Modified Capabilities
<!-- none -->

## Impact

- `crates/sluice/src/runs.rs`: `flat_name` and `dir_name` shorten a
  name that would not fit; every caller in `runs.rs`, `sessions.rs`,
  `profiles/places.rs` and `profiles/local_files.rs` gets the rule with
  no change of its own.
- SHA-1 from the `gix` dependency sluice already has; no new crate.
- Tests: unit tests on both functions, a CLI test of `sluice exec` from
  a worktree whose path is longer than 255 bytes.
- Docs: `site/src/content/docs/reference/state.md`.
- Rules kept: core.11 (every record still readable with plain tools,
  each carrying the whole path it is for), core.16 and session-runs-1
  clause 97 (a worktree's place is still found by its path, so it is
  released only when that path is gone), core.18 (a worktree's run is
  still counted from `sessions/`).
