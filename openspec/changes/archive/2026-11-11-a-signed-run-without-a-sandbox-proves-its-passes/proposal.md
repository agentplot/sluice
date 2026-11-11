## Why

On a branch whose records are on its remote, a pass counts only where
moon's own manifest proves its key, read where moon keeps it (C.2 moon
row, C.6; rule core.9). Once switchboard-kit's `main` named
`[signers.sessions]` at f192c075 (#266), on 2026-11-10 from 18:49:56Z,
every later run of it on chuck-herdr-alpha was blamed: all 117 checks
of round 2f13b2a, whose logs show passes. breadboard-kit #48 and
atlas-kit #351 went the same way once their `main`s named signers. That
host runs the open executor, with no sandbox, under sluice 0.78.1. Its
runs share moon's cache: `MOON_CACHE_SHARED_WORKTREE_CACHE` from the
host's `[[caches]]`, and `unstable_sharedWorktreeCache` and
`casOutputsCache` in the workspace. Run c982 (submission 1732) names
the store `/home/chuck/.moon/cache/shared`, which links to
`/workspace/caches/moon`. moon writes blobs there all day, yet no blob
exists anywhere on the host for the keys the run recorded.

Three things went wrong, and the branch's log says none of them:

- **The manifests are missing, and nothing says why.** moon 2.6.1
  writes the manifest of every task it hashes, cached or not. A write
  that fails is a warning moon logs and passes over. So a missing
  manifest means one of two things: moon could not write the store, or
  it kept the manifest in a store sluice did not read. sluice says
  neither. It names only the store it looked in.
- **0.78.1 blames every such pass.** Record 0105's no-outcome reading
  of an unkept pass arrived after 0.78.1, so that host reads each one as
  a key the tree does not bear out. Even under the record, three runs
  in a row with no proof blame the newest submission (115). Either way
  a signed branch there lands nothing.
- **The `unproven` line names no key.** The log's `unproven` line, the
  first thing a signed branch logs, is the reading of 108: replays that
  no counted pass proves, run again with no replay. It names only the
  combination and its tasks, so operators read it as the missing
  manifests. And where a key really is not borne out, the blame says
  why only in a tracing warning.

This change makes a pass provable wherever the run's moon kept its
manifest, reads moon's dependency marks as moon writes them, and makes
every key that goes unproven name its key and its cause in the
branch's records. Where moon kept no manifest at all, the cause names
the fact that explains it, so the host's operator can fix the host.

## What Changes

- **A manifest is read where the run's moon kept it.** The proof reads
  a key's manifest in the store the run's record names. Where that
  store lacks it, the proof also reads the checkout's own
  `.moon/cache`, the other store moon's settings can choose, which
  holds this run's alone. Either way the manifest is checked by its
  content, as before (C.2 moon row; rule core.9).
- **moon's dependency marks are read as moon writes them.** A
  dependency a manifest names `passthrough`, `outputs` or
  `outputs-passthrough` contributes no key, just as `ignored` does.
  Each is read as nothing to walk, not as a key the tree does not bear
  out (moon 2.6.1, `dep_hash_input`).
- **An unkept pass names its key and what the store says.** The reason
  names whose manifest is missing, the check's own or one it depends
  on, the store, and one fact about that store: it does not exist; the
  host's user cannot write it, or one of its directories; moon kept
  none of the run's manifests there; or moon kept others of them there.
  The facts come from the store itself, never from what moon printed
  (114; record 0105). Each test the run left not run carries the
  missing manifest's key in the `lost` line and under `lost` in the
  JSON.
- **A key the tree does not bear out is blamed with why.** The
  submission's failed check carries the check profile's words for it as
  `unsigned`, and a waiting submit and `sluice wait` say it.
- **The `unproven` line names each replay's key and why.** Beside its
  tasks, `unproven` carries `keys`: each replay's task, key and why no
  counted pass proves it. The reasons are: no run of the key is
  recorded; the key ran only on another system; no result of the key
  is signed by a signer the branch counts, or is by a run the branch
  trusts; no result of the key was held to this run's limits; or no
  counted result of the key is a pass (108, 19, 83).
- The configuration reference, the state reference, record 0105 and
  rules core.9 and core.35 say so.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pass-proof`: a manifest is read in either store moon's settings can
  choose for the run, moon's dependency marks are read as moon writes
  them, an unkept pass names its key and the store's fact, and a key
  the tree does not bear out is blamed with why.
- `run-signing`: a replay no counted pass proves is logged with its key
  and why.

## Out of scope

- **The host's own fix.** Where a store cannot be written by the run's
  user, or a sluice process runs under a confinement its caller set,
  the host is fixed where its configuration is managed. This change
  makes the branch's log name which fact holds. Running a sluice that
  carries record 0105 there, so that unkept passes blame nobody, is a
  deploy.
- **Reading what moon printed.** moon's own warning, "Failed to store
  blobs, will skip caching the manifest", names the failure. Record
  0105 and rule core.35 read why from the fact itself, never from a
  tool's words.
- **A task whose inputs are a dependency's outputs.** moon names those
  outputs among the task's inputs. The combination's tree does not hold
  them, so such a key is still not borne out. It is now blamed with
  why.
- **Searching the host's declared caches, or any store but the two
  moon's settings choose between.** Rule core.9 rules out the first.
  The second would read manifests that other runs and sessions write.
- **The verdict report's rows.** A pull request's report lists a check
  not borne out as failed, as today. Giving the why there waits for a
  reader who asks for it.

## Impact

- `crates/sluice-seams/src/lib.rs`: `NotRun::key`, `Proof::refused`,
  `Refused`.
- `crates/sluice/src/profiles/moon.rs`: `check_key`'s dependency marks,
  `StoreFact` and `store_fact`.
- `crates/sluice/src/profiles/checks.rs`: `proven` reads the
  checkout's own store, names each unkept key and the store's fact, and
  answers the keys it refuses with why.
- `crates/sluice-fakes/src/lib.rs`: `FakeTests::proven` answers keys
  and a forged key's why.
- `crates/sluice-core/src/records.rs`: `FailedTask::unsigned`,
  `UnprovenKey`, `Event::Unproven::keys`.
- `crates/sluice-core/src/verdict.rs`: `Verdict::unproven` with each
  replay's key and why.
- `crates/sluice-core/src/step.rs`: `finish_run` logs `keys` and fills
  `unsigned`.
- `crates/sluice/src/lib.rs`, `crates/sluice/src/words.rs`: a failure's
  text says why a key was not borne out.
- Tests: `crates/sluice/tests/profiles/moon.rs`,
  `crates/sluice-conformance/tests/unkept_proof.rs`,
  `crates/sluice-conformance/tests/verdicts.rs`, the unit tests of the
  files above.
- Docs: `site/src/content/docs/reference/configuration.md`,
  `site/src/content/docs/reference/state.md`,
  `docs/adr/0105-a-sandboxed-run-writes-the-store-its-check-profile-reads-and-a-pass-it-cannot-prove-has-no-outcome.md`,
  `docs/architecture/core.md`.
- Sources: clauses 16, 18, 20, 108, 114, 115, C.2 (its moon row), C.6;
  record 0105; rules core.1, core.7, core.9, core.22, core.26, core.28,
  core.35, core.41.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice/src/profiles/moon.rs
crates/sluice/src/profiles/checks.rs
crates/sluice-fakes/src/lib.rs
crates/sluice-core/src/records.rs
crates/sluice-core/src/verdict.rs
crates/sluice-core/src/step.rs
crates/sluice-core/src/ops.rs
crates/sluice/src/lib.rs
crates/sluice/src/words.rs
crates/sluice/tests/profiles/moon.rs
crates/sluice/tests/profiles/conformance.rs
crates/sluice-conformance/tests/unkept_proof.rs
crates/sluice-conformance/tests/verdicts.rs
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/state.md
docs/adr/0105-a-sandboxed-run-writes-the-store-its-check-profile-reads-and-a-pass-it-cannot-prove-has-no-outcome.md
docs/architecture/core.md
openspec/changes/a-signed-run-without-a-sandbox-proves-its-passes/**
```
