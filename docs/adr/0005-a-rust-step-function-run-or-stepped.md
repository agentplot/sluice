---
number: 5
title: A Rust core that is one step function, run as a loop or stepped by a driver
status: accepted
date: 2026-10-08
decision-makers:
- Chuck Swanberg
---

# A Rust core that is one step function, run as a loop or stepped by a driver

Deciders: the user, in the requirements interview of 2026-10-08.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 26, 46, 51, 52, S10 and section 9; ADR 0003.

## Context and problem statement

Checks take minutes, so something must keep advancing a line while they run: forming candidates, starting checks up to the window, reading verdicts, attributing and landing. S10 speaks of sluice being stopped, which implies a process, while clauses 26 and 51 require every state to be readable with none running. What advances a line, and what is sluice built in?

## Considered options

1. A daemon per line that drives its chains and is the only writer.
2. No resident process: each call advances the line as far as it can and exits, with drivers looping.
3. One core that is a step function over the records, with two ways in: `sluice run`, a loop around it, and `sluice step`, one pass for a driver that already loops.

## Decision outcome

Option 3, in Rust. The core reads a line's records and the seams' answers, decides the next actions, performs them and writes the records, and holds nothing between steps. A step takes the line's lock, a file under the line's record directory, so two steppers never act on one line at once; a stepper that finds the lock held leaves. `sluice run` is what a team with only worktrunk uses; crew's conductor and flywheel's effects call `sluice step`. Because every state is in the records (26), stopping either is safe and starting either continues where the records stand (S10).

Option 1 needs a supervisor, which is a service manager the core may not name (core.1), and makes a driver that already loops run a second loop. Option 2 leaves a finished check with nothing to advance the line unless something calls back, which is option 3 without the loop.

### Consequences

- Rule core.14: nothing the core decides is held between steps outside the records.
- `sluice run` is started by the operator or the driver, never by the core, so core.10 is untouched.
