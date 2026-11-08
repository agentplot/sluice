# Design

## Context

### What the statement asks

51: an outcome is delivered once, "whichever of the push, a harness's
collect and a reporting wait comes first marks it delivered", and a
session that starts "collects every outcome pending for the entries it
submitted, found by the submitter the entry recorded, by `sluice
inbox`". 50: notify delivers "to that session and no other". 2 and 3: a
place on no branch names no session's work; what a place holds beyond
its commits is read from its files. Nothing in the statement says a
folder with no files checked out is no session's place, and nothing says
a delivery read needs `HEAD`'s commit.

### What sluice does today

At 68d68a9a, the bolt `sluice-tells-each-submitter-its-own-end`:

- **The delivery read asks the driver for a submit's session.**
  `recipient` in `crates/sluice/src/lib.rs` calls `Driver::session(dir)`
  and reads three cases: `To(me, to)`, `Detached` and `Bare`. `to` is
  `me.session`, else `me.path`, else `me.worktree`.
- **The worktrunk driver's `session` reads `HEAD`'s commit first**
  (`git rev-parse --verify HEAD`) and fails where `HEAD` names a branch
  with no commit. Since record 0085 it then asks `git rev-parse
  --is-inside-work-tree`. Where the answer is not `true` it runs no
  `git status`, reads no path, and sets `Session::bare`
  (`crates/sluice/src/profiles/worktrunk/mod.rs`).
- **`Bare` delivers nothing.** `delivering` says, for a pending
  submission not by name, "sluice: #<n> stays pending for its session's
  sluice inbox: <dir> is a repository with no files checked out, so no
  session works in it and no outcome is delivered there". `inbox` says
  "sluice: <dir> is ..." the same, prints `{"outcomes":[]}` under
  `--json`, and exits 0. The change a-submit-names-its-head-from-no-worktree,
  task 4.3, kept the line off a submission by name.
