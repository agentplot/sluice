## MODIFIED Requirements

### Requirement: A take-up says which tables changed

For each change it takes up, `sluice run` SHALL say once on stderr the
file and the tables whose settings changed, in the order the
configuration reference gives them: `[runs]`, `[forge]`, `[signing]`,
`[peers]`, `[deliveries]`, `[telemetry]`, `[[caches]]`. It SHALL never
say it took up a change that names no table. Settings equal to those it
holds SHALL be taken up silently, with nothing bound afresh.

#### Scenario: The run says what it took up

- **WHEN** the bound grows from one to two while `sluice run` is kept
  running
- **THEN** stderr says exactly once `sluice: took up the host's settings
  in <path>: [runs] changed`
  (`a_run_takes_up_a_changed_bound_at_its_next_pass` in
  `crates/sluice/tests/instance.rs`)

#### Scenario: Which tables changed

- **WHEN** the settings taken up differ from those held in `[runs]
  max` and `[forge] discovery` alone
- **THEN** the tables named are `[runs]` and `[forge]`, in that order
  (`the_tables_a_change_moves_are_named` in
  `crates/sluice/src/instance.rs`)

#### Scenario: A changed telemetry table is named

- **WHEN** the settings taken up differ from those held in
  `[telemetry] collector` and `[[caches]]`, or in `[telemetry] logs`
  alone
- **THEN** the tables named are `[telemetry]` and `[[caches]]`, in that
  order, or `[telemetry]` alone
  (`a_changed_telemetry_table_is_named_and_taken_up_afresh` in
  `crates/sluice/src/instance.rs`)

## ADDED Requirements

### Requirement: A change no binding takes up in place starts the run afresh

Where the tables a change moves include one no binding takes up in
place, `[telemetry]` alone of today's, `sluice run` SHALL take the
change up between two passes by starting itself afresh in its own
process (135; record 0107; rule core.54). Before it does, it SHALL
bind the new settings and check the host's systems under them, as any
take-up does, and send what its telemetry holds to where it was
sending. Every other change SHALL be taken up in place, as before.

#### Scenario: Which tables start the run afresh

- **WHEN** a change moves `[telemetry]`, alone or with other tables, or
  moves `[runs]` and `[forge]` alone
- **THEN** the first starts the run afresh and the second is taken up
  in place
  (`a_changed_telemetry_table_is_named_and_taken_up_afresh` in
  `crates/sluice/src/instance.rs`)

#### Scenario: A kept run sends to the collector it is changed to

- **WHEN** the host's settings say `[runs] max = 1` and `[telemetry]`
  with `send = true` and one collector, `sluice run` is running with
  two submissions queued on `bolt/a`, and the file is changed to name
  another collector while the first combination's run is going
- **THEN** the first collector is sent traces and metrics, the second
  collector is sent traces, and both submissions land
- **AND** stderr says exactly once `sluice: took up the host's settings
  in <path>: [telemetry] changed, in this sluice run started afresh`
- **AND** each combination's run started once, given every core sluice
  may use
  (`a_run_takes_a_changed_telemetry_table_up_by_starting_afresh` in
  `crates/sluice/tests/instance.rs`)

### Requirement: The run started afresh is the run as it was started

The process a `sluice run` starts afresh SHALL be the same process,
keeping its id, its output and whatever runs it. It SHALL run the
executable the run started as, resolved through every link as the run
started, with the command line the run was given, the folder it ran
in, and the environment it started with before telemetry took anything
out of it (151; rule core.53).

#### Scenario: The hand-over's command

- **WHEN** a run that started as `/x/sluice` with `run --rounds 12` and
  an environment holding `OTEL_EXPORTER_OTLP_ENDPOINT` hands over, with
  that variable since taken out of its own environment
- **THEN** the command it starts afresh is `/x/sluice` with `run
  --rounds 12`. Its environment is cleared and set to exactly the one
  it started with, the variable among it, and the hand-over's document.
  That document, holding settings, tables, passes, a driver's
  descriptor, supervisors and what was said, reads back as it was
  written
  (`the_run_started_afresh_is_the_one_that_started` in
  `crates/sluice/src/hand_over.rs`)

### Requirement: The driver stays the driver across a hand-over

