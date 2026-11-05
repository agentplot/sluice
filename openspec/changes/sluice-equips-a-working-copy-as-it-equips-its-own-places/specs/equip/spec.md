## Purpose

How sluice makes a working copy ready for its branch's runner before
anything runs in it: one sequence of steps for every checkout, worktree
and preview it runs a check or a command in, and `sluice equip` for a
working copy sluice did not make.

## ADDED Requirements

### Requirement: A working copy is equipped by one sequence

sluice SHALL equip a working copy by these steps, in order: the
toolchain the branch's policy names, its output kept as the environment
of the steps after it; then, under the moon runner, `proto install
--config-mode upwards` where the working copy holds a `.prototools`,
`moon --version`, and `moon setup`. Under the command runner the
toolchain SHALL be the only step (147; record 0081; rule core.46).

#### Scenario: A moon repository whose pins nothing has installed

- **WHEN** a worktree holds a `.prototools`, its policy names a toolchain
  and the moon runner, and no moon is installed until `proto install`
  installs it
- **THEN** `sluice equip main` runs the toolchain, `proto install
  --config-mode upwards`, `moon --version` and `moon setup` in that
  order, each passes, and the command exits 0
  (`equip_runs_the_sequence_in_order` in `crates/sluice/tests/checks.rs`)

#### Scenario: Nothing pinned

- **WHEN** the worktree holds no `.prototools`
- **THEN** no `proto install` step is run or named, and `moon --version`
  follows the toolchain (`equip_installs_only_what_is_pinned`)

#### Scenario: The command runner

- **WHEN** the branch's policy names `runner = "command"` and a toolchain
- **THEN** the toolchain is the one step; with no toolchain there is no
  step, `sluice equip` says there is nothing to equip, and exits 0
  (`equip_under_the_command_runner_runs_the_toolchain_alone`)

### Requirement: The sequence stops at the first step that fails

Each step SHALL give its outcome and its words, and no step SHALL run
after one that failed. A moon below 2.6 SHALL fail `moon --version`.
A step's words SHALL be what it printed, its output then its error
stream, the toolchain's error stream alone; a step that failed SHALL be
given sluice's reason before them (147; record 0081).

#### Scenario: The setup fails

- **WHEN** `moon setup` exits 1 printing `error: no such toolchain`
- **THEN** `sluice equip main --json` prints the steps `toolchain`,
  `proto install` and `moon --version` passed, then `moon setup` failed
  with words `exited 1: error: no such toolchain`, and exits 1
  (`equip_stops_at_the_step_that_failed` in
  `crates/sluice/tests/checks.rs`; `a_failed_step_exits_1` in
  `crates/sluice/tests/equip.rs`)

#### Scenario: An old moon

- **WHEN** the moon the toolchain puts first answers `moon 2.5.3`
- **THEN** `moon --version` fails with words naming `moon 2.5.3` and
  moon 2.6, and `moon setup` is not run (`an_old_moon_is_refused` in
  `crates/sluice/tests/profiles/moon.rs`)

#### Scenario: The toolchain fails

- **WHEN** the toolchain exits 1 printing `error: flake.lock is not up to
  date` on its error stream
- **THEN** the one step is `toolchain`, failed, with words `exited 1:
  error: flake.lock is not up to date`, and nothing of the runner is run
  (`a_toolchain_that_fails_runs_nothing_after_it`)

### Requirement: sluice equip equips the working copy it runs in

`sluice equip <branch>` SHALL equip, through the setup, the top of the
git worktree holding the folder it runs in, by the policy at
`--at <revision>` where one is named and at the branch's current
revision otherwise. With `--at` it SHALL read nothing of the branch,
and the revision need not be one the branch has landed (147, 56; record
0081).

#### Scenario: At a revision named

- **WHEN** `main`'s current revision names one toolchain and an earlier
  revision another, and `sluice equip main --at <earlier>` runs
- **THEN** the earlier revision's toolchain is the one asked for, and
  the document's `revision` is the earlier revision in full
  (`equip_reads_the_policy_at_the_revision_named` in
  `crates/sluice/tests/equip.rs`)

#### Scenario: At the branch's revision

- **WHEN** `sluice equip main` runs with no `--at`
- **THEN** the policy at `main`'s current revision is the one asked for
  (`equip_reads_the_branchs_policy`)

