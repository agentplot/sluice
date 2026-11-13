---
number: 86
title: One fetch at a time in a repository, and a race lost for its own ref is no refusal
status: accepted
date: 2026-11-06
decision-makers:
- the code stage of fix/a-landed-bolt-is-never-reported-withdrawn
---

# One fetch at a time in a repository, and a race lost for its own ref is no refusal

Status: accepted, a decision of the code stage of `fix/a-landed-bolt-is-never-reported-withdrawn` under the statement as written (133, 134) and rules core.28 and core.29, which it keeps; cheap to change later.

Deciders: the code stage of `fix/a-landed-bolt-is-never-reported-withdrawn`.
Sources: `boomdrivedev/blueprints:design/sluice/requirements.md` clauses 129, 133 and 134 at c188bf3; rules core.28, core.29; records 0061, 0062; swancloud-ops's report of 2026-11-06 on landing swanclouds-bumps-of-the-5th, #23 in the runner's copy of swancloud on deneb, landed as f24499c9; that copy's `sluice/main/log.jsonl`, `read-refused` at 12:57:13Z and 13:56:33Z; `crates/sluice/src/profiles/pushed_refs.rs` `call`, `fetch_specs` and `judged` as main held them at 6d18567b.

## Context and problem statement

After #23 landed in the runner's copy of swancloud, a sluice command exited 3 on a fetch. The copy's `main` log holds the cause twice, as `read-refused`: `error: cannot lock ref 'refs/sluice-fetched/origin/intake/main': is at <a> but expected <b>`. Two processes of the one repository fetched at once, a waiting submit beside a Flume settle's `sluice step` or a reader, and raced to write sluice's own fetched ref. The remote refused nothing. sluice read git's words as the remote's refusal and logged it on the branch (133, core.28). It never tried the fetch again, though the remote never judged it (134, core.29). How do one repository's fetches stop racing, and how is a lost race read?

## Considered options

1. **One fetch at a time in a repository, under a lock held for each try** (chosen): a file in the repository's git directory, `sluice-fetch.lock`, locked by each fetch for its try alone and never over the retry policy's pause, so a fetch waits seconds at most behind another and a long pause holds no one. Every worktree of the repository shares it, since they share the refs a fetch writes.
2. Each process fetching into refs of its own. Every reader of the fetched refs would need to know which process last fetched; the intake mark and the remote-tracking refs are read by name throughout sluice.
3. Try the fetch again on any failure. The retry policy already tries again what the remote never judged. Without the lock, two fetches go on racing. And git's words for the lost race carried no sign the remote never judged it, so they read as an answer.

**How a race still lost is read.** A fetch beside one that holds no lock, a person's `git fetch` or an older sluice's, can still lose. Git's words `cannot lock ref '<name>'`, or a ref's lock file standing, name a ref of this repository, never a remote's. Such a try is one the remote never judged, and the retry policy tries it again. One that still loses is read as nothing refused. The refs hold what the other fetch brought a moment before, and the next fetch brings the rest. It is logged on no branch.

## Decision outcome

Option 1, and a race still lost read as no refusal.

### Consequences

- Fetches of one repository by sluice's processes run one after another; a round's fetch may wait behind a waiting submit's.
- A lost race is never a `read-refused` line, and fails no command.
- Rules core.28 and core.29 name the case in their details.