A `sluice run` that hands over SHALL hold its repository's driver's
lock without a moment's let-go, so no wait on the host runs rounds and
no other `sluice run` takes the repository (135; rule core.31). The run
started afresh SHALL count the passes already driven toward `--rounds`.
It SHALL not say again a hold, a refusal, or that another `sluice run`
drives the repository, where the run said it before.

#### Scenario: A driver's lock handed over

- **WHEN** a driver's lock is handed over as a descriptor open across a
  start, and taken up again from it
- **THEN** the handed descriptor is not closed on a start, and the lock
  taken up from it is; a probe finds the driver live throughout; and
  once the lock taken up is dropped, a new driver takes it
  (`a_driver_handed_its_lock_holds_it` in `crates/sluice/src/reader.rs`)

#### Scenario: A run handed over counts its passes

- **WHEN** a `sluice run --rounds 3` over `main` drives one pass, hands
  over what it carries, and drives on from what it handed over
- **THEN** it ends after two passes more, and between every two passes
  a probe finds the repository's driver live
  (`a_run_handed_over_keeps_driving_and_counts_its_passes` in
  `crates/sluice/tests/cli.rs`)

### Requirement: The run started afresh starts on the settings handed over

The run started afresh SHALL start on the settings the run handed over,
never on a reading of the file made as it starts. Its first look SHALL
read the file once. Settings equal to those handed over SHALL be taken
up silently. A change made meanwhile SHALL be taken up as any change
is. A file that does not read SHALL leave it on the settings handed
over, said once (rule core.54).

#### Scenario: The first look after a hand-over

- **WHEN** a run holds `[runs] max = 3` handed over, and the file holds
  `max = 3`, then `max = 4`, then `max = "two"`
- **THEN** the first look finds nothing to take up, the second finds a
  bound of four, and the third refuses the file naming `max` and holds
  a bound of four
  (`a_run_handed_over_holds_what_it_was_handed_and_reads_the_file_once`
  in `crates/sluice/src/host_settings.rs`)

### Requirement: A hand-over that cannot start leaves the run as it was

Where the run's settings do not bind, or its process cannot start
afresh, `sluice run` SHALL keep the settings, the binding and the
telemetry it holds. It SHALL say on stderr once for that change the
file, the tables, and why. It SHALL read the file again once it changes
again, and SHALL never end for it (rule core.54).

#### Scenario: An executable that cannot start

- **WHEN** a hand-over's command names an executable that is not there
- **THEN** starting it answers the system's words, naming the
  executable, and the driver's descriptor it would have handed over is
  closed
  (`a_hand_over_that_cannot_start_says_why` in
  `crates/sluice/src/hand_over.rs`)

#### Scenario: A start that fails leaves the process as it was

- **WHEN** a hand-over's start fails while another thread of the
  process reads its environment
- **THEN** that thread never reads the environment handed over, and
  `SIGPIPE` keeps its disposition
  (`a_hand_over_that_cannot_start_leaves_the_process_as_it_was` in
  `crates/sluice/src/hand_over.rs`)

#### Scenario: Settings it would start afresh for that do not bind

- **WHEN** the host's settings say `[runs] max = 1`, `sluice run` is
  running with two submissions queued on `bolt/a`, and the file is
  changed in `[runs]`, `[forge]` and `[telemetry]`, with a `[forge]` key
  that holds no private key, while the first combination's run is going
- **THEN** the second combination's run starts only once the first has
  ended, with the cores the first had, and both submissions land
- **AND** stderr says exactly once `sluice: warning: <path> changed
  [runs], [forge], [telemetry], which this sluice run takes up only by
  starting afresh, and its settings do not bind, so it keeps the
  settings it holds until the file changes: <why>`, naming the key, and
  nothing is taken up
  (`a_run_keeps_its_settings_when_a_change_it_starts_afresh_for_does_not_bind`
  in `crates/sluice/tests/instance.rs`)

### Requirement: Runs started before a hand-over are reaped

The run started afresh SHALL reap every run supervisor the run started
before handing over and had not reaped. It SHALL keep reaping them
across any later hand-over, so no supervisor is left unreaped under a
kept `sluice run`.

#### Scenario: A supervisor handed over

- **WHEN** a child of the process ends, its id handed over as a
  supervisor not yet reaped
- **THEN** it is reaped, and no longer named among the supervisors not
  yet reaped
  (`a_supervisor_handed_over_is_reaped` in `crates/sluice/src/runs.rs`)
