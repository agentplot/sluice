## ADDED Requirements

### Requirement: A check that failed on what the sandbox refused records no result
After a sandbox run in which a check did not pass, sluice SHALL read the
sandbox's own report of what it refused that run. When it refused
anything, each check that did not pass SHALL be not run, with what was
refused as the reason, and SHALL have no result under its key; a check
that passed SHALL keep its pass (114, 139; rule core.35).

#### Scenario: A pseudo-terminal refused
- **WHEN** a run's profile refuses `/dev/ptmx` and the tests that open one fail
- **THEN** `results.jsonl` gains no line for them, and each is listed as not run with "the sandbox refused" and `/dev/ptmx`

#### Scenario: The other tests pass
- **WHEN** in that run another test passes
- **THEN** its pass is recorded under its key

#### Scenario: No refusal
- **WHEN** a sandbox run's test fails and the sandbox reports no refusal for the run
- **THEN** the test's fail is recorded under its key, as before

### Requirement: A refusal is the sandbox's word, never the test's
sluice SHALL read a refusal only from the executor's own report, never
from what a check printed, its exit or how it failed, and SHALL read the
report only after a run in which a check did not pass, once, keeping
what it read with the run (38, 139; rule core.35).

#### Scenario: A test that prints a refusal
- **WHEN** a test fails printing "Operation not permitted" and the sandbox reports no refusal
- **THEN** the fail is recorded under its key

#### Scenario: A passing run
- **WHEN** every check of a sandbox run passes
- **THEN** sluice reads no report of refusals for it

### Requirement: A report that cannot be read is said with the run
When sluice cannot read the sandbox's report of its refusals, the run
SHALL say so in the report's own words, and SHALL be read as a run with
no refusal (139).

#### Scenario: The log refused
- **WHEN** the unified log cannot be read after a failing sandbox run
- **THEN** the fail is recorded under its key and the run says sluice could not read the sandbox's refusals, with the log's words
