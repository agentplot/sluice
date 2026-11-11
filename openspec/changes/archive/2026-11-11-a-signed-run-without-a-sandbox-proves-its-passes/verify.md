# Verify: a-signed-run-without-a-sandbox-proves-its-passes

Verified on 2026-11-11, on `change/a-signed-run-without-a-sandbox-proves-its-passes`
at `38e8a7fd`, whose base is `bolt/a-signed-run-without-a-sandbox-proves-its-passes`
at `92d39e53`.

## Outcome

Archived. The whole checks pass, all 17 tasks are ticked and hold, and
every scenario of both delta specs is met by the test it names.

## The whole checks

`sluice exec bolt/a-signed-run-without-a-sandbox-proves-its-passes`
(sluice 0.78.1) ran the bolt's gate, `#check-fast`, once against `38e8a7f`.
It exited 0. All 12 tasks passed: `cli:test`, `conformance:test`,
`core:test`, `fakes:test`, `seams:test`, `nix:typecheck`, `root:adrs`,
`root:changes`, `root:format`, `root:lint`, `site:prose` and `site:rules`.
The suites ran 1189 tests, with 0 failed and 0 ignored. `site:prose`
reported 0 errors and 0 warnings (37 suggestions) in 19 files. sluice
recorded 12 results under their keys for `38e8a7f`.

moon printed "Failed to store blobs" for each of the session run's 12
tasks, with the store under `sluice.git/moon-home/cache/shared`, and
"Failed to package task outputs" for the checkout's `.moon/cache`. That is
record 0105's known cause on this host under 0.78.1's sandbox, as the
design's "What a run kept on deneb" says. It decides nothing here, since a
session run belongs to no branch and is not signed.

Task 2.7's own check, which the gate cannot run, also passed: `sluice exec
bolt/a-signed-run-without-a-sandbox-proves-its-passes -- sh -c 'cargo doc
-p sluice --lib --no-deps && … grep -q dep_hash_input fn.check_key.html &&
! grep -q "Design D9" constant.DEPENDENCY_MARKS.html'` exited 0. The
rustdoc build gave 6 warnings, all on lines this change did not write
(`github/app.rs:528`, `records.rs:141`, `685`, `701`, `progress.rs:190`,
`lib.rs:1129`).

`openspec validate a-signed-run-without-a-sandbox-proves-its-passes --strict`:
valid.

## Specs against the work

### pass-proof

