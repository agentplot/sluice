# A branch the overview names has a status

## Context

The overview lists every branch the records seam holds records for
(`State::branches`, in `crates/sluice-core/src/overview.rs`). `sluice
status <branch>` (`crates/sluice/src/lib.rs`, the branch arm of
`Command::Status`) read `Sluice::status` and refused the branch as no
branch when `Status::is_unknown` held: no revision, no live submission
and nothing finished. A branch made by `sluice branch create` holds, in
its records, the revision it was made at (`landed`, 121) and nothing
else until something is submitted to it. Removed, with every submission
to it made in another copy of the repository, it has records, no
revision and nothing in them a status counted, so the overview named it
and a status refused it, naming it among the branches with records.

The statement keeps a removed branch's records (105), has every entry's
state readable (26) and the records read given a branch (61), and has a
status of a branch read the live records and the index (131). It does
not say how a status names a branch that is gone, nor whether the
overview shows one with the others. Clause 105 is amended on
`agentplot/blueprints` branch
`design/sluice-a-removed-branch-answers-its-status`: a removed branch's
records are read as any branch's, a status of it answers from them and
says the branch is gone, a view of the repository's branches reads it
and names it gone, and only a branch with neither records nor a
revision is no branch to a status.

### Looking across

- **Flume** (`crates/flume/src/profiles/merge_queue/sluice/mod.rs`):
  `branches` reads `sluice overview --json`'s `branches[].branch`, and
  `read_submissions` asks `sluice status <branch> --all --json` of each,
  passing a refused one over with sluice's words. That passing over is
  what printed the `cannot read` lines. Once the status answers, Flume
  reads the removed branch's (possibly empty) history as any other's,
  and needs no change. It reads no new field: `gone` is for a reader
  that wants it.
- **herdr's plugin** (`plugins/herdr/bin/document.sh`) reads each
  branch's `submissions` and `recent`; a gone branch with neither adds
  nothing to it, as an idle branch never did.

## Goals / Non-Goals

**Goals:** a status and the overview agree on which branches exist to
be read; a removed branch's records answer its status; a removed branch
is named gone wherever it is shown.

**Non-Goals:** reading a removed branch's records on its remote where
no revision names the remote (core.27); any change to a branch's
records.

## Decisions

### D1. A branch is no branch to a status only with neither records nor a revision

`Status::recorded` says the branch has records: its read found any, a
version or a file, or, where it found none of a branch the host holds no
revision of, `State::branches` lists it, as the overview does, so a
status never calls absent a branch the overview names.
`Status::is_unknown` is no revision and not recorded.

Rejected: keeping the old reading and dropping such branches from the
overview, which hides records 105 keeps; removing a branch's records
with it, against 105.

### D2. "Gone" is read by the read the status already makes

`Status::gone`: where the records have seen no revision of the branch,
its own read (`Branch::read`, a fetch for a branch on a remote) found
none; otherwise the revision the host holds of it (`Branch::held`, no
fetch) is none. A status asks the remote nothing it did not ask before
(record 0046). A branch on a remote removed by another host, whose
remote-tracking ref this host has not pruned since, reads as there until
a fetch of it.

The overview's `BranchView::gone` is the same holding its `revision`
gives: the host holds no revision of the branch, and the read of its
records was not refused (`stale`).

Rejected: reading the branch itself on every status, a fetch for each
status of each branch on a remote, which Flume asks for every branch on
each of its own reads.

### D3. The overview names a removed branch gone, and its text shows one with nothing to watch apart

The statement makes the overview a view of the repository's branches
that reads every branch it can and hides none (26), written by one
reader for everyone watching the repository's queues and followed by
every watch and wait on the host (129; core.25).

- `sluice overview --json` keeps the branch in `branches[]`, with
  `"gone": true`, left out while the branch is there. Every reader on
  the host follows this document, and one that finds a branch's history
  by the branches it names, as Flume does, keeps finding a removed
  branch's.
- The text, which a person reads to watch the queues, shows a gone
  branch with nothing to watch apart: nothing queued, under test or
  recent, no hold, no held checkout, no refused write and no read
  refused. Such branches are named on one line after the branches it
  lands on, before the held places of branches with no records:
  `gone, nothing queued or recent: <branch>, ...`.
- A gone branch with something to watch, its landings among its recent
  lines above all, is shown as any branch, its first line
  `<branch> is gone, last at <revision>` in place of the queue's
  counts, which say nothing of a branch nothing is formed on.

Rejected: leaving it out of the overview (a follower loses the records
105 keeps, against 26's reading every branch it can); a list of its own
in the document (every reader of `branches[]` would need a second list
to keep reading a removed branch's history, Flume among them); listing
it unmarked (a person reads an idle queue for a branch that is gone).

### D4. A status of a gone branch says so first

`sluice status <branch>`'s first line for a gone branch is the same
`gone_line` the overview gives, then its warnings, holds and refusals as
for any branch, its queue (`nothing in the queue`), and the count of
those no longer in it with the `--all` that lists them. With `--all`
its history is listed as any branch's. `--json` gives `gone`, always,
beside `at`. A status of one submission on a gone branch heads it the
same way.

## Risks / Trade-offs

- A branch on a remote removed by another host reads as there in a
  status of a branch whose records have seen a revision, until this host
  fetches it (D2). The status answers from the records either way, so
  nothing is refused or lost; only the word `gone` waits.
- A removed branch whose records are on its remote under signers is
  read from the host's own files (out of scope, core.27): a status and
  the overview agree, both showing what the host's own files hold.