#### Scenario: From a folder inside the worktree

- **WHEN** `sluice equip main` runs in a folder below the worktree's top
- **THEN** the steps run at the worktree's top, where its `.prototools`
  is (`equip_runs_at_the_top_of_the_worktree` in
  `crates/sluice/tests/equip_tools.rs`)

### Requirement: sluice equip answers its steps in one document

With `--json`, `sluice equip` SHALL print on stdout one document,
`{"branch", "revision", "steps": [{"step", "passed", "words"}]}`, each
step named as sluice runs it. It SHALL exit 0 when every step passed,
and when there was none, and 1 when one failed. Without `--json` it
SHALL print each step and its outcome on a line of its own, and a
failed step's words after it (147, the command).

#### Scenario: Every step passed

- **WHEN** every step passes under `--json`
- **THEN** stdout is one document whose `steps` name each step with
  `passed: true`, stderr is empty, and the command exits 0
  (`every_step_passed_exits_0` in `crates/sluice/tests/equip.rs`)

#### Scenario: Printed for a person

- **WHEN** `sluice equip main` runs without `--json` and `moon setup`
  fails
- **THEN** it prints `equipping <worktree> for main at <revision>`, a
  `<step>: passed` line for each step that passed, `moon setup: failed`,
  and the step's words indented beneath it
  (`a_failed_step_is_printed_with_its_words`)

### Requirement: A refusal gives no steps

`sluice equip` SHALL refuse, running nothing, a branch it cannot read, a
revision that is no commit in the repository, a revision whose policy
names no `[branches]` table for the branch, and a folder in no worktree.
With `--json` a refusal SHALL be `{"error": {"kind", "message",
"next"}}` with no `steps`, and every refusal SHALL exit 2 (147; record
0081).

#### Scenario: No such branch

- **WHEN** `sluice equip nosuch --json` runs
- **THEN** stdout is one error document whose `kind` is `refused`, whose
  `message` names `nosuch`, and which holds no `steps`; the fake check
  seam was asked nothing; and the command exits 2
  (`equip_refuses_a_branch_it_cannot_read` in
  `crates/sluice/tests/equip.rs`)

#### Scenario: A branch the policy does not manage

- **WHEN** the revision's policy has no `[branches]` table naming the
  branch
- **THEN** the refusal names `.config/sluice.toml`, the revision and the
  branch, and the command exits 2
  (`equip_refuses_a_branch_its_policy_does_not_manage`)

#### Scenario: A revision that is no commit

- **WHEN** `sluice equip main --at nosuch` runs
- **THEN** the refusal says `nosuch` names no revision here, and the
  command exits 2 (`equip_refuses_a_revision_it_cannot_read`)

#### Scenario: A folder in no worktree

- **WHEN** `sluice equip main` runs in a repository's bare git directory
- **THEN** the refusal says the folder is in no worktree, nothing is run,
  and the command exits 2 (`equip_refuses_a_folder_in_no_worktree` in
  `crates/sluice/tests/equip_tools.rs`)

### Requirement: sluice equip adopts and keeps nothing

`sluice equip` SHALL hold no ports, write no record of the working copy,
take no part of the host's bound and wait for no room in it, and SHALL
leave nothing of its own once it answers: the toolchain's script SHALL
be gone (147, 71; rules core.16, core.18, core.46).

#### Scenario: Nothing to release after it

- **WHEN** `sluice equip main` has run in a worktree sluice never ran in
- **THEN** the host's state names no record and no ports for the
  worktree, and `sluice release` there says `nothing to release` and
  exits 0 (`equip_adopts_nothing` in
  `crates/sluice/tests/equip_tools.rs`)

#### Scenario: At the host's bound

- **WHEN** the host's bound is full of running checks
- **THEN** `sluice equip main` runs its steps at once, and the count of
  running checks is unchanged by it
  (`equip_takes_no_part_of_the_bound` in `crates/sluice/tests/equip.rs`)

#### Scenario: Its script is gone

- **WHEN** `sluice equip main` has answered
- **THEN** no directory of its own is left under the repository's
  `sluice/.runs/` (`equip_keeps_nothing` in
  `crates/sluice/tests/checks.rs`)