| Requirement / scenario | Where it is met | Holds |
|---|---|---|
| Why is read from the missing proof itself: A manifest gone from the store | `a_pass_whose_manifest_is_gone_is_unkept_naming_the_store` expects `key: Some(alpha)` and D3's reason ending `where moon kept others of the run's manifests` | yes |
| … What the store says of itself | `the_store_says_why_a_manifest_is_missing`: each of the five facts in its own directory, with the two unwritable cases skipped as root; `store_fact` reads `access(2)` on `blobs`, or on the store where `blobs` is absent, then on the key's shard | yes |
| … A dependency's manifest missing | `a_missing_manifest_is_told_from_one_the_tree_does_not_bear_out` asserts `NoManifest { task, key }` two deep; `Checks::proven` takes `<whose>` from `NoManifest::task` | yes |
| A manifest is read where the run's moon kept it: moon kept the manifest in the checkout / Neither store holds it | `a_manifest_moon_kept_in_the_checkout_proves_its_key`; `Checks::proven`'s one `blob` lookup falls back to `<place>/.moon/cache` only where the record's store is another, and `check_key` walks with it | yes |
| moon's dependency marks are no key: Each mark | `moons_dependency_marks_are_no_key` walks `b` then `a` alone and refuses `stale` as no key; `check_key` matches `DEPENDENCY_MARKS` | yes |
| A task moon does not cache is proven: An install and a check depending on it | `a_task_moon_does_not_cache_and_its_dependent_are_proven`: `alpha:install` with `cache: false`, `replay: false`, both manifests in `cache/shared`, the dependent's naming it `ignored` | yes |
| Each test left without proof names the missing key: The lost line | `a_lost_line_names_each_unkept_key`: both keys on the `lost` line and in its JSON, the reason carrying none, and one `proof-missing` | yes |
| A key the tree does not bear out is blamed with why: A forged key | `a_key_the_tree_does_not_bear_out_is_blamed_with_why`: `finish_run` sets `unsigned` from `proof.refused` matched by task and key | yes |
| … The words | `a_key_not_borne_out_is_said_with_why`; `report` writes the line under `failed:`, which the waiting submit and `sluice wait` both print | yes |
| … The check profile says why | `proven_drops_a_key_the_tree_does_not_bear_out` asserts `refused` with the key and `check_key`'s `{:#}` | yes |

### run-signing

| Scenario | Where it is met | Holds |
|---|---|---|
| Each why | `an_unproven_replay_says_why_by_its_key`: all five whys on a branch in a host's own files, and the signed wording on a branch whose state is on its remote | yes |
| The unproven line | `a_replay_no_trusted_pass_proves_runs_again_with_no_replay`: three tasks under `keys` with their replayed keys and the limits why | yes |
| An older line | `an_unproven_line_and_an_unsigned_check_read_back`: reads with none and is written back unchanged | yes |

## Tasks

All 17 are ticked (1.1 to 2.7, 3.1, 4.1 to 4.3, 5.1, 5.2, 6.1, 7.1, 7.2).
Each named test exists, and each ran in the gate or, for 2.7, in its own
check above. `check_key` carries its own doc, citing `dep_hash_input` and
design D2, and `DEPENDENCY_MARKS` carries only the marks'. The docs of 7.1
and 7.2 say what the tasks ask, and `site:prose` passed. Record 0105's
amendment of 2026-11-11 and the details of rules core.9 and core.35 are
written.

## The archive

`openspec archive` applied both deltas: 5 requirements added to and 1
modified in `pass-proof`, and 1 added to `run-signing`. Afterwards,
`openspec validate --specs --strict` passes 73 specs and fails one,
`pass-proof`. The failure is a warning: the modified requirement "Why is
read from the missing proof itself" has 995 characters of text, over
openspec's 500, where it had 444 before. The change itself validated
`--strict` with the same text, since openspec measures a requirement's
length only in a main spec. The gate's `root:changes` runs `openspec
validate --changes --strict` and reads no main spec, so no gate fails on
it, this branch's or a later change's.

This stage archived anyway. The work meets every scenario, and the
requirement's length is the spec's wording, which a later change can
split cheaply: for example, moving the fact list into a requirement of
its own. Sending the change back to its design for this warning alone was
the path not taken. Whoever next changes `pass-proof` should split the
requirement so `--specs --strict` passes again.

## Observations

These need no change:

- The proposal's Touches lists `crates/sluice-core/src/ops.rs` and
  `crates/sluice/tests/profiles/conformance.rs`, and the work changed
  neither. It added `key: None` to literals in
  `crates/sluice-core/src/proof.rs` and `crates/sluice/src/progress.rs`,
  which Touches does not list. The field is new, and neither file needed
  more.
- The spec's scenario "An install and a check depending on it" names
  `root:install`. The test's uncached task is `alpha:install`, in the
  workspace the profile's tests share. The scenario's claim is the same.
- No test drives `Checks::proven` to the `<task>, which it depends on`
  wording. The spec's scenario for a missing dependency names the
  `check_key` unit test, which shows the task and key that wording is
  built from.
- In `site/src/content/docs/reference/state.md`, the `unsigned`, `lost`
  and `unproven` paragraphs each end "… written by an older sluice has no
  `<field>`:" and then show an example that has the field. The example
  shows the field, not an older record, so the colon may mislead a reader.
  Task 7.2 asks for an example of each, and each is given.
