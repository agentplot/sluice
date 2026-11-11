## Context

See proposal.md for why. The statement already says what holds. Clause
114 gives an unkept pass no outcome. The C.2 moon row proves a key "by
moon's own manifest of it, read where moon keeps it, in the checkout or
in the shared store moon's own settings place it". Clause 108 counts a
replay only through a signed pass. Record 0105 reads why from the fact
itself. No clause changes. No other tool of the family signs runs or
reads a task runner's record of them, so nothing here needs reconciling
with Flume, Boomdrive, timber, weir or boom.

### What moon 2.6.1 does

Read from moon's source at v2.6.1, the version every repository named
here pins:

- **Every hashed task gets a manifest.** `TaskRunner::internal_run`
  hashes every task it does not skip, cached or not ("Always generate a
  hash"). `TaskRunner::hash` stores the manifest through
  `Storage::store_hash_manifest_with_hasher`, into every writable local
  backend. It puts the same digest in the run report's
  `hash-generation` operation, the key sluice records. So the key sluice
  records is moon's hash for the task as moon stored it. A task with
  `cache: false`, such as `root:install` or `root:format`, is hashed and
  its manifest stored like any other. Only its outputs go uncached.
- **The local backend is always registered.** `Session::get_cache_engine`
  always registers it, in the shared directory where the shared worktree
  setting is on in a linked worktree, else in the checkout's
  `.moon/cache`. `moon::store` mirrors that choice.
- **A failed write is passed over.** `StorageBackend::store_blobs_batched`
  answers `Ok` when a write fails. It logs "Failed to store blobs, will
  skip caching the manifest" and leaves no manifest. A missing manifest
  is therefore a write that failed, or a write to a store sluice did not
  read.
- **A dependency names a key or a mark.** `dep_hash_input` gives each
  dependency's entry in `deps` a key, or one of four marks:
  `ignored`, the default for a dependency with no outputs;
  `passthrough`, for an edge the graph ignores or a dependency that
  passed through; and `outputs` and `outputs-passthrough`, for a
  dependency whose outputs are injected as inputs. `check_key` reads
  only `ignored` as a mark and fails on the other three as "no key", a
  key the tree does not bear out.

### What a run kept on deneb

No branch of any family repository on deneb names signers today, so
deneb reads no proof. Each `ran` line its repositories recorded since
2026-11-09 was checked against the store `moon::store` names for its
runs:

- **Open runs kept every manifest.** For crew, 1031 of 1031 keys were
  in `<common>/moon-home/cache/shared`, the home its devenv gives moon.
  For weir, 260 of 260. For swancloud, whose moon home is the default
  and whose shared store is the host's cache dir reached through
  `~/.moon/cache/shared`, 2751 of 2751. That is chuck-herdr-alpha's
  layout.
- **Sandboxed runs kept none.** flume, sluice and boomdrive ran under
  0.78.1, whose sandbox refused moon's write to the store: 245, 1134
  and 1696 keys, with no blob anywhere under the repositories, the
  caches, `~/.moon` or the scratch. This is record 0105's cause.
  The sandbox grant that fixes it first ships in v0.86.0.
- **A scratch run of moon 2.6.1 kept all three.** It reproduced the
  candidate's shape: a bare repository with a linked worktree, the
  shared store reached through a link, `MOON_CACHE_SHARED_WORKTREE_CACHE`
  set, `unstable_sharedWorktreeCache` and `casOutputsCache` on,
  `root:install` and `root:format` with `cache: false`, and a test
  depending on `root:install`, run as sluice runs it with `--force`.
  All three manifests landed in the shared store, and each dependent
  named `root:install` as `ignored`.

So on a host where moon can write the store `moon::store` names, an
open run's passes are provable, uncached tasks and their dependents
among them. sluice's resolution, its key and its walk agree with moon's.

### What is not known of chuck-herdr-alpha

The host cannot be read from here. Its facts fit a write moon could not
make, or a store moon chose that sluice did not read: no blob anywhere
for the run's keys, while other processes write the shared store all
day. 0.78.1 predates record 0105's no-outcome reading, which first
ships in v0.81.0, so every unkept pass there is read as a key the tree
does not bear out and blamed. Its own words reach only a tracing
warning.

### The code today

- `Checks::proven` (`crates/sluice/src/profiles/checks.rs`) reads
  manifests from `Meta::store` alone. Where a manifest is missing it
  says `moon's manifest of it is not in <store>` for the check, whether
  the missing one is its own or a dependency's. It names no key. Every
  other refusal goes to `tracing::warn!` only.
- `Proof { proven, unkept: Vec<NotRun> }` (`crates/sluice-seams`).
  `NotRun { task, reason }`.
- `proof::note` (`crates/sluice-core/src/proof.rs`) logs `proof-missing`
  once for each host and distinct reason.
- `verdict::read` (`crates/sluice-core/src/verdict.rs`) puts a replay
  with no counted pass for its key in `unproven: Vec<String>`.
  `finish_run` (`step.rs`) logs `Unproven { combination, tasks }` and
  runs the combination again with no replay.
- `verdict::read` reads a ran task that `proven` leaves out as failed
  under its key (`FailedTask { task, hash, .. }`), with no why.

## Goals / Non-Goals

**Goals:**

- A pass is proven wherever the run's moon kept its manifest, in either
  store moon's settings can choose.
- moon's four dependency marks are read as moon writes them.
- Each key a signed run leaves unproven, unkept or refused, is named in
  the branch's records with its key and its cause.
- Where moon kept no manifest, the cause names the store's own fact, so
  the host's operator sees what to fix.

**Non-Goals:**

- Changing where moon keeps its manifests, or what the host declares.
- Reading moon's printed words (record 0105; rule core.35).
- Proving a key whose manifest names inputs the tree does not hold.

## Decisions

### D1. A manifest is read in the record's store, then in the checkout's own

`proven` reads a key's manifest with `moon::blob_in` in `Meta::store`.
Where that store is not the checkout's own `.moon/cache` and lacks the
blob, it reads `<place>/.moon/cache`. The walk in `check_key` reads
every manifest, the key's own and each dependency's, through that one
lookup. Nothing else changes: the manifest must still hash to its key,
name its task, and name inputs the tree holds.

The C.2 moon row reads a manifest where moon keeps it, "in the checkout
or in the shared store moon's own settings place it". moon chooses
between those two stores from its reading of the checkout and of its
settings. `moon::store` mirrors that reading, and a host where the two
readings part keeps its manifests in the checkout, which its release
then removes. The checkout's store is the run's alone: sluice made the
checkout for this run, and no other run or session writes it. A
manifest found there proves nothing weaker than one found in the shared
store. The proof is the manifest's content, not where it lay.

Not taken: searching the host's declared caches (rule core.9 rules it
out); reading the base checkout's `.moon/cache` or moon's home when the
record names the other (both are shared, written by every run and
session, and `moon::store` already names whichever moon's rule gives).

### D2. moon's dependency marks are no key

In `check_key`, a dependency whose entry is `ignored`, `passthrough`,
`outputs` or `outputs-passthrough` contributes no key, and nothing is
walked for it. Any other value that is not a key is refused as today.

moon 2.6.1 writes those four marks (`dep_hash_input`), and none names a
manifest. A dependency marked `outputs` has its outputs among the
task's inputs, so the inputs check still reads them. Since the tree
holds no outputs, such a key is still refused, now with why (D5).

### D3. An unkept pass names its key and the store's fact

`Proof::unkept` stays `Vec<NotRun>`. `NotRun` gains
`key: Option<String>`, `#[serde(default, skip_serializing_if =
"Option::is_none")]`: the key of the record that is missing, set only
where the check profile names one. `check_key`'s `NoManifest` already
names the task and key whose manifest is missing. The reason reads:

```
it passed, but nothing the run kept proves it ran: moon's manifest of <whose> is not in <store>, <fact>
```

`<whose>` is `it`, where the missing manifest is the check's own, or
`<task>, which it depends on`. `<store>` is the record's store,
`Path::display`. The checkout's path is left out of the reason, since a
checkout's path differs every run. `<fact>` is the first that holds of:

1. `which does not exist`: the store is not a directory;
2. `which <user> cannot write`: `access(2)` with `W_OK` fails on the
   store's `blobs`, or on the store where `blobs` is absent;
3. `a directory of which <user> cannot write`: the key's shard,
   `blobs/<first two>`, exists and `access(2)` with `W_OK` fails on it;
4. `where moon kept none of the run's manifests`: none of the keys of
   the report's tasks and dependencies, ran or replayed, has a blob in
   either store;
5. `where moon kept others of the run's manifests`.

`<user>` is the name of the user sluice runs as, else its uid. The
facts are read by sluice's own process, after the run and before its
checkout is released, from the store itself. Nothing is run and nothing
moon printed is read. This is record 0105's "from the fact itself",
widened from "the manifest is missing" to what the store says of it.
`moon::store_fact(store, key, kept) -> StoreFact` reads it, and
`StoreFact`'s `Display` gives the words. Its tests make each fact in a
directory of their own.

The reason carries no key, so `proof::note` still logs `proof-missing`
once for each host and reason. The testing line still groups tests by
reason, one reason for a run whose store kept nothing. The key goes on
`NotRun::key`, which the `lost` line, `LostRun` and the JSON under
`lost` carry.

The facts are ordered so the first names what the host must change:
a store that is absent or that the user cannot write is the host's
setting, and a store kept by others but not by this run is the run's
own environment.

Not taken: moon's warning in the run's output (rule core.35 reads the
cause from the fact, never from a tool's words); a key in the reason
(one `proof-missing` line and one testing-line group per key);
counting the run's manifests in the reason (a reason that changes every
run is a new cause every run).

### D4. A replay no counted pass proves names its key and why

`Verdict::unproven` becomes `Vec<UnprovenKey>`, `UnprovenKey { task,
key, why }` in `crates/sluice-core/src/records.rs`. `verdict::read`
reads `why` from the results it already holds, as the first that
holds of:

1. `no run of its key is recorded`: no `ran` line for the task at the
   key;
2. `its key ran only on another system`: none on the run's system;
3. `no result of its key is signed by a signer the branch counts`, on a
   signed branch, or `no result of its key is by a run the branch
   trusts`, otherwise: none whose signer, or executor, the branch
   counts (78, 79);
4. `no result of its key was held to this run's limits`: on a branch in
   one host's own files, trusted results only under other limits
   (19, 83);
5. `no result the branch counts for its key is a pass`.

`Event::Unproven` keeps `tasks` and gains `keys: Vec<UnprovenKey>`,
`#[serde(default, skip_serializing_if = "Vec::is_empty")]`, so an older
line reads with none. `finish_run` logs both.

A signed branch's first runs replay from moon's cache keys no signer
has signed yet. Each is now logged `no result of its key is signed by a
signer the branch counts`, which says this is 108's reading and not the
manifests.

### D5. A key the tree does not bear out is blamed with why

`Proof` gains `refused: Vec<Refused>`, `Refused { task, key, why }`.
`proven` puts there each task whose `check_key` fails other than with
`NoManifest`, with `why` the error's `{:#}`, which names the task. It
keeps today's `warn!`. `FailedTask` gains `unsigned: Option<String>`,
`#[serde(default, skip_serializing_if = "Option::is_none")]`. Once
`verdict::read` answers, `finish_run` sets it on each failed task the
refusals name at its key. A waiting submit's outcome and `sluice wait`
say, under the failed line, as they say a test that could not run,
`  <task>: the combination's tree does not bear its key out: <why>`
(`words::not_borne_out`).

Not taken: a log line per refusal (the submission's record is where a
blamed check's why is read, as `not_run`'s is); the verdict report's
rows (out of scope).

### D6. The fakes

`World::keeps_no_proof` gives each unkept pass `key: Some(<its key>)`,
with its reason as today. `World::forges` answers the forged task in
`refused` with `why` `<task>: its record names an input the tree does
not hold`. The fakes name no moon (rule core.1).

### D7. The record and the rules

Record 0105 gains an amendment in its status paragraph: on 2026-11-11
the construct stage of this change chose, under the C.2 moon row and
point 2, to read a manifest in the checkout's own store where the
record's store lacks it, and to give the store's fact with why, read
from the store itself. Rule core.9's details gain under "Not ruled out"
the checkout's own store read where the record's store lacks a
manifest. Rule core.35's details gain under "Not ruled out" the
store's own facts, its absence and its user's access, read with why.

## Risks / Trade-offs

- **`access(2)` answers for sluice's process, not for the run's.** An
  executor that confined the run differently from sluice would give
  another answer. The open executor runs the command as sluice's child
  with sluice's user and namespaces, so the answers match. The sandbox
  writes the store since v0.86.0 (record 0105, point 1), and the fact
  of D3 then reads 4 or 5.
- **The checkout's store read on a host where moon shares.** It can
  only find what this run's moon wrote there, since nothing else
  writes it. A manifest found there means `moon::store` and moon's own
  choice parted on that host. The fix to `moon::store` would follow
  once a host shows it, and the proof holds meanwhile.
- **0.78.1 hosts gain nothing until they upgrade.** The box needs a
  sluice that carries record 0105 and this change. Until then its
  signed branches land nothing, as today.

## Migration Plan

Additive fields, each skipped when empty and read as empty when absent:
`NotRun::key`, `FailedTask::unsigned`, `Event::Unproven::keys`. Older
lines, records and documents read as before. Rollback is a revert.

## Open Questions

- **Which fact holds on chuck-herdr-alpha.** Its first signed run under
  a sluice with this change logs `proof-missing` with the fact. If the
  fact is 1, 2 or 3, the host's store or the confinement of the process
  that starts sluice's runs is fixed where the host's configuration is
  managed. If it is 4, moon kept nothing in either store sluice reads:
  the run's environment gave moon another home, and its `MOON_HOME`,
  `XDG_DATA_HOME` and `HOME` as the run saw them are the next reading.
  If it is 5, the store's lifetime or size settings removed manifests.
  If D1 proves the keys, the host's moon chose the checkout's store,
  and `moon::store` is corrected to match.
