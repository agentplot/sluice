## ADDED Requirements

### Requirement: The overview's document carries a branch's held checkouts

`sluice overview --json` SHALL give each branch `held_checkouts`, every
checkout its state names whose release failed, each with `place`,
`failed`, `tries`, `at`, `stopped` and, on a branch whose state is on its
remote, `host`, as `sluice status <branch> --json` lists them. It SHALL
read them from the branch's state alone, so each host's overview shows
them, and SHALL leave the list empty when there are none (123, 26, 61).

#### Scenario: A held checkout in the document

- **WHEN** `main`'s checkout `sluice/try/main/48` has had its release
  fail four times with `usage: scripts/preview up` and is tried no more
- **THEN** `sluice overview --json` lists under `main` a `held_checkouts`
  entry with `place` `sluice/try/main/48`, `tries` 4, `stopped` true and
  `failed` ending `usage: scripts/preview up`

#### Scenario: Another host's overview

- **WHEN** `main`'s state is on its remote, host `deneb` holds
  `sluice/try/main/48`, and the overview is read on host `vega`
- **THEN** `vega`'s document lists the same entry with `host` `deneb`

#### Scenario: Nothing held

- **WHEN** no release of `main` has failed
- **THEN** `main`'s `held_checkouts` is an empty list

### Requirement: The overview names each held place under its branch

`sluice overview` SHALL show under each branch every checkout, worktree
and preview of that branch held because its release failed and tried no
more: its name, how many times its release failed, the last line of what
failed, and the command that runs its release again and the one that
frees it without. A held checkout of another host SHALL name that host.
A branch with no records SHALL get its own line for them (123, 26).

#### Scenario: A held checkout

- **WHEN** `sluice/try/main/48` is held after four failed releases ending
  `usage: scripts/preview up`
- **THEN** under `main`'s line `sluice overview` prints
  `sluice/try/main/48 is held: its release failed 4 times: usage:
  scripts/preview up`, then `sluice release sluice/try/main/48 runs it
  again; sluice release sluice/try/main/48 --accept-failure frees it
  without`

#### Scenario: A held worktree

- **WHEN** the worktree `/work/app/unit-a`, whose runs are held to
  `main`, had its `sluice release` fail once
- **THEN** under `main`'s line the overview names `/work/app/unit-a` as
  held with that failure, and gives `sluice release --worktree
  /work/app/unit-a` and the same with `--accept-failure`

#### Scenario: A held preview

- **WHEN** a preview of `main` had its release fail until its tries were
  spent
- **THEN** under `main`'s line the overview names the preview as held
  with what failed, and gives `sluice release <preview>` and the same
  with `--accept-failure`

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

### Requirement: Showing a held place changes nothing

Reading the overview, and any view drawn from it, SHALL run no release,
count no try and write nothing; a held place SHALL stay held until the
operator's `sluice release` asks for it (123, 61).

#### Scenario: Read many times

- **WHEN** `sluice overview --json --watch` runs for ten minutes while
  `sluice/try/main/48` is held
- **THEN** its release is never run, its tries stay as they were, and
  the branch's state and log are unchanged
