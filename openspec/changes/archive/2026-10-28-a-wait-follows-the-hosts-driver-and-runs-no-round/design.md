# Design

## Context

See proposal.md for why. As built on main:

- `crates/sluice/src/reader.rs` keeps one reader per repository per host.
  The reader holds the lock `readers/<repository>.lock` and writes the
  overview to `readers/<repository>.json`. Every other watch or wait
  follows that file (129; ADR 0056; rule core.25). Each tick, a
  follower calls `lead_once_free` and becomes the reader when the lock
  is free.
- Only `sluice overview --json --watch` (`overview::run`) and the waits
  (`wait_on` through `Reading`) take part. `sluice run` (`Command::Run`
  in `lib.rs`) never touches the reader's files. It reads the records
  for itself every pass.
- Every wait loop runs a round between reads, whatever it reads from:
  - `wait_on`, under `sluice wait <n>`, a waiting `sluice submit` and a
    group submit
  - `submit_pull` and `submit_pull_group`, under a pull request's submit
    and `sluice wait <branch>`
  The pull-request loops also take requests in (`intake`) on every
  pass.
- `wt submit` waits by running `sluice wait`
  (`profiles/worktrunk/wt_submit.rs`), so it needs no change of its own.
- A wait or run pauses three seconds between passes, and a watch one
  second (`main.rs`).

Separately, `a-newer-watch-replaces-an-older-reader` is built on its own
bolt, not on main. It adds a `.version` file and a `.lease` lock beside
the reader's files, and replaces `lead_once_free` with
`Reader::step(may_lease)`. In that change, a reader lets go once the
lease is taken, and a free lock goes to the lease holder.

## Goals / Non-Goals

**Goals:**

- Any sluice command can tell whether the repository's `sluice run` is
  live on the host. The check costs no process and reads nothing of the
  records.
- Driving and reading are kept apart, so either still works when the
  other cannot be had.
- A plain install, with nothing running, behaves exactly as today.

**Non-Goals:**

- Making `sluice step` a driver.
- Moving a pull-request wait's forge read into the reader.
- Any configuration that names a driver, and anything that starts one.

## Decisions

### The driver holds a lock of its own: `readers/<repository>.driver`

`sluice run` takes an exclusive lock on `readers/<repository>.driver`
and holds it for as long as it runs. The file sits beside the reader's
files and its name is built the same way (`flat_name`). The suffix,
`.driver`, is seven bytes, within the eight that the reader's other
files allow for.

A probe, `reader::driver_live(dir, repository)`, opens the file, tries a
shared lock, and drops the lock at once. If the try would block, a
driver is live. If the probe fails, it reads as no driver, so a wait
falls back to running rounds and never to waiting on nothing.

Because the lock is released with its process however that process
ends, a crashed `sluice run` leaves nothing to clean (as the reader's
lock does).

Alternatives considered:

- **The reader's lock alone, with a mark in the document saying the
  reader drives.** A wait that is the reader when `sluice run` starts
  would never learn that a driver exists, because the run could not take
  the lock to say so. A document outlives its writer, so the mark could
  not tell a live driver from a dead one.
- **A pid file.** It is stale after a crash, and a pid can be reused.
  The house pattern is a lock that goes with its process (ADR 0056).
- **The lease of `a-newer-watch-replaces-an-older-reader`.** It is not
  on main. It is also taken only by a newer version and given up once
  the holder leads, whereas the driver's mark must last as long as the
  run does.

### A probe and the run's own try do not mistake each other

A probe holds the shared lock only for the instant of its try. A `sluice
run` whose exclusive try meets that probe would wrongly read another run
as live. So the run tries its exclusive lock again over about a second,
at short intervals, before it reads the driver as another run's.

This is a local lock, not a call to a remote, so rule core.29's retry
policy does not apply.

### Driving and reading are separate parts of `sluice run`

At the start of each pass, `sluice run` keeps or takes the driver's
lock.

- **Without the lock**, it says once on stderr that another `sluice run`
  drives this repository on this host. That pass it runs nothing: no
  releases, no intake, no rounds. The pass still counts toward
  `--rounds`.
- **With the lock**, it takes part as a reader (`Reader::take` at its
  first pass, and the step below each pass after). It runs the pass as
  today. Then, if it holds the reader's place, it writes the reader's
  document. It does this every pass, queue or none.

