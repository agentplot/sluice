# Design

## Context

See the proposal for what happened on deneb and what clause 124 says.
What the code does today, on this branch:

- `protect::hook` (`crates/sluice/src/protect.rs`) takes each updated
  `refs/heads/<branch>` and reads its current value with
  `seams.branch.read(branch)`. The branch seam is `Branches`, which
  routes a branch whose policy names a remote to `PushedRefs`, whose
  `read` is a fetch. So the hook's "current value" of a bolt on GitHub
  is GitHub's revision, and any transaction that leaves the host's ref
  where it is, but not at GitHub's revision, is refused: `git reset
  --hard` to HEAD, `pack-refs`' packed write and its prune (`pruned`
  compares `packed-refs` with that same remote revision). That is the
  `from 8079db3 to 2111d21` in the bolt's log. It also means each ref
  transaction on the host fetches.
- `main.rs` opens the repository for every command, the hook's among
  them, with `Bound::open(current_dir)`, and `pruned` finds the common
  directory by running git in `current_dir` with `GIT_DIR` removed. git
  starts the `reference-transaction` hook in the folder the command was
  run from, with `GIT_DIR` set to the repository it names: a `git
  --git-dir=/srv/x.git fetch` run from `/tmp` starts it in `/tmp` with
  `GIT_DIR=/srv/x.git`, and from inside the bare repository with
  `GIT_DIR=.` (checked with git 2.54). So from outside, the hook cannot
  open a repository, exits 3, and git aborts every ref update of the
  fetch (swancloud's case); from inside another repository, it would
  judge the fetch's refs by that repository's policy.
- The hook prints `refusal(branch, why)` while it walks the updates and
  only reads the caller, `caller()`, afterwards for the log; the words
  to the user name neither the command nor the revisions.
- `PullRequests::open` (`profiles/github/pulls.rs`) always runs
  `push_branch(session.head, session.from.worktree)`, a forced push of
  the worktree's branch, and opens or updates the request from it.
  From a worktree on `bolt/x`, that is a forced push to a branch sluice
  lands on.
- `Sluice::submit` (`sluice-core/src/ops.rs`) takes the driver's
  `session.head`, which from a worktree on `bolt/x` is the host's
  `refs/heads/bolt/x`.
- `PushedRefs::move_to` and `make` call `self.local.set(branch, to)?`
  after an accepted push. A failure there, as on the ref's lock, turns
  an accepted landing into an `Err`; the next round settles the landing
  from the remote but never sets the local ref again. No other code
  sets it, so on a host that did not land, the local ref never moves.
- `decide` (`sluice-core/src/step.rs`) already knows, after the 121
  check, the revision the branch must contain (`line.doc.landed`) and
  that the revision read contains it. That is the point where a
  levelling belongs.

Constraints: rules core.1 and core.2 (the core reaches the copy only
through the branch seam, with no profile-specific branch), core.4 (the
submit operation writes no branch ref), core.5 (a ref is moved only
from an expected value), core.11 and core.14 (what a reader needs is in
the records), core.20 (no worktree sluice did not make is moved).

## Goals / Non-Goals

**Goals:**

- The hook judges by the host's own ref in the repository git names,
  and never touches the network.
- A submit from a worktree on a branch sluice lands on writes no ref of
  it, anywhere, and submits the remote's revision.
- Every host's copy follows the landings its rounds read, a failed set
  is retried, and the branch's state says when a copy lags and why.

**Non-Goals:**

- Moving a worktree's files or index with its copy (ADR 0024).
- The overview showing the copy; `sluice status <branch>` alone does.
- Changing how a branch kept in the host's own refs is landed: there the
  host's ref is the branch, and it has no copy apart from it.

## Decisions

### D1. The branch seam answers for the host's copy, and levels it

Two methods on `Branch` (B.4):

- `copy(branch) -> Result<HostCopy>`, where `HostCopy` is `Itself`, the
  branch has no copy apart from it on this host, or `At(Option<Rev>)`,
  the host's ref and its value. Local refs answer `Itself`. Pushed refs
  answer `At(local.read(branch))`, reading `refs/heads/<branch>` with no
  fetch. `Branches` routes it like `read`. The fakes keep a copy per
  branch for a branch marked as on a remote, and can be told to fail the
  next levellings with a given error, so the conformance suite can play
  S64.
- `level(branch, expected: Option<&Rev>, to: &Rev) -> Result<Moved>`:
  set the copy to `to` only while it is `expected` (absent when
  `None`), with gix and a reflog message `sluice: level <branch> to its
  landing`, as `LocalRefs::edit` does with `PreviousValue::MustExistAndMatch`
  or `MustNotExist`. A copy found elsewhere is `Moved::Elsewhere`, never
  forced (core.5). Local refs never receive it: the core calls it only
  for `HostCopy::At`, and local refs bail if it is called.

`PushedRefs::move_to` and `make` stop calling `local.set` after the
push: an accepted push is `Moved::Moved`, whatever the copy does next
(122). `remove` keeps deleting the local ref and the tracking ref once
the remote's branch is gone, since the branch no longer exists to have a
copy of; a failure there is the error it is today.

*Alternatives:* level inside `PushedRefs::read`, which would need the
records' landing in the profile and would write a ref on every read,
including the submit's (core.4); keep the set in `move_to` and add a
retry, which leaves other hosts' copies never levelled.

### D2. The round levels the copy, and only the round

In `decide`, once the 121 check passes (the branch is not held), the
round reads `seams.branch.copy(branch)`. For `At(copy)` with `copy !=
Some(landed)`, where `landed` is `line.doc.landed`, it calls
`level(branch, copy, landed)`. The landing confirmation in
`perform_landing`, after it writes `landed`, does the same for the
landing's `to`. Nothing else levels: not the submit, not `sluice
status`, not the hook. A waiting submit or `sluice wait` levels only
through the rounds it runs, as any process may run a round (core.4).

124 has the levelling tried "each round and on each read of the branch".
The reads of the branch that level are the rounds' reads, `sluice step`,
`sluice run` and the rounds a waiting submit or `sluice wait` runs, so
every read that can level is a round's. The submit's own read of the
branch, for the head it submits (D4), never levels, since nothing a
submit does writes the copy (124); nor does `sluice status`, which reads
the records (core.14), nor the hook, which judges a write and makes none.

Levelling targets `landed`, the landing the records say the branch
holds, not the revision read: a hand's commit on top of the landing,
which the remote's protection allowed (121, S8), is no landing, and the
copy stays at the landing until sluice lands on top of it. A held
branch levels nothing (121); once the operator accepts a revision,
`landed` is that revision and the next round levels the copy to it.

The levelling runs under the branch's lock the round already holds; it
reads the copy's value immediately before it sets it, so a copy moved
between, which the hook refuses for any hand, is found elsewhere and
tried again next round.

*Alternative:* level also on `sluice status`; rejected because `follow`
in a waiting pull-request submit calls `Sluice::status`, which would put
a ref write in the submit's own path, and because a read that writes
breaks the reader/round split core.14 keeps.

### D3. A copy left behind is in the branch's records, per host

`BranchDoc` gains `copies: BTreeMap<String, CopyBehind>`, keyed by the
instance's name (`self.peer.name`, the name run claims use), skipped
when empty. `CopyBehind { at: Option<Rev>, lacks: Rev, why: String,
since: String }`. A levelling that fails or finds the copy elsewhere
writes this host's entry in the round's own write, and a log line
`copy-behind { host, at, lacks, why }` once per distinct `(at, lacks,
why)`, as `held` is written once per revision read. A round that finds
the copy at `landed`, or levels it, removes this host's entry and logs
`copy-levelled { host, at }` when there was one. A branch whose records
are on its remote thus carries each host's lagging copy, and no host
overwrites another's.

`Status` gains `copy: Option<CopyBehind>`: this host's entry. The text
of `sluice status <branch>` prints, above the queue, `this host's
bolt/x is at 10e4791, behind the landing c43609b: <why>`; `--json`
carries it under `copy`. The `why` is sluice's sentence with git's or
gix's error after a colon, such as `git holds the lock on
refs/heads/bolt/x`.

*Alternative:* keep it in the host's own state, beside the session
count; rejected because the branch's state is where status reads, and
core.14 wants what a round observed in the records.

### D4. A submit from a worktree on a branch sluice lands on submits the remote's revision

A worktree's branch `b` is one sluice lands on when `Sluice::protects(b,
at)` holds at `at`, the branch seam's `read(b)`. `Sluice` gains
`submitted_head(session) -> Result<SubmittedHead>`: for such a `b` whose
`copy(b)` is `At(_)`, the head is `at` and, where the worktree's head
differs, a `CopyDiffers { copy, remote: at, how }` with `how` behind,
ahead or apart, read with `contains` both ways; for any other worktree,
the head is `session.head` and nothing differs. The driver's
`Session` is unchanged: the core asks the branch seam about the name
the driver gave, and a name that is no branch, such as `pull/7`, reads
`None` and keeps the session's head.

- `Sluice::submit` uses it for the submission's head and its published
  head, and `Accepted` carries the `CopyDiffers`; the CLI prints `this
  host's bolt/x is at 10e4791, behind origin's 8079db3; submitted
  8079db3` and `--json` adds `copy`.
- `PullRequests::open` uses it too. For a protected `b`, it skips
  `push_branch`, finds or opens the request with `head_ref = b`, and
  uses the remote's revision wherever it now uses `session.head`: the
  messages for a new request's title, the withdrawal of an older head's
  entry (67), the ready mark. `Opened.said` drops `pushed <b>` and adds
  the copy's line. For any other `b`, it pushes as today.

No `remote` named in the worktree branch's policy means `copy(b)` is
`Itself`: the host's ref is the branch, the worktree's head is it, and
the submit takes `session.head` as today.

*Alternative:* refuse a submit whose copy differs; 124 says a copy
behind or ahead refuses no submission.

### D5. The hook reads the host's ref in the repository git names, and says what it refused

`protect::hook` replaces `seams.branch.read(branch)` with the host's
value: `copy(branch)`'s `At(v)`, or `read(branch)` for `Itself`, which
for local refs is the host's ref already; neither fetches. `None` is a
creation and passes, as today. `protects(branch, &cur)` reads the
policy from that value's tree, a local read. `pruned` compares
`packed-refs` with that same value, so git's prune of a loose file it
has just packed passes on a copy apart from the remote.

`caller()` is read once, before the updates are walked, and the words
printed for each refused update become:

```
error: bolt/x is protected by sluice; submit the change to sluice instead
  `git update-ref refs/heads/bolt/x 8079db3` would move it from 10e4791 to 8079db3
  sluice levels this host's bolt/x to each landing itself
  the worktree may hold the attempted change: git merge --abort or git reset --hard
