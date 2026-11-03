## ADDED Requirements

### Requirement: The host user's scratch directory is made the user's own

sluice SHALL make `/tmp/sluice-<uid>`, at its resolved path, and the
repository's directory under it, readable, writable and searchable by
the user alone, whenever it makes a checkout's scratch under them and
they are absent (35 as amended; record 0063 as amended; rule core.30).

#### Scenario: Made by the first checkout

- **WHEN** neither the user's directory nor the repository's exists and
  a worktree's checkout is made for its first run
- **THEN** both exist afterwards, owned by the user, with no permission
  for anyone else, and the checkout's scratch is under them
  (`the_users_scratch_directory_is_made_its_own` in
  `crates/sluice/tests/profiles/places.rs`)

#### Scenario: Emptied by the system

- **WHEN** between two runs of a worktree its scratch, the repository's
  directory and the user's directory are removed
- **THEN** the second run is given the same scratch path, and the user's
  and the repository's directories are made again the user's own
  (`a_scratch_the_system_emptied_is_made_again` in
  `crates/sluice/tests/profiles/places.rs`)

### Requirement: A scratch directory that is not the user's own is refused

Where `/tmp/sluice-<uid>` exists and is a link, is not a directory,
belongs to another user or may be entered by others, sluice SHALL make
no scratch under it, SHALL say its path and what is wrong, and SHALL
change and remove nothing of it (35 as amended; record 0063 as amended;
rule core.30).

#### Scenario: A directory that is not the user's own

- **WHEN** the user's directory is a link to another directory, or a
  directory others may enter, and a checkout is made
- **THEN** the checkout is not made, the error names the directory and
  says it is a link or that others may enter it, and the link or the
  directory is as it was, its mode included
  (`a_scratch_directory_not_the_users_own_is_refused` in
  `crates/sluice/tests/profiles/places.rs`; unit tests of `own_dir` in
  `crates/sluice/src/profiles/places.rs`, another user's directory
  among them)

### Requirement: A check under the sandbox writes its scratch and binds sockets in it

Under the sandbox, a check SHALL be able to write anywhere under its
scratch and to bind and connect to a Unix socket anywhere under it, at
the path `SLUICE_TMP` names, which SHALL be the path the checkout's
record names and the sandbox's rules name, resolved (35 as amended,
C.4 sandbox row).

#### Scenario: A tool's session socket in a folder under the scratch

- **WHEN** on macOS a worktree at a path longer than 255 bytes, of a
  branch whose policy names `[executor] kind = "sandbox"`, runs
  `sluice exec` with a check that makes `$SLUICE_TMP/x/sessions/work/`
  and binds a Unix socket at `x-client.sock` in it
- **THEN** the check passes, and the socket's path is shorter than 104
  bytes
  (`exec_under_the_sandbox_binds_a_socket_in_a_folder_under_its_scratch`
  in `crates/sluice/tests/exec.rs`)

## MODIFIED Requirements

### Requirement: A checkout's scratch is at a short path whatever its name

A checkout sluice makes or adopts SHALL be given a scratch directory
under a short directory of the host user's own, apart from every
repository: the resolved path of `/tmp`, then `sluice-<uid>`, then 10
hex digits of a digest of the repository's git directory's path, then
10 hex digits of a digest of the checkout's name. Its path SHALL be the
same length for every checkout of every repository on the host, and
SHALL hold nothing of the checkout's name, its worktree's location or
the repository's location (35 as amended at agentplot/blueprints
f15896b; record 0063 as amended; rule core.30).

#### Scenario: A worktree at a long path

- **WHEN** a worktree whose path is longer than 255 bytes runs
  `sluice exec` with a command that prints `$SLUICE_TMP`
- **THEN** the path printed is the resolved path of `/tmp`, then
  `/sluice-` and the user's id, then `/` and 10 hex digits twice, and
  holds no part of the worktree's path or the repository's
  (`exec_runs_from_a_worktree_at_a_long_path` in
  `crates/sluice/tests/exec.rs`)

#### Scenario: Two worktrees, and one worktree twice

