# Design

## Context

This change's base is `bolt/sluice-takes-mended-work-again`, at main's
`0b15d250`. Where each read of a branch's policy takes its revision on
that base:

| Read | Revision | Reached by |
|---|---|---|
| `Sluice::status` (`crates/sluice-core/src/ops.rs`) | `doc.base`; `Branch::read` where the state has none | `sluice status <branch>`, `sluice status <id>`, `submit --status`, `last_submitted`, `carried`, `submit_pull`'s follow, and intake (`crates/sluice/src/profiles/github/pulls.rs`) |
| `Sluice::branch_view_of` (`crates/sluice-core/src/overview.rs`) | `doc.base` | `overview()`, so `sluice overview`, `sluice` with no command (`home.rs`) and what a watch shows; `progress.rs`; `status --json`'s `waiting`; a wait's view |
| the overview's order (`overview()`) | the default branch's `view.at`, which is `doc.base` | `sluice overview` |
| `submit_group`, `takes_pulls` (`crates/sluice/src/lib.rs`) | `Branch::read`, which fetches a branch on a remote | `sluice submit` |
| a round's `decide` (`crates/sluice-core/src/step.rs`) | `Branch::read`; `doc.base` set to it at `step.rs:1060`, the gate read there | every round |
| a round's `start_from`, before `decide` | `doc.base` | only where a run of this host is starting |
| a round's `perform_landing`, `same_gate` | the landing intent's `from` and `to` | only where a landing is unfinished |

`Branch::held` (`crates/sluice-seams`) is the branch's revision as the
host's own files hold it. For a branch in the host's own refs it is that
ref, and for a branch on a remote its tracking ref as last fetched, read
by gix with no process. `branch_view_of` already reads it for the
view's `revision`.

What moves `doc.base` is a round alone, and no round runs on a branch
with nothing in play. Under record 0093 sluice's host process runs a
branch's rounds while it has work in play and ends once no queue has
any (135). So a quiet branch's state keeps the revision its last round
stood on. Every read in the first two rows refuses the branch while
that revision's policy is refused, however long the host has held a
clean one. On deneb, flywheel-next's `origin/main` held `f2ce8e3f` from
2026-10-28T15:12:12Z, and `main` stayed refused at `2924aa2f` until a
hand's `sluice step main` at 2026-11-06T16:46:34Z.

### Looking across

- **flume** reads a base at origin's copy after flume's own fetch,
  wherever a merge queue lands it (flume 9, 126, C.1; flume record
  0041, proposed). Its sluice profile reads `sluice status <base> --all
  --json` and `sluice overview --json`, and passes over a branch sluice
  refuses, with sluice's words. After this change the refusal flume shows is the
  one at the revision flume's own fetch brought. So a clean push clears
  it in flume's next read with nothing of flume's changed.
- **crew** reads `sluice overview --json --watch` on its board, and so
  takes the change through what the watch shows.
- **Boomdrive**, **weir** and **timber** read no sluice policy. Reading
  sluice's policy is sluice's alone, so there is nothing to reconcile
  but flume's reading of a base, which this matches: a branch's
  revision as the host's own fetch last brought it, never a revision
  remembered from an earlier step.

## Goals / Non-Goals

**Goals:**

- A read of a branch outside a round reads its policy where 56, 124 and
  129 say: at the branch's revision as the host holds it.
- A clean policy pushed to a branch's remote ends sluice's refusal of
  that branch the next time a sluice command reads the branch, so
  submit and status take work again, and the overview shows the branch
  whole, without anyone running a round.

**Non-goals:**

- A fetch that no read makes today (D2).
- The reader's tick and intake, which record 0093 moves into sluice's
  host process and onto the forge (D2).
- The round's reads before `decide` (D4).
- How a policy is parsed, what it refuses, and the routing's narrow
  read (record 0069), which stand as they are.

## Decisions

### D1. Outside a round, the policy is read at the branch's revision as the host holds it

`Sluice::status` and `Sluice::branch_view_of` read the policy at
`Branch::held`. Where the host holds no revision of the branch, they
read it at the state's `base`. Where the state has seen none either,
status reads it at `Branch::read`, as it reads `at` today. Nothing is
fetched for the read, and nothing is written. The refusal names the
revision it was read at, as `policy_at` already does.

`at` stays the revision the state last saw, in status and the overview
alike: the one the queue was formed on (record 0046). The overview
already names `revision`, the host's revision of the branch, beside it
(64). The policy's fields a read shows are the gate of the revision the
next round will form on, since a round reads the branch and moves its
`base` there before it decides (`step.rs`, `decide`). These are status's
`parallel` and warnings, and the view's executor and required checks.
Where the branch moved since the last round, the combinations under
test were formed on a revision it has left, and that round forms them
again (121). The new revision's gate is the one they will be judged by.

The policy is still kept once per revision (129, `Policies`), so a read
at `held` reads git again only once `held` has moved.

Options weighed:

1. **As built: the state's revision.** A quiet branch is refused as long
   as its last round's revision is, and nothing short of a round clears
   it. That breaks 124's "read at the branch's revision on its remote,
   where a push clears it".
2. **Fetch for a status read.** Record 0046, the user's, declined this,
   at 0.75 s of a 0.9 s status on this repository's `main`. A view
   would pay it too.
