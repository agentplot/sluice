# Design

## Context

See proposal.md, Why. Rests on clause 129 of the statement
(`agentplot/blueprints:design/sluice/requirements.md`, A.8), ADR 0056 and
rule core.25; the rounds a wait runs rest on rule core.4 and ADR 0011.

Today `overview::run` (`crates/sluice/src/overview.rs`) calls
`overview::document` once a tick and prints the document whenever it
differs, `at` aside, from the last one printed. `wait_on`
(`crates/sluice/src/lib.rs`), the one loop behind `sluice wait` and a
waiting `sluice submit`, runs `s.round(&branch)` each tick, then reads
its submission with `s.state(id)` and, without `--json`, its branch with
`s.branch_view_of(&branch)` for its progress line and hold. A watch
ticks every second, a wait every three (`main.rs`). The document already
carries what a wait reads: each branch's `submissions` are the records
`sluice status <id> --json` prints, its `candidates` carry their tasks,
and its `held` the hold (`sluice_core::overview::BranchView`).

The host's own state is under `instance::state_dir()`,
`$XDG_STATE_HOME/sluice`, beside `running/` and `sessions/`, whose holds
are files locked by the process that holds them (`host_count.rs`). The
repository is named by `Host::repository()`, the path of its common git
directory, which every worktree of it shares.

## Goals / Non-Goals

**Goals:**

- At most one process per repository per host reads the records to watch
  its queues (core.25), with no change to any line a watch or wait prints.
- A reader's end, however it comes, frees its place for the next watch or
  wait with nothing left to clean.

**Non-Goals:**

- Fewer rounds: every wait still runs its round each tick, as now.
- A service, socket or daemon (ADR 0056 option 3).
- A reader that is alive but stuck: it keeps its place, as a stuck watch
  today keeps printing nothing.

## Decisions

### The lease is a file lock the reader holds for its life

`$XDG_STATE_HOME/sluice/readers/<repository>.lock`, the repository's
common git directory written as one file name by `runs::flat_name`, as
`running/` writes a run directory. A watch or wait opens it and tries
`File::try_lock`: got it, it is the reader and keeps the open file until
it ends; refused, a live reader holds it and the process follows. The
lock goes with the process, by exit, Ctrl-C or crash, so a stopped reader
needs no lapse time and no clean-up, and two processes never both read.
A follower tries the lock again each tick, which is what makes it the
reader within a tick (129).

Alternatives: a heartbeat file, which the herdr plugin's lease is, needs
a lapse time (60 s there) longer than a tick, so a reader that stops
would leave followers on a stale document for longer than clause 129
allows; a pid file needs a liveness check that a reused pid fools. The
host's other holds are file locks already (`host_count.rs`).

### The document is written whole beside the lock

`readers/<repository>.json`, written to a temporary file in the same
directory and renamed over it, so a follower reads the last whole
document or the one before it, never part of one (as the herdr-watch
spec requires of its state files). The reader writes it at its start and
each time the document, `at` aside, changes: the same test it prints by.
It is the document `sluice overview --json` prints, `sluice/overview@1`,
unchanged, so a follower prints it as it is.

A follower reads the file each tick and treats it as changed when it
differs from the last it read, `at` aside. A file absent or not yet a
whole document (a reader just started) is read as no document yet: a
watch prints nothing until one is there, and a wait says nothing new.

### One module holds the reader

`crates/sluice/src/reader.rs`: `Reader::take(dir, repository)` returning
either `Lead` (the lock held, with `write(&doc)`) or `Follow` (with
`read() -> Option<Value>` and `try_lead()`), so `overview::run` and
`wait_on` share it and neither knows a path. `exec::Host` gains
`readers(&self) -> Result<PathBuf>`: the real host's is
`state_dir()?.join("readers")`, the tests' fake host's a directory of the
test's own, so in-process tests never touch the user's state. Nothing
here names a tool (core.1).

### A watch: lead, or follow and print

`overview::run` with `--watch` takes the reader first. As the lead it
does what it does now and also writes each document it prints. As a
follower it calls no `document` and reads no seam: each tick it reads the
file, prints the document when it changed, and tries the lock; once it
gets it, it is the lead from that tick on, reading for itself. The
one-shot path is untouched and never opens the lock.

### A wait: lead or follow, and its rounds stay

`wait_on` takes the reader beside its loop. Each tick it still runs its
round (core.4; ADR 0011): on a branch with no instance running rounds, a
wait's rounds are what land its submission, and a reader that is a watch
runs none. Then:

- as the lead, it calls `overview::document`, writes it when it changed,
  and reads its submission and branch from that same document;
- as a follower, it reads its submission, progress and hold from the
  file's document: the branch's `submissions` entry for its id, its
  candidate, its `held`. While the entry is there it is the record, and
  its `--json` out of time prints it as it stands. Once a document no
  longer holds it, the submission has left the queue, and the wait reads
  `s.state(id)` once, and `report` reads its log once, to say the outcome
  as now. That one read is no watch: rule core.25 rules out a wait that
  polls the records beside a reader, and this reads one record once.

The wait's tick stays three seconds. A lead wait therefore writes the
document every three seconds, and a watch that follows it sees changes
up to three seconds late until the wait ends; a lead watch writes every
second. A submission the wait's own round has just landed shows as
landed one tick later than now when a follower's file was written before
the round.

The pull-request paths that wait (`submit_pull`, `PullMode::Wait`) reach
`wait_on` once sluice has taken the pull request in; before that they
follow the forge, which reads no records, and are unchanged.

### Where `Waiting` gets its reader

`Waiting` gains the host's readers directory and repository, given by
`run` from its `host`, so `await_submission` and the three callers of
`wait_on` pass them on and the loop stays one loop (submission-wait).

## Risks / Trade-offs

- [A reader stuck in a git call keeps its lock, and its followers print
  nothing new] → The same stuck call stops a lone watch today; the lock
  is the process's, so killing it frees the place at once.
- [The state directory on a file system where locks are not kept, a
  network home] → Every process takes the lock and reads for itself, as
  today; nothing is lost but the saving. The host's run count already
  needs the same locks.
- [An older sluice on the host takes no lock and reads for itself] → It
  costs what it costs today and confuses no follower; a newer one beside
  it may be a reader or follow one.
- [Followers of a lead wait see changes every three seconds, not one] →
  Accepted; the reader is usually a watch, since the board and the herdr
  plugin start theirs before any wait, and the lead wait's end hands the
  place to a watch within a second.
- [A follower wait sees its outcome up to a reader's tick late] →
  Accepted: within the tick clause 129 allows.

## Migration Plan

None. The `readers/` directory is made on first use; a host with none
runs as today. Rolling back leaves files no sluice reads.
