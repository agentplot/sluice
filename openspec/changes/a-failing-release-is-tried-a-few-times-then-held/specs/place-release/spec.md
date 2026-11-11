## ADDED Requirements

### Requirement: A worktree's failed release is tried its branch's retries, then held

A worktree's release that fails SHALL count one try on the worktree's
place, whoever ran `sluice release` there: its agent, `sluice exec
--release`, `wt submit` or a tool that removes the worktree. Once its
tries pass the `release_retries` of the branch its runs are held to,
read from the branch's current revision, or 3 where the branch states
none or cannot be read, the place SHALL be held. sluice SHALL never run
the release on its own (123, 25, 56; rule core.16).

#### Scenario: Retries named

- **WHEN** a worktree ran `sluice exec` on a branch whose policy says
  `release_retries = 1`, its release fails on every try, and
  `sluice release` runs there twice
- **THEN** each exits 1 with what failed, the first leaves the
  worktree's place tried once and not held, and the second leaves it
  held after two tries
  (`a_worktrees_release_is_tried_its_branchs_retries_then_held` in
  `crates/sluice/tests/cli.rs`)

#### Scenario: No retries named

- **WHEN** a worktree's release fails once on a branch whose policy
  names no `release_retries`
- **THEN** `sluice status` lists the worktree with one try of four and
  says `sluice release` runs it again
  (`status_marks_a_worktree_whose_release_failed` in
  `crates/sluice/tests/cli.rs`)

#### Scenario: Counted on the place

- **WHEN** a worktree's place is released with retries 1 and its
  release command fails twice
- **THEN** the place is kept after the first failure with one try, not
  held, its ports and scratch kept, and after the second with two
  tries, held
  (`a_failed_session_release_is_held_once_its_retries_are_spent` in
  `crates/sluice/tests/profiles/places.rs`)

### Requirement: A held place's release runs nothing unless asked again

`sluice release` of a held worktree, a held checkout by its name, or a
held preview by its name SHALL run nothing, count no try and read no
policy. It SHALL say the place is held, with how many times its release
failed and what failed last, SHALL name `sluice release … --again` and
`sluice release … --accept-failure`, and SHALL exit 1. `--json` SHALL
answer `released` false, `held` true, its `tries` and `failed` (123).

#### Scenario: A held worktree asked again and again

- **WHEN** a worktree is held because its release failed on every try
  its branch gives it, and `sluice release` runs there again
- **THEN** no release command runs, the command says the worktree is
  held, names `--again` and `--accept-failure`, and exits 1, and its
  tries are as they were
  (`a_worktrees_release_is_tried_its_branchs_retries_then_held`)

#### Scenario: A held preview

- **WHEN** a preview is held because its release failed, and its caller
  runs `sluice release <preview>`
- **THEN** no release command runs and the command exits 1 saying the
  preview is held; `sluice release <preview> --again` runs it
  (`a_held_previews_release_runs_again_only_when_asked_again` in
  `crates/sluice/tests/previews.rs`)

### Requirement: Asked again, a held place's release runs once more

`sluice release … --again` SHALL run a held place's release once more
and count the try: passing, the place is released; failing, it stays
held with what failed now. `--again` on a place that is not held SHALL
release it as `sluice release` does, and `--again` with
`--accept-failure` SHALL be refused (123).

#### Scenario: Asked with --again

- **WHEN** a worktree is held because its release failed on every try
  its branch gives it, and `sluice release --again` runs there once
  while the release still fails and once after it is fixed
- **THEN** the first runs the release command, exits 1 and leaves the
  worktree held with one more try, and the second runs it, exits 0 and
  releases the worktree
  (`a_worktrees_release_is_tried_its_branchs_retries_then_held`)

#### Scenario: Both asked at once

- **WHEN** `sluice release --again --accept-failure` runs in a worktree
- **THEN** the command is refused before anything is read or run
  (`a_worktrees_release_is_tried_its_branchs_retries_then_held`)

## RENAMED Requirements

- FROM: `### Requirement: An unheld worktree's release answers as a held one's and keeps nothing`
- TO: `### Requirement: An unheld worktree's release answers as a held one's and keeps only its failure`

## MODIFIED Requirements

### Requirement: The operator releases a held checkout by its name

`sluice release <name> --again` SHALL release the held checkout of that
name on this host, running its release once more; on success the
checkout is gone and its branch's state no longer lists it, and on
failure it stays held with what failed and the command exits 1.
`sluice release <name>` without `--again` SHALL run nothing for a held
checkout and answer that it is held. A name that names no checkout
SHALL exit 0 saying there is nothing to release (123).

#### Scenario: Released once more

- **WHEN** the operator fixes what the release needed and runs `sluice
  release sluice/try/main/48 --again`
- **THEN** the release command runs, the checkout is removed, its ports
  are free, and the log holds `released` naming it
  (`release_by_name_releases_a_held_checkout` in
  `crates/sluice/tests/cli.rs`)

#### Scenario: Still failing

- **WHEN** the operator runs `sluice release sluice/try/main/48 --again`
  and the release fails again
- **THEN** the command prints what failed, exits 1, and the checkout is
  still listed held

#### Scenario: Asked without --again

- **WHEN** the operator runs `sluice release sluice/try/main/48` while
  it is held
- **THEN** no release command runs, the command says the checkout is
  held and names `sluice release sluice/try/main/48 --again`, and it
  exits 1 (`release_by_name_releases_a_held_checkout`)

