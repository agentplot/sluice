## ADDED Requirements

### Requirement: The pane shows a branch's held places

The `/sluice` pane SHALL show under a branch's heading, headed RELEASE
FAILED, each checkout, worktree and preview of that branch held because
its release failed and tried no more, in the herdr popup's words: its
name, how many times it failed, the last line of what failed, and the
command to release it, with `--accept-failure` to free it without, on
its host when another's. No row SHALL run a release (123, 26).

#### Scenario: A held checkout

- **WHEN** the overview shows `main` with `sluice/try/main/48` held after
  four failed releases ending `usage: scripts/preview up`
- **THEN** under `main`'s heading the pane shows RELEASE FAILED, then
  `✗ sluice/try/main/48 held · failed 4 times · usage: scripts/preview up`,
  then `sluice release sluice/try/main/48 · --accept-failure frees it
  without`

#### Scenario: An idle branch with a held place

- **WHEN** `main` has nothing queued, no recent event and a held preview
- **THEN** the pane shows `main`'s block with the preview under RELEASE
  FAILED

#### Scenario: Still being tried

- **WHEN** a checkout's release has failed once and is not yet stopped
- **THEN** the pane shows no RELEASE FAILED row for it

#### Scenario: An older sluice

- **WHEN** the overview has no `held_checkouts` and no `release_failed`
- **THEN** the pane shows no RELEASE FAILED heading and draws as before

### Requirement: The band and the status line count held places

The band's line for a branch with held places SHALL end with `N held:
release failed`, and a branch with held places and nothing queued SHALL
get its line. The status line SHALL end with the repository's count in
the same words, and SHALL be shown for it with nothing queued (123).

#### Scenario: The band

- **WHEN** `main` has 3 submissions and one held checkout
- **THEN** the band's line for `main` ends `· 1 held: release failed`

#### Scenario: The status line with nothing queued

- **WHEN** nothing is queued, the session has no submission, and one
  worktree is held
- **THEN** the status line is `sluice: 1 held: release failed`

#### Scenario: The status line beside the session's own

- **WHEN** the session's `#13` is under test and one checkout is held
- **THEN** the status line ends `yours #13 testing · 1 held: release
  failed`
