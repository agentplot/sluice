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
cites for "said once". Passing a branch by does not change the run's
exit.

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

### D4. Rounds pass other branches they cannot read

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

### D5. The default branch's refusal is its own

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
- [Words carrying a revision change each pass and are said each pass]
  → The words are the policy read's, naming a revision that changes
  only when the branch moves.

## Migration Plan

None: no state, config or document changes shape.