3. **Read at `held` only when the state's revision is refused.** This
   is two readings, and a view whose gate comes from one revision while
   its refusal comes from another.
4. **Count a refused branch as in play, so sluice's host process runs
   its round.** The process would then never end while the refusal
   stood, fetching the branch at every tick, against 129 and 135. And a
   status on a host where no process runs would still refuse.

Option D1 reads 56, 124 and 129 as written. It is a read of a revision
the host already holds, so it costs no process (rule core.31).

### D2. A sluice command reads the branch where it reads it today, and nothing is added to bring the clean revision

This change adds no fetch, and the outcome rests on no tick and no
intake. A clean policy pushed to a branch's remote ends sluice's
refusal of that branch the next time a sluice command reads the branch,
each command reading it where it reads it today:

- **A submit** reads the branch fetched and judges the submission at
  the fetched revision (124). So the next submit after the push takes
  the work, whatever the host held before it.
- **Status, the overview, `sluice` with no command, and what a watch or
  a wait shows** read the branch at its revision as this host holds it
  (D1). They read it whole once any fetch on the host has brought the
  clean revision: a submit's read, a round's, flume's, or the person's
  own `git fetch`. Where the host held the clean revision already, as
  deneb did for nine days, the next read is whole.

So submit and status take work again without anyone running a round.
A quiet branch that nothing on the host has fetched since the push
still shows the refusal at the revision it last fetched, naming it,
until the next fetch. Its next submit takes the work.

The outcome asks nothing of a reader's tick or of intake. Under record
0093, nothing of sluice's runs for a quiet branch: the host process
ends once no queue has work in play (135). And no host takes a
branch's requests in: they are taken in only where the branch's
records are on its remote, by the forge's jobs (137). Intake reads the
queue through `status` on this base, so it takes D1 with the rest, and
this change asks nothing more of it. The change's tests show the
outcome by status, the overview and a submit, and hold no case of
intake or of a reader's tick. The reader-tick capability's policy read
once per revision stands as it is: it reads at the branch's revision,
which D1 makes the view's revision too.

Options not taken:

- **Status fetching where the policy at `held` is refused.** It would
  end the refusal at a status read even where nothing on the host has
  fetched. It departs from record 0046's ruling, the user's, that a
  status read fetches nothing. And the one read that takes work, a
  submit's, already fetches.
- **Something of sluice's fetching a refused quiet branch on its own.**
  Under 129 no branch is fetched on its own until a round forms on it,
  and under 135 nothing of sluice's runs while no queue has work in
  play. Either would need a clause amended, for a read a submit already
  makes.

### D3. The overview's order reads the default branch's policy where its view does

The order is read from the default branch's policy at the revision D1
reads that branch's view at. That is its `held`, then its `base`, then
its local revision, never fetching. The order still decides nothing,
and a policy it cannot read still leaves the branches in name order.
This keeps one revision for everything the overview reads of a branch's
policy. On the base the order could come from the state's revision
while the view's gate came from the host's.

### D4. The round's own reads stay

A round reads the branch, fetched for a branch on a remote, sets `base`
to it, and reads the gate there (56, 121). Before that, `perform` runs
what an earlier step left: `start_from` reads the gate at `base` for the
runs already decided there, and `perform_landing` compares the gates at
a landing's `from` and `to`. Each reads the revision its intent was
decided on, which is right for a combination formed on it. Each returns
before reading anything where nothing is starting and no landing is
left. So a quiet branch's round reaches `decide`, as the hand-run
`sluice step main` did. This change leaves them.

### D5. Where the decision is written

No clause changes: 56, 124, 129 and 135 already say it, as
agentplot/blueprints main words them at `b0df70b`. Record 0046 is
amended: status reads the queue where its state last saw it and the
policy at the branch's revision as the host holds it, with D1's options
and D2's. Record 0069's More information names that amendment as how a
quiet branch's refusal ends. Rule core.15's details rule out a read of
the policy, for a status, a branch's view or the overview's order, at
the revision the state last saw while the host holds another, citing
flywheel-next's `main` from 2026-10-28 to 2026-11-06 and the signal.

These are this change's construct stage's own, inside the rules, and
cheap to change. Record 0046's amendment keeps what the user decided
there, a status read that fetches nothing, and changes only the
revision the policy is read at. That the outcome rests on no tick and
no intake, and that the change keeps no case of either, follows the
planner's case for proposal 20 as the user approved it, reading record
0093 (accepted 2026-11-06; rules core.31 and core.33 as main words them
at `ed8474b6`).

## Risks / Trade-offs

- **A read's gate can be ahead of the queue it shows.** Where the branch
  moved since the last round, status's `parallel` and the view's
  required checks are the new revision's, while `at` and the
  combinations are the old one's. The overview names both revisions
  (64). The next round forms on the new one under that gate (121).
- **A quiet branch the host never fetches shows a refusal the remote
  has cleared** (D2). It names the revision it read, and a submit is
  never refused by it.
- **A run left starting, or a landing left unfinished, on a revision
  whose policy this sluice refuses fails each round before the round
  reads the branch** (D4). This needs a sluice that refuses the policy
  to arrive between a round's decision and its start or its landing. No
  quiet branch reaches it, and this change does not touch it.
- **Tests of the view that relied on the state's revision for the gate**
  read the host's revision now. The tasks name the suites that read
  both.
