## ADDED Requirements

### Requirement: A run's time limit holds whatever became of what started it
A combination's run SHALL be ended with everything it started when its
branch's time limit passes, by the supervisor that holds the run, whether
or not the step, wait or driver that started it still runs, and the
host's next read of the branch SHALL record it timed out (66; rule
core.10).

#### Scenario: The step that started it has exited
- **WHEN** `sluice step main` starts a combination's run under a one-second limit and exits, and the run's test sleeps for five minutes
- **THEN** within the grace after the limit the run's processes are gone with no sluice command running, and the next `sluice step main` records the run timed out