### Requirement: A worktree's or a preview's failed release holds it

A failed `sluice release` in a worktree, or a failed release of a
preview, SHALL keep the place with what failed: `sluice status` lists
the worktree with it, and `sluice previews` the preview. Each SHALL be
held once its tries pass its branch's `release_retries`, a worktree's
tries counted over every `sluice release` there and a preview's as a
combination's checkout's are. sluice SHALL never release a worktree
whose directory exists on its own (123, 25, 101).

#### Scenario: A worktree's release fails

- **WHEN** an agent runs `sluice release` in its worktree and the
  release command exits 1
- **THEN** the command exits 1 with the command's output, the worktree
  keeps its ports, and `sluice status` lists it with what failed, and no
  round releases it

### Requirement: The overview names each held place under its branch

`sluice overview` SHALL show under each branch every checkout, worktree
and preview of that branch held because its release failed and tried no
more: its name, how many times its release failed, the last line of what
failed, and the command that runs its release again, with `--again`,
and the one that frees it without. A held checkout of another host
SHALL name that host. A branch with no records SHALL get its own line
for them (123, 26).

#### Scenario: A held checkout

- **WHEN** `sluice/try/main/48` is held after four failed releases ending
  `usage: scripts/preview up`
- **THEN** under `main`'s line `sluice overview` prints
  `sluice/try/main/48 is held: its release failed 4 times: usage:
  scripts/preview up`, then `sluice release sluice/try/main/48 --again
  runs it again; sluice release sluice/try/main/48 --accept-failure
  frees it without`
  (`overview_names_a_held_checkout_under_its_branch` in
  `crates/sluice/tests/cli.rs`)

#### Scenario: A held worktree

- **WHEN** the worktree `/work/app/unit-a`, whose runs are held to
  `main`, had its `sluice release` fail on every try `main` gives it
- **THEN** under `main`'s line the overview names `/work/app/unit-a` as
  held with that failure, and gives `sluice release --worktree
  /work/app/unit-a --again` and the same with `--accept-failure`
  in place of `--again`
  (`overview_names_a_held_worktree_under_its_branch`)

#### Scenario: A held preview

- **WHEN** a preview of `main` had its release fail until its tries were
  spent
- **THEN** under `main`'s line the overview names the preview as held
  with what failed, and gives `sluice release <preview> --again` and
  `sluice release <preview> --accept-failure`

#### Scenario: Another host's checkout

- **WHEN** the overview is read on `vega` and `sluice/try/main/48` is
  held by `deneb`
- **THEN** its line says it is held on `deneb`, and its commands are
  given as to run on `deneb`

#### Scenario: Still being tried

- **WHEN** `sluice/try/main/48`'s release has failed once and the next
  round will try it again
- **THEN** `sluice overview` prints nothing of it, and `sluice status
  main` lists it as tried again by the next round

#### Scenario: A worktree's branch with no records

- **WHEN** a worktree held to `release`, a branch with no records, is
  held because its release failed
- **THEN** the overview prints `release · no records` followed by the
  worktree's held lines
  (`overview_names_a_held_worktree_of_a_branch_with_no_records`)

### Requirement: Showing a held place changes nothing

Reading the overview, and any view drawn from it, SHALL run no release,
count no try and write nothing; a held place SHALL stay held until the
operator's `sluice release --again` runs its release or `sluice release
--accept-failure` frees it (123, 61).

#### Scenario: Read many times

- **WHEN** `sluice overview --json --watch` runs for ten minutes while
  `sluice/try/main/48` is held
- **THEN** its release is never run, its tries stay as they were, and
  the branch's state and log are unchanged

### Requirement: An unheld worktree's release answers as a held one's and keeps only its failure

The release of a worktree sluice holds nothing for SHALL answer as a
held worktree's release does: `released` and exit 0 when the command
passed, and what the command printed and exit 1 when it failed. A
release that passed SHALL keep nothing for the worktree, and a release
asked again SHALL run the command again. A release that failed SHALL
keep the worktree's place, with no ports, under the branch whose policy
named the command, with what failed and one try, so the worktree is
held as any worktree whose release failed: its tries counted, listed
by `sluice status`, and freed by `--accept-failure` (95, 123; record
0112 as amended).

#### Scenario: A failing release

- **WHEN** `main`'s policy names a release command that prints
  `emulator still answers` and exits 3, and `sluice release` runs in a
  worktree sluice holds nothing for
- **THEN** the command exits 1 with `emulator still answers` in what it
  says, `sluice status` lists the worktree with one failed try, and
  `sluice release --accept-failure` there frees it
  (`an_unheld_worktrees_release_runs_its_release_command` in
  `crates/sluice/tests/exec.rs`)

#### Scenario: Kept on the place

- **WHEN** `release_session` releases a linked worktree with no record
  under a policy whose release command exits 3, with retries 0
- **THEN** the worktree's place is listed with the policy's branch, no
  ports, one try and held, and its scratch exists
  (`an_unheld_worktrees_failed_release_keeps_its_place` in
  `crates/sluice/tests/profiles/places.rs`)

#### Scenario: Released again

- **WHEN** `sluice release` runs in a worktree whose place it has just
  released, and the policy still names a release command
- **THEN** the release command runs again, and the command says the
  worktree was released and exits 0
  (`release_runs_the_policys_release_command_once` in
  `crates/sluice/tests/cli.rs`)
