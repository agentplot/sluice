---
number: 61
title: A read of the remote a round could not make is an event in the branch's log
status: accepted
date: 2026-10-26
decision-makers:
- Chuck Swanberg
---

# A read of the remote a round could not make is an event in the branch's log

Status: accepted by the user on 2026-10-27, on their card.

Deciders: madswan-design, on swancloud-design's ask of 2026-10-26, relayed by madswan-planner as signal 2026-10-26-madswan-planner-c13058ca/01; the user's yes is asked on the record's card.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 16, 61, 73, 121, 124, 128, 132, 133, C.7 pushed refs; ADRs 0017, 0053; rules core.11, core.14; `crates/sluice-core/src/records.rs` `Event` and `crates/sluice-core/src/step.rs` `decide` as origin/main held them on 2026-10-26 (ef6d414b).

## Context and problem statement

A branch on a remote is read by a fetch on every round (ADR 0017). When the fetch is refused, the round fails and lands nothing, and the failure reaches the process that ran the round: a waiting submit or `sluice wait` prints it and exits as sluice itself failed. The branch's log, which holds every other thing that happened to its queue, submitted, testing, ran, result, landed, held, copy-behind, records nothing of it. On 2026-10-26 swancloud-ops read a branch's `main/log.jsonl` for half an hour while every round's fetch was refused and found no trace of why the queue stood still; the refusal was in one waiting submitter's output and nowhere else. swancloud-design asked that a fetch the round could not make be an event in the queue log, with the remote's words, as a result is, so the queue's stalls can be read from the log by whoever reads it after.

Where is a refused read of the remote recorded, how often, and what of a branch whose records are themselves on the refusing remote?

## Considered options

1. In the branch's log, as clause 133 has it: once per distinct cause and host while the refusal stands, with the remote's own words, and its end logged when the host's read succeeds again, with how long it stood and how many rounds it refused; where the records are on the remote, the host keeps the two in its own state, as it keeps its observation of a lead (73), and writes them in its first write after.
2. In a log of the host's own, beside the branch's records on that host, and not in the branch's log.
3. Nowhere but the output of the process that met it, which is what was built.

## Decision outcome

Option 1.

**The branch's log is where the queue's history is.** A reader of the queue reads one log (61, 132), and a stall is a thing that happened to the queue as a held branch or a copy left behind is (121, 124); both of those are already logged, once per distinct cause, and the refused read follows their shape. The remote's words go in whole and unchanged, as a run's result carries its tool's words (16), since a reader who was not there can act only on what the remote said.

**Once per cause and host, with its end.** A round runs every few seconds while a submission waits; a line per refused round would bury the log. One line opens a refusal, one closes it, and the pair says how long the queue stood and how many rounds it refused. The host's name is on each, since a refusal is between one host and the remote: another host's rounds may be reading the same branch unhindered.

**A remote that refuses the records too.** On a branch whose records are on its remote, a refused fetch of the branch is most often a refused fetch of the records as well, and nothing can be written to the log until the remote answers. The host keeps the refusal's first time, its words and its count in its own state, which is where it already keeps what it observed of a lead (73), and writes the opening and the closing line in the first write it can make. Rule core.14 holds: nothing is decided from the kept observation; it is carried into the log.

Option 2 leaves a reader on any other host, and the overview and the site that read the branch's log, with no sign of the stall, and makes two logs where the statement has one (61). Option 3 is the half hour swancloud-ops spent.

### Consequences

- The log gains two events, a refused read's opening and its closing, each naming the host, the remote, what was asked for and the remote's words; readers of `sluice/log@1` that do not know them pass over them as they pass over `copy-behind`.
- `sluice status <branch>` shows an open refusal of this host's above the queue, as it shows a copy behind.
- slc-2's chore, which runs a refused read once more on a fresh credential, comes first: the refusal logged is the one that stands after it.
- Rule core.28 is drawn from this record.
