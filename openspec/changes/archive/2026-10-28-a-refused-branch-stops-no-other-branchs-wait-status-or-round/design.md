# Design

## Context

Every read of a branch's state goes through the records profile, which
first routes the branch by its policy (`profiles/records.rs`, `route`):
a branch whose policy is refused at any revision the host holds fails
every read, lock and write of its state (124; record 0069 describes the
deneb case). The overview already confines that failure: it reads each
branch with `branch_view_of` and turns any error into
`BranchView::unread(branch, &e)`, the words `format!("{e:#}")`, failing
whole only when `state.branches()` fails (`sluice-core/src/overview.rs`,
`overview`). Nothing else does. These read across branches and stop on
the first one they cannot read:

- `Sluice::find` (`sluice-core/src/lib.rs`), which `state`, `landed_as`,
  `amend` and `withdraw` (`ops.rs`) call, and `moved_to` (`ops.rs`),
  which loops the branches itself. `sluice wait <n>` and `sluice status
  <n>` call `state` then `moved_to`; a wait's `Reading` reads its
  submission through `record`, which calls `state`.
- `Command::Run` (`sluice/src/lib.rs`), whose per-branch
  `s.status(&branch)?` and `s.round(&branch)?` end the run.
- `capacity` under `Serves::Every` (`step.rs`), which reads every other
  branch's line, so a round of `main` under `sluice run` fails on
  `bolt/b`.
- `check_host_systems` (`step.rs`), which reads every branch's tip and
  policy before `sluice run`'s first pass.
- `release_abandoned` (`session.rs`) and `release_unusable_previews`
  (`preview.rs`), which every round, `sluice exec`, `sluice preview`
  and `sluice previews` run first, and which take the lock of, and
  log on, the branch of each checkout they release.
- `signed_remotes` (`profiles/records.rs`), which reads the default
  branch's policy to list the remotes a state may be on, so a refused
  default branch fails `branches()` itself.

## Goals / Non-Goals

**Goals:**

- One rule for every cross-branch read, the overview's: any error from
  reading one branch is that branch's, carried with its words; only the
  list of branches failing is the whole command's.
- The unfound case says which branches were not read, never that the
  submission does not exist.

**Non-Goals:**

