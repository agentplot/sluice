# Design

## Context

What deneb's records hold for swancloud's `main` on 2026-11-06 (the
proposal gives the timeline):

- The runner's copy, `…/flywheel/repositories/afterthought/swancloud/swancloud.git/sluice/main/log.jsonl`:
  `submitted` #20 at 12:38:39 (head `68a1b49a`); `moved` `6f35bad` to
  `562606d` at 13:01:10 with `testing` `[20]` on it at once; `withdrawn`
  #20 `by: swancloud-ops` at 13:01:33. `finished.jsonl`: `{"id":20,
  "state":"withdrawn","worktree":"pull/330"}`.
- The person's copy, `~/Code/github_afterthought/swancloud/swancloud.git/sluice/main/log.jsonl`:
  `submitted` #425 at 12:39:12 (the same head); `landing` and `landed`
  `[425]` to `562606d` at 13:01:05 to 13:01:08; `delivered` `425:landed`
  at 13:01:40.
- `562606d`'s parents are `6f35bad` and `68a1b49a`; its trailers name
  `Sluice-Submission: 425`, `Sluice-From: swancloud-ops (pull/330)`.

Three faults of sluice's own, each against its clause:

1. **The round formed work the branch held** (`crates/sluice-core/src/step.rs`,
   the round after `Moved`): a round that reads the branch moved by
   another hand forms every candidate again on it (10, 24), and nothing
   asks whether the revision read already holds an entry's head. `git
   merge-tree` of a head the base holds gives the base's tree, so the
   combination is the branch with nothing added: tested, then landed as
   an empty landing commit in the entry's name. 122 makes that entry's
   outcome landed already; 6 and 22 form and land no such combination.
2. **Intake withdrew a landed request in its submitter's name**
   (`pulls.rs`, intake's pass, "A submitted pull request closed, back in
   draft or on another head leaves the queue"): a request absent from
   the open list is withdrawn as `Requester::Session(sub.from.agent)`.
   GitHub drops a request from the open list once the branch holds its
   head, which C.5 calls the forge marking it merged after sluice's
   push. Its submitter withdrew nothing (5), and its outcome is landed
   (122).
3. **The wait reported a withdrawal of work that landed, and read a
   session where no work tree is.** `wait_on` in `crates/sluice/src/lib.rs`
   ends at the first state out of the queue. `report_delivering` asks
   the driver who works in the wait's directory; the worktrunk driver's
   `session` runs `git status`, which git refuses in a bare repository.

### Looking across

- **crew** waits on a landing with `sluice wait <id> --timeout 9m --json`
  in the kit's main checkout (`plugin/lib/plan.py` `WAIT`), or `sluice
  wait main` from the landing's worktree for a request, and reads the
  outcome by its exit, 4 being withdrawn (`LANDED`, `landing_said`). On
  deneb the kit's main checkout is the runner's bare clone. crew needs
  no change: sluice's exit and record become right.
- **Boomdrive**, the runner, keeps a bare clone of each repository under
  `~/.local/share/flywheel/repositories/`, and the person keeps their
  own. One copy per repository per host is Boomdrive rule core.14, with
  the user now; sluice builds nothing of it here.
- **flume**'s places are worktrees, and it reads a submission's outcome
  from `sluice status --json` and the landing commit's
  `Sluice-Submission`. No tool of the family reads a session where no
  work tree is: the worktrunk profile is the one reader, so the fix is
  its alone.

## Goals / Non-Goals

**Goals:** each copy's records say landed for work its branch holds,
never withdrawn; no combination of work the branch holds is formed, run
or landed; a wait whose submission's work goes on reports where that
work went; a delivery read where no work tree is runs no `git status`
and says so.

**Non-goals:** one copy per repository per host; telling two copies of
one repository apart in the intake mark; one notice for one landing
across two copies.

## Decisions

### D1. The round lands work the branch already holds

Before it chooses anything to land beside the chain or forms the chain,
a round reads each submission in the queue, not landing, that no
combination formed on the branch's revision holds: the ones it is about
to form, once each. One whose head that revision holds is landed:
`state` `landed`, `landed_as` the commit that landed it, and
`landed_elsewhere: true` on its record; its outbox item is `<id>:landed`
with that commit, so the round delivers it as any landing (51); and the
log says `{"event":"found_landed","ids":[…],"to":<commit>}`. It is then
out of the queue, so the chain holds no combination of it: one under
test is dropped and its checkout released as any obsolete one is (10).

The commit that landed it is the oldest of the branch's first-parent
line that holds its head, and the head itself where it is on that line:
the new `Branch::landed_by`, in git `git rev-list --first-parent
--ancestry-path <head>..<tip>`, the last line, unless that commit's
first parent is the head. A profile that cannot walk the line answers
the revision read, which holds it all the same.

Only the submissions about to be formed are read, so a round with every
submission formed reads none, and a round reads each once per revision
it forms on. A submission whose head is on the branch when it is
submitted, from a worktree or a request taken in after another queue
landed it, is landed at its first round in the same way.

Options weighed:

- **At the round, from the branch** (chosen): the branch is what 122
  reads, the round is the one step that forms (6), and it holds the
  branch's lock (core.14).
- **At intake, when a request leaves the open list:** covers requests
  alone, and only those whose request GitHub has closed; a worktree's
  submission and a request still listed would be formed all the same.
- **In `arrange`, as each candidate is formed:** `choose_beside` reads
  unformed submissions first and would land work the branch holds beside
  the chain, as an empty landing commit, before `arrange` saw it.
- **Record it under the existing `landed` event:** that event is this
  queue's landing, and 121 reads its `to` as the last landing sluice
  recorded. A landing this queue did not make is no such landing.

### D2. Intake withdraws no request whose work the branch holds

A submitted request absent from the forge's open list is withdrawn only
where the branch, as this pass fetched it, does not hold its head. Where
it holds it, the request left the list because the branch took its head
(C.5: the forge marks it merged after the push): its entry stays, and
the round lands it (D1). A request closed by a person, or merged by hand
without its head reaching the branch, a squash or a rebase, is withdrawn
as before. Nothing more is asked of the forge.

Options weighed: asking the forge whether a request gone from the list
was merged, and leaving it one more pass for the branch's next fetch, is
a decision held in a process's memory between passes, lost between
steps and waits (core.14), and a read of a request per pass (core.32).

### D3. A wait follows the work its submission carried

A wait, a waiting submit and `sluice wait <id>`, reading their
submission withdrawn, look for where its work went, in order:

1. **The branch holds its head:** the outcome is landed (122), reported
   as its record with `state` `landed`, `landed_as` the commit that
   landed it and `landed_elsewhere`, and exit 0. The record on disk is
   not changed: a withdrawal is final (5), and 67 keeps it readable as
   withdrawn.
2. **A later submission from its place holds its head**, as a
   resubmission does (67): the wait says `#<a> withdrawn by <who>: #<b>
   carries its work, so this wait follows it`, and follows `#<b>` to its
   outcome, reporting `#<b>`'s record.
