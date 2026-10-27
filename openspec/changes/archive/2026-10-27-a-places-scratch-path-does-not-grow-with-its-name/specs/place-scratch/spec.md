# Spec Delta

## Purpose

Where a checkout's scratch directory, its `SLUICE_TMP`, is kept, how the
checkout's record names it, and how it is removed, so a check's path
under it fits the tightest limit a check meets whatever the checkout is
called or where its worktree is (35, 34, 123; record 0063; rule core.30).

## ADDED Requirements

### Requirement: A checkout's scratch is at a short path whatever its name

A checkout sluice makes or adopts SHALL be given a scratch directory under
the repository's own `sluice/.tmp/`, named by 10 hex digits of a digest of
the checkout's name, so its path is the same length for every checkout of
the repository and holds nothing of the checkout's name or its worktree's
location.

#### Scenario: A worktree at a long path

- **WHEN** a worktree whose path is longer than 255 bytes runs
  `sluice exec` with a command that prints `$SLUICE_TMP`
- **THEN** the path printed is the repository's git directory followed by
  `/sluice/.tmp/` and 10 hex digits, and holds no part of the worktree's
  path (`exec_runs_from_a_worktree_at_a_long_path` in
  `crates/sluice/tests/exec.rs`)

#### Scenario: Two worktrees, and one worktree twice

- **WHEN** two worktrees of one repository each run `sluice exec` twice
- **THEN** each worktree is given the same scratch path on both runs, and
  the two worktrees' paths differ (`a_worktree_keeps_one_short_scratch`
  in `crates/sluice/tests/profiles/places.rs`)

### Requirement: The checkout's record names its scratch

A checkout's record SHALL name its scratch path, written before the
directory is made, and a run in the checkout, its release, and a release
run for a checkout whose working copy is gone SHALL use the path the
record names.

#### Scenario: A candidate's checkout

- **WHEN** a candidate's checkout is made and then released
- **THEN** its record named its scratch path while it was held, its
  checks were given that path as `SLUICE_TMP`, and the release removed
  that directory and the record (`a_place_names_its_scratch_on_its_record`
  in `crates/sluice/tests/profiles/places.rs`)

#### Scenario: A worktree whose working copy is gone

- **WHEN** a worktree with a stored release command is removed and the
  next `sluice exec` in another worktree releases its checkout
- **THEN** the stored release ran in the scratch directory the checkout's
  record named, and that directory is gone afterwards
  (`crates/sluice/tests/exec.rs`, the gone worktree's scenario)

### Requirement: A checkout from an older sluice is released where it was made

A checkout whose record names no scratch, made by a sluice from before
this change, SHALL keep its scratch where that sluice put it, under the
checkout's name written as one directory name, until it is released, and
its release SHALL remove that directory.

#### Scenario: A worktree's checkout with no scratch on its record

- **WHEN** a worktree's checkout record names no scratch and its scratch
  directory is at the former name, and the worktree runs `sluice exec`
  and then `sluice release`
- **THEN** the run is given the former directory as `SLUICE_TMP`, the
  release removes it, and the worktree's next run is given a short
  scratch path (`a_place_from_before_keeps_its_scratch_until_released` in
  `crates/sluice/tests/profiles/places.rs`)

### Requirement: Scratch no record names is removed with the abandoned checkouts

When a round or `sluice exec` releases the checkouts abandoned on its
host, it SHALL remove every directory under `sluice/.tmp/` that no
checkout's record names, and SHALL leave every directory a record names,
an older record's former path among them, and every directory a record
that exists and cannot be read may name (35 as amended).

#### Scenario: A stray directory and a held checkout's

- **WHEN** `sluice/.tmp/` holds a directory no record names, the scratch
  of a worktree's checkout that is held, and the former-name scratch of a
  checkout whose record names none, and `sluice exec` runs in another
  worktree
- **THEN** the stray directory is gone and the other two are as they were
  (`scratch_no_record_names_is_removed` in
  `crates/sluice/tests/profiles/places.rs`)

#### Scenario: A checkout being made as the sweep runs

- **WHEN** a checkout's record is written and its scratch made after the
  removal has begun
- **THEN** the checkout's scratch is not removed
  (`a_scratch_made_after_the_listing_is_kept` in
  `crates/sluice/src/profiles/places.rs`)

#### Scenario: A checkout's record that cannot be read

- **WHEN** a checkout's record exists and cannot be read, and
  `sluice/.tmp/` holds that checkout's scratch and a directory no record
  names
- **THEN** the stray directory is removed, the unreadable checkout's
  scratch is kept, the record is reported by its path with the reader's
  words, and the cleanup succeeds (35 as amended at agentplot/blueprints
  9f35001; rule core.30 as amended at sluice 4c29e618;
  `an_unreadable_record_keeps_its_scratch` in
  `crates/sluice/tests/profiles/places.rs`)