- Making the refused branch itself readable (record 0069's unit).
- Telling apart, inside a branch's failure, a refused policy from a
  refused revision or unreadable state: the words say which, as the
  overview's do.

## Decisions

### D1. The lookup returns what it could not read

`find(id)` becomes a lookup that reads every branch it can and returns
both what it found and what it passed:

```rust
pub struct Unread { pub branch: String, pub refused: String }
pub enum Found { On(String), Unfound(Vec<Unread>) }
```

A branch is passed when `read_line` fails, or, for a number its live
records list as finished, when `finished_sub` fails. `refused` is
`format!("{e:#}")`, as `BranchView::unread` words it. `state`, `landed_as`
and `moved_to` keep their `Option` answers for their many internal
callers, passing unreadable branches by. A new `Sluice::lookup(id) ->
Result<Lookup>`, `Lookup::Found(Submission) | Lookup::Unfound(Vec<Unread>)`,
does what `sluice wait <n>` and `sluice status <n>` do today, `state`
then `moved_to`, and is what they and the wait's `record` call.
`AmendRefusal::NoSuchSubmission` and `WithdrawRefusal::NoSuchSubmission`
carry the `Vec<Unread>`; empty means every branch read.

Alternative: make `find` fail with an error naming the unread branches
when it finds nothing. Rejected: `state`'s callers that ask after a
submission just made, or ask who may change one, would then fail on
another branch's refusal, which is what clause 26 rules out.

### D2. Unfound among the readable branches is exit 3

Words, text: `submission <n> is on none of the branches sluice could
read`, then one line per branch `<branch> could not be read: <words>`,
the overview's own form. `--json`: `told::tell` with `Kind::Internal`,
the message the same lines joined by `; `, next `sluice overview`. Exit
3 (`INTERNAL`). One helper words it for wait, status, amend and
withdraw, beside `no_submission`.

Exit 1 is "no such submission", and an agent's loop that reads exit 1
as the submission gone would stop following one that is only unread
(the deneb wait was exactly that submission's agent). Exit 3 is what a
command that could not read a record already exits, and the exit-code
reference says so; the help's exit table for `wait` says it too.

### D3. sluice run passes a branch by, said once

In `Command::Run`'s loop, a failure of `s.status(&branch)` or of
`s.round(&branch)` is caught for that branch, and the loop goes on. A
`BTreeMap<String, String>` beside `said` holds the words last said per
branch; the line `<branch> could not be read, passed by: <words>` is
written to stdout when the words differ from those held, and a branch
whose status read and round succeed leaves the map, so a refusal after
it is said again. This mirrors `tell_hold` (121), which the clause
cites for "said once".

The round's failure is caught too, not only the status read's: a round
reads the branch's policy at its tip, which the status read (at the
revision the state last saw) may not have read, and whatever else fails
in one branch's round is that branch's alone under core.14. Each round
already logs what it can on its branch (133), so nothing is lost by
going on.

Alternative: mark the reads that can be refused with a typed error, as
`read_refused` marks a remote's refusal, and pass by only those.
Rejected: every seam read would need the mark, and a failure it missed
would still end every branch's run.

### D4. A bounded run ends as the last pass's failed step would

Clause 26 says a run bounded to a number of rounds that passed a branch
by in its last round ends as a step of that branch would, naming the
branch. A failed `sluice step` prints `sluice: <words>` on stderr and
exits 3 (`main.rs`, the `Err` arm of `run`). So the loop keeps, per
pass, the branches it passed by in that pass with their words, apart
from the said-once map; when `--rounds` is reached and that list is
not empty, it writes `sluice: <branch>: <words>` for each, in branch
order, through `telemetry::error` as `main` writes a step's failure,
and returns `Ok(INTERNAL)`. An empty list returns 0 as now.

- Only the last pass counts: a branch passed by on an earlier pass and
  read on the last is no failure of the run's, as a step run then would
  have succeeded.
- The stderr line is written whether or not the stdout line was said
  in that pass: the said-once rule is for the output a person reads;
  the exit is for whatever started the run.
- Interrupted by Ctrl-C, the run exits 130 as now; a branch whose read
  Ctrl-C ended is not passed by (the existing `INTERRUPTED` arm).
- An unbounded run never reaches the bound; it only says it once while
  it stands, and its exit is unchanged.

`Ok(INTERNAL)` rather than an `Err`: the run did its work for every
other branch, and the in-process tests read the exit without the
error path; the words still go where a step's go.

Alternative: one line on stderr joining every branch. Rejected: a step
names one branch per line, and a log reader greps by branch.

### D5. Rounds pass other branches they cannot read

- `capacity`: a branch whose line cannot be read adds nothing. A run
  this instance marked starting there has, once started, its checks in
  `tests.running()`, which the bound already counts.
- `check_host_systems`: a branch whose tip or policy cannot be read is
  skipped; its own round, when it reads, uses the same policy.
- `release_abandoned` and `release_unusable_previews`: a failure to
  take the place's branch's lock is treated as `None`, a busy lock: the
  place is left for a later pass, nothing freed, nothing logged, and a
  warning names the branch and the words. A failure of the log write
  after a place was freed is warned and the place is reported as freed
  (the profile already let it go); the rest go on.

### D6. The default branch's refusal is its own

`signed_remotes` takes the default branch's policy only when it reads;
when `policy_of(&default)` fails, it lists the remotes this host has
fetched a state from (`fetched_state`), the second half it already
reads. A remote named only by a refused default policy that this host
has never read a state from is not listed; the default branch's
refusal then shows wherever the default branch is read, the overview
and an unfound lookup's list among them.

## Risks / Trade-offs

- [A host fault, not a branch's, now said once per branch by `sluice
  run` instead of ending it] → Each line carries the words, so the
  operator sees the same cause on every branch; the run keeps no
  branch from moving that could.
- [A remote known only through a refused default policy is unlisted
  until this host reads it] → Only on a host that never read a state
  from that remote; record 0069's unit, which reads the remote and
  signers alone, lists it again.
- [A bounded run from cron exits 3 every time while a branch stays
  refused] → That is the clause's intent: whatever started it learns
  from the exit, as it would from a step of that branch; stderr names
  the branch, and every other branch moved in the same run.
- [Words carrying a revision change each pass and are said each pass]
  → The words are the policy read's, naming a revision that changes
  only when the branch moves.

## Migration Plan

None: no state, config or document changes shape.
