# Verify: a-signed-run-without-a-sandbox-proves-its-passes

Verified on 2026-11-11, on `change/a-signed-run-without-a-sandbox-proves-its-passes`
at `ef9c337e`, whose base is `bolt/a-signed-run-without-a-sandbox-proves-its-passes`
at `92d39e53`.

## Outcome

Not archived. The whole checks pass, every task is ticked, and every
scenario of both delta specs is met by the test it names. One task falls
short: 2.1 left `check_key` with no doc, as described under Finding.

## The whole checks

`sluice exec bolt/a-signed-run-without-a-sandbox-proves-its-passes`
(sluice 0.78.1) ran the bolt's gate, `#check-fast`, once against
`ef9c337`. Exit 0. All 12 tasks passed: `cli:test`, `conformance:test`,
`core:test`, `fakes:test`, `seams:test`, `nix:typecheck`, `root:adrs`,
`root:changes`, `root:format`, `root:lint`, `site:prose` and `site:rules`.
The suites ran 1189 tests, with 0 failed and 0 ignored. `site:prose`
reported 0 errors and 0 warnings in 19 files. sluice recorded 12 results
under their keys for `ef9c337`.

moon printed "Failed to store blobs" for this session's run, under 0.78.1's
sandbox. That is record 0105's known cause on this host, as the design's
"What a run kept on deneb" says. It decides nothing here, since a session
run is no branch's and is not signed.

`openspec validate a-signed-run-without-a-sandbox-proves-its-passes --strict`:
valid. The MODIFIED requirement "Why is read from the missing proof itself"
is in `openspec/specs/pass-proof/spec.md`, and the delta keeps both of its
scenarios.

## Specs against the work

### pass-proof

| Requirement / scenario | Where it is met | Holds |
|---|---|---|
| Why is read from the missing proof itself: A manifest gone from the store | `a_pass_whose_manifest_is_gone_is_unkept_naming_the_store` expects `key: Some(alpha)` and the D3 reason ending `where moon kept others of the run's manifests` | yes |
| … What the store says of itself | `the_store_says_why_a_manifest_is_missing`: each of the five facts in its own directory, with the unwritable cases skipped as root | yes |
| … A dependency's manifest missing | `a_missing_manifest_is_told_from_one_the_tree_does_not_bear_out` asserts `NoManifest { task, key }` two deep; `proven` takes `<whose>` from `NoManifest::task` | yes |
| A manifest is read where the run's moon kept it: moon kept the manifest in the checkout / Neither store holds it | `a_manifest_moon_kept_in_the_checkout_proves_its_key`; `Checks::proven` falls back to `<place>/.moon/cache` only where the record's store is another | yes |
| moon's dependency marks are no key: Each mark | `moons_dependency_marks_are_no_key` walks `b` then `a` alone and refuses `stale` as no key | yes |
| A task moon does not cache is proven: An install and a check depending on it | `a_task_moon_does_not_cache_and_its_dependent_are_proven`, with `alpha:install` as the uncached task, `replay: false` (`--force`), and the dependent's manifest naming it `ignored` | yes |
| Each test left without proof names the missing key: The lost line | `a_lost_line_names_each_unkept_key`: both keys on the `lost` line and in its JSON, and one `proof-missing` | yes |
| A key the tree does not bear out is blamed with why: A forged key | `a_key_the_tree_does_not_bear_out_is_blamed_with_why`: `unsigned` set from `proof.refused` in `finish_run` | yes |
| … The words | `a_key_not_borne_out_is_said_with_why`; `report` writes the line under `failed:`, which both the waiting submit and `sluice wait` use | yes |
| … The check profile says why | `proven_drops_a_key_the_tree_does_not_bear_out` asserts `refused` with the key and `check_key`'s `{:#}` | yes |

### run-signing

| Scenario | Where it is met | Holds |
|---|---|---|
| Each why | `an_unproven_replay_says_why_by_its_key`: all five whys on a branch in a host's own files, and the signed wording on a branch whose state is on its remote | yes |
| The unproven line | `a_replay_no_trusted_pass_proves_runs_again_with_no_replay`: three tasks under `keys` with their replayed keys and the limits why | yes |
| An older line | `an_unproven_line_and_an_unsigned_check_read_back`: reads with none and is written back unchanged | yes |

## Tasks

All 17 are ticked (1.1 to 7.2). Each named test exists and ran in the gate.
The docs of 7.1 and 7.2 say what the tasks ask, and `site:prose` passed.
Record 0105's amendment of 2026-11-11 and the details of rules core.9 and
core.35 are written.

## Finding

**Task 2.1: `check_key` lost its doc to `DEPENDENCY_MARKS`.** In
`crates/sluice/src/profiles/moon.rs`, `pub const DEPENDENCY_MARKS` was
inserted between `check_key`'s doc block and `pub fn check_key`, so one
`///` block now runs on without a break. It opens with "Design D9 (C.6):
whether `key` is moon's key for `task` on the combination's tree", and it
cites `dep_hash_input` and design D2 as the task asks. It then goes on into
the marks' own doc, and rustdoc gives all of it to the const. `check_key`,
a public function, has no doc, and the citation task 2.1 asks for "in its
doc" is not on it. The gate cannot catch this. The fix is to put the const
and its own doc (the paragraph beginning "The marks moon 2.6.1's
`dep_hash_input` gives…") above `check_key`'s doc, so each item carries
its own.

## Observations

These need no change:

- The proposal's Touches lists `crates/sluice-core/src/ops.rs` and
  `crates/sluice/tests/profiles/conformance.rs`, and the work changed
  neither. It added `key: None` to literals in
  `crates/sluice-core/src/proof.rs` and `crates/sluice/src/progress.rs`,
  which Touches does not list. Both are fine: the field is new, and
  neither file needed more.
- No test drives `Checks::proven` to the `<task>, which it depends on`
  wording. The spec's scenario for a missing dependency names the
  `check_key` unit test, which shows the task and key that wording is
  built from.
