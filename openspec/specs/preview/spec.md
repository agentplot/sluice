# preview Specification

## Purpose
Gives a person a running copy of a revision a branch has landed, on the
repository's own stack, at addresses they can open, without anyone
keeping a worktree of the branch.

## Requirements

### Requirement: A preview is made only at a landed revision

`sluice preview <branch> <revision>` SHALL make a preview only when the
revision is reachable from the branch's current revision (98). Any other
revision, such as a combination under test or an agent's unsubmitted
head, SHALL be refused with the reason and SHALL make nothing.

#### Scenario: A landed revision

- **WHEN** a caller asks for a preview of a revision the branch has landed
- **THEN** a detached checkout of exactly that revision is made and its
  preview command starts

#### Scenario: A revision not on the branch

- **WHEN** a caller asks for a preview of an agent's head that has not
  landed
- **THEN** sluice refuses, saying the branch has not landed it, makes no
  checkout and takes no port (S35)

### Requirement: The preview command runs once under the executor

A preview's checkout SHALL receive its ports and caches as a
combination's checkout does, and SHALL run the `[preview] command` of
the previewed revision's policy once, under the host's executor and
the limits of the branch's current policy, and end its process group
when it exits (98, 66). What the command hands to another process SHALL
stay until the preview is released.

#### Scenario: The stack is handed off

- **WHEN** the preview command starts a container engine and servers,
  hands them off and exits 0
- **THEN** the preview's state is up and the containers and servers are
  still running

#### Scenario: The command reads its revision's policy

- **WHEN** the branch's current policy has a different `[preview] command`
  from the previewed revision's
- **THEN** the previewed revision's command is the one run

#### Scenario: No preview table

- **WHEN** the previewed revision's policy has no `[preview]` table
- **THEN** sluice refuses, naming the missing table, and makes nothing

### Requirement: Each port has a name unique on the host

For each port name in `[checkout] ports`, a preview SHALL receive a name:
one DNS label of lowercase letters, digits and hyphens, at most 63
characters, built from the port name, the short revision, the branch and
the repository, with the port name and short revision kept whole when it
is shortened (99). No two checkouts alive on the host SHALL share a name.

#### Scenario: A long branch name

- **WHEN** a branch's name would make the label longer than 63 characters
- **THEN** the label is shortened with a hash of the whole and still begins
  with the port name and the short revision

#### Scenario: Two branches

- **WHEN** two branches of one repository each hold a preview while
  combinations and session runs are under test on the host
- **THEN** no two of them share a port or a name (S34)

### Requirement: The command receives its names

The preview command SHALL receive `SLUICE_PLACE`, `SLUICE_PORT_<NAME>`
and `SLUICE_NAME_<NAME>` for each port name, `SLUICE_BRANCH` and
`SLUICE_REVISION` (the full revision).

#### Scenario: A command routes a name

- **WHEN** the preview command runs
- **THEN** `SLUICE_NAME_WEB` and `SLUICE_PORT_WEB` are set for a port named
  `web`, and the release command later receives the same values

### Requirement: Addresses are known when the preview is asked for

`[preview] addresses` SHALL map a label to a template in which
`{name.<port>}` and `{port.<port>}` are replaced by the preview's name
and port. `sluice preview` SHALL print the addresses filled in when it
returns, before the command has ended. sluice SHALL never contact them.

#### Scenario: Addresses before the stack is up

- **WHEN** `sluice preview` returns without `--wait`
- **THEN** its output holds the preview's name, state `starting` and every
  address filled in (S30)

#### Scenario: A template names an unknown port

- **WHEN** an address names a port `[checkout] ports` does not list
- **THEN** sluice refuses, naming the address and the port, and makes
  nothing

### Requirement: Asking again returns the same preview

A revision SHALL have at most one preview on a host, whichever branch it
was asked for through. Asking for a preview of a revision whose command
has started SHALL return that preview, its names, ports, addresses and
state unchanged, and SHALL start nothing (100).

#### Scenario: A caller asks twice

- **WHEN** a caller asks for a preview of R2, crashes, and asks again
- **THEN** the second asking returns the first preview and runs no
  command (S32)

### Requirement: The command counts against the bound while it runs

The preview command SHALL wait for room in the host's bound and SHALL
never take the bound's reserve for submissions, as a session run does.
A preview whose command has ended SHALL count against nothing (102).

#### Scenario: The bound is full

- **WHEN** the host's bound is full and a caller asks for a preview
- **THEN** `sluice preview` waits, saying so, and the preview is listed as
  `waiting`; once there is room its command starts and `sluice preview`
  returns

#### Scenario: The asker dies while waiting

- **WHEN** a caller is killed while its preview is `waiting`, and asks
  again
- **THEN** the second asking takes up the same preview, with the same
  names and ports, and starts its command when there is room

#### Scenario: A held preview

- **WHEN** a preview is up and its command has ended
- **THEN** the host's count of runs does not include it

### Requirement: A preview decides nothing

Nothing of a preview's command SHALL be recorded as a result, read by
any gate, or waited on by any submission (103).

#### Scenario: A failed command

- **WHEN** a preview command exits non-zero
- **THEN** no result line is written for it and no submission on the
  branch waits or lands differently (S33)

### Requirement: A preview's state is readable

A preview's state SHALL be one of `waiting`, `starting`, `up` or
`failed`, read from files on the host, so it is the same whichever
process asks. `--wait` SHALL return once the state is `up` or `failed`,
exiting 0 for `up` and 1 for `failed`. `--json` SHALL print one
document on stdout.

#### Scenario: Waiting for the stack

- **WHEN** a caller runs `sluice preview <branch> <revision> --wait` and
  the command exits 0
- **THEN** it prints state `up` and the addresses, and exits 0
