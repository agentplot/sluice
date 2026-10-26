# Design

## Context

See proposal.md for why. Both changes are checked with the openspec
`devenv.nix` pins, 1.14.3, which `root:specs` already runs over
`openspec/specs`. On `main` today:

- `openspec validate actions-pool --strict` fails: its MODIFIED block
  for "forge check verifies the setup" has the two pool scenarios but
  not "No permission to post checks", which `pull-request-reports`
  added to the archived spec on 2026-10-21 (8fef162), a day after
  `actions-pool` was split off (137fab4). The same archive widened the
  permission list to "contents, workflows, checks and pull requests";
  the block still says "contents".
- `openspec validate finished-submissions --strict` fails: the change
  has no `specs/` directory, so it has no delta.

Validating the edits below on a copy of the tree gives all five open
changes passing `openspec validate --changes --strict`.

## Goals / Non-Goals

**Goals:**

- Both changes pass strict validation, and archiving either one leaves
  the archived specs saying what `main` says plus what the change adds.

**Non-Goals:**

- Rewording anything in either change the ruling does not name.

## Decisions

### D1. actions-pool's block keeps the whole of main's requirement

The MODIFIED block replaces the archived requirement whole, so it
starts from `main`'s text, "contents, workflows, checks and pull
requests" and its three scenarios, and adds what `actions-pool` adds:
the pool's sentence and its two scenarios, in the order below. The
ruling names checks and workflows beside contents; pull requests stay
too, since `main`'s requirement has them and nothing in `actions-pool`
gives a reason to drop them.

*Alternative:* contents, workflows and checks only, as the ruling's
words list them. It would drop the pull requests permission from the
archived spec when `actions-pool` archives, which no ruling asked for.

### D2. finished-records is a new spec of the target state

The requirements describe the state `finished-submissions` proposes,
part 1 as built (its CLI test `a_landed_submission_moves_to_the_finished_records`
and the state page) and part 2 as its proposal and design describe it.
They cite the statement change the change names, as the code of part 1
already does: `finished-records-1` 61, 104, 105 and 106, with the
statement's own clauses that hold beside them (26, 49, 53, 63). Each
scenario is one a CLI or local files test can check. Part 1's tests
already check the move, the cut-short write, status by number and by
branch, `--after`, attempts and the refusals; a blamed submission
staying and older state moving have no test of their own yet, and
part 2's scenarios wait on its tasks. Adding tests is not this change's.

*Alternative:* skip the change's specs. The ruling refuses it: the
change moves where a branch's state lives and what reads it, which is
behaviour.

### D3. This change carries no spec

It edits two changes' planning files and changes no behaviour, so its
`.openspec.yaml` sets `skip_specs: true`.

## The text to write

### `openspec/changes/actions-pool/specs/forge-identity/spec.md`, whole

````markdown
## MODIFIED Requirements

### Requirement: forge check verifies the setup
`sluice forge check [<branch>]` SHALL report, for the branch's remote,
whether the key reads, a token mints, the app is installed on the
repository with permission to write contents, workflows, checks and
pull requests, and the branch's rules on the forge let only the app
bypass them. For a policy with a GitHub pool, it SHALL also report
whether the pool's workflow is on the branch as `sluice init actions`
would write it, the app may dispatch it, the environment holding the
key admits only the signer's branch, and the forge signer names that
workflow. It SHALL exit non-zero when any of these fails, naming which.

#### Scenario: A ruleset that lets the operator through
- **WHEN** the ruleset on `main` lists the operator's account as a bypass beside the app
- **THEN** `sluice forge check main` exits non-zero and names the extra bypass

#### Scenario: All set
- **WHEN** the app is installed with the permissions it needs and is the ruleset's only bypass
- **THEN** `sluice forge check main` exits 0

#### Scenario: No permission to post checks
- **WHEN** the app is installed with Contents and Workflows write but only Checks read
- **THEN** `sluice forge check main` exits non-zero and names the Checks permission

#### Scenario: The workflow and the policy disagree
- **WHEN** `[signers.forge]` names `.github/workflows/sluice.yml` and the branch has no such workflow
- **THEN** `sluice forge check main` exits non-zero and names the workflow and `sluice init actions`

#### Scenario: An environment any branch may use
- **WHEN** the environment holding the app's key has no branch rule
- **THEN** `sluice forge check main` exits non-zero and names the environment
````

### `openspec/changes/finished-submissions/specs/finished-records/spec.md`, new

````markdown
## ADDED Requirements

### Requirement: A finished submission moves out of its branch's live state
A submission that has landed or been withdrawn SHALL move from
`submissions/<id>.json` to `.finished/<id>.json`, unchanged, in the
write that tells its outcome, once no combination or landing names it;
a submission one still names SHALL move on a later write. The same
write SHALL add the submission's id, state and worktree as one line of
`finished.jsonl`, and SHALL make the move whole or not at all. A blamed
or conflicting submission SHALL stay in `submissions/` (finished-records-1
104; 53, 63).

#### Scenario: A landed submission moves
- **WHEN** submission 1 from `unit/a` lands on `main`
- **THEN** `main`'s state holds `.finished/1.json` and no `submissions/1.json`, and `finished.jsonl` holds `{"id":1,"state":"landed","worktree":"unit/a"}`

#### Scenario: A blamed submission stays
- **WHEN** submission 2 is blamed for a failed check
- **THEN** `submissions/2.json` stays where it is and `finished.jsonl` has no line for 2

