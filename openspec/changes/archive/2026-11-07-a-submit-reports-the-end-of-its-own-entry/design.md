# Design

## Context

### What happened

deneb's records of agentplot/sluice's `main` on 2026-11-07:

- The runner's copy,
  `~/.local/share/flywheel/repositories/agentplot/sluice/sluice.git/sluice/main/log.jsonl`:
  #2 `submitted` at 03:50:35Z with head `daeaf98`, `testing` `[2]` at
  03:50:39Z, `moved` `9c8159a` to `ed8474b` at 03:52:04Z, `testing`
  `[2]` again on it at 03:52:04Z, `withdrawn` `by: agentplot-ops` at
  03:52:10Z. `finished.jsonl`: `{"id":2,"state":"withdrawn",
  "worktree":"pull/204"}`.
- The person's copy,
  `~/Code/github_agentplot/sluice/sluice.git/sluice/main/log.jsonl`
  lines 7835 to 7846: #399 `submitted` at 03:50:36Z with the same head,
  `landed` as `ed8474b` at 03:52:03Z, `delivered` to `pull/204` at
  03:52:06Z. `ed8474b`'s parents are `9c8159a` and `daeaf98`.
- crew's session (raw record `d4947942`): `bolt/sluice-tasks-check-only-what-they-touch
  submitted to sluice's main, sluice's #2 from pull request #204`, then
  `sluice exited 4 after its push: #2 withdrawn by agentplot-ops`.

The runner ran sluice 0.68.1. Record 0085 (sluice 0.70.0) stops each
link of that chain, as its change's design D1 to D3 give it. Today's
code would land #2 as found landed at its first round on `ed8474b`,
withdraw nothing, and report #2 landed, exit 0.

### What still goes against the unit

`crates/sluice/src/lib.rs` as the bolt holds it at 68d68a9a:

- `carried` turns a withdrawn submission into `Carried::Entry(e)`, a
  later submission from its place that holds its head, or
  `Carried::Request(sub, n)`, its pull request. `wait_on` then sets
  `sub = e` and goes on. `await_submission` reports whatever `wait_on`
  returns, `#<b>`'s record and `exit_for(#<b>)`. `Waited::Request` runs
  `submit_pull` with `PullMode::Follow`, which reports the `Followed::Done`
  submission of the request's current head. Under `--json`, `follows`
  writes nothing, so the document is `#<b>`'s record with no word of
  `#<a>`. If `#<b>` is blamed, a wait on `#<a>` exits 1.
- `submit_pull`, waiting, reports `Followed::Done(e)` for whatever `e`
  `Pulls::follow` finds: the latest submission of `pull/<n>` on the
  request's current head. When the request moves to a head the submit
  never sent, a unit landing onto the bolt a by-name submit is landing
  among them, the submit reports that head's submission as its own.
- `report_delivering` marks delivered whatever it reports, when it is
  the waiting worktree's and pending. So a wait that followed `#<b>`
  marks `#<b>`'s outcome delivered by a report naming `#<b>` as if it
  were the one waited on.
- `Pulls::follow`, for a request no longer open whose latest submission
  is not landed, asks `landed_by(&t.tip, &e.head)` with `t.tip` from
  `Branch::held`, the host's copy, which no fetch moved. In a waiting
  submit's first pass intake fetched the branch just before, so the
  copy is current. Beside a live driver no intake runs, and the copy is
  as old as the driver's last tick. In the race of record 0085 D5,
  intake's fetch came before the landing and its open list after it, so
  the copy holds no landing. Either way the wait prints `#<n> was closed
  without landing` and exits 1, though `carried`, which reads
  `Branch::read`, would have said landed.

### Looking across

- **crew** lands a bolt with `sluice submit main --head <bolt>` and waits,
  or with `sluice wait <id> --timeout 9m --json`, in the runner's bare
  clone (`plugin/lib/plan.py` `SUBMIT`, `WAIT`). It reads the outcome by
  the exit, 0 landed and 4 withdrawn, and the record's `state`. It needs
  no change: the exit and `state` now always describe the submission it
  submitted. Under `--json` it can read `carried_by` when it wants to say
  which later submission landed or failed the work.
- **flume** reads a place's submission from `sluice status --json` and
  `Sluice-Submission`, never from a wait. Its places are worktrees, so
  `sluice wait <branch>` there reads the worktree's last submission, as
  now.
- **The host process**, record 0093, being built in
  `change/a-hosts-queues-move-in-sluices-own-host-process`, changes how
  a wait follows: by the host process's document, running no round. It
  does not change what a wait reports. `carried` and the request's follow
  are where the end is read, and that change keeps both. The two touch
  `wait_on` and `submit_pull` beside each other. Whichever lands second
  carries the other's lines.
