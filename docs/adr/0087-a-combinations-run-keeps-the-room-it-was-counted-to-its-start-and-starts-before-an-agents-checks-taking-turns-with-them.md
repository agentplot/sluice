---
number: 87
title: A combination's run keeps the room it was counted to its start, and starts before an agent's checks, taking turns with them
status: accepted
date: 2026-11-06
decision-makers:
- Chuck Swanberg
---

# A combination's run keeps the room it was counted to its start, and starts before an agent's checks, taking turns with them

Status: accepted on the user's card in Pending You, 2026-11-06, in the words "Approve: push its statement change, then start its code", given on the design of the change `a-queues-test-keeps-the-slot-it-counted` and the statement's amendment, now on agentplot/blueprints main at 8bf02d9 (71, 150, S75). Proposed that day by that change's construct stage, from its brief, on the user's approval, on Pending You, of the plan's sluice part: "A queue's test holds its slot from the count to the start, and is started before an agent's check (statement change, for your yes)". How agents' checks are kept from waiting for ever, by turns, was this record's own choice, and was in the statement's words the user approved.

Amended in part by record 0093, accepted on 2026-11-06: the host's count, each run's part and the runs waiting for room are kept in sluice's host process, which holds a run's part from the count that chose it to its start (135); no process coordinates with another through the host's files. Who starts first, by turns, and the order of agents' runs stand as decided (150).

Deciders: the user, approving the direction on 2026-11-06 and accepting the statement's words and the turns that day; the construct stage of `a-queues-test-keeps-the-slot-it-counted`, proposing the words and the turns.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 31, 60, 66, 71, 75, 83, 88, 129, 135, S60, and 71, 150 and S75 as agentplot/blueprints main words them at 8bf02d9; `design/sluice/proposals/session-runs-1.md` clauses 89 and 91; `previews-1.md` clause 102; `design/flume/requirements.md` clauses 70, 130, 145 and 181; records 0012, 0018, 0035, 0047; rules core.2, core.11, core.14, core.18, core.23, core.31; `crates/sluice-core/src/step.rs` `choose_starts`, `capacity`, `perform_starts` and `start_run`, `crates/sluice/src/profiles/checks.rs` `Tests::start` and `admit`, `crates/sluice/src/host_count.rs`, `crates/sluice/src/exec.rs` and `crates/sluice/src/preview.rs` as main held them at 6d18567b; the research note of 2026-11-06 on how a host drives the queues of the repositories it keeps, read on deneb against those files.

## Context and problem statement

A round counts the host's room when it decides which combinations to test (`choose_starts`, from `capacity`), and writes the runs it chose as starting. It then lands what is due, releases checkouts, pushes outcomes, and only then starts the runs: for each, it reads the host's count once more, makes the combination's checkout, and calls the check seam's start, which admits the run against the host's count (record 0047). Between the count and the admission the room is held by nothing. An agent's `sluice exec`, which asks for room every three seconds, takes it as soon as a run ends. On deneb, on 2026-11-06, a round on `bolt/swanclouds-bumps-of-the-5th` claimed a combination into `sluice/try/bolt/swanclouds-bumps-of-the-5th/1` at 02:01:58; an agent's run took the free run at 02:02:00; at 02:02:03 the admission found the bound full, the checkout was released and the run left open. The same came at 02:34:23 on that branch, at 02:25:16 on `bolt/a-sessions-box-stops-with-only-plumbing-on-it`, and on flume main/99, swancloud main/222 and 225 and crew main/217. Record 0047 called this rare, "only when two starters decide at the same moment"; with agents asking every pass, it is what happens whenever a run ends.