The document is the one a watch's reader writes, from the same function
(`overview::read`, made reachable from `lib.rs`): the overview whole,
with `"set_up": false` where the repository has no sluice. If the read
fails, the failure document is written in the words it was refused
with (133), and the pass goes on.

Because the two are separate, a driver that cannot take the reader's
place still drives. That happens when the reader is a sluice from before
this change, which never looks at the driver's lock. The waits of this
sluice follow that older reader's document and run no round. The
driver's rounds move the queue, and the older reader's document shows
it.

### One step decides each tick's part, given whether another process drives

`lead_once_free` becomes `Reader::step(others_drive: bool)`:

- **A reader** lets the lock go and becomes a follower when
  `others_drive` is true. Its last document stays in place for the
  others to follow until the driver writes one.
- **A follower** takes a free lock only when `others_drive` is false.

A watch or a wait passes `driver_live(..)`, probed once per tick. The
driver itself passes `false`, so it takes a free lock at its next pass.

`Reader::take` probes once and takes no free lock while a driver lives.

When `a-newer-watch-replaces-an-older-reader` lands, the two steps
compose into one:

- A reader lets go when its lease is taken, or when another process
  drives.
- A follower leads when the lock is free and either it is the driver, or
  no other process drives and no other process holds the lease.

The driver ranks above the lease holder, because clause 135 makes the
driver the reader wherever one runs. Whichever of the two changes lands
second makes that merge in `reader.rs`, and keeps both sets of tests.

### Each wait loop probes once per tick and skips its round while a driver lives

- **`wait_on`.** After its pause, it probes. If a driver lives, it
  skips the intake and `s.round`, steps the reader with
  `others_drive = true`, and reads as a follower. Otherwise it runs as
  today.
- **`submit_pull` and `submit_pull_group`.** These keep the first pass's
  intake of a submit or a `--status`, because that is the submit's own
  act: the answer names the entry intake took. They skip the intake of
  every later pass, and every round, while a driver lives.
  `sluice wait <branch>` (`PullMode::Wait`) takes nothing in while a
  driver lives, not even on its first pass.

Because the probe runs each tick, a driver that starts or stops while a
wait is under way is noticed within one tick: three seconds for a wait,
one for a watch. The round-failure paths (`Waited::Unread`, `landed`
after a round's error) are reached only through a wait's own round, so
under a driver they do not arise. The submission's outcome is read from
its record, as a follower already reads it once the submission leaves
the reader's document.

### The watch gives way the same way

In `overview::run`, the per-tick `lead_once_free` becomes
`step(driver_live(..))`. A watch runs no round, so nothing else in it
changes.

### The bound needs no change

Clause 71 makes the host's bound that one process's to keep. Once waits
run no rounds, the only checks the core starts on a driver's host are
the driver's rounds and the agents' own `sluice exec` runs. Both already
count in the host's one count. Nothing in `host_count.rs` changes.

## Risks / Trade-offs

- [A driver that is live but wedged, stuck in a long pass, leaves every
  wait on the host waiting with nothing moving.] → This is the host's
  choice in running one (135). A wait's `--timeout` still ends it, with
  exit 5, and stopping `sluice run` hands the rounds back to the waits
  within a tick.
- [A run's pass is three seconds plus its rounds, so its document is
  older than a watch's would be.] → Followers already read whatever the
  reader last wrote. A slow pass delays the document as it delays the
  landing. Nothing decides on the document (rule core.11).
- [A sluice from before this change that is waiting beside a driver
  still runs rounds.] → The branch's lock keeps two steps of a branch
  apart (rule core.14). The extra rounds end when that wait does.
- [Conflicts with `a-newer-watch-replaces-an-older-reader` in `reader.rs`,
  `overview.rs`, `lib.rs` and `tests/cli.rs`.] → The composition is set
  out above. Whichever lands second takes main into its bolt and
  reconciles.
- [Conflicts with `a-refused-branch-stops-no-other-branchs-wait-status-or-round`,
  which changes the run loop and the waits' reads in `lib.rs`.] → These
  are different lines of the same functions. The second to land merges.

## Migration Plan

Nothing to migrate. A host with no `sluice run` sees no change. On a
host with one, waits started after the new sluice is installed follow
it. An older sluice's waits keep running rounds until they end.
Rolling back restores today's behaviour, and the stray `.driver` file
is harmless.