```

The third line only for a branch whose `copy` is `At`; for a deletion
the second reads `would delete it at <rev>`, followed by what still
needs it. git prints its own `fatal:` after the hook exits; sluice's
lines come first because the hook writes them before it exits 1. The
`protected` log line is unchanged in shape, its `from` now the host's
value.

The hook opens the repository git names. In `main.rs`, the hook's
command opens `Bound` at `$GIT_DIR`, resolved against the working folder
when relative, and at the working folder only when git set no
`GIT_DIR`; `gix::open` takes a git directory, bare or a worktree's, as
it takes a work tree. `pruned` reads the common directory from the
branch seam's `common_dir()` of that repository rather than running git.
The hook makes no git call of its own that discovers a repository from
the working folder.

A transaction the hook cannot read for a reason of its own passes,
said once: opening the repository, reading a branch's value, its policy
or what still needs it, failing with an error, the hook prints one line,
`warning: sluice's branch protection let this through unread: <why>`,
and exits 0, however many refs the transaction holds. A refusal is still
exit 1. A ref outside `refs/heads/`, and a branch whose policy at its
value does not name it protected, is never refused, as today.

*Alternatives:* have the hook compare the transaction's `<old-value>`
field; git passes zeros for an update with no expectation, which most
commands make, so the ref's value must be read. Have the hook refuse a
transaction it cannot read; 124 says it passes, said once, so that a
fault of sluice's never aborts git's own write.

