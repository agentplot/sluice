## ADDED Requirements

### Requirement: A test that did not pass in a run the sandbox refused something records no result

After a run in which a test did not pass and its time limit did not
end, sluice SHALL read what the sandbox refused the run. Where it
refused anything, each test that failed, was failed by a task it
depends on, or was left without a result SHALL be not run, naming what
was refused, with nothing recorded under its key and nobody blamed. A
pass SHALL stand (114, 139, 149).

#### Scenario: A refused run

- **WHEN** a run of `main`'s combination has `a:test`, which fails, and
  `b:test`, which passes, and the sandbox reports refusing the run
  `network-outbound /private/var/run/mDNSResponder`
- **THEN** `b:test`'s pass is recorded under its key, `a:test` is listed
  as not run, saying sluice's sandbox refused the run a name lookup,
  `results.jsonl` gains no line for it, no submission is blamed, and the
  combination is tested again
  (`a_check_failed_in_a_refused_run_has_no_outcome_and_a_pass_stands` in
  `crates/sluice-conformance/tests/could_not_run.rs`)

#### Scenario: No refusal

- **WHEN** a test fails in a run the sandbox reports refusing nothing
- **THEN** the fail is recorded under its key and blamed, as before, as
  the next run of the combination in the scenario above is

#### Scenario: A test failed by a task the run was refused

- **WHEN** a required test is skipped for a task it depends on that
  failed, in a run the sandbox reports refusing something
- **THEN** neither the test nor the task has a result, and no line is
  recorded under the task's key
  (`a_refusal_leaves_a_check_failed_by_a_dependency_unjudged` in
  `crates/sluice-conformance/tests/failed_dependency.rs`)

#### Scenario: A refusal in every run

- **WHEN** the sandbox refuses every run of a combination something,
  and a test fails in each
- **THEN** after as many runs in a row as the deployment states, the
  combination is not passed and its session is told the test could not
  run, naming what the sandbox refused
  (`a_refusal_in_every_run_reads_not_passed_at_the_bound_naming_it`)

### Requirement: The sandbox's refusals are read only from its own report

sluice SHALL read what the sandbox refused a run only from the kernel's
reports of the run's own mark, kept as the run goes, never from what a
test printed, its exit or how it failed. It SHALL read them only after
a run in which a test did not pass and its time limit did not end, and
never for the open executor (38, 139; rule core.35).

#### Scenario: What the sandbox refused

- **WHEN** a sandboxed test writes outside its checkout and fails
- **THEN** sluice reads that the sandbox refused the run
  `file-write-create` of that path, naming the program that tried
  (`the_sandbox_reports_what_it_refused_a_run` in
  `crates/sluice/tests/checks.rs`)

#### Scenario: A test that prints a refusal

- **WHEN** a sandboxed test writes nothing outside its checkout, prints
  "Operation not permitted" and fails
- **THEN** sluice reads that the sandbox refused the run nothing, and
  the fail is recorded under its key
  (`a_sandbox_run_that_writes_nothing_outside_reports_no_refusal`)

#### Scenario: Two runs at once

- **WHEN** two sandboxed runs go at once on a host and only one is
  refused something
- **THEN** only that run reads a refusal
  (`two_sandbox_runs_at_once_report_only_their_own_refusals`)

#### Scenario: A passing run, or one its time limit ended

- **WHEN** every test of a run passed, or its time limit ended it, and
  the sandbox refused it something
- **THEN** its results are read as before, and what it was refused is
  never read
  (`a_run_that_passed_or_timed_out_never_reads_its_refusals` in
  `crates/sluice-conformance/tests/could_not_run.rs`)

### Requirement: The sandbox's reports are kept from before the run's command starts until after it ends

Each refusal rule of a sandboxed run's profile SHALL carry the run's own
mark, but for a write to `/dev/dtracehelper`, refused with no report.
The kernel's reports of that mark SHALL be kept in the run's directory
from before its command starts until a second after it ends. Keeping
them SHALL never hold the run back (139, C.2 sandbox row; record 0072).

#### Scenario: The command starts once the reports are kept

- **WHEN** a run's watch takes a while to start
- **THEN** its command starts only once the watch has written its first
  line, and the watch is ended after the command
  (`a_watch_is_ready_before_the_command_starts_and_ends_after_it` in
  `crates/sluice/tests/runs.rs`)

#### Scenario: A watch that never starts

