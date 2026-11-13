## ADDED Requirements

### Requirement: A worktree sluice holds nothing for is released by the policy's release command

`sluice release` in a worktree of the repository that sluice holds no
ports, runs or record for SHALL run the release command named by the
policy of the repository's default branch, read at its current
revision. It SHALL run it at the worktree's top as a held worktree's
release runs it: under the executor the policy's limits choose,
entering the toolchain alone, with a scratch its own record names and no
ports (95, 34, 56; rules core.10, core.30, core.46).

#### Scenario: A release that leaves a mark

- **WHEN** a worktree on `unit/a`, in which `sluice exec` never ran, is
  released with `sluice release`, and `main`'s policy names a release
  command that writes the folder it runs in to a mark file
- **THEN** the mark names the worktree, the command says the worktree
  was released and exits 0, and `sluice status` lists no worktree held
  (`an_unheld_worktrees_release_runs_its_release_command` in
  `crates/sluice/tests/exec.rs`)

### Requirement: An unheld worktree's release answers as a held one's and keeps nothing

The release of a worktree sluice holds nothing for SHALL answer as a
held worktree's release does: `released` and exit 0 when the command
passed, and what the command printed and exit 1 when it failed. Either
way sluice SHALL keep nothing for the worktree once it answers, and a
release asked again SHALL run the command again (95, 123).

#### Scenario: A failing release

- **WHEN** `main`'s policy names a release command that prints
  `emulator still answers` and exits 3, and `sluice release` runs in a
  worktree sluice holds nothing for
- **THEN** the command exits 1 with `emulator still answers` in what it
  says, and `sluice status` lists no worktree held
  (`an_unheld_worktrees_release_runs_its_release_command`)

#### Scenario: Released again

- **WHEN** `sluice release` runs in a worktree whose place it has just
  released, and the policy still names a release command
- **THEN** the release command runs again, and the command says the
  worktree was released and exits 0
  (`release_runs_the_policys_release_command_once` in
  `crates/sluice/tests/cli.rs`)

### Requirement: Nothing to release only where no release command is named

In a worktree sluice holds nothing for, `sluice release` SHALL say
`nothing to release` and exit 0 only where the default branch's policy
names no release command, or where the folder is in no worktree of the
repository. Where the default branch or its policy cannot be read, the
release SHALL fail, exit 1, naming the branch and the words it was
refused with, and run nothing (rule core.10).

#### Scenario: A policy that cannot be read

- **WHEN** `main`'s policy names a key sluice does not know, and
  `sluice release` runs in a worktree sluice holds nothing for
- **THEN** the command exits 1 naming `main`, and nothing is run
  (`an_unheld_worktrees_release_runs_its_release_command`)

#### Scenario: A policy that names no release command

- **WHEN** `sluice release` runs in a worktree sluice holds nothing
  for, under a policy that names no release command
- **THEN** it says `nothing to release` and exits 0
  (`equip_adopts_nothing` in `crates/sluice/tests/equip_tools.rs`)
