## ADDED Requirements

### Requirement: A checkout or a preview git no longer lists is gone

A combination's checkout or a preview, each made by sluice, SHALL be
read as gone when its directory is gone, and also when git no longer
lists its directory as a worktree of the repository, whether or not the
directory is still there. sluice SHALL match the directory by the
directory itself, never by how its path is spelled, and SHALL start no
process to read it. A reading that fails SHALL never be read as gone
(123; rule core.31).

#### Scenario: A checkout git stopped removing part way

- **WHEN** git's removal of `sluice/try/main/63` stopped part way, so
  its directory still holds the checkout's files while git no longer
  lists it as a worktree
- **THEN** the checkout is read as gone (`a_checkout_git_no_longer_lists_is_gone`
  in `crates/sluice/tests/profiles/places.rs`)

#### Scenario: A checkout git lists under another spelling of its path

- **WHEN** a checkout was made through a path that reaches the
  repository by a symbolic link, and git lists its directory by another
  spelling
- **THEN** the checkout is not read as gone
  (`a_checkout_git_no_longer_lists_is_gone`)

#### Scenario: A preview git no longer lists

- **WHEN** git no longer lists a preview's checkout as a worktree, and
  its directory is still there
- **THEN** `sluice previews` reads it as gone
  (`a_checkout_git_no_longer_lists_is_gone`), and the next pass frees it
  as any gone preview (`a_preview_whose_checkout_is_gone_is_freed` in
  `crates/sluice-conformance/tests/previews.rs`)

### Requirement: A worktree's place is gone only once its directory is

A worktree's place SHALL be read as gone only once the worktree's
directory is gone, whether or not git lists the worktree, so no round or
run releases the place of a worktree whose directory is still there
(123; rule core.16).

#### Scenario: A worktree git no longer lists

- **WHEN** git no longer lists an agent's worktree, which sluice holds a
  place for, and its directory is still there
- **THEN** its place is not read as gone, and no round or run releases
  it (`a_checkout_git_no_longer_lists_is_gone` in
  `crates/sluice/tests/profiles/places.rs`)

### Requirement: sluice removes what is left of a checkout git no longer lists

When sluice frees a checkout or a preview it made, by a round or by
`--accept-failure`, it SHALL remove the directory through git where git
lists it as a worktree, and otherwise SHALL remove what is left of the
directory itself, first making writable every directory in it that its
owner cannot write. A directory it still cannot remove SHALL be a
failure to free, named with the directory and what refused it, and the
checkout SHALL stay held with it (123).

#### Scenario: What git left of a checkout

- **WHEN** git no longer lists `sluice/try/main/63`, its directory still
  holds the checkout's files and a directory a check left read-only, and
  its run stored a release command
- **THEN** the release command runs once from its scratch directory, the
  directory is removed, git lists nothing there, its scratch directory
  and record are gone, and its ports go to the next checkout
  (`a_checkout_git_no_longer_lists_is_freed` in
  `crates/sluice/tests/profiles/places.rs`)

## MODIFIED Requirements

### Requirement: The operator may accept a release's failure

`sluice release <name> --accept-failure` SHALL free what sluice itself
holds for the checkout without running its release command: end its
runs, remove the checkout sluice made, its scratch directory and record,
and let its ports go. The log SHALL hold `released` naming it and the
failure accepted. In a worktree, `sluice release --accept-failure` SHALL
do the same for the worktree's place and leave the worktree (123).

#### Scenario: Accepted

- **WHEN** a checkout is held because its release command fails, and the
  operator runs `sluice release sluice/try/main/48 --accept-failure`
- **THEN** the release command is not run, the checkout is removed, its
  ports are free for the next checkout, and `sluice status main` no
  longer lists it

#### Scenario: Accepted where git no longer lists the checkout

- **WHEN** `sluice/try/main/63` is held because git no longer lists its
  directory as a worktree, the directory still holds the checkout's
  files and a directory a check left read-only, and the operator runs
  `sluice release sluice/try/main/63 --accept-failure`
- **THEN** nothing is run, the directory is removed, its ports are free,
  and `sluice status main` no longer lists it
  (`a_checkout_git_no_longer_lists_is_freed` in
  `crates/sluice/tests/profiles/places.rs` for the checkout, and
  `release_accepting_the_failure_frees_a_held_checkout` in
  `crates/sluice/tests/cli.rs` for the command)

### Requirement: A checkout whose working copy is gone is released once and freed

When the working copy of a checkout sluice holds is gone, whether a
worktree's, a combination's or a preview's, the next run or round on the
host SHALL run its release once, from what sluice itself holds for it:
the release command its last run stored, with its stored environment
and ports, in the checkout's scratch directory, entering no toolchain
and reading no policy. A checkout whose runs stored no release command,
or whose stored release has run already, SHALL have nothing run (123).

#### Scenario: A worktree removed by hand

- **WHEN** an agent's worktree that ran `sluice exec` under a policy
  with a `[tests] toolchain` and a release command is removed with
  `rm -rf`, and another agent runs `sluice submit` and waits
- **THEN** the first round runs the stored release command once, from
  the worktree's scratch directory with the worktree's port in its
  environment, the toolchain is not run, the round says once that the
  worktree is gone and its ports are free, the log holds `gone` naming
  it, and no later round runs its release or mentions it

### Requirement: A gone checkout is freed whatever its release did

Once a gone checkout's release has run, or found nothing to run, sluice
SHALL end its runs and let go its scratch directory, ports and record,
whatever the release did. Its branch's log SHALL hold `gone` naming it
and saying how the release went, and sluice SHALL never run its release
again (123).

#### Scenario: A gone checkout's release fails

- **WHEN** a removed worktree's stored release command exits 2
- **THEN** its ports and record are freed all the same, the log holds
  `gone` with what the command printed, and the release is never run
  again

#### Scenario: A combination's checkout deleted

- **WHEN** a combination's checkout directory is deleted before its
  result is read
- **THEN** on release its stored release command runs once in its
  scratch directory, git no longer lists it as a worktree, its ports are
  free, and the log holds `gone` naming it

#### Scenario: Nothing stored to run

- **WHEN** a removed worktree's runs stored no release command
- **THEN** nothing is run, its ports and record are freed, and the log
  holds `gone` saying no release ran

#### Scenario: A checkout held because git refused it

- **WHEN** `sluice/try/main/63`'s first release ran its release command
  and then failed in git, `Directory not empty`, and its later tries
  were refused with `is not a working tree` until it was held
- **THEN** the next round reads it as gone, runs nothing, since its
  release command ran already, removes what is left of its directory,
  logs `gone` naming it, and `sluice status main` no longer lists it
  (`a_checkout_git_no_longer_lists_is_freed` in
  `crates/sluice/tests/profiles/places.rs` for the checkout, and
  `a_held_checkout_deleted_since_is_released_once_and_freed` in
  `crates/sluice-conformance/tests/places.rs` for the round)
