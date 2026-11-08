## Why

A wait, a waiting submit and `sluice inbox` each read whether an outcome
is theirs to deliver before they mark it delivered (51). Run where no
files are checked out, a bare repository or a git directory, that read
fails or delivers nothing:

- On deneb on 2026-11-07, crew's wait for a landing on agentplot/sluice's
  `main` ran in the bare `sluice.git` and said, under sluice 0.68.1:
  "could not read whether #2 is delivered, so it stays pending for
  sluice inbox: git status in .: fatal: this operation must be run in a
  work tree" (signal 2026-11-07-agentplot-ops-d4947942/02).
- Record 0085 (sluice 0.70.0) stopped the `git status`. But it reads such
  a folder as one where no session works, so it delivers nothing there
  and says "#<n> stays pending for its session's sluice inbox: <dir> is
  a repository with no files checked out". That line is false for #2, a
  pull request's entry from a branch sluice lands on, which is pending in
  no inbox, and false for every worktree's submission read there. A
  submission that is the folder's own is never delivered there.
- The driver reads `HEAD`'s commit first, so a folder whose `HEAD` names
  a branch with no commit yet fails the read outright.

The change a-submit-names-its-head-from-no-worktree, task 4.3, kept that
line off a submission by name only.

## What Changes

- **A folder with no files checked out reads as a worktree on its
  branch.** A wait, a waiting submit and `sluice inbox` run in a bare
  repository or a git directory take as their own exactly what a
  worktree on the branch its `HEAD` names would take: one reading of
  whose a submission is (record 0090), wherever it runs. They deliver
  and mark that, and nothing else.
- **Who is there is read with no files and no commit.** The driver gives
  the delivery read who works in a folder by one operation of its seam,
  `Driver::recipient`, reading the branch `HEAD` names and nothing of
  the folder's files or `HEAD`'s commit. `Session::bare` goes, since
  nothing reads it then.
- **`delivered_to` names the agent where there is no path.** An outcome
  collected in no pane and where no files are checked out is marked
  delivered to the agent the harness names, `SLUICE_AGENT`, or the
  branch.
- **Nothing is said for want of files checked out.** The line "stays
  pending for its session's sluice inbox ... no files checked out" and
  the inbox's line of the same words go. An outcome that is another's is
  reported and left to its own carrier with nothing said, as from any
  worktree.

## Out of scope

- A session that collects what it submitted from any worktree, by the
  agent its harness names: crew's submission from a short-lived worktree,
  waited on in the kit's main checkout. The index of the finished names
  no agent (130), so it needs 130 amended first. Record 0099 weighs it
  as its option 4.
- A wait that reports the end of the very entry its submit made: the
  unit a-submit-reports-the-end-of-its-own-entry, on this same bolt.
- What a submit run where no files are checked out submits: unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `outcome-delivery`: a wait or a waiting submit where no files are
  checked out reads and marks delivery as a worktree on its branch would,
  with no files and no commit read, and never fails or says an outcome
  stays pending because no files are checked out; its `delivered_to`
  is the agent where there is no path.
- `inbox`: `sluice inbox` where no files are checked out collects what a
  worktree on its branch would, saying nothing for want of files checked
  out; its `delivered_to` is the agent where there is no path.

## Impact

- `crates/sluice-seams`: `Driver::recipient`; `Session::bare` removed.
- `crates/sluice`: the worktrunk driver's `recipient`, the dispatcher
  `Drivers` forwarding it; `recipient`, `delivering` and `inbox` in
  `lib.rs` reading it, with `Recipient::Bare` and `no_work_tree` gone.
- `crates/sluice-fakes`, `crates/sluice-core`'s test drivers and
  `crates/sluice-conformance`'s: `Session` built with no `bare`.
- The open change a-landed-bolt-is-never-reported-withdrawn: its
  outcome-delivery delta keeps its requirement that the driver reads a
  repository with no files checked out with no `git status`, and no
  longer adds the one this change replaces, that a delivery read there
  delivers nothing and says why.
- Records: no field changes. `delivered_to` may name an agent.
- The site's commands and state references say what a wait and
  `sluice inbox` deliver where no files are checked out, and what
  `delivered_to` names there.
- Statement: none; 2, 3, 50 and 51 as written. Record 0099 and rule
  core.48, as amended, hold the decision; record 0085 carries its
  amendment line.
- Release: a `fix`, the patch CI tags at this bolt's landing.