- **WHEN** two worktrees of one repository each run `sluice exec` twice
- **THEN** each worktree is given the same scratch path on both runs, and
  the two worktrees' paths differ (`a_worktree_keeps_one_short_scratch`
  in `crates/sluice/tests/profiles/places.rs`)

#### Scenario: Two repositories

- **WHEN** two repositories on the host each make a checkout of the same
  name
- **THEN** the two scratch paths are the same length and differ in the
  repository's 10 hex digits alone
  (`two_repositories_keep_their_scratch_apart` in
  `crates/sluice/tests/profiles/places.rs`)

### Requirement: A checkout from an older sluice is released where it was made

A checkout made by an older sluice SHALL keep its scratch where that
sluice put it until it is released, and its release SHALL remove that
directory: a checkout whose record names no scratch, under the
repository's `sluice/.tmp/` and the checkout's name written as one
directory name; a checkout whose record names a scratch under the
repository's `sluice/.tmp/`, at the path its record names. A worktree's
checkout so released SHALL be given a scratch under the user's
directory at its next run (35 as amended; record 0063 as amended; rule
core.30).

#### Scenario: A worktree's checkout with no scratch on its record

- **WHEN** a worktree's checkout record names no scratch and its scratch
  directory is at the former name, and the worktree runs `sluice exec`
  and then `sluice release`
- **THEN** the run is given the former directory as `SLUICE_TMP`, the
  release removes it, and the worktree's next run is given a scratch
  under the user's directory
  (`a_place_from_before_keeps_its_scratch_until_released` in
  `crates/sluice/tests/profiles/places.rs`)

#### Scenario: A worktree's checkout whose record names a scratch in the repository

- **WHEN** a worktree's checkout record names a scratch under the
  repository's `sluice/.tmp/` and that directory exists, and the
  worktree runs `sluice exec` and then `sluice release`
- **THEN** the run is given that directory as `SLUICE_TMP`, the release
  removes it and makes nothing under the user's directory, and the
  worktree's next run is given a scratch under the user's directory
  (`a_place_in_the_repository_keeps_its_scratch_until_released` in
  `crates/sluice/tests/profiles/places.rs`)

#### Scenario: A candidate's checkout whose record names a scratch in the repository

- **WHEN** a candidate's checkout record names a scratch under the
  repository's `sluice/.tmp/` and the checkout is released
- **THEN** that directory and the record are gone
  (`a_place_in_the_repository_keeps_its_scratch_until_released` in
  `crates/sluice/tests/profiles/places.rs`)

### Requirement: Scratch no record names is removed with the abandoned checkouts

When a round or `sluice exec` releases the checkouts abandoned on its
host, it SHALL remove every directory under the repository's own
directory in the user's scratch directory, and under the repository's
`sluice/.tmp/`, that no checkout's record names. It SHALL leave every
directory a record names, an older record's among them, every directory
a record that exists and cannot be read may name, and every other
repository's scratch, and SHALL never list the user's scratch directory
itself. Where the user's scratch directory is not the user's own, it
SHALL remove nothing under it, say so, and still sweep the repository's
`sluice/.tmp/` (35 as amended, 123; rule core.30).

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

#### Scenario: Another repository's scratch

- **WHEN** two repositories' directories in the user's scratch directory
  each hold a directory no record of the repository names, and the
  first repository's sweep runs
- **THEN** the first repository's stray directory is gone and the second
  repository's directory is as it was
  (`another_repositorys_scratch_is_left` in
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
  words, and the cleanup succeeds (35 as amended at agentplot/blueprints
  9f35001; rule core.30 as amended at sluice 4c29e618;
  `an_unreadable_record_keeps_its_scratch` in
  `crates/sluice/tests/profiles/places.rs`)

#### Scenario: A user's scratch directory that is not the user's own

- **WHEN** the user's scratch directory may be entered by others, holds
  the repository's directory with a directory no record names, and the
  repository's `sluice/.tmp/` holds a directory no record names
- **THEN** the sweep removes the one under `sluice/.tmp/`, removes
  nothing under the user's scratch directory, and warns naming it
  (`a_users_directory_not_its_own_is_not_swept` in
  `crates/sluice/tests/profiles/places.rs`)
