## ADDED Requirements

### Requirement: The popup shows a branch's held places

The popup and the dashboard SHALL show under a branch's heading, headed
RELEASE FAILED, each checkout, worktree and preview of that branch held
because its release failed and tried no more: its name, how many times
it failed, the last line of what failed, and the command to release it,
with `--accept-failure` to free it without, on its host when another's.
They SHALL be worded as the board's `/sluice` pane (123, 26).

#### Scenario: A held checkout

- **WHEN** the repository's document shows `main` with
  `sluice/try/main/48` held after four failed releases ending
  `usage: scripts/preview up`
- **THEN** under `main`'s heading the popup shows RELEASE FAILED, then
  `✗ sluice/try/main/48 held · failed 4 times · usage: scripts/preview up`,
  then `sluice release sluice/try/main/48 · --accept-failure frees it
  without`

#### Scenario: A held worktree

- **WHEN** the document's worktree `/work/sluice/unit-a`, held to `main`,
  has a failed release it no longer tries
- **THEN** under RELEASE FAILED the popup shows `✗ worktree unit-a held`
  with its failure, then `sluice release --worktree /work/sluice/unit-a
  · --accept-failure frees it without`

#### Scenario: Another host's checkout

- **WHEN** the document is `vega`'s and `sluice/try/main/48` is held by
  `deneb`
- **THEN** its row says `held on deneb`, and the command beneath it
  begins `on deneb:`

#### Scenario: An idle branch with a held place

- **WHEN** `main` has nothing queued, no recent event and a held
  checkout
- **THEN** the popup shows `main`'s block with its RELEASE FAILED rows,
  rather than counting it among the branches with nothing queued

#### Scenario: Still being tried

- **WHEN** a checkout's release has failed once and is not yet stopped
- **THEN** the popup shows no RELEASE FAILED row for it

#### Scenario: An older sluice

- **WHEN** a document has no `held_checkouts` and no `release_failed`
- **THEN** the popup shows no RELEASE FAILED heading and reads the
  document as before

### Requirement: The segment counts a branch's held places

The tab-bar segment's line for a branch with held places SHALL end with
`N held: release failed`, after the clause for agents' checks; a branch
with held places and nothing queued SHALL get its line (123).

#### Scenario: Held beside a queue

- **WHEN** the focused repository's `main` has its queue and one held
  checkout
- **THEN** the segment's line for `main` ends `· 1 held: release failed`

#### Scenario: Held with nothing queued

- **WHEN** `main` has nothing queued and two held places
- **THEN** the segment prints `sluice main · 0 submissions · 2 held:
  release failed`