#### Scenario: A write cut short moves nothing by half
- **WHEN** sluice is stopped partway through the write that moves submission 1
- **THEN** the next read finds submission 1 either in `submissions/` alone or in `.finished/` with its line in `finished.jsonl`, never in both or neither

### Requirement: A branch's own reads cost its queue
A round, `sluice overview`, `sluice` with no command and `sluice status
<branch>` SHALL read only the branch's live state: `branch.json`,
`submissions/`, `finished.jsonl` and the log the live state carries,
never `.finished/`. `sluice status <branch>`, as text or `--json`,
SHALL list the queue alone and say how many submissions are no longer
in it (finished-records-1 61, 104).

#### Scenario: Status lists the queue
- **WHEN** `main` has had two submissions land and has none queued, and the operator runs `sluice status main`
- **THEN** it lists no submission and says `2 no longer in the queue: sluice status main --all`

#### Scenario: Status as JSON lists the queue
- **WHEN** `main` has one submission queued and two landed, and the operator runs `sluice status main --json`
- **THEN** its submissions are the queued one alone

### Requirement: A finished submission stays readable and counted
`sluice status <id>` of a finished submission SHALL read its one file
in `.finished/`, and `sluice status <branch> --all`, as text or
`--json`, SHALL read every finished submission beside the queue, with
no sluice process running. A finished submission SHALL count as landed
for an `--after` that names it and toward its worktree's attempts, and
`sluice amend` and `sluice withdraw` SHALL refuse it as they did before
it moved (finished-records-1 105; 26, 49).

#### Scenario: A landed submission read by its number
- **WHEN** submission 1 has landed and moved, and the operator runs `sluice status 1 --json`
- **THEN** its state is `landed`

#### Scenario: After a finished submission
- **WHEN** submission 1 has landed and moved, and `unit/b` submits with `--after 1`
- **THEN** the submission is taken as coming after a landed one and lands

#### Scenario: Attempts count the finished
- **WHEN** submission 1 from `unit/a` has landed and moved, and `unit/a` submits again
- **THEN** the new submission's `attempt` is 2

#### Scenario: A finished submission past changing
- **WHEN** submission 1 has landed and moved, and the operator runs `sluice amend 1 --priority 2`
- **THEN** it is refused with `#1 is landed, past changing: sluice status 1`

### Requirement: Older state moves on its branch's next write
A branch whose state was written before finished submissions moved
SHALL have each of its landed or withdrawn submissions that nothing
names moved, as any other, by the branch's next write, with no step
run by hand (finished-records-1 104).

#### Scenario: A branch from before the move
- **WHEN** `main`'s state holds landed submissions under `submissions/` and no `finished.jsonl`, and a new submission is queued
- **THEN** after that write the landed ones are under `.finished/`, each with its line in `finished.jsonl`

### Requirement: The log is kept in parts
`log.jsonl` SHALL be the log's current part. A write that would append
its 1,001st line SHALL first close it as `log/<n>.jsonl`, numbered from
1 oldest first, and start a new `log.jsonl`, in the same write and
under the branch's lock as every write. No line SHALL change in a move
(finished-records-1 106; 53, 63; core.14).

#### Scenario: The 1,001st line starts a new part
- **WHEN** `main`'s `log.jsonl` holds 1,000 lines and a round writes three more
- **THEN** `log/1.jsonl` holds those 1,000 lines unchanged and `log.jsonl` holds the three

#### Scenario: Parts are numbered oldest first
- **WHEN** `main` already has `log/1.jsonl` and its `log.jsonl` is closed again
- **THEN** the closed part is `log/2.jsonl`

### Requirement: The live state carries the recent log and history reads every part
The branch's live state SHALL carry `log.jsonl` and the newest closed
part only, so `sluice overview`'s recent lines are always in it. A
reader that searches the whole log for a submission's events, the pull
request driver's removal time, a report's run counts, or the time a
combination was under test, SHALL read every part in order (finished-records-1
106).

#### Scenario: Recent lines just after a part closes
- **WHEN** `main`'s `log.jsonl` has just been closed into `log/3.jsonl` and holds two lines
- **THEN** `sluice overview` shows the branch's ten most recent lines, eight of them from `log/3.jsonl`

#### Scenario: A report reads runs from an older part
- **WHEN** a submission's runs were logged in `log/2.jsonl` and it lands after `log/3.jsonl` closed
- **THEN** its report counts every one of those runs
````

### `openspec/changes/finished-submissions/proposal.md`, a section after "What Changes"

````markdown
## Capabilities

### New Capabilities
- `finished-records`: where a branch keeps its finished submissions and
  its log's closed parts, and which reads take them.

### Modified Capabilities
None.
````

## Risks / Trade-offs

- [The statement change is missing] `finished-submissions` and part 1's
  code cite `agentplot/blueprints:design/sluice/proposals/finished-records-1.md`,
  but blueprints holds no such file, in any branch or in its history,
  and the statement's 61 still reads "its records as last written",
  while its 104 to 106 now number other clauses (making and removing
  a branch). → The new spec cites the proposal as its change does, so
  nothing here departs from what is built; writing the statement
  change, or renumbering its clauses, is the design agent's, and when
  it lands with other numbers the spec's citations follow it.
- [Archiving order] `pull-request-reports`' widening was lost once
  because `actions-pool` was written before it archived. → Archiving
  `actions-pool` now carries the whole requirement, and the chore that
  follows makes the gate catch the next such drift.