### Requirement: Each step runs outside the executor with a run's environment

Each step SHALL run outside the executor, with the environment a run of
the branch's checks starts from: what sluice keeps of its own, the
working copy's paths and the host's caches' variables, and nothing else
of the caller's (147, 35; rule core.46).

#### Scenario: The caller's variables do not reach the steps

- **WHEN** `sluice equip main` runs with `FROM_THE_CALLER=1` in its
  environment, under a host whose cache declares `CACHE_DIR`, and the
  toolchain prints its environment on its error stream
- **THEN** the toolchain's words hold `CACHE_DIR` and `SLUICE_CHECKOUT`,
  and not `FROM_THE_CALLER`
  (`equip_steps_get_a_runs_environment` in
  `crates/sluice/tests/checks.rs`)

### Requirement: Every working copy a check or a command runs in is equipped through the setup

Before sluice runs a check or a command in a working copy, the working
copy SHALL be equipped through the setup by the runner the branch's
policy names: a combination's checkout and a session's worktree before
their checks run, and a preview's checkout and a session's worktree
before the command, though the command runs in place of the runner's
checks (147, 33, 89, 98, S72; rule core.46).

#### Scenario: A session's command on a fresh clone

- **WHEN** a repository is cloned with a `.prototools` pinning a moon
  nothing has installed, and `sluice exec main -- moon run app:test`
  runs in a worktree of it
- **THEN** the pins are installed, and `moon --version` and `moon setup`
  run, before `moon run app:test` runs and passes
  (`exec_equips_before_its_command` in
  `crates/sluice/tests/equip_tools.rs`)

#### Scenario: A preview on a fresh clone

- **WHEN** the same repository's policy names a `[preview]` command
  `moon run app:preview`, and `sluice preview main <revision> --wait`
  runs
- **THEN** the preview's checkout is equipped through `moon setup`
  before its command, and the preview is up
  (`a_preview_is_equipped_before_its_command`)

#### Scenario: A worktree sluice did not make

- **WHEN** a caller runs `sluice equip main` in a worktree of the same
  repository that sluice never ran in
- **THEN** the same four steps run in the same order, and the worktree
  is adopted no further (`equip_installs_the_pins_before_moon_runs`)

#### Scenario: A combination that runs a check

- **WHEN** a combination affects a required check
- **THEN** its checkout is set up after moon's queries and before the
  check runs (`a_run_that_runs_a_check_is_set_up` in
  `crates/sluice/tests/checks.rs`)

### Requirement: A working copy sluice only reads in is equipped up to the version check

sluice SHALL equip a working copy it only asks the runner a question in,
through `moon --version` and never through `moon setup`: a submitter's
worktree at `sluice submit` (record 0050), the affected reading's tree
(140), and a combination that affects no required check, which runs
nothing (147; rule core.46).

#### Scenario: A combination that affects nothing

- **WHEN** a combination affects no required check
- **THEN** its run passes with nothing run, and `moon setup` was not run
  in its checkout (`a_run_that_runs_nothing_is_never_set_up` in
  `crates/sluice/tests/checks.rs`)

#### Scenario: A submitter's worktree

- **WHEN** `sluice submit` reads which required checks moon keeps out of
  CI in the submitting worktree
- **THEN** the toolchain, `proto install` and `moon --version` ran
  there, and `moon setup` did not
  (`the_submitters_read_is_never_set_up`)

### Requirement: A run whose working copy could not be equipped could not start

When a step fails before a run, sluice SHALL start nothing of the run
and SHALL say why as the step's name and its words. A combination's
run SHALL then be recorded as one whose tests could not be started,
every test not run with those words, nobody blamed and nothing recorded
under any test's key (114, 147).

#### Scenario: A combination's setup fails

- **WHEN** a combination's checkout cannot be set up, `moon setup`
  exiting 1 with `error: no such toolchain`
- **THEN** the branch's log records the run with no result, each test
  not run with `could not start: moon setup: exited 1: error: no such
  toolchain`, and nobody is blamed
  (`a_run_whose_working_copy_could_not_be_equipped_could_not_start` in
  `crates/sluice/tests/checks.rs`, for the words the start fails with;
  the record of a run that could not start is `run-outcome`'s, held as
  before)
