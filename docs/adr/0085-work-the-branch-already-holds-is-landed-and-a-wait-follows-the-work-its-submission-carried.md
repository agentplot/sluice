---
number: 85
title: Work the branch already holds is landed, never formed again or withdrawn, and a wait follows the work its submission carried
status: accepted
date: 2026-11-06
decision-makers:
- the code stage of fix/a-landed-bolt-is-never-reported-withdrawn
---

# Work the branch already holds is landed, never formed again or withdrawn, and a wait follows the work its submission carried

Status: accepted, a decision of the code stage of `fix/a-landed-bolt-is-never-reported-withdrawn` under the statement as written (2, 5, 6, 22, 51, 67, 122, 137, C.5), which no clause of it changes, and cheap to change later. How a second copy of a repository on one host reads the intake mark is not decided here: it is the user's, with the design of one copy per repository per host (Boomdrive rule core.14).

Amended in part by record 0099, accepted on 2026-11-07: a delivery read where no files are checked out takes as its own what a worktree on the branch its `HEAD` names would, reading neither the folder's files nor its `HEAD`'s commit, and says nothing of an outcome staying pending because no files are checked out.

Deciders: the code stage of `fix/a-landed-bolt-is-never-reported-withdrawn`.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 2, 5, 6, 10, 22, 24, 50, 51, 67, 122, 137 and C.5's pull request row, at c188bf3; rules core.6, core.14, core.31, core.32, core.33; the signal `afterthought/crew-state:swancloud/main signals/2026-11-06-swancloud-ops-41b81e17/01-sluice-reports-a-landed-bolt-withdrawn`; deneb's records of swancloud's `main` in the runner's copy, `~/.local/share/flywheel/repositories/afterthought/swancloud/swancloud.git/sluice/main/`, and the person's, `~/Code/github_afterthought/swancloud/swancloud.git/sluice/main/`, for 2026-11-06; `crates/sluice-core/src/step.rs` (the round after `Moved`), `crates/sluice/src/profiles/github/pulls.rs` (intake's pass) and `crates/sluice/src/profiles/worktrunk/mod.rs` (`session`) as main held them at 6d18567b; afterthought/crew 0.41.0's `plugin/lib/plan.py` `SUBMIT` and `WAIT`.

## Context and problem statement

deneb keeps two copies of swancloud, each with its own records and queue of GitHub's `main`, and each reads `main`'s intake mark as its own, since the mark names the host (137). Both took pull request #330 in. The person's copy landed it as #425 at 13:01:08, `562606d`, a merge whose second parent is #330's head. The runner's copy read `main` moved to `562606d` at 13:01:10 and formed its #20, of that head, again on it, a combination of work the branch held. At 13:01:33 its intake read #330 gone from GitHub's open requests and withdrew #20 in its submitter's name, `by: swancloud-ops`. crew's `sluice wait 20`, run in the runner's bare clone, then exited 4, withdrawn, after its delivery read ran `git status` there and git refused.

By 122 #20's outcome was landed once `main` held its work, and nothing after changes it; by 5 sluice drops no entry on its own, and #20's submitter withdrew nothing. How does sluice keep its own records and answers true when a landing not its queue's takes a submission's work, and where its waits run in a repository with no files checked out?

## Considered options

**Where sluice finds work the branch already holds.**

1. **The round, from the branch, for each submission it is about to form** (chosen). The round reads the branch under its lock (core.14) and is the one step that forms (6). It reads the submissions no combination formed on the branch's revision holds, before it lands anything beside the queue or forms the chain. One whose head that revision holds is landed as the commit that landed it, the oldest of the branch's first-parent line holding it. A round with everything formed reads nothing.
2. Intake, when a request leaves the open list. Covers requests alone, and only once the forge has closed one; a worktree's submission, or a request still listed, is formed all the same.
3. As each combination is formed, in `arrange`. The landing beside reads unformed submissions first and would land work the branch holds beside the queue as an empty landing commit.

**How it is written.** Its record says `landed`, `landed_as` that commit, and `landed_elsewhere: true`; its outbox item is a landing's, so its session is told as for any landing (51); the log says `found_landed`, apart from `landed`, which is this queue's landing and which 121 reads as the last landing sluice recorded.

**When intake withdraws a request gone from the open list.**

- a. **Only where the branch, as that pass fetched it, does not hold its head** (chosen). A request the branch took the head of left the list as merged after the push (C.5); its submission is landed by the round. A request closed by a person, or merged by hand without its head reaching the branch, is withdrawn as the spec says.
- b. Ask the forge whether it was merged, and leave it a pass for the next fetch. A decision held in one process's memory between passes, lost between steps and waits (core.14), and a read of a request each pass (core.32).

**What a wait reports of a submission withdrawn.**

- i. **Where its work went** (chosen). The branch holds its head: landed, as the commit that landed it, the record kept withdrawn (5, 67). A later submission from its place holds its head, as a resubmission does (67): the wait says so and follows that one. Its pull request is open and no draft on a head that holds its head, taken in again after its submit's new head or on another host (137): the wait follows the request. Otherwise withdrawn, exit 4.
- ii. Rewrite the withdrawn record as landed once the branch holds its head. A final outcome decided twice (5), and delivered twice (51).
- iii. Report the record as it stands. What crew's waiter read on deneb.

**What a repository with no files checked out reads as.**

- **A session with no files checked out** (chosen): the driver asks `git rev-parse --is-inside-work-tree` first, and where none are runs no `git status`, reads nothing held beyond its commits (2) and no path. A delivery read there delivers nothing, since no session works there, and says so plainly; the outcome stays pending for its session's own carrier (51).
- A driver error in plainer words: a failed read where nothing failed.
- A session read as detached: a submit there would be refused with detachment's words, which say the wrong thing.

**What a second copy does with the intake mark (137).** Not decided here; the readings are the change's design D5: (a) one copy per repository per host, the copy being the host, Boomdrive rule core.14; (b) the mark naming the copy as well as the host, a change to 137's reading that needs the statement first (core.12); (c) a second copy refusing intake, which needs a list of copies sluice does not keep.

## Decision outcome

Options 1, a, i, and a session with no files checked out. Rule core.48 holds them.

### Consequences

- A submission's record may say `landed_elsewhere`, and the log `found_landed`; records written before read as they were. An older sluice passes `landed_elsewhere` by, and refuses a log holding `found_landed` as it refuses any line it does not know, so hosts sharing a branch's records take this sluice together, as for every line added before.
- Two copies of one repository on one host still both take a branch's requests in and both test them, until the one-copy design lands. Each copy's records now say landed for work its branch holds, and its waits answer landed; one landing is still told to its session once by each copy, under each copy's number.
- A landing another copy pushes between one pass's fetch and its list of open requests is read by that pass as a closing and withdrawn. A wait on it still answers landed.
- `sluice wait <id>` may follow, and report, a later submission that carries its submission's work, saying so; a wait on a submission handed over to another host now follows its request there instead of exiting 4.
