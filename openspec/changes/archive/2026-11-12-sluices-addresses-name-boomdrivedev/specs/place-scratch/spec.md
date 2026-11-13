## MODIFIED Requirements

### Requirement: A checkout's scratch is at a short path whatever its name

A checkout sluice makes or adopts SHALL be given a scratch directory
under a short directory of the host user's own, apart from every
repository. Its path SHALL be the same length for every checkout of
every repository on the host, and SHALL hold nothing of the checkout's
name, its worktree's location or the repository's location (35 as
amended at boomdrivedev/blueprints f15896b; record 0063 as amended; rule
core.30).

#### Scenario: A worktree at a long path

- **WHEN** a worktree whose path is longer than 255 bytes runs
  `sluice exec` with a command that prints `$SLUICE_TMP`
- **THEN** the path printed is the user's directory, then `/` and 10
  hex digits twice, and holds no part of the worktree's path or the
  repository's (`exec_runs_from_a_worktree_at_a_long_path` in
  `crates/sluice/tests/exec.rs`, under the directory
  `SLUICE_USER_SCRATCH` names for the tests)

#### Scenario: Two worktrees, and one worktree twice

- **WHEN** two worktrees of one repository each run `sluice exec` twice
- **THEN** each worktree is given the same scratch path on both runs, and
  the two worktrees' paths differ (`a_worktree_keeps_one_short_scratch`
  in `crates/sluice/tests/profiles/places.rs`)

### Requirement: A checkout's scratch is named by its repository and its own name

A checkout's scratch directory SHALL be the resolved path of `/tmp`,
then `sluice-<uid>`, then 10 hex digits of a digest of the repository's
git directory's path, then 10 hex digits of a digest of the checkout's
name (35 as amended at boomdrivedev/blueprints f15896b; record 0063 as
amended; rule core.30).

#### Scenario: Two repositories

- **WHEN** two repositories on the host each make a checkout of the same
  name
- **THEN** the two scratch paths are the same length and differ in the
  repository's 10 hex digits alone
  (`two_repositories_keep_their_scratch_apart` in
  `crates/sluice/tests/profiles/places.rs`)

### Requirement: Scratch no record names is removed with the abandoned checkouts

When a round or `sluice exec` releases the checkouts abandoned on its
host, it SHALL remove every directory under the repository's own
directory in the user's scratch directory, and under the repository's
`sluice/.tmp/`, that no checkout's record names. It SHALL leave every
directory a record names, an older record's among them, and every
directory a record that exists and cannot be read may name (35 as
amended, 123; rule core.30).

#### Scenario: A stray directory and a held checkout's

- **WHEN** the repository's directory in the user's scratch directory
  holds a directory no record names and the scratch of a worktree's
  checkout that is held, and the repository's `sluice/.tmp/` holds a
  directory no record names, a scratch an older record names, and the
  former-name scratch of a checkout whose record names none, and
  `sluice exec` runs in another worktree
- **THEN** the two stray directories are gone and the other three are as
  they were (`scratch_no_record_names_is_removed` in
  `crates/sluice/tests/profiles/places.rs`)

#### Scenario: A checkout being made as the sweep runs

- **WHEN** a checkout's record is written and its scratch made after the
  removal has begun
- **THEN** the checkout's scratch is not removed
  (`a_scratch_made_after_the_listing_is_kept` in
  `crates/sluice/src/profiles/places.rs`)

#### Scenario: A checkout's record that cannot be read

- **WHEN** a checkout's record exists and cannot be read, and the
  repository's directory in the user's scratch directory holds that
  checkout's scratch and a directory no record names
- **THEN** the stray directory is removed, the unreadable checkout's
  scratch is kept, the record is reported by its path with the reader's
  words, and the cleanup succeeds (35 as amended at boomdrivedev/blueprints
  9f35001; rule core.30 as amended at sluice 4c29e618;
  `an_unreadable_record_keeps_its_scratch` in
  `crates/sluice/tests/profiles/places.rs`)