Nor does anything start a waiting combination's run before an agent's. The session-runs-1 proposal's clause 91 said the queue's run is started first; the statement never carried it, and the code offers only `[runs] reserved`, runs of the bound agents never take. deneb keeps one reserved now (swancloud #415), which leaves one run the queue always has and the other still a race.

How is a combination's room held from the count to its start, and who starts first when a combination's run and an agent's both wait, without either waiting for ever?

## Considered options

**Where the room is held.**

1. **The count is the admission** (chosen). The check seam gains `hold`: in one step under the host's lock it reads the count and, where there is room for the run, writes the run's entry in the host's registry with a hold its process keeps locked, as an admission does today (record 0047). The round holds a part for each run it would start before it writes its decision, keeps the parts in its process while it lands, releases and pushes outcomes, and hands each to the run's start with its place; the start hands it on to the run's supervisor as an admission is handed today. A part dropped, by a start that failed, a decision whose write was refused, or a process that stopped, is given back at once, since the hold's lock goes with it. Every start, a session's and a preview's among them, takes its part the same way, so start admits nothing of its own.
2. **Admit in `perform_starts`, before the checkout is made.** Closes the three to five seconds of the checkout, and leaves the gap between the decision and the starts, where a round lands, pushes and runs release commands.
3. **Mark starting runs in the host's state at the decision**, counted by sessions' admissions, and admit them in start as today. Two kinds of entry for one fact, and a mark whose round has ended must lapse by time, where option 1's lapses with its process's lock.

Record 0047 turned down a separate `admit` because the core would keep a token across a seam call and would have to know when the run's supervisor took over. Option 1 keeps the first and not the second: the core holds the part only within one round, opaque to it, and the start consumes it, so when the supervisor holds the run stays the check profile's alone (rule core.2). The race is what that refusal cost.

**Who starts first when both wait.**

- a. **The combination's, by turns** (chosen). Where a combination's run and a session's or a preview's wait for room at once, the combination's starts first. Once a combination's run has started in room a waiting session's run could have taken, the next such room goes to a session's run, and the turn is the queue's again. Session runs and previews start in the order they came to wait. The reserve stays the queue's alone. Under full contention the queue has its reserve and every other run of the rest, and an agent waits for at most one combination's run beyond the agents ahead of it.
- b. **The combination's, always**, as 91 was drafted. With a window at or above the bound and work flowing, agents wait as long as the queue has work, hours on a busy day.
- c. **flume's reserve** (flume 70, 181): agents owed a part of the bound, lent while none waits and theirs again at the next run to end, combinations first beyond it. Where the bound leaves agents one run, the owed run is all of it and combinations wait behind every agent: deneb's bound of two with none reserved, and the default bound on a four-core machine, a quarter of its cores, is one. It also asks the keeper for a second number.
- d. **Order by time waited, combinations given a head start.** A length of time with no reason behind it.
- e. **The reserve alone**, deneb's fix. A reserved run stands empty while no combination needs it, and the rest is still a race.

flume's rooms agree with this where the two meet: a wait is the host's, never its caller's (flume 70, 130), equals start in the order they came to wait (70), and an account is chosen under the same lock as the room, so two starts never take its last room between them (145). sluice's `reserved` is what flume calls a reserve kept free (181). The two bound different things, agents and checks, each tool its own; neither calls the other.

**Where a wait is kept.**

- **The host's state, beside the count** (chosen): `running/` gains each waiting run, with its kind, repository, what it waits for and since when. Each counts only within thirty seconds of its last ask, ten of a driver's passes: a combination's needs no more, since no process stands between rounds and none need (135; rule core.31), and a session's or a preview's also needs its process to hold the wait's lock, as an admitted run's starter holds its hold, so a command killed lapses at once and one suspended within the thirty seconds. The registry also keeps whose turn the next contested room is.
- The branch's records. A session's run is no branch's (rule core.18; record 0035), and another repository's sessions read no branch of this one.

## Decision outcome

Option 1, a, and the host's state (71 amended, 150 and S75 added, agentplot/blueprints 8bf02d9).

A round holds a part of the bound for each run it would start, before the decision that marks the run starting is written: a run on a combination of its window, or one opened again, its own or claimed on a branch whose records are on its remote (75). Where `hold` gives none, the run is not marked starting; the round goes on deciding, and the run's wait is recorded for the host. A run marked starting is started with its part; one marked starting with no part, a round's that stopped, is opened again by the next round as today.

The registry's admission, for a part asked of kind queue or kind session, under the host's lock:

- A combination's run is held where a run of the bound is free, unless that run is one a session may take, a session's run waits, and the turn is the sessions'. Held in room a session could take while one waits, the turn becomes the sessions'.
- A session's or a preview's run is held where a run of the bound less the reserve is free and is not owed: no session's run waits ahead of it, and either the turn is the sessions' or the combinations waiting are fewer than the free runs. Held, the turn becomes the queue's.
- A run refused records or renews its wait. A run held removes its own.

`sluice exec` and `sluice preview` ask for their part at every pass, under the same rule, and say `waiting: a combination's run starts first` while a free run is owed to a waiting combination. `sluice overview --json` gives the waiting runs by kind and whose the next contested room is in `room`.

### Consequences

- Record 0047's consequence that a start refused after its place was made wastes a checkout no longer holds: a checkout is made only with a part held. Its decision that `Tests::start` admits the run is replaced by `hold`, whose part start takes; the registry, its lock, its holds and its files stand.
- A round that lands or releases before it starts keeps the runs it chose idle for as long as that takes, which they were before, unseen.
- An agent's check may wait while the overview shows a free run: it is owed to a waiting combination, and the check says so.
- A combination whose rounds stop holds no run. Its wait is passed over thirty seconds after the last round asked for it, and agents take the room.
- Combinations of several repositories still start in whichever order their rounds come (S60): ordering them across repositories would keep a run empty until the round of the one first in line comes.
- A host with no `reserved` gives the queue at least every other run its agents contend for; a keeper who wants more for the queue keeps a reserve, as before.
- A run started by a sluice built before this holds no part between its count and its start, and an older `sluice exec` records no wait, until each host runs this release.
