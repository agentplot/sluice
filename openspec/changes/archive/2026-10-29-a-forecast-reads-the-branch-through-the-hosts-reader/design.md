# Design

## Context

See proposal.md, Why, for the signal. What a forecast does today
(`crates/sluice-core/src/ops.rs`):

- `forecast_branch` reads the `--head` branch through the branch seam,
  which for a branch on a remote is `PushedRefs::read`, a fetch on every
  read (`crates/sluice/src/profiles/pushed_refs.rs`).
- `forecast_head` reads the target branch the same way, then checks its
  policy at that revision (`rules_at`, a local read).
- `forecast_onto` reads the branch's records (`read_line`, which for
  records on a remote is `RemoteRefs::read_remote`, a fetch of
  `refs/sluice/state/<branch>`), orders the queue as a round would with
  the head at the back, takes each combination a round formed on the tip
  with exactly the submissions ahead (`c.on == tip`), and forms the rest
  locally. `contested` (`step.rs`) names who the head conflicts with.

So a bolt's forecast on a remote makes three fetches. It never fetches
the work submitted from other hosts or the combinations they formed
(`fetch_heads`, `fetch_candidates`, which only a round calls), so it
forms only when a round on this host already brought them.

The host's reader (`crates/sluice/src/reader.rs`, 129, ADR 0056) is the
first watch or wait of the repository: it holds
`readers/<repo>.lock` and writes `readers/<repo>.json`, the whole
overview, only when it changed but for its time (`Lead::write`,
`untimed`). Its reads go through `overview::document` →
`Sluice::overview` → `branch_view_of` per branch, which reads the
records (`State::read`, a fetch for records on a remote) but not the
branch's revision: `BranchView.at` is the records' `base`, where the
last round saw the branch. `CandidateView` carries a combination's
submissions and revision but not the revision it was formed on, and the
view holds only the submissions still queued, so which `after` ids have
landed is not in it. A watch ticks every second, a wait or a waiting
submit every three (`main.rs`, `every`).

Clause 64 as amended (blueprints ac694b3) and its commit say the reader
asks a remote once a tick for all its branches and the forecast takes
the branch's revision and the queue from what it wrote.

## Goals / Non-Goals

**Goals:**

- A forecast near a live, current reader asks the remote nothing, for the
  target branch, its records, or a `--head` branch the reader read.
- The forecast on the reader's read is the same forecast the records
  would give at that revision: one forming code path, fed from either.
- A forecast with no current reader reads as a round does, and can form
  on another host's submissions.

**Non-Goals:**

- `sluice exec`'s forecast, which forms on the revision its run reads.
- The idle tick's cost and one ask per remote a tick (the companion
  unit).
- Changing what a forecast prints, its JSON or its exits.

## Decisions

### 1. One forming path over a queue, fed from the records or the reader

`forecast_onto` is rewritten over a small value, `Queue`: the tip, the
queued submissions by number (head, from, state, priority, after), the
combinations (on, submissions, revision, state), and the landed ids the
queue's `after` names. `Queue::from_line(&Line, tip)` builds it from the
records, as today; `Queue::from_view(&BranchView)` from the reader's
document. `contested` takes a lookup of a submission's head and `from`
by number rather than a `Line`, so rounds and both forecasts share it.

Alternative: rebuild a `Line` from the view. Rejected: a `Line` is the
records' whole state, which the view does not hold, and a half-filled
one would read as the records to any code that took it.

### 2. The view carries what the forecast forms from

`BranchView` gains `revision` (the branch's revision as the read found
it, omitted when the read could not ask the branch's remote or the
remote has no such branch), `after_landed`, and `CandidateView.on`. All
three are added fields; a document from before them lacks them, and a
forecast reads such a document as giving no revision, so it reads as a
round does.

### 3. The whole overview's read brings the branch, its work and its combinations in one call

`Sluice::overview` reads each branch with a new records-seam call,
`State::read_with_branch(branch)`, whose default is `read`. Its
`RemoteRefs` form makes one `git fetch` with the refspecs of the state
ref, `refs/heads/<branch>` into the branch's tracking ref
(`refs/remotes/<remote>/<branch>`, the one `PushedRefs` reads), the
submitted work (`refs/sluice/heads/*`, as `fetch_heads`) and the
branch's combinations (as `fetch_candidates`), under the retry policy
(core.29). `PushedRefs::fetch_spec` takes several refspecs for this.
Where that call is refused because the remote lacks one of the exact
refs, it falls back to today's read of the records alone, and the view
gives no `revision`. The view then reads `revision` through a new
branch-seam call, `Branch::fetched(branch)`: the branch's revision as
the host holds it, with no call to the remote (`PushedRefs`: the
tracking ref; `LocalRefs`: the branch; `Branches` routes it). Where
`State::stale(branch)` says the read was refused, `revision` is left
out.