- **Whose a submission is** is one reading in the core, `Here::made`
  (`crates/sluice-core/src/overview.rs`; record 0090's D6), which
  `Sluice::pending` and `Sluice::pending_one` call with
  `Here::on(&me.worktree)`. It takes a submission naming no head whose
  work is the branch, or a pull request's from the branch where sluice
  lands on it nowhere, and never one by name. Nothing in it needs a
  folder's files.
- **Nothing reads `Session::bare` but `recipient`.**

Signal 2026-11-07-agentplot-ops-d4947942/02 is this read under sluice
0.68.1, before record 0085: crew's `sluice wait` for #2, run in the bare
`sluice.git`, ran `git status` and said "could not read whether #2 is
delivered, so it stays pending for sluice inbox". #2 was a pull
request's entry from a bolt into agentplot/sluice's `main`, which takes
pull requests: pending in no inbox, its carrier the request (124, C.5).

### Looking across

| tool | where it reads delivery | what this change gives it |
|---|---|---|
| crew | its landing's waiter runs `sluice wait <n>` in the kit's main checkout (`plugin/lib/plan.py`, `SUBMIT`), the bare `sluice.git` on deneb, with `SLUICE_AGENT` the one who asked | the wait reports and exits as before, and says nothing of #n pending or of files checked out; a request's outcome is the request's, and a worktree's submission from crew's short-lived worktree stays its own worktree's, as from the main checkout |
| the sluice board | `sluice inbox` in its session's root (`plugins/sluice-board/hooks/register.tsx`, `collect`) | a session rooted in a git directory collects what the branch its `HEAD` names submitted, and nothing is said on stderr |
| Flume | a place's outcomes by `sluice inbox --json` in its worktree; a base place's by `sluice status` (C.6) | nothing: a place is a worktree, a base place is submitted by name |
| the runner | `sluice status <id> --json` | nothing |
| the herdr plugin | rows and toasts from the reader's document, marking nothing | nothing |

One reading, one owner: the core's `Here::made` decides whose a
submission is, for every folder; the driver says who is in the folder.
Flume's own recipients, its `flume inbox`, are Flume's and read no
sluice record.

### Rules relied on

core.2 (a seam through one interface), core.31 (sluice alone marks
delivered, on its carrier's word), core.41 (a wait reads the live
records and the index, and its own finished record), core.48 (the clause
this change amends), core.50 (no worktree collects a submission by
name). The change draws record 0099.

## Goals / Non-Goals

**Goals:**

- A wait, a waiting submit and `sluice inbox` in a bare repository or a
  git directory read delivery as a worktree on the branch its `HEAD`
  names would: the same reading, the same marks.
- They never fail, or say an outcome stays pending, because no files are
  checked out, and never need `HEAD`'s commit.
- A git directory's own submissions are delivered there.

**Non-Goals:**

- Finding a session's outcomes by the agent its harness names, across
  worktrees (record 0099, option 4): it needs 130 amended for the index
  of the finished to name an agent.
- What a submit run where no files are checked out submits.
- The wait's report of its own entry's end: the unit
  a-submit-reports-the-end-of-its-own-entry.

## Decisions

### D1. A folder with no files checked out is a worktree on its branch

The delivery read reads a bare repository or a git directory as the
worktree on the branch its `HEAD` names, with no path. `pending` and
`pending_one` take it with `Here::on(<that branch>)`, unchanged, so it
takes what that worktree would: a submission naming no head whose work
is the branch, a pull request's from the branch where sluice lands on it
nowhere, never one by name. A folder whose `HEAD` names no branch takes
nothing, as a detached worktree.

So for each case where no files are checked out:

| what is read there | taken | said |
|---|---|---|
| a submission from a worktree on another branch | no | nothing; it stays pending for its own carrier, as from any worktree |
| a pull request's entry from a branch sluice lands on (the signal's #2) | no | nothing; its carrier is the request |
| a submission by name | no | nothing (record 0090) |
| a submission whose work is the branch `HEAD` names, naming no head | yes | marked delivered |

### D2. Who is there: `Driver::recipient`

```rust
/// 51: who collects outcomes in `folder`: the submitter a submit there
/// would record, read from neither the folder's files nor its HEAD's
/// commit, so a folder with no files checked out, or whose branch has no
/// commit yet, is read as the worktree on the branch its HEAD names with
/// no path; None where HEAD names no branch, a place that submitted
/// nothing (2, 3).
fn recipient(&self, folder: &str) -> Result<Option<Submitter>> {
    let s = self.session(folder)?;
    Ok((!s.detached).then_some(s.from))
}
```

A default method on the seam, so a driver whose `session` reads no
commit keeps it. The worktrunk driver overrides it:

- `git symbolic-ref -q --short HEAD`; empty, `None`.
- `git rev-parse --is-inside-work-tree`; where `true`, `git rev-parse
  --show-toplevel` as the path, else no path.
- `SLUICE_AGENT`, else the branch, as the agent; its host; the herdr
  session as `session` reads it.
- No `git rev-parse --verify HEAD`, no `git status`.

`Drivers` (`crates/sluice/src/profiles/mod.rs`) forwards `recipient` to
worktrunk's: a folder is never a pull request's place. Without the
forward, its `session` would be called and read `HEAD`'s commit, which
the test of a `HEAD` with no commit catches through the binary.

`Session::bare` goes: nothing reads it once `recipient` does. The
worktrunk driver's `session` keeps the same check for itself, so it runs
no `git status` where no files are checked out (record 0085).

### D3. The name an outcome is delivered to

`to` is `me.session`, else `me.path`, else `me.agent`. In a worktree
`me.path` is always given, so nothing changes there. Where no files are
checked out it is the herdr pane, else the agent `SLUICE_AGENT` names,
else the branch. The agent names who the outcome was put before, which
the state reference says `delivered_to` is; a branch named `main` on its
own would read as the branch the outcome is on.

### D4. The words

`no_work_tree`, `Recipient::Bare` and both lines go. `delivering`
returns `None`, saying nothing, for a submission that is not the
folder's, as for one from another worktree. A read that fails, a folder
git cannot read as a repository, is said as today: "could not read
whether #<n> is delivered, so it stays pending for sluice inbox: …". The
inbox, where `recipient` is `None`, prints nothing, `{"outcomes":[]}`
under `--json`, and exits 0.

### D5. The open change a-landed-bolt-is-never-reported-withdrawn

That change shipped in sluice 0.70.0 and is not archived. Its
outcome-delivery delta adds two requirements: the driver reads a
repository with no files checked out with no `git status`, which stands,
and a delivery read there delivers nothing and says why, which this
change replaces. That delta drops the second. This change adds its own
requirement, so the two archive in either order to specs that agree.
Archiving that change now is not this change's: its specs leave three
warnings `root:specs` refuses, a new capability's Purpose and two
requirements past 500 characters.

### D6. How it is held

- **Record 0099**, with the options weighed.
- **Rule core.48**'s last clause, which read "no session is read where
  no files are checked out", now says what such a read takes, with its
  details; record 0085 carries an amendment line.
- **The site**: the commands reference's wait and `sluice inbox`
  sections, and the state reference's `delivered_to`.

## Alternatives Considered

- **As record 0085, delivering nothing there and saying so.** False for a
  request's entry and for the folder's own, and fails on a `HEAD` with no
  commit. Rejected for D1.
- **A wait delivers what it reports wherever it runs.** Hands one
  session's outcome to whichever process waits on it, against 50.
  Rejected for D1.
- **By the agent the harness names as well** (record 0099, option 4).
  Delivers crew's submission from a short-lived worktree to its wait in
  the bare clone, but an inbox would read every finished record (130,
  131; core.41), and every worktree's reading changes. Left to a change
  that amends 130 first.
- **Keep `Driver::session` for the read, tolerating a `HEAD` with no
  commit there.** `Session::head` is what a submit sends; a session with
  no head would be read by a submit too. Rejected for D2.
- **`delivered_to` the git directory's path.** Names where it was read,
  not to whom; a session in that folder is the agent. Rejected for D3.

## Risks / Trade-offs

- **crew's submission from a short-lived worktree** to a branch that
  takes no pull requests stays pending for its own carrier when crew
  waits in the kit's main checkout. Under herdr its pane takes it; with
  no pane recorded nothing does, as before this change, and the wait's
  exit is still crew's outcome. Record 0099's option 4 is the way on.
- **A git directory's own submissions** were left pending there and are
  now delivered there. A session that also runs `sluice inbox` in a
  worktree on that branch finds them delivered, once, as 51 asks.
- **An older sluice sharing a branch's records** reads nothing new: no
  record changes form, and `delivered_to` is free text.

## Migration Plan

Nothing held changes form. An outcome left pending by an earlier
sluice's read where no files are checked out is collected by the next
delivery read that takes it.

## Open Questions

None.
