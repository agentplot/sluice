---
number: 48
title: A branch read without its last landing is held
status: accepted
date: 2026-10-23
decision-makers:
- Chuck Swanberg
---

# A branch read without its last landing is held

Deciders: the user, after a bolt's round formed on a remote's older copy and dropped two landings, 2026-10-23.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 10, 17, 22, 24, 26, 45, 104, 105, 121, C.4 pushed refs, C.5 branch, S8, S57; ADRs 0005, 0017, 0026, 0046; rules core.2, core.4, core.6, core.11, core.14, core.15.

## Context and problem statement

On 2026-10-23 sluice landed two submissions on a bolt in the host's own refs, the second bringing the policy that names the bolt's remote. The next round read the bolt from the remote, where a refused landing had left an older copy. It logged `moved`, formed the next submission on that copy and landed it, and the two landings were gone from the branch. A round took any revision it had not moved the branch to as a hand's move to form on (S8), whether the branch had gone forward or back.

Clause 121 says a revision read that lacks the last landing sluice recorded is a branch whose reading and records disagree: nothing is formed or landed on it, and it is reported, until the landing is back in the revision read or the operator accepts the revision read. How does a round know what the branch must contain, how is the hold kept and said, and how does the operator accept?

## Considered options

1. The branch's document keeps the one revision every read must contain; a round that reads a revision lacking it holds the branch, recorded in the document and once in the log; `sluice branch accept` is the operator's acceptance.
2. Each round scans the branch's log for the last `landed` line and fails the round with an error when the read lacks it.
3. Each round reads the submissions' `landed_as` for the newest landing and forms nothing on a read that lacks it, moving `base` to the read.

## Decision outcome

Option 1.

**One revision the branch must contain.** `branch.json` gains `landed`, the revision every read of the branch must contain (121). A confirmed landing sets it to the landing's revision, in the write that logs `landed`. `sluice branch accept` sets it to the revision accepted, and `sluice branch create` to the revision it made the branch at. Records written before this have no `landed`: the first round takes the last `landed` line's revision, or `base` when `base` lacks it. That second case is a branch removed and made again, whose log still holds the removed branch's landings, which 121 says bind no reading of the new one.

**The hold is a field and one log line.** `branch.json` gains `held`: the revision read, the landing it lacks, and since when. In the round's decide step, after the branch is read and before `base` is touched, a read that lacks `landed` sets `held`, logs `held` once for each revision read, writes, and ends the round as `held`. Nothing is polled, formed, started or landed. A read that contains it clears `held` and goes on as before, a hand's commit on top formed on as S8 says. `base` stays at the last revision formed on, so a hold ended by the landing coming back keeps every combination and its runs (10, 17). The check sits in the core, above the branch seam, so local refs and pushed refs hold a branch the same way (rule core.2). The hold is in the branch's records, read by every reader with no sluice running (rule core.11), and nothing of it is held in a round's memory (rule core.14).

**Runs under test are left alone.** A held round neither polls nor releases. A run under test goes on, and the first round after the hold reads its result. When the landing comes back, the result counts as it would have; when the operator accepts the read, the round sees the branch moved, logs `moved` and forms again (S8), releasing the run's place as for any combination formed again.

**`sluice branch accept <branch> <revision>`.** Beside `create` and `delete` (C.5 branch; ADR 0026), under the branch's lock: exit 1 when the branch does not exist, 2 when it is not held or the revision given, a prefix as git takes one, is not the one it is held at. Otherwise it sets `landed`, clears `held` and logs `accepted` with the revision and the landing it lacked. It moves no branch and touches no combination; the next round forms on the accepted revision. It is the operator's act and no session's: no round runs it, so no waiting driver does (rule core.4).

**Making a branch sets the revision it stands on.** `make_branch` writes `landed` as the revision made and clears `held` once its move succeeds; a refused make writes nothing. `remove_branch` clears `held` once its move succeeds, so no reader sees a hold on a branch that is gone (104, 105, 121).

**How it is said.** `sluice step` says `held` with both revisions and the two ways out, and `--json` carries the outcome `held` and `held: {read, lacks}`. `sluice run` says it once per hold. `sluice status <branch>` and `sluice overview` show it above the queue and under `held` in JSON. A waiting `sluice submit` or `sluice wait` prints it once in its text, and its JSON record, the submission's, is unchanged.

Option 2 reads a busy branch's log of hundreds of kilobytes on every round for one revision. A round that ends in an error writes nothing, so the hold would go unrecorded, and `sluice run` would log the error on every pass. Option 3 misses a landing of several submissions, which is one move, and the finished submissions moved out of the live ones. Moving `base` to the read would form every combination again when the hold ends, even when the landing came back.

### Consequences

- A remote copy older than what landed in local refs, as when a branch gains `remote`, holds the branch until the operator pushes the landing there or accepts the remote's revision (S57).
- A force-push that drops a landing on purpose stops the queue; the hold names the command that accepts it.
- The local refs profile's `contains` answers `false` for a revision that is not a commit on this host, as when another host's landing is in neither the read nor this repository.
- `sluice/branch@1` is unchanged: `landed` and `held` are absent by default, and an older sluice ignores them.
- The host's own ref of a branch on a remote as sluice's copy (124, S58) is another change; this one reads the branch where a round read it before.