`branch_view_of`, which `sluice status` and a follower's fallback use,
keeps its plain read, so status still never fetches the branch.

Alternative: read the branch with `Branch::read` beside the records.
Rejected: a second call to the remote per branch per tick, against 129's
one ask, and the work and combinations would still be missing. When the
companion unit folds each remote's records into one ask a tick, these
refspecs go into that ask: the rule is that they ride on whatever call
reads the records.

### 4. The reader marks its reads; the forecast judges the marks

`Lead` gains `begin()`, called before each read, which sets the lock
file's modification time to now through the file it holds; and
`Lead::write` sets the document's modification time to now when it
skips an unchanged document (the file is opened and `set_modified`, as
`profiles/github/kept.rs` does), so the document's time is always its
last read's end. The document names `tick`, the reader's pause in whole
seconds, which the caller passes from `main.rs`'s `every`; `Pause` is
left as it is.

`reader::current(dir, repository, now) -> Option<Value>` answers the
forecast: `None` unless a live process holds the lock (a `try_lock`
that fails with `WouldBlock`; one that succeeds is released at once),
the document parses with a `branches` array and a `tick`, and either
`now - end <= tick`, or `begin > end` and `begin - end <= tick`. A mark
more than a tick ahead of now counts as no mark. The forecast never
becomes the reader and takes no lease.

Alternative: the literal reading, only `now - end <= tick`. Rejected as
the default because a read slower than the tick, GitHub's 10 to 24
seconds in the signal, would make every forecast fetch exactly when the
remote is slow, which is what the clause was amended to stop. The
reading chosen counts a reader as current while it is in the read that
follows one ended on time; the retry policy bounds that read (core.29).
This is named to the design agent as a reading of "within its tick".

Alternative: write a mark into the document each tick. Rejected: it
would rewrite the document, and make every follower see a change, at
every tick.

### 5. `sluice submit --check` tries the reader first

`forecast` in `crates/sluice/src/lib.rs` calls `reader::current` with
the host's readers directory and repository. With a document, it finds
the target branch's view and, for `--head`, the `<from>` view, and asks
the core for `Sluice::forecast_read(view, head, own, after)`. That
returns `None`, and the command goes on as today, when the view has no
`revision` or is `unread`, or when `Branch::holds(rev)` (a new seam
call, an object lookup with gix in `LocalRefs`, routed by `Branches`)
says the host lacks the tip, the head, a queued submission's work or a
taken combination's revision. The policy check at the tip (`rules_at`)
is made on the reader's revision, locally. A `--head` branch the
document does not hold is read as today.

### 6. Without a current reader, as a round reads

The fallback is today's path, with one addition: where `holds` says a
queued submission's work or a taken combination is not on the host,
`forecast_onto` calls `State::fetch_heads` and `State::fetch_candidates`
once, as a round does, then forms. Today such a forecast fails on the
missing object.

## Risks / Trade-offs

- [The companion unit reworks the same read in `overview.rs` and
  `remote_refs.rs`] → both are in this bolt and their Touches overlap;
  the second to merge keeps the rule of decision 3, that the branch, its
  work and its combinations ride on the call that reads the records.
- [The driver bolt makes `sluice run` the reader and adds a driver's
  lock] → its reader writes through `Lead`, so it marks its reads too
  once it calls `begin`; whichever lands second makes `reader::current`
  count `sluice run`'s reader, as the driver bolt's `Reader::take` does.
- [A forecast's `try_lock` succeeds for an instant while no reader runs,
  and a watch starting then follows for one tick] → it leads at its next
  step, as after any reader stops.
- [A reader's revision lags the remote by up to a tick and a read] →
  a forecast decides nothing (64), and the round forms on the remote's
  revision; the JSON's `onto` says what it was formed on.
- [The whole overview's one-shot read now brings the branch, its work
  and its combinations] → one call, as before; the objects it brings are
  the ones a forecast or a round would fetch next.

## Migration Plan

None: the document's new fields are added, and a reader from before them
leaves a forecast reading as a round does. Rollback is the previous
release.
