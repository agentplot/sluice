## MODIFIED Requirements

### Requirement: A session's outcome is delivered in sluice's words

While waking is on, once the board has raised the toast for one of the
session's outcomes, it SHALL collect the session's outcomes that no way
has handed over yet by `sluice inbox --new` run in the session's
worktree, and SHALL start a turn whose text is what sluice printed, as
printed, and no turn when sluice printed nothing. The turn SHALL hold
none of the board's own words.

#### Scenario: Blamed while idle

- **WHEN** the session's submission #13 is blamed for `cli:test` outside
  any multiplexer, and `sluice inbox --new` prints `[sluice] #13 failed
  on main: cli:test. sluice wait 13 shows why.` followed by the lines
  `sluice wait 13` prints and the line naming `sluice confirm
  13:failed`
- **THEN** a toast says `sluice: #13 was blamed`, and one turn starts
  whose text is exactly what `sluice inbox --new` printed

#### Scenario: Under a multiplexer the notice has come

- **WHEN** the session's submission #13 lands, the round has pushed the
  notice into the session's pane, and `sluice inbox --new` prints
  nothing
- **THEN** a toast says `sluice: #13 landed on main`, and no turn starts

### Requirement: The board keeps its last overview while no watch is open

While no watch is open, the band, the pane and the status line SHALL
keep the last overview read, unless the host's sluice has no watch or the
last watch said the repository has no sluice. An outcome of the session's
decided meanwhile SHALL be raised and collected, as a new outcome is,
once a watch prints an overview that holds it and the last overview read
did not.

#### Scenario: The watch can't be started

- **WHEN** the session's watch printed `main` with #3 queued and ended,
  and every later `sluice overview --json --watch` fails to start
- **THEN** the board keeps trying the watch, runs no `sluice overview
  --json` in between, and the band still shows `main` with #3 queued

#### Scenario: Landed while no watch was open

- **WHEN** the session's watch printed `main` with the session's #13
  queued and ended, and the next watch prints `main` with #13 landed
- **THEN** the board runs `sluice inbox --new` once, and starts a turn
  with what it printed, if anything

### Requirement: The board keeps no list of what it delivered

The board SHALL keep no record of its own of which outcomes it handed
over: what a session is owed, and what no way has handed it yet, SHALL
be sluice's to say, through `sluice inbox` and `sluice inbox --new`. A
reload of the mod is a first overview, and SHALL start a turn only for
what sluice still holds owed to the session, which its agent has not
confirmed.

#### Scenario: A reload after a turn

- **WHEN** the board started a turn for #13's blame, the agent ran
  `sluice confirm 13:failed`, the mod is reloaded, and `sluice inbox`
  now prints nothing
- **THEN** no turn starts

#### Scenario: A reload after a turn the agent did not confirm

- **WHEN** the board started a turn for #13's blame, nobody confirmed
  it, the mod is reloaded, and `sluice inbox` prints #13's blame again
- **THEN** one turn starts whose text is what `sluice inbox` printed

### Requirement: The board collects at its first overview

While waking is on, the board SHALL run `sluice inbox` in the session's
worktree when it takes its first overview: at the session's start, after
a reload of the mod, and after a watch said the repository has no
sluice, so a session is handed again what it never confirmed. It SHALL
start a turn with what sluice printed, and raise no toast for outcomes
from before that overview.

#### Scenario: A session restarted after its submission landed

- **WHEN** the session starts, its first overview shows #13 landed, and
  `sluice inbox` prints `[sluice] #13 landed on main as abc1234.`,
  `passed · landed as abc1234` and the line naming `sluice confirm
  13:landed`
- **THEN** one turn starts whose text is those three lines, and no toast
  is raised for #13

#### Scenario: Nothing pending at the start

- **WHEN** the session starts and `sluice inbox` prints nothing
- **THEN** no turn starts

## ADDED Requirements

### Requirement: The board runs with a sluice that hands over by --new

The board's `.claude-plugin/sluice.json` SHALL name as `from` the first
sluice release that has `sluice inbox --new` and `sluice confirm`, so a
host whose sluice is older is told, once a session, that the board needs
that release, and the board collects nothing there.

#### Scenario: A host on the release before

- **WHEN** the host's `sluice --version` prints the release before
  `from`
- **THEN** the board says it needs sluice `from` or later, which has
  `sluice inbox`, runs no `sluice inbox`, and starts no turn