- **WHEN** a run's watch exits before writing a line
- **THEN** the run's command runs all the same, and its directory keeps
  what the watch said
  (`a_watch_that_never_gets_ready_is_said_and_the_run_goes_on`)

#### Scenario: A run ended from outside

- **WHEN** a run is ended through its place
- **THEN** its watch is ended too (`ending_a_run_ends_its_watch`)

#### Scenario: Every refusal rule carries the mark

- **WHEN** a sandboxed run's profile is written
- **THEN** every refusal rule in it carries the run's mark, but the one
  for `/dev/dtracehelper`, which reports nothing
  (`each_refusal_of_the_profile_carries_the_runs_mark` in
  `crates/sluice/src/profiles/checks.rs`)

#### Scenario: The reports read

- **WHEN** the kept reports hold the stream's first line, a report of
  another mark, a duplicate report and five refusals of the run's mark
- **THEN** the run's refusals are the five, each once, named as the
  first three and 2 more, a refusal of the resolver's socket named as a
  name lookup
  (`the_runs_marked_reports_are_its_refusals` in
  `crates/sluice/src/profiles/refusals.rs`)

### Requirement: A report that cannot be read is said with the run

Where sluice cannot read what the sandbox refused a run, the run SHALL
be read as one refused nothing, and SHALL say `sluice could not read
what its sandbox refused the run:` with the words of what failed:
`sluice exec` after the run's lines, and a submission's status from the
branch's log (139).

#### Scenario: The log cannot be streamed

- **WHEN** the program that streams the kernel's reports exits at once,
  saying why
- **THEN** sluice reads the run's refusals as unread, with its words
  (`a_sandbox_run_whose_log_cannot_be_read_reports_it_unread` in
  `crates/sluice/tests/checks.rs`)

#### Scenario: Through the queue

- **WHEN** a test fails in a run whose refusals could not be read
- **THEN** the fail is recorded under its key and blamed, and the run's
  `detail` line in the log carries `refusals_unread` with the words
  (`a_run_whose_refusals_could_not_be_read_is_read_as_before_and_says_so`
  in `crates/sluice-conformance/tests/could_not_run.rs`)

#### Scenario: The words

- **WHEN** the words are `log: exited 1`
- **THEN** sluice says `sluice could not read what its sandbox refused
  the run: log: exited 1` (`an_unread_report_is_said` in
  `crates/sluice/src/words.rs`)

### Requirement: A switch, a refusal and an engine gone give one reason

After a run in which a test did not pass, sluice SHALL ask first whether
the host's system was switched, then what the sandbox refused, then,
where the policy asks for one, whether the container engine has gone.
The first that holds SHALL be the reason, and none after it SHALL be
asked (84, 114, 139; records 0074, 0097).

#### Scenario: A switch and a refusal

- **WHEN** a test fails in a run during which the host's system was
  switched and the sandbox refused something
- **THEN** the test is not run, saying the host's system was switched,
  and what the sandbox refused is not read
  (`a_switch_and_a_refusal_name_the_switch` in
  `crates/sluice-conformance/tests/could_not_run.rs`)

#### Scenario: A refusal and an engine gone

- **WHEN** `main`'s policy says `docker = true`, a test fails, the
  sandbox refused the run something, and the container engine no longer
  answers after it
- **THEN** the test is not run, naming what the sandbox refused, and
  the engine is not looked at
  (`a_refusal_and_an_engine_gone_name_the_refusal`)

### Requirement: sluice exec reads a refusal as a combination's run does

A session's run of the branch's tests SHALL read what the sandbox
refused it as a combination's run does, recording nothing for a test
left not run. A command after `--` SHALL never be read so (92, 139;
record 0074).

#### Scenario: A session's run refused something

- **WHEN** a session's run of the branch's tests has a test that fails
  and the sandbox refused the run something
- **THEN** the test is listed as not run, naming what was refused, and
  nothing is recorded under its key
  (`a_session_run_refused_something_records_no_fail` in
  `crates/sluice-conformance/tests/could_not_run.rs`)

#### Scenario: S66

- **WHEN** `sluice exec` runs a test that writes a folder the host does
  not declare, and then runs it again once the host's `[[caches]]`
  declares that folder
- **THEN** the first run lists the test as not run, naming the write the
  sandbox refused, and records nothing; the second records its pass
  under the wider grant, the only result under the test's key
  (`exec_records_nothing_for_a_test_the_sandbox_refused_and_its_pass_under_a_wider_grant`
  in `crates/sluice/tests/exec.rs`)
