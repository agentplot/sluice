---
number: 89
title: A host's waits hold its intake mark together, and the last of them to end lets it go
status: accepted
date: 2026-11-06
decision-makers:
- the code stage of fix/a-hosts-intake-never-withdraws-anothers-submission
- the user, on the amendment of 137
---

# A host's waits hold its intake mark together, and the last of them to end lets it go

Status: accepted with the amendment of 137 that says it, which the user approved on 2026-11-06 and which is on agentplot/blueprints main at 421c77e. It reads 137's "a wait that holds it lets it go when it ends" for a host with several waits at once, and is built here on that reading.

Superseded by record 0093, proposed on 2026-11-06, once it is accepted: no host takes a branch's requests in, so no intake mark is kept, renewed or handed over (137).

Deciders: the code stage of `fix/a-hosts-intake-never-withdraws-anothers-submission`; the user, on the amendment.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 73, 135 and 137, at c94c496, and 137 as main words it at 421c77e; rule core.33; record 0088; chuck-herdr-alpha's records of WilldanGroup/switchboard-kit's `main` for 2026-11-06 and its processes, crew's `sluice wait <id> --timeout 9m --json` run again while it times out, one per landing; `crates/sluice/src/profiles/github/pulls.rs` (`held`, `let_go_marks`) as main held it at b2263f63; `crates/sluice/src/host_count.rs`, whose runs are counted by files locked while they live.

## Context and problem statement

137: the mark names the host and whether its holder is the host's driver or a wait; "a wait that holds it lets it go when it ends"; and "a branch one host reaches with no driver, by waits and steps alone, keeps no mark between them". On a host with no driver, crew runs one `sluice wait` per landing, each run again every nine minutes while the landing is undecided, so several waits of one host read a branch's requests at once. Each held the mark as its own: the one that pushed it, or any that took the requests in under it, let it go when it ended, though its host's other waits still took them in. Between that and another of them pushing it again, the branch stood with no mark while its host still waited, open to another host's wait, and a hand-over of everything the host had taken in. On chuck-herdr-alpha the drop also came between another wait's fetch and its renewal, which record 0088 settles. Which of a host's waits lets the mark go?

## Considered options

1. **The host's waits hold it together, and the last of them to end lets it go** (chosen). Each wait that reads the branch's requests keeps a hold among the host's waits from its first pass, a file of the host's state named for its process and locked while it lives, under a lock over them. Ending, it lets its hold go, and removes the mark only where no other wait's hold is locked, and the mark, as the repository last read or wrote it, names its host and a wait; by a push leased on that value, as before. A hold whose process is gone reads as gone and is removed. The holds are the host's, by the repository's address on the forge, as the mark is.
2. Each wait lets go what it holds, as before. The branch keeps no mark between a host's waits, but also none while its host still waits.
3. A wait never lets go; the mark lapses after the lease. A host that stopped waiting holds the branch a lease longer, against "keeps no mark between them".
4. The intake lock held over the let-go. Keeps a drop out of another wait's pass, but not out of the time between its passes.

## Decision outcome

Option 1, on 137's reading that the host holds the mark and a branch keeps none between waits, so none while one of its host's waits still runs. The amendment of 137, on agentplot/blueprints main at 421c77e, says so: "a host's waits hold it together, the last of them to end letting it go".

### Consequences

- The mark stands from a host's first wait to its last, whatever the waits' order, and a wait that never pushed it lets it go when it is the last.
- A wait killed outright leaves its hold unlocked, read as gone; the mark then goes with the host's last wait, or lapses after the lease where none is left to let it go, as before.
- Two copies of one repository on one host share the holds, as they share the mark (record 0085's D5).
