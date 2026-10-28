# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: A session is told its submission's outcome`
- TO: `### Requirement: A session's outcome is delivered in sluice's words`

## MODIFIED Requirements

### Requirement: A session's outcome is delivered in sluice's words

A submission is the session's when its `from.path` is the session's
worktree, or the session's own `sluice submit` or `wt submit` printed its
number or its pull request. When an overview shows a landing, blame or
conflict of one of the session's submissions that the overview before it
did not show, the board SHALL raise a toast, and SHALL raise the host's
notification for a blame. While waking is on, it SHALL then collect the
session's pending outcomes by `sluice inbox` run in the session's
worktree, and SHALL start a turn whose text is what sluice printed, as
printed, and no turn when sluice printed nothing. The turn SHALL hold
none of the board's own words. The toast and the notification are the
host's user's and SHALL NOT be taken as a delivery to the session.

#### Scenario: Blamed while idle

- **WHEN** the session's submission #13 is blamed for `cli:test` outside
  any multiplexer, and `sluice inbox` prints `[sluice] #13 failed on
  main: cli:test. sluice wait 13 shows why.` followed by the lines
  `sluice wait 13` prints
- **THEN** a toast says `sluice: #13 was blamed`, and one turn starts
  whose text is exactly what `sluice inbox` printed

#### Scenario: Under a multiplexer the notice has come

- **WHEN** the session's submission #13 lands, the round has pushed the
  notice into the session's pane, and `sluice inbox` prints nothing
- **THEN** a toast says `sluice: #13 landed on main`, and no turn starts

#### Scenario: Another session's outcome

- **WHEN** an overview shows submission #20, not the session's, landed
- **THEN** the board raises no toast and runs no `sluice inbox`

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
- **THEN** the board runs `sluice inbox` once, and starts a turn with
  what it printed, if anything

## ADDED Requirements

### Requirement: The board keeps no list of what it delivered

The board SHALL keep no record of its own of which outcomes were
delivered: whether a session has had an outcome SHALL be sluice's to
say, through `sluice inbox`, which marks what it prints delivered and
prints nothing for what it delivered before. A reload of the mod SHALL
therefore start no turn for an outcome sluice has marked delivered.

#### Scenario: A reload after a turn

- **WHEN** the board started a turn for #13's blame, the mod is reloaded,
  and the next watch prints the same overview, where `sluice inbox` now
  prints nothing
- **THEN** no turn starts

### Requirement: The board collects at its first overview

While waking is on, the board SHALL run `sluice inbox` in the session's
worktree when it takes its first overview: at the session's start, after
a reload of the mod, and after a watch said the repository has no
sluice. It SHALL start a turn with what sluice printed, and raise no
toast for outcomes from before that overview.

#### Scenario: A session restarted after its submission landed

- **WHEN** the session starts, its first overview shows #13 landed, and
  `sluice inbox` prints `[sluice] #13 landed on main as abc1234.` and
  `passed · landed as abc1234`
- **THEN** one turn starts whose text is those two lines, and no toast
  is raised for #13

#### Scenario: Nothing pending at the start

- **WHEN** the session starts and `sluice inbox` prints nothing
- **THEN** no turn starts

### Requirement: The board collects nothing while waking is off

With waking off, the board SHALL NOT run `sluice inbox`, so an outcome
it would have delivered stays pending in sluice's records for another
carrier. It SHALL still raise the toast.

#### Scenario: Waking off

- **WHEN** waking is off and the session's #13 is blamed
- **THEN** a toast says `sluice: #13 was blamed`, no `sluice inbox`
  runs, and no turn starts

### Requirement: The board collects once at a time

The board SHALL run at most one `sluice inbox` at a time. When a collect
is wanted while one runs, the board SHALL run one more once it ends,
however many were wanted meanwhile.

#### Scenario: Two outcomes while collecting

- **WHEN** the board is running `sluice inbox` for #13, and two
  overviews come meanwhile, each showing another of the session's
  outcomes
- **THEN** one more `sluice inbox` runs after the first ends

### Requirement: A collect sluice refuses is a toast in sluice's words

When `sluice inbox` exits 3 having printed some outcomes, the board
SHALL start the turn with what it printed and raise a toast holding the
first line it wrote to its error output. When it exits otherwise than 0
or 3, or cannot be started, the board SHALL start no turn and SHALL
raise a toast `sluice: couldn't collect this session's outcomes:
<words>`, the words the first line of its error output, or how it
exited where it wrote none.

#### Scenario: One branch could not be read

- **WHEN** `sluice inbox` prints #13's landing, writes `sluice: could
  not read bolt/x's records: it is gone` to its error output, and exits 3
- **THEN** one turn starts with #13's landing, and a toast says `sluice:
  could not read bolt/x's records: it is gone`

#### Scenario: A sluice with no inbox

- **WHEN** `sluice inbox` exits 2 having written `error: unrecognized
  subcommand 'inbox'`
- **THEN** no turn starts, and a toast says `sluice: couldn't collect
  this session's outcomes: error: unrecognized subcommand 'inbox'`
