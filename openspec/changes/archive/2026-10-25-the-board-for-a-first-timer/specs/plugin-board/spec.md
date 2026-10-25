# Spec Delta

## ADDED Requirements

### Requirement: The pane closes in either layout

With full-screen mode off, `/sluice` SHALL open the pane asking for the keyboard and to close on Esc. A pane seated above the prompt SHALL show a `close` button, pressed by `x` or Enter, that closes it; a docked pane SHALL show none of the board's. With full-screen mode on, `/sluice` SHALL open the pane closing on Esc without asking for the keyboard. In both layouts, `/sluice` run while the pane is open SHALL close it.

#### Scenario: Full-screen off

- **WHEN** the user runs `/sluice` with full-screen mode off
- **THEN** the pane opens asking for the keyboard and to close on Esc, and shows a `close` button

#### Scenario: The close button

- **WHEN** the pane is open above the prompt and the user presses its `close` button
- **THEN** the pane closes

#### Scenario: Full-screen on

- **WHEN** the user runs `/sluice` with full-screen mode on
- **THEN** the pane opens docked, closing on Esc, without asking for the keyboard, and shows no `close` button of the board's

#### Scenario: /sluice again

- **WHEN** the pane is open in either layout and the user runs `/sluice`
- **THEN** the pane closes and the reply reads `sluice's pane is closed.`

### Requirement: The reply to /sluice says how to close the pane

The text `/sluice` returns when it opens the pane SHALL say how to close it in the layout it opened in: with full-screen mode off, `sluice's queue is open above the prompt. Esc, its close button or /sluice again closes it.`; with full-screen mode on, `sluice's queue is open in a pane. Its close mark or /sluice again closes it.`

#### Scenario: Opened above the prompt

- **WHEN** the user runs `/sluice` with full-screen mode off
- **THEN** the reply reads `sluice's queue is open above the prompt. Esc, its close button or /sluice again closes it.`

#### Scenario: Opened docked

- **WHEN** the user runs `/sluice` with full-screen mode on and the terminal 200 columns wide
- **THEN** the reply reads `sluice's queue is open in a pane. Its close mark or /sluice again closes it.`

### Requirement: A pane with no records says how a submission gets there

When sluice's overview names no branch and the session names none with `SLUICE_BRANCH`, the pane SHALL say, under the repository and host line, `Nothing submitted yet.` and, on the next line, `When an agent in a worktree runs sluice submit <branch>, its submission shows here until it lands.`

#### Scenario: A repository set up with nothing submitted

- **WHEN** `sluice overview --json` lists no branches and the user runs `/sluice`
- **THEN** the pane shows `Nothing submitted yet.` and `When an agent in a worktree runs sluice submit <branch>, its submission shows here until it lands.`, and not `No branch has a queue.`

#### Scenario: Branches with records but no queue

- **WHEN** the overview lists `main` with nothing queued and no event in the last quarter hour
- **THEN** the pane shows `No branch has a queue.` as before

### Requirement: A pane that could not read sluice says why and what to do

When the board has no overview because `sluice overview --json` could not run or failed, the pane SHALL say `sluice could not be read here:`, then the reason (`the sluice command was not found` when it could not be started, else the first line of what sluice wrote to stderr, else its exit code), then `Ask Claude to set sluice up in this repository.`

#### Scenario: sluice not installed

- **WHEN** `sluice` cannot be started in the session and the user runs `/sluice`
- **THEN** the pane shows `sluice could not be read here: the sluice command was not found` and `Ask Claude to set sluice up in this repository.`

#### Scenario: The overview fails

- **WHEN** `sluice overview --json` exits 3 having written `sluice: not a git repository` to stderr
- **THEN** the pane shows `sluice could not be read here: sluice: not a git repository` and `Ask Claude to set sluice up in this repository.`

### Requirement: The pane reads sluice again when the first read failed

While the board has no overview, each `/sluice` that opens the pane SHALL run `sluice overview --json` in the session's directory again. When that succeeds, the board SHALL show and follow the queue from then on, as it does after a first read that succeeded; when it fails, the pane SHALL say why, as for the first read.

#### Scenario: sluice set up during the session

- **WHEN** the session started where `sluice overview --json` failed, and later, with sluice set up, it lists `main` with #3 queued and the user runs `/sluice`
- **THEN** the pane shows `main`'s section with #3 under QUEUED, and the band shows `main`'s line

#### Scenario: Still failing

- **WHEN** the session started where `sluice` could not be started and it still cannot when the user runs `/sluice`
- **THEN** the pane shows `sluice could not be read here: the sluice command was not found`