3. **Its request carries it**, for a pull request's submission: its
   latest submission holds the head; or the request is open and no
   draft on a new head that holds it, waiting to be taken in again after
   its submit's new head; or it is open on its own head after this host
   handed it over, or another host takes it in (137). The wait says so
   and follows the request as `sluice wait <branch>` does, to its
   submission here or to what the forge shows of it there. A request
   left open on its own head by its submitter's, its planner's or the
   operator's withdrawal is rested (120), and the wait reports it
   withdrawn.

A branch or a request the wait cannot read leaves the outcome as the
records hold it, and the wait says what it could not read.

Otherwise, a withdrawal by its submitter, its planner or the operator
with no work going on, it is reported withdrawn and exits 4.

`sluice wait <branch>` from a request's worktree reads a closed request
whose latest submission's head the branch holds as that submission
landed, by the same commit, never as "closed without landing".

Options weighed: rewriting a withdrawn record to landed when the branch
later holds its head contradicts 5 and 67, and a record decided twice
would be delivered twice (51).

### D4. A repository with no work tree is read with no `git status`

The worktrunk driver's `session` asks `git rev-parse
--is-inside-work-tree` first. Where it is not `true`, a bare repository
or a checkout's git directory, the session is `bare`: no `git status`,
which git refuses there; nothing held beyond its commits, since there is
no work tree to hold anything (2); and no path. A submit there is read
as one from that repository's `HEAD`, refused or taken as before for its
branch.

A delivery read there finds no session working in it. `report_delivering`
and `sluice inbox` read it as `Recipient::Bare`, deliver nothing, and
say on stderr: `sluice: #<n> stays pending for its session's sluice
inbox: <dir> is a repository with no files checked out, so no session
works in it and no outcome is delivered there`, and the inbox the same
words after `sluice: <dir>`. The outcome stays pending for its session's own carrier (51).

Options weighed: a driver error with plainer words still reports a
failed read where nothing failed; a session marked `detached` would give
a submit there detachment's refusal, which says the wrong thing.

### D5. Two copies of one repository on one host

The intake mark names a host, `[peers] name` or the hostname (137), and
intake compares `mark.host == self.peer.name` (`pulls.rs`). Two copies
of one repository on one host each read the mark as their own, so each
takes the branch's requests in: two queues of one request, each
testing it, and whichever lands first pushes its landing, as #425 did.
137 binds hosts; two copies on one host are, in its words, one host.

The readings:

- **(a) The copy is the host.** One copy per repository per host, so
  the mark's host names one queue. Boomdrive rule core.14; the user's
  design, now with them. Nothing in sluice changes but what the design
  names.
- **(b) The mark names the copy.** The mark names the host and the
  copy, by a digest of where its records are, so a second copy reads it
  as another holder's: it takes nothing in, hands its requests over
  (137), and its waits follow the request on the forge. A change to
  137's reading and to the mark's form, which every sluice reads (C.6);
  the statement first, on the user's word (core.12).
- **(c) A second copy refuses.** A copy that finds another copy of its
  repository on the host taking the branch in takes nothing in and says
  so. Needs a host-wide list of copies sluice does not keep.

Built here, correct on sluice's own terms whichever is chosen: each
copy's records say landed for work its branch holds (D1, D2), and its
waits answer landed (D3). Left for the one-copy design: which copy takes
a branch's requests in; a request tested in both copies; one landing
told to its session twice, under each copy's submission number; and a
landing pushed by the other copy between one pass's fetch and its list
of open requests, which that pass reads as a closing and withdraws,
though a wait on it answers landed.

### D6. One fetch at a time in a repository

The runner's copy logged `read-refused` twice on `main`, at 12:57:13Z
and 13:56:33Z: `cannot lock ref 'refs/sluice-fetched/origin/intake/main':
is at <a> but expected <b>`. Two of its processes fetched at once, a
waiting submit beside a Flume settle's `sluice step` or a reader, and
the one that lost read its own lock's words as the remote's refusal
(`fetch_specs`), logged it (133) and never tried again (134). After
#23 landed as `f24499c9`, one such fetch failed a command, exit 3.

Each fetch, in `PushedRefs::call`, holds the lock of `sluice-fetch.lock`
in the repository's git directory for its try alone, so sluice's
fetches of one repository, in any of its worktrees, run one after
another, and none holds the lock over the retry policy's pause. A fetch
beside a git that holds no lock, a person's or an older sluice's, can
still lose: git's `cannot lock ref '<name>'`, or a ref's lock file
standing, names a ref of this repository and never a remote's, so
`judged` reads it as a call the remote never judged, tried again, and
`fetch_specs` reads one still lost as nothing refused, the refs holding
what the other fetch brought. Record 0086 weighs fetching into refs of
each process's own, which every reader of the fetched refs would have
to tell apart.

## Risks / Trade-offs

- **A round reads containment for each unformed submission** → one
  `git merge-base --is-ancestor` each, and a `rev-list` for one the
  branch holds; a round with everything formed reads none.
- **`sluice wait <id>` may report another submission's record** →
  the wait says which it follows, and the record names its own number,
  so a reader of `id` reads the submission that decided the work.
- **A wait on a request handed over now waits** where it exited 4 →
  it follows the request on the forge, as 137 has a wait on a request
  another host takes in do, to that host's outcome.
