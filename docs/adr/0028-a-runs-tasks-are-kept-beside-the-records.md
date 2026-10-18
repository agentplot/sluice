---
number: 28
title: A run's tasks are kept beside the records, on the host that runs it
status: accepted
date: 2026-10-18
decision-makers:
- Chuck Swanberg
---

# A run's tasks are kept beside the records, on the host that runs it

Deciders: the operator, planning the board's and the herdr popup's task progress, 2026-10-18.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 16, 26, 37, 38, 61 to 63, C.2; `design/sluice/models/many-hosts-1.md` §1.2; flywheel-next `design/storage/storage.md` rule 1, decisions 2, 7 and 8; rules core.2, core.7, core.9, core.11, core.14; ADRs 0004, 0008.

## Context and problem statement

While a combination is under test, the operator wants each of its tasks' states: waiting, running, passed, failed or replayed. moon writes what it knows as each task ends, in the checkout and in the run's output. The overview reads only the records, through their seam (rule core.11). Where should a run's tasks be kept, and who writes them?

## Considered options

1. The round writes them onto the candidate in `branch.json`.
2. The round writes them as a record of their own beside the branch's records, through the records seam, on the host that runs the combination.
3. The overview asks the check seam for them, read from the run's directory.

## Decision outcome

Option 2. The round already asks the check seam after each run under test, under the branch's lock. Before it writes, it asks the check seam for the tasks of each run still under test (`Tests::tasks`) and keeps exactly what the runner says now (`State::put_progress`). The local files profile writes them under `<common>/sluice/.progress/<branch>/`, a directory no branch's records can name. When a combination is no longer under test, its tasks are removed. No verdict, gate, blame or result reads them (rules core.7, core.9). They change no branch's version.

Option 1 puts a field that is no result into the records a verdict is read from, and moves the branch's version at every task's end. Once the records are shared under `refs/sluice/state/<branch>`, each of those writes is a push with a lease, which many-hosts-1 §1.2 costs at 0.6 to 1.5 s, against a budget of about 8 writes per entry. Clause 63 would also make every host read the tasks, which publishes them before anyone has decided they should travel. Option 3 has the overview read something other than the records (rule core.11), and the overview change turned down reading the run's directory.

The family's storage model keeps a piece of state in the lowest layer that holds every reader it has, and keeps run status with the host. A run's tasks have readers only on the host that runs it today, so that is where they stay.

### Consequences

- `sluice overview --json` gives a combination under test `tasks[]`, `{target, state, started?, ended?, exit?}`, while a round has kept them.
- The tasks are as fresh as the rounds: every 3 s under a waiting `wt submit` or `sluice run`.
- A host shows only the tasks of the runs it runs. Whether a shared records profile carries them, as each host's own side record the way many-hosts-1 carries `refs/sluice/hosts/<host>`, is for that profile's change to decide.
- Rule core.11's and core.7's Details say the tasks are not ruled out.
- The records seam has two more methods, and the check seam one, for every profile and the fakes to implement. The fakes and the local files profile are each tested on them.
