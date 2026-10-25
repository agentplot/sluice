## Purpose

The words in which sluice says how a submission moves on after its
outcome or a refusal, addressed to the agent working in the worktree the
submission came from, so that no reader takes them as their own to do.

## ADDED Requirements

### Requirement: A way on names the agent in the worktree

Where sluice's text output says how a submission moves on, after a
failure, a conflict, a time limit, a check the host could not run, or a
refusal for uncommitted changes, a detached head or a check kept out of
CI, it SHALL name who does it: the agent working in the worktree the
submission came from, by that worktree's branch, or by the worktree
itself when it is on none. It SHALL NOT say "your worktree".

#### Scenario: Blamed

- **WHEN** a submission from `unit/a` to `main` is blamed for `tests:t-cards`
- **THEN** its report ends `the agent working in unit/a fixes it there and submits again`

#### Scenario: A conflict

- **WHEN** a submission from `unit/a` to `bolt/x` is removed for a conflict
- **THEN** its report ends `the agent working in unit/a merges or rebases onto bolt/x there and submits again`

#### Scenario: Refused for uncommitted changes

- **WHEN** `sluice submit main --worktree unit/a` is refused because `unit/a` holds `plugin/scratch.py` uncommitted
- **THEN** after naming the path it says `the agent working in unit/a commits or removes them and submits again`

### Requirement: A way on never sends its reader elsewhere

No way-on line SHALL ask its reader to go into a worktree, to run a
command from inside one, or to pass a command to a person. A step that
is the operator's, setting a result aside, SHALL still name the
operator.

#### Scenario: Read from the main checkout

- **WHEN** a session in the repository's main checkout runs `sluice wait 12`, and #12, from `unit/a`, is blamed
- **THEN** the report names the agent working in `unit/a` as the one who fixes it and submits again, and says nothing of running a command inside `unit/a`

#### Scenario: The operator's step

- **WHEN** a blamed submission's failed check is read as flaky against a fail recorded earlier
- **THEN** the report still says the operator can set that fail aside, with the `sluice set-aside` command

### Requirement: Outside a worktree, sluice says who submits

`sluice` with no command, run outside a worktree it can submit from,
SHALL say that work is submitted by the agent working in its worktree,
from that worktree.

#### Scenario: In a checkout that is no worktree sluice knows

- **WHEN** `sluice` runs with no command outside a worktree it can submit from
- **THEN** its first line is `not in a worktree sluice can submit from: the agent working in a worktree submits its work from there`

### Requirement: What scripts read is unchanged

A command's `next` commands, and every field of its JSON documents but
an error's `message`, SHALL be the same as before; an error's `message`
SHALL carry the same words as the text.

#### Scenario: A refusal under --json

- **WHEN** `sluice submit main --json` is refused for uncommitted changes in `unit/a`
- **THEN** `error.next` is `["sluice submit main"]` as before, and `error.message` names the agent working in `unit/a`
