## Context

See proposal.md for why. The statement (114 and the C.2 sandbox row, as
amended at agentplot/blueprints 0c38f24), record 0105 point 2 and rules
core.9 and core.35 already say what holds. This design says how the
code reads it. No other tool of the family signs runs or reads a task
runner's record of them, so nothing here needs reconciling with Flume,
Boomdrive, timber, weir or boom. Logging a cause once per cause and
host, with its end, is sluice's own way for a standing refusal (133,
core.28) and for refused writes (148, core.47), and this design follows
it.

What the code has today:

- `Tests::proven` (`crates/sluice-seams/src/lib.rs`) answers the tasks of
  a finished run whose keys the combination's tree bears out, read while
  its place exists. `Checks::proven` (`crates/sluice/src/profiles/checks.rs`)
  answers every task for the command runner. For moon, it reads each
  task's key with `moon::check_key` against the store the run's `Meta`
  names (`Meta::store`, else the place's `.moon/cache`). A task that
  fails the check is left out and named only in a `tracing::warn!`.
- `moon::check_key` (`crates/sluice/src/profiles/moon.rs`) walks a key's
  manifest and, depth-first, each dependency's. It fails with
  `"{task}: no manifest for {key}"` where a blob is missing, and with
  other words where a manifest does not hash to its key, names another
  task, or names an input the tree does not hold as it says.
- `Sluice::finish_run` (`crates/sluice-core/src/step.rs`) asks `proven`
  only where `rules.signed`. It records a `ran` or `replayed` line and
  result only for a proven task. `verdict::read` then reads a ran task
  with no counted result as failed under its key where `proven` left it
  out (design D9 of the signing change), so the submission is blamed.
  `finish_run` then returns early for an unproven replay (108, the
  `Unproven` event) and for results that do not count (87, `Uncounted`).
  Only after that does it read `report.not_run` with the verdict's
  `unjudged` through `read_not_run`.
- `read_not_run` (`step.rs`) reads a run that left checks not run and
  judged nothing. Under `NOT_RUN_BOUND` (3) it opens the run again with
  no place, host or handle, lists its place to release, and logs `Lost`
  with each check and why. At the bound, or where a check failed on its
  own, it names each check failed with why (`FailedTask::not_run`), and
  the culprit is told (115, 20). Its two callers are `finish_run` and
  `not_started`, for a run whose tests could not be started.
- `sluice status` and `sluice wait` show a combination under test by
  `progress::testing_line` (`crates/sluice/src/progress.rs`), read from
  `overview::CandidateView`. A run some host turned down already shows
  there, by `CandidateView::waiting`, which `with_waiting`
  (`crates/sluice/src/lib.rs`) also puts in `status <n> --json` and
  `wait --json`. Nothing shows a run that had no outcome until the
  bound blames it.

## Goals / Non-Goals

**Goals:**

- On a branch whose records are on its remote, a ran check that passed
  in a run that kept no manifest of its key has no outcome. Nothing is
  recorded under its key, nobody is blamed, and it runs again within
  115's bound (114, core.35).
- Why is read from the fact, the manifest missing where the profile
  reads it, never from what moon printed (record 0105).
- The branch's log keeps the cause once per cause and host, with its
  end (record 0105; core.28's way).
- `sluice status` and `sluice wait` say why a combination is tested
  again (record 0105).
- The core names no moon (core.1): the words are the check profile's.

**Non-Goals:**

- Letting the sandbox write moon's store (record 0105 point 1, the
  change `a-sandboxed-run-writes-its-runners-store`).
- A fail, a replay or a manifest the tree does not bear out, each read
  as before (proposal, Out of scope).
- A branch-level line for a standing cause in `sluice status <branch>`.

## Decisions

### D1. `proven` answers a `Proof`: the proven tasks and the unkept passes

`Tests::proven(&self, run, report) -> Result<Proof>`, where
`Proof { proven: Vec<String>, unkept: Vec<NotRun> }`. `proven` is what
it answers today. `unkept` holds each ran check that passed and whose
proof the run kept none of, with why. A task in neither list is no
pass, as today. One walk of the store answers both.

- Rejected: a second method, `unkept(run, report)`. It would walk the
  store twice, and a wrapper that forwarded one method and not the
  other would compile and read nothing (the reason
  `Tests::switched` is required, record 0097 design D3).
- Rejected: a `NotRun` reason prefix that the core reads to tell causes
  apart. The core would then parse the profile's words.

### D2. Only a missing manifest is nothing kept

`check_key` fails with a typed `moon::NoManifest { task, key }`, whose
`Display` keeps today's words, where a blob a key needs is missing:
the key's own or any dependency's at any depth. The chain is the key's
proof (C.6), so a missing link is a proof the run did not keep.
`Checks::proven` tells it apart with `downcast_ref::<NoManifest>()`.
Every other failure, a manifest that does not hash to its key, names
another task or names inputs the tree does not hold, is a key the tree
does not bear out. It stays out of both lists and keeps its `warn!`, as
today.

- Rejected: every failure of `check_key` read as no outcome. A check
  that lies about its key would then never be blamed for it. It would
  run again until 115's bound and be named only as could not run.
- Rejected: looking only at the key's own blob before the walk. A
  missing dependency's manifest would then read as a key the tree does
  not bear out, and blame.

### D3. Only a ran pass is unkept

`unkept` takes only `report.tasks` entries with `Source::Ran` and
`Outcome::Pass`. 114 gives no outcome to a check that passed. A fail
with no manifest is read as before. In a sandboxed run, the sandbox's
own report (139) reads what it refused, which names a refused write to
the store. A replay is proven by a trusted signed pass for its key
(108), never by this run's manifest, so it is read as before. A task a
check was failed by (149) is a fail.

### D4. The words

`Checks::proven` gives each unkept pass the reason
`it passed, but nothing the run kept proves it ran: moon's manifest of
it is not in <store>`, where `<store>` is the store it read, as
displayed by `Path::display`. The reason names neither the task nor the
key, so every unkept pass of one run has one reason, and one store on
one host is one cause (D6). It is read from the file's absence, never
from moon's output (record 0105; rule core.35). It names moon as the
product the check profile drives, which rule core.26 allows.

### D5. The core reads an unkept pass as a check that could not run

In `finish_run`, after `proven` answers and before anything is
recorded, each unkept task leaves `report.tasks` and joins
`report.not_run` with its reason. The record loop never meets it, so
nothing is recorded under its key (core.7, which does not rule out a
check with no outcome). `verdict::read` never meets it, so it is not
failed. The `not_run` reading that follows does the rest, by what the
code already does:

- nothing failed on its own result, under the bound: `Lost` with each
  check's reason, the run open again in a new place, nobody blamed
  (114, 16);
- a check failed on its own: the combination fails on it, and the
  unkept pass is named as could not run (`FailedTask::not_run`);
- the third such run in a row: each check is named failed with why, and
  the culprit is told (115, 20).

The early returns for an unproven replay (108) and for results that do
not count (87) stay first. A run that hits one of them is run again
anyway, and its unkept passes count toward no bound. `verdict::read`'s
doc says that an unkept pass never reaches it.

- Rejected: reading unkept passes in `verdict::read` as `unjudged`.
  `verdict::read` reads recorded results. An unkept pass has none, and
  the run's `not_run` is the run's own word on what it did not judge.

### D6. The standing cause: `proof_missing`, `proof-missing` and `proof-kept`

A new module, `crates/sluice-core/src/proof.rs`, holds one pure
function:

`note(standing: &mut Vec<ProofMissing>, host: &str, unkept: &[NotRun],
proved: bool, now: &str) -> Vec<Event>`

- For each distinct reason among `unkept`, in order of first
  appearance: where `standing` holds `(host, why)`, its `runs` grows by
  one. Otherwise an entry `{host, why, since: now, runs: 1}` is pushed,
  and `Event::ProofMissing { host, why }` is returned.
- Where `unkept` is empty and `proved` is true, so the run proved at
  least one ran pass, each of `host`'s entries is removed. For each,
  `Event::ProofKept { host, why, since, stood, runs }` is returned,
  with `stood` the whole seconds from `since` to `now`.
- Otherwise nothing changes. A run that passed nothing proves nothing.

`BranchDoc::proof_missing: Vec<ProofMissing>`, with
`#[serde(default, skip_serializing_if = "Vec::is_empty")]`, keeps the
standing causes in the branch's live records, as `refused_writes` keeps
148's. A record from before this change has none, and an older sluice
passes over the field. The events serialize as `proof-missing` and
`proof-kept`. `finish_run` calls `note` on a signed branch right after
`proven` answers, before the early returns of D5, with
`self.peer.name`, the host that reads the run and holds its place, so
ran it. It logs what `note` returns at the round's time. The cause
stands in the records because the round that reads the run writes them
there. Nothing has to be carried in the host's own state, as 133 must
for a remote it cannot write. No round, gate or landing reads
`proof_missing` (core.14). The log and `branch.json` show it.

- Rejected: the run's `Lost` line as the only record. Each of a
  combination's three runs would say it again, on every combination
  the host takes, and a reader would have to gather a standing cause
  from them.
- Rejected: ending a cause only by a run of the same store. A run whose
  proof is kept says the host keeps proof again. A cold run kept in its
  own checkout that ends a cause the shared store still has costs one
  more `proof-missing` line at the next warm run, and the cause is
  logged again.
- Rejected: a bound on the causes a host keeps, as 148 has twenty. A
  cause is one store on one host, and a branch's runs on one host read
  one or two.

### D7. A run that had no outcome says why, for every cause

`SystemRun::lost: Option<LostRun>`, where
`LostRun { host: String, system: Option<String>, not_run: Vec<NotRun> }`,
with `#[serde(default, skip_serializing_if = "Option::is_none")]`.
`read_not_run` takes the reading host and, where it opens the run
again, sets the run's `lost` to that host, the run's system and the
checks it left not run. `finish_run` clears the run's `lost` as it
starts reading the run's next report, whatever that report says.
`CandidateView::lost: Vec<LostRun>` holds each run's that has one, with
`#[serde(default, skip_serializing_if = "Vec::is_empty")]`, so a
follower reading an older document reads none.

`progress::testing_line` adds a part after `waiting`'s:
`its last run, on <host>, had no outcome: <tasks> could not run: <why>`,
and `; <tasks> could not run: <why>` for each further reason, in order
of first appearance. `<tasks>` names up to three checks in the run's
order, then ` and <n> more`. Where the run names a system, the host is
followed by ` (<system>)`. Several runs are joined by ` · `.
`progress::moved` includes `lost`, so a wait prints the line again when
it appears or changes. `with_waiting` puts `lost` in
`sluice status <n> --json` and `sluice wait --json` as
`[{"host": …, "system": …, "not_run": [{"task": …, "reason": …}]}]`,
`system` left out where `None`.

This holds for every cause 114 gives a run with no outcome: a check
that could not start, a `not_run` exit, a refusal, an engine gone, a
switch, a pass with no proof. The core cannot tell one cause from
another without a kind on `NotRun` (D1). A reader waiting on a
combination is served the same whichever cause it is.

- Rejected: showing only a pass with no proof. The core would need a
  kind it reads from the profile, and a combination tested again for
  any other cause would still say nothing until the bound blamed it.
- Rejected: reading the last `Lost` line from the log. A status and a
  wait read the live records and the index alone (core.41), and the log
  holds no host for it.

### D8. The fakes

`World::keeps_no_proof(Option<&str>)`: with `Some(store)`, from now
on, `FakeTests::proven` answers each ran pass of a run in `unkept` with
`it passed, but nothing the run kept proves it ran: its record is not
in <store>`, and leaves it out of `proven`. Fails, the tasks a check
was failed by, and replays are left as today. A run that keeps no proof
keeps no pass in the fakes' cache either, as a store a run could not
write holds neither, so the runs that 115 counts come in a row. `None`
keeps proof again. The fakes name no moon (core.1). `World::forges(task)`
still makes a key the tree does not bear out, which is in neither list.

- Rejected: caching a pass whatever proof the run kept. The next run
  would replay it, find no trusted pass for its key (108) and run again,
  so an unproven replay would sit between each pair of runs with no
  outcome, a run that moon, whose cache is the store it could not
  write, never makes.

## Risks / Trade-offs

- [A host whose store stays unwritable makes every combination run
  three times, then blames its newest submission with why] → That is
  115 as written. The `proof-missing` line names the host and the store
  once, and record 0105 point 1 removes the cause.
- [A lost run's checks are listed with one reason each, which is long
  for a run of 28 tasks] → The `Lost` line is the run's record (16). The
  testing line names three checks and how many more.
- [The testing line now says why for causes other than a missing proof]
  → It adds a line to status and wait between a lost run and the next
  result. No JSON field changes but the added `lost`.
- [A run read on a host other than the one that ran it] → `proven` reads
  the run's place, which only the host that ran it holds. The note's
  host is that host.

## Migration Plan

None. Records an older sluice wrote have no `lost` and no
`proof_missing`, and are read as before. An older sluice reading newer
records passes over both fields. The two log events are new: an older
sluice that reads the whole log by its own events does not know them,
as with every event added before them. The overview reads the log as
JSON and keeps only the events it shows.
