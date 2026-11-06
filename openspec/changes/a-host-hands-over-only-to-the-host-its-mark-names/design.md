# Design

## Context

What the records hold for WilldanGroup/switchboard-kit's `main` on
2026-11-06 (the proposal gives the timeline):

- chuck-herdr-alpha's `main/log.jsonl`: `withdrawn` #1406, #1407 and
  #1408 at 15:07:04, each `by: another host takes main's pull requests
  in`; `released` `sluice/try/main/801` and `802` at 15:09:26 and
  15:12:08, the combinations those withdrawals made obsolete;
  `submitted` #1411 to #1413 at 15:12:12 to 15:12:18, the same three
  heads; `landed` `[1411, 1412, 1413]` to `b51477b` at 15:55:09.
- Its processes: no `sluice run` for switchboard-kit; crew's `while
  sluice wait "$2" --timeout 9m --json; [ $? = 5 ]; do :; done`, one per
  landing; `/etc/profiles/per-user/chuck/bin/sluice`, 0.68.1.
- GitHub: `refs/sluice/intake/main` at 18:15:04, `54dad0a8`, naming
  chuck-herdr-alpha and a wait; the timelines of 241 to 243,
  `convert_to_draft` by `afterthought` at 15:07:16 to 15:07:18 and
  `ready_for_review` by the same account at 15:09; every draft sluice
  made that day on switchboard-kit by `sluice-for-willdan[bot]`.

The faults, each against its clause:

1. **A refused push named a host the mark did not**
   (`crates/sluice/src/profiles/github/pulls.rs`, `put_mark`): refused
   at its lease, the push read the branch again and returned
   `Taking::Elsewhere` with the host the mark named, or "another host"
   where none stood; intake then withdrew every submission it had taken
   from a request (`hand_over`). One of the host's own waits had let
   the mark go between another wait's fetch and that wait's renewal, so
   none stood. A mark another process of the host had pushed named the
   host itself, which would hand its requests over to itself. 137
   withdraws on "the mark another's", "naming the host that holds it".
2. **The host's waits each let its mark go** (`let_go_marks`): every
   wait that pushed the mark, or took the requests in under it, removed
   it when it ended, though other waits of the host still read the
   branch's requests. 137 keeps no mark between a host's waits; here the
   branch kept none while its host still waited, open to another host's
   wait and a hand-over of all it had taken in.
3. **A hand-over withdrew work the branch holds** (`hand_over`): a host
   that finds the mark another's withdraws its request submissions, none
   landing, without asking whether the branch, as the pass fetched it,
   holds their heads. A gate host back after another host landed its
   requests would withdraw them so, against core.48's "never ...
   withdrawn" for work the branch holds.

The drafts were crew's, made with the person's account when sluice
0.68.1's wait reported the hand-over's withdrawal as withdrawn; record
0085 (0.70.0) has a wait follow a request its host handed over. No path
of sluice's marks a request a draft when the mark moves: intake drafts
a request held for its submitter's act (120), whose latest submission
here left the queue unlanded, and a submission withdrawn by a hand-over
is passed by there (`latest_of`); C.5 drafts on a new head or a culprit
alone.

### Looking across

- **crew** runs one `sluice wait <id> --timeout 9m` per landing on the
  team's host, again while it times out, and drafts a landing's request
  by `gh pr ready --undo` when the wait exits 4. Its waits are what put
  several of one host's waits on one mark; crew needs no change, since
  sluice no longer reports a hand-over withdrawn nor makes one for
  nothing.
- **flume**, **Boomdrive**, **weir** and **timber** keep no mark on a
  remote that several processes of a host hold. weir's shared branch
  lock is one writer's, with a stale age. sluice's own count of running
  checks (`host_count.rs`) is of files locked while their process
  lives, made under a lock over them, which the holds here follow. The
  intake mark is sluice's alone, so nothing is reconciled across the
  tools.

## Goals / Non-Goals

**Goals:** a host hands its requests over only to the host its mark
names; a mark moved under a push by the host's own process, or let go,
changes nothing of the host's queue; the mark stands from a host's
first wait on a branch to its last; work the branch holds is never
handed over.

**Non-goals:** a driver for switchboard-kit (the gate box is
swancloud-ops's); two copies of one repository on one host (record
0085's D5); crew's drafting of a withdrawn landing's request.

## Decisions

### D1. A refused push is decided again, once

`takes` asks `decide` what the mark as last fetched calls for. `decide`
returns `None` where `put_mark` was refused at its lease; `takes` then
reads the branch again and asks once more. On the second reading:

- the mark names this host: `decide`'s own rule, held where it is the
  same holder renewed within a third of the lease, else pushed over
  leased on what was read;
- none stands: pushed leased on its absence, as a host finding none
  does;
- it names another host: `Taking::Elsewhere` with that host, and the
  hand-over.

A second refusal is `Taking::Undecided`: the pass takes nothing in,
hands nothing over, warns once, and the next pass decides again.
`put_mark` no longer reads or names anything after a refusal, so no
`Taking` names a host the mark does not.

Options weighed: reading a refusal as another host's take whatever the
read finds, which is the fault; taking nothing in on any refusal, which
holds a host whose mark another did take one pass longer and sits a
host out a pass for its own process's move.

### D2. A host's waits hold its mark together

A wait's intake keeps, from its first pass on a branch whose records
each host keeps, a hold among its host's waits: `Kept::hold`, the file
`forge/intake/<owner>/<repo>/<base>.waits/<pid>.hold` in the host's
state, locked while the process lives, made under the `.lock` beside
it. It is kept whether or not the pass is due, since a wait whose
passes are not due still waits on the branch.

`let_go_marks`, at a wait's every exit, lets each hold go under that
lock and reads the others: one locked is a live wait of the host, and
one not locked a gone one, removed. Where another lives, the mark stays
the host's. Where none does, the mark as the repository last read or
wrote it is removed, by a push leased on that value, where it names
this host and a wait; one naming another host, or the host's driver, is
left. The rev a wait pushed is no longer kept: the repository's fetched
mark is what any of its processes last read or wrote.

The holds are keyed by the repository's address on the forge, as the
mark names a host and not a copy.

Options weighed: each wait letting go what it holds, the fault; a wait
never letting go, so a host that stopped waiting holds the branch a
lease longer; the intake lock held over the let-go, which keeps a drop
out of another wait's pass but not out of the time between its passes.
This reads 137's "a wait that holds it lets it go when it ends" as the
host's last wait; record 0089 is accepted with the amendment that says
so, on agentplot/blueprints main at 421c77e.

### D3. The hand-over passes by work the branch holds

`hand_over` takes the branch's revision as the pass fetched it and
passes by a submission whose head it holds, as it passes by one
landing. The round then lands it as the commit that landed it
(`land_found`, record 0085's D1). A wait whose host handed over runs no
round, so such a submission stays in the queue until the host's next
round, a step's or a driver's, finds it landed; a wait on it reports it
landed meanwhile (record 0085's D3).

### D4. What the new holder does with requests in another host's queue

Nothing of that host's: each host's records are its own (C.6), and no
host writes another's. The new holder takes each open request in as its
conditions hold (137). The other host, at its next pass that finds the
mark, withdraws its own submissions of them, naming the holder, none
landing and none whose head the branch holds (D3); one it was landing
lands, and the holder's submission of that head is then found landed
(122). A host that is gone advances nothing until it returns.

## Risks / Trade-offs

- **A refused push costs a second fetch and push** → only where the
  mark moved under the pass; a host whose waits come and go keeps its
  queue through it.
- **A wait killed outright leaves its hold** → unlocked, read as gone
  and removed by the next let-go; the mark goes with the host's last
  wait, or lapses after the lease where none is left.
- **The holds are written by every wait on a branch whose records each
  host keeps** → one file a wait, made once, under the host's state.
- **D2 rests on a reading of 137** → 137 says it, as amended on
  agentplot/blueprints main at 421c77e.
