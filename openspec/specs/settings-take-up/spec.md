# settings-take-up Specification

## Purpose
How a `sluice run` kept running takes up a change of the host's own
settings, `~/.config/sluice/config.toml`, between its passes and never
within one, so nothing on the host restarts it (135; record 0107; rule
core.54). It tells a change by the file's sign alone, keeps what it
holds where the file does not read or the settings do not bind, said
once, and leaves a run already started on what it started with.

## Requirements

### Requirement: A kept sluice run takes up changed host settings before its next pass

A `sluice run` that is running when the host's settings file changes
SHALL take up the settings the file then holds before its next pass,
whether or not it drives the repository, between passes and never
within one, so every round of a pass and the reader's document it
writes act under one set of settings. Nothing on the host SHALL need to
restart it for a change to count (135; record 0107; rule core.54).

#### Scenario: The bound grows while a run is kept running

- **WHEN** the host's settings say `[runs] max = 1`, `sluice run` is
  running with two submissions queued on `bolt/a`, whose policy tests
  four at once, and the file is changed to `max = 2` while the first
  combination's run is going
- **THEN** the second combination's run starts while the first is still
  going, and is given the cores sluice may use divided by two, and both
  submissions land
  (`a_run_takes_up_a_changed_bound_at_its_next_pass` in
  `crates/sluice/tests/instance.rs`)

#### Scenario: A file written after the run started

- **WHEN** the run started with no settings file, and so with every
  default, and a file naming `[runs] max = 3` is then written
- **THEN** the next look finds it changed and reads it, and the settings
  it gives have a bound of three
  (`a_changed_file_is_read_once_and_the_same_settings_change_nothing` in
  `crates/sluice/src/host_settings.rs`)

#### Scenario: The same settings written again

- **WHEN** the file is written again with the settings the run holds,
  as an apply of the host's configuration that changed another tool's
  file does
- **THEN** the next look finds nothing to take up
  (`a_changed_file_is_read_once_and_the_same_settings_change_nothing` in
  `crates/sluice/src/host_settings.rs`)

### Requirement: A take-up says which tables changed

For each change it takes up, `sluice run` SHALL say once on stderr the
file and the tables whose settings changed, in the order the
configuration reference gives them: `[runs]`, `[forge]`, `[signing]`,
`[peers]`, `[deliveries]`, `[[caches]]`. Settings equal to those it
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

### Requirement: A look at a file that stands reads nothing

Between passes, `sluice run` SHALL tell whether the settings file
changed from the file's sign alone, with no process started and no git
read (rule core.31). It SHALL read the file only once the sign differs
from the one taken as its settings were last read. A read during which
the sign moved SHALL count as no read, and the file SHALL be read again
at the next look.

#### Scenario: An unchanged file made unreadable

- **WHEN** the run holds the file's settings, and the file's
  permissions are then taken away with its contents, time and size
  unchanged
- **THEN** the next look finds nothing changed, and nothing is said
  (`a_look_at_a_file_that_stands_reads_nothing` in
  `crates/sluice/src/host_settings.rs`)

### Requirement: The sign takes in a link and the file it reaches

The file's sign SHALL be the modification time and size of the file's
own entry, the path the file resolves to through every link, and the
modification time and size of the file found there, so a link moved
into a store whose files all share one time is seen.

#### Scenario: A link moved to a store file of the same size and time

- **WHEN** the settings file is a link to a file whose modification time
  is one second past the epoch, as Nix's store gives every file, and the
  link is moved to another such file of the same size holding other
  settings
- **THEN** the next look finds it changed and gives the other settings
  (`a_link_moved_to_a_file_of_the_same_size_and_time_is_a_change` in
  `crates/sluice/src/host_settings.rs`)

### Requirement: A file that does not read leaves the run on what it holds

Where the changed settings file is gone, cannot be read, or holds
settings sluice refuses, `sluice run` SHALL keep the settings it holds,
and SHALL say on stderr, once for that change, the file and why. A gone
file SHALL be read as one that does not read, never as sluice's
defaults. The run SHALL read the file again once it changes again, and
SHALL never end for it (record 0107; rule core.54).

#### Scenario: A file sluice refuses, and then mends

- **WHEN** the run holds a bound of two, and the file is changed to
  `[runs] max = "two"`, then looked at again with no change, and then
  changed to `max = 3`
- **THEN** the first look says the file is refused, naming it and why,
  and keeps a bound of two; the second finds nothing changed and says
  nothing; and the third gives a bound of three
  (`a_file_that_does_not_read_leaves_the_settings_held_and_is_said_once`
  in `crates/sluice/src/host_settings.rs`)

#### Scenario: A gone file

- **WHEN** the file the run took `[runs] max = 3` up from is removed
- **THEN** the next look says the file is gone and that an empty file
  gives sluice's defaults, and keeps a bound of three
  (`a_file_that_does_not_read_leaves_the_settings_held_and_is_said_once`
  in `crates/sluice/src/host_settings.rs`)

### Requirement: Settings that do not bind leave the run on what it holds

Where the changed file reads but its settings cannot be bound, such as
a key that does not parse or systems a branch's policy gives otherwise,
`sluice run` SHALL keep the settings and the binding it holds, and SHALL
say on stderr, once for that change, the file and why (rule core.54).

#### Scenario: A key that holds no private key

- **WHEN** the host's settings say `[runs] max = 1`, `sluice run` is
  running with two submissions queued, and the file is changed to
  `max = 2` with a `[forge]` table whose key, mode `0600`, holds no
  private key
- **THEN** the second combination's run starts only once the first has
  ended, given the cores the first was, and both submissions land
- **AND** stderr says once that the file is refused and the run keeps the
  settings it holds, naming the file and the key
  (`a_run_keeps_its_settings_when_the_new_ones_do_not_bind` in
  `crates/sluice/tests/instance.rs`)

### Requirement: A run keeps the settings it started under

A run that started before `sluice run` took up changed settings SHALL
go on with the cores, caches and grant it was started with. No run SHALL
be stopped or started again because the settings changed, and its
result SHALL count as it would have (rule core.54).

#### Scenario: The first run keeps its cores

- **WHEN** the bound grows from one to two while the first combination's
  run is going
- **THEN** that run was given all the cores sluice may use, keeps them
  to its end, is not started again, and its submission lands
  (`a_run_takes_up_a_changed_bound_at_its_next_pass` in
  `crates/sluice/tests/instance.rs`)

### Requirement: The driver stays the driver across a take-up

A `sluice run` that takes up changed settings SHALL stay its
repository's driver and reader on the host, and SHALL carry to the next
pass what it carries between passes: the passes counted toward
`--rounds`, the holds and refusals it has already said, and whether it
said that another `sluice run` drives the repository.

#### Scenario: A pass under a new binding

- **WHEN** a `sluice run` has driven one pass of `main` with #1 queued,
  and its next passes run under a binding made afresh
- **THEN** the repository's driver is held throughout, and #1 lands
  (`a_run_keeps_driving_across_a_new_binding` in
  `crates/sluice/tests/cli.rs`)
