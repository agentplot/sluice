---
number: 47
title: The host counts its running checks once
status: accepted
date: 2026-10-23
decision-makers:
- Chuck Swanberg
---

# The host counts its running checks once

Deciders: the operator, after three repositories' queues ran on one host at once beside 44 agent sessions and its load reached 95 on 18 cores, 2026-10-23.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clause 71 and S60 as changed at `807a7e5`, clauses 83, 91, 102, 107; rules core.2, core.7, core.11, core.14, core.18; ADRs 0012, 0018, 0035; the change `the-host-counts-its-running-checks-once`.

## Context and problem statement

ADR 0012 left the cap on concurrent runs to the instance that starts them, counted from the state of the branches it serves, and built nothing for independent processes to agree on a count. Each repository's sluice therefore counted only its own runs: three repositories on one host started up to three times the bound, each run given a share of the cores as if it were alone. Clause 71 now says the bound is the host's, one count shared by every instance of sluice on it whatever repositories and branches it serves, kept where every instance reads it. Where is that count kept, and how does a start, by any repository's round, an agent's `sluice exec` or a preview, take its part of it without two starts both taking the last free run?

## Considered options

1. Hold the session runs' lock, `$XDG_STATE_HOME/sluice/sessions/.lock`, across every start, as `sluice exec` did, and count the session runs and every repository's runs under it.
2. Keep a list of the host's repositories in its state, and count the runs under test in each one's records.
3. A cgroup, or another kernel slot, per run.
4. A registry of running checks in the host's state, beside its blocks of ports: a file per admitted run, counted while the process that admitted it holds a lock on the run's own hold file or a supervisor holds the run's directory, admitted under a lock held only across counting and writing that file. The check seam's `start` admits the run before it runs anything, and refuses at the bound with a typed error.

## Decision outcome

Option 4. `$XDG_STATE_HOME/sluice/running/` holds `.lock`, and for each admitted run `<run dir, flat>.json` (its kind, queue or session, its repository, place, run directory, starter and when it was admitted) and `<run dir, flat>.hold`. A starter takes `.lock`, counts the live runs, locks its hold, writes its file and lets `.lock` go; then it sets the run up and starts it, which may take minutes, and once the run's supervisor holds the run directory it lets the hold go. A file neither held nor supervised, a run that ended or a starter that died, counts for nothing and the next admission removes it. A submission's run is admitted under the bound, a session's or a preview's under the bound less `[runs] reserved` (91, 102); nothing else, no load or use of the host, decides.

`Tests::start` admits the run before the toolchain, `[tests] ready` or the runner's setup, and at the bound fails with `sluice_seams::AtBound`, which the core finds by downcasting and treats as no run: the checkout it made is released and the run is open again for a later round, its claim given up on a branch whose records are on its remote. `Tests::running` replaces `Tests::sessions` and reports the host's live runs by kind, from which the core's capacity, the overview's `room` and a host's sign of life are read.

Option 1 makes every repository's round wait while another worktree's toolchain is set up. Option 2 reads records that may live on a remote (rule core.11) and still lets two rounds decide on the same last run. Option 3 is unavailable on macOS, where the host that prompted this runs, and holds a run's cores without counting runs. A separate `admit` method the core calls before `start` would hand the core a token to keep across a seam call and leave it to know when the run's supervisor took over; inside `start` that knowledge stays with the profile that owns the run directory (rule core.2).

### Consequences

- Supersedes ADR 0012's consequence that the core starts no more runs than its bound less the combinations under test in the state of the branches the instance serves: the core subtracts every check running on the host, every repository's of both kinds, and the runs it has marked starting and not yet started. Its executors and limits stand.
- Instances that each step their own branch, or serve different repositories, share one bound; `parallel` is no longer the only limit between them.
- A run whose checks ended stops counting before its round reads its result, though its checkout may still hold ports until it is released; ports are their own count.
- A start refused after its place was made wastes a checkout; it happens only when two starters decide at the same moment, since the core checks the count again before it makes the checkout.
- The bound's default, a quarter of the cores, is now shared by every repository on the host, so a host with no `[runs] max` runs fewer checks at once than before.
- A run started by a sluice built before this decision has no entry and is not counted while both run on one host, until the old rounds' runs end.
- `sessions/.lock` guards only the session runs' own records (ADR 0035); the count a session run starts under is the registry's.