### D6. One ADR

The next free ADR number records D1 to D5: the host's copy as a seam
answer; the round as its only writer, to the landing; the records'
per-host lag; the remote's revision as the submission from a worktree
on a branch sluice lands on, with no push; the hook judging by the
host's value in the repository git names, passing what it cannot read.
Rule core.4's Details gain "a submit pushing, or writing
the host's copy of, a branch sluice lands on (124)" under Rules out and
"a pull request's submit pushing the session's own branch, which sluice
lands on nowhere (124)" under Not ruled out; core.20's Details gain "a
round levelling the host's copy under such a worktree (124)" under Not
ruled out. Sources name 124, S58, S64.

## Risks / Trade-offs

- [A worktree on the bolt has its HEAD moved by every levelling, so
  `git status` there shows the landings as staged reverts, as on the
  landing host today] → ADR 0024 already says not to check such a
  branch out; status, overview and protect keep naming the worktree.
  The S58 test checks the worktree's files and index are unmoved.
- [A copy with commits a hand made before the protection, ahead of the
  landing but holding it, is moved back to the landing] → those commits
  stay in the reflog and in any branch that has them; the round logs
  nothing for it, so the ADR says it, and a copy ahead with landings the
  remote lacks is held by 121 before any levelling.
- [The submit from a bolt worktree whose copy is ahead submits the
  remote's revision, leaving the copy's commits out] → that is 124's
  ruling; the submit says the copy is ahead with both revisions, so the
  operator sees it.
- [A transaction the hook cannot read passes, so a broken policy on a
  protected branch leaves it unguarded on that host until it is fixed]
  → 124 rules it so; the warning names why on every such write, and the
  forge's rules still guard a branch on a remote.
- [git's own `fatal:` line still follows sluice's words] → git prints it
  after a hook refuses; sluice cannot suppress it and says so first.
- [Each host writes its copy's lag into shared records] → only on a
  failure and once per distinct cause; a host that never fails writes
  nothing new.

## Migration Plan

Nothing to migrate: `copies` is absent in older records and reads as
empty. A host whose copy lags today is levelled by its next round once
this sluice runs there, the deneb bolts' included, unless the branch is
held.