- **The forge queue**, record 0093's 137, being built in
  `change/pull-requests-are-queued-on-the-forge`, leaves no host taking
  requests in. The in-host branch of `Pulls::follow`, which D5 mends,
  then runs only where the branch's instance takes requests in. Every
  host follows across, and its own submission is the one sluice's check
  on the request's head names (record 0095). D4's reading carries over:
  the first submission the wait reads for its request is its own.
- **`change/a-delivery-read-needs-no-work-tree`**, beside this change on
  the bolt, mends the delivery read where no files are checked out. This
  change only narrows what a wait marks to its own outcome. It reads
  delivery as that change leaves it.

## Goals / Non-Goals

**Goals:** a waiting submit's and a wait's document and exit are always
those of the submission it waits on. A landing of its work, by any
queue, reads as landed, exit 0. A carrier's end is never reported as the
wait's own, and is named apart from it. A closed request reads as a
closing only once the branch, read as a round reads it, does not hold
its head.

**Non-goals:** which copy of a repository takes a branch's requests in.
How a wait follows the host process. The follow of a request across
hosts. Group submits.

## Decisions

### D1. The submission a wait waits on

A wait's own submission is fixed when it starts and never changes:

| Command | Its own submission |
|---|---|
| `sluice wait <id>` | `<id>` |
| a waiting `sluice submit`, from a worktree or by name | the submission the submit made |
| a waiting `sluice submit` to a branch that takes pull requests | the first submission of its request this host's records hold, as the wait first reads it (D4) |
| `sluice wait <branch>` | the worktree's last submission to `<branch>`, as now; on a branch that takes pull requests, as for a waiting submit |

`wait_on` keeps it apart from the submission it follows. A `following`
of its own holds the carrier, where there is one, and the reading of the
overview follows the carrier's combination, so its progress lines and
`waiting` are the carrier's while the wait follows it.

### D2. Its end is its own submission's

Whatever the wait follows, it ends with its own submission:

| What it reads | It reports | Exit |
|---|---|---|
| its own submission landed, failed or in conflict | as decided | 0, 1, 2 |
| its own withdrawn, the branch holding its head | landed as the commit that landed it, `landed_elsewhere` (record 0085) | 0 |
| a carrier landed, the branch now holding its own head | its own landed as `landed_by(read tip, own head)`, `landed_elsewhere`, `carried_by` the carrier | 0 |
| a carrier blamed or in conflict | its own withdrawn, `carried_by` the carrier with `failed` or `conflict` | 4 |
| a carrier withdrawn, nothing carrying it on | its own withdrawn, `carried_by` the carrier with `withdrawn` | 4 |
| its request closed, or sent back by another host | its own withdrawn, `carried_by` the request with `closed`, `failed` or `conflict` | 4 |
| a withdrawal no work goes on from | its own withdrawn, as now | 4 |
| out of time or stopped while a carrier is undecided | its own record as it stands, `carried_by` the carrier as it stands | 5, 130 |

A carrier withdrawn whose own work a further submission or the request
carries on is followed on, as `carried` follows the first. The branch is
read with `Branch::read` each time the wait asks whether it holds its
own head. A carrier that landed while the branch holds no own head
cannot happen while `carried` follows only heads that hold its own. If
it does, the wait reports its own withdrawn with `carried_by` the
carrier `landed`, rather than claim a landing the branch does not show.

The record on disk is never changed: a withdrawal is final (5) and
readable as withdrawn (67). The wait reports what 122 says the outcome
is, as record 0085 D3's first case does.

Options weighed, as record 0100 gives them: the carrier's record and
exit, as built, which tells a submit another submission's end as its
own; stopping at the withdrawal, which reads a landing as a failure.

### D3. What a wait prints of a carrier

- **Words.** On starting to follow, as now: `#<a> withdrawn by <who>:
  #<b> carries its work, so this wait follows it` (or `its pull request
  #<n>`). At its end, `report`'s lines for its own submission. A landing
  by a carrier says `landed as <rev>: <branch> holds its work, by #<b>'s
  landing` (or `by pull request #<n>'s landing`). A landing found
  without one keeps record 0085's `by a landing not its own`. A carrier
  that stopped short says `#<a> withdrawn by <who>: #<b>, which carried
  its work, failed` (`is in conflict`, `was withdrawn by <who>`; `pull
  request #<n>, which carried its work, was closed without landing` or
  `was sent back`), and the next line `sluice status <b>` where `#<b>`
  is a submission. Out of time: `still waiting: sluice wait <a>`.
- **The document.** Its own submission's record, as `sluice status <a>
  --json` prints it, with `carried_by` added once the wait has followed
  a carrier. That is `{"id": <b>, "state": "<b's state>"}`, or
  `{"pull_request": <n>, "state": "open" | "landed" | "failed" |
  "conflict" | "closed"}` for a request with no submission on this host.
  `carried_by` names the last carrier followed. The record keeps every
  field it had (`sluice/submission@1`), and `copy` and `waiting` are
  added as now.
- **Delivery.** `report_delivering` marks delivered only the submission
  it reports, its own, when it is the waiting worktree's and pending: a
  landing found by a round, whose outbox item is pending, among them. A
  withdrawn own submission has nothing pending, so nothing is marked.
  The carrier's outcome stays pending for its own carrier, the round's
  notice or `sluice inbox` (51; records 0068, 0070). Its session is told
  `#<b>`'s end as `#<b>`'s.

### D4. A waiting submit to a branch that takes pull requests

`submit_pull` keeps `own: Option<Submission>`: the first submission of
the request it reads, from `Followed::Open { submission: Some(_) }` or
`Followed::Done(_)`. The submit's first pass takes the request in, which
is its own act (135), so this is the submission the submit made. Until
one is read, it follows the request as now, `Followed::Landed`,
`SentBack` and `Closed` read across hosts as now.

Once `own` is set:

- `Followed::Done(e)` or `Open { submission: Some(e) }` with `e.id ==
  own.id`: `e` is its own. `Done` is reported as now.
- with `e.id != own.id`: its own left the queue for the request's new
  head. The waiting submit then waits on `own` as `sluice wait <own>`
  does, through `await_submission`. `carried` finds the request carrying
  its work and follows it with `PullMode::Follow`, or finds its work
  gone and reports it withdrawn.

`PullMode::Follow` names the submission it follows for: `Follow {
number, own: &Submission, deadline }`. Its `Done(e)` with `e.id !=
own.id`, `Landed`, `SentBack` and `Closed` end as D2's table, for
`own`. It never recurses into `await_submission`.

Options weighed: whichever submission of the request is decided, as
built, which reports a head the submit never sent; the submission at the
head the submit sent, read by head alone, which a head submitted twice,
by name and from a worktree, would make ambiguous.

### D5. A closed request is read against the branch as a round reads it

In `Pulls::follow`, a request no longer open whose latest submission is
not landed reads the branch with `Branch::read`, which fetches a branch
on its remote under the retry policy (134), and asks `landed_by` of that
tip. One read per closing, as `carried` already reads the branch, and
only once the forge has said the request is closed. So the wait's
passes cost nothing more while the request is open (129, rule core.32).
A read that fails is the follow's error: the wait ends as one that could
not read, exit 3, never `closed without landing`.

GitHub's `merged` is not read as landed. sluice lands only by its push
(core.45), and only the branch holding the head says the work landed
(122).

### D6. How it lands in the specs

The requirements this change amends, `submission-wait`'s three that a
wait follows the work, were added by `a-landed-bolt-is-never-reported-withdrawn`.
That change's nine tasks are done and built since 0.70.0, but it was
never archived, so they are not yet in `openspec/specs`. Its archive is
this change's first task. Two of its requirements are over openspec's
500 characters once archived, in `pull-request-intake` and
`request-intake-host`, and `landed-elsewhere` needs a `## Purpose`. Its
deltas take both before the archive. The archive is the same text
whichever change on the bolt runs it first.
`change/a-delivery-read-needs-no-work-tree` amends that change's
`outcome-delivery` requirement too. A task finding it already archived
on the base does nothing more.

`The exits tell the outcomes apart`, as record 0085 words it ("4 when it
was withdrawn and no work of it goes on"), holds under D2. A carrier
that stopped short leaves no work of it going on. It is not amended
here. The requirements of a wait's loop and its rounds, which the host
process's change amends, are left alone.

## Risks / Trade-offs

- **A wait on a superseded submission exits 4 when the resubmission is
  blamed** → it exited 1 under record 0085. A session that waited on its
  first submission learns the second failed from the words, from
  `carried_by` and from the second's own notice. It never reads that
  failure as the first's.
- **A carrier's outcome reaches its session by its own carrier, not by
  the wait that read it** → under record 0085 the wait marked it
  delivered. One more notice now reaches a session that resubmitted
  while a wait stood: the second submission's own, which 51 tells to its
  submitter.
- **One fetch per closing read** → only after the forge says closed, and
  `carried` already makes the same read. Beside a live driver it is the
  one read the wait makes of the branch.
- **Two changes on two bolts edit `wait_on` and `submit_pull`** → the
  host process's and this one. Their requirements are apart (D6), and
  their lines meet only where `carried` is called and where a request's
  `Done` is reported.
