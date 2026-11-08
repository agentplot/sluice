# Verify

This verify checked the work on `change/a-run-a-switch-of-its-host-broke-runs-again`
at b9929214 on 2026-11-07. The branch was cut from
`bolt/a-hosts-switch-fails-no-run` at a476703c. The work was checked
against this change's proposal, its design, its delta spec
(`host-requirements`) and its tasks.

**The work holds, and the change is archived.** Every scenario of the
delta spec has a test that passes. All 20 tasks are ticked and their
conditions are met. The bolt's gate passes. Once archived, the change
leaves all 67 specs valid.

## Checks

| Check | Result |
|---|---|
| `sluice exec bolt/a-hosts-switch-fails-no-run` on b9929214, against the bolt at a476703 | **exit 0**: `passed: 9 tasks` in 1m 47s, `recorded 9 results under their keys for b992921` |
| The tasks of `#check-fast` | `root:changes`, `root:format`, `root:lint`, `site:prose`, `seams:test`, `core:test`, `fakes:test`, `conformance:test`, `cli:test`; each passed |
| `sluice exec bolt/a-hosts-switch-fails-no-run -- cargo test -p sluice --features tools --test exec -- --exact <5.1's test> <5.2's test>` | `2 passed; 0 failed`. These two tests are in `tests/exec.rs`, which compiles only under `tools`. `tools` is in `#check-tools`, main's gate and not the bolt's, so this run proves tasks 5.1 and 5.2 by their own commands |
| `openspec validate a-run-a-switch-of-its-host-broke-runs-again --strict` | valid |
| `openspec archive … --yes` in a scratch copy of `openspec/`, then `openspec validate --specs --strict --no-interactive` (openspec 1.16.0) | `host-requirements` gains 5 requirements, and **67 of 67 specs pass** |
| `git merge-tree --write-tree HEAD origin/bolt/a-hosts-switch-fails-no-run`, the bolt at a476703c | merges cleanly |
| `grep -niE '\bnix\b\|current-system'` over `crates/sluice-core/src` and `crates/sluice-seams/src` | no match: the core names no Nix (rule core.1, design's goal). The words and the link are the check profile's |

During the gate, moon warned that it could not store the tasks' outputs
in `.moon/cache/outputs`, and the OTLP exporter warned at shutdown.
Every task still passed. Neither warning comes from this change.

## Requirements and scenarios

Every scenario has a test that sets up its WHEN and asserts its THEN.
Every test named passed in the run above that holds it.

### A switch of the host's system during a run leaves its failed tests without a result

| Scenario | Test |
|---|---|
| The system was switched during the run | `a_check_failed_after_the_hosts_system_was_switched_has_no_outcome_and_a_pass_stands` (conformance): `a:test` is not run with `the host's system was switched during the run, from /nix/store/aaa-darwin-system to /nix/store/bbb-darwin-system`; `b:test`'s pass is recorded; no `a:test` line for the combination; nothing is delivered |
| The system stood | the same test's second half: the run again, with the system standing, fails `a:test` under its key, and the submission is failed and blamed |
| Every test passed | `a_run_whose_checks_all_passed_never_reads_the_system`: `system_looks` is unchanged and the submission lands |
| The time limit ended the run | `a_run_its_time_limit_ended_never_reads_the_system`: `system_looks` is unchanged and the result is `timed out` |
| A policy that asks no engine | `a_switch_is_read_whatever_the_policy_asks_of_the_host`: under the bolt policy, with no `docker`, the test is not run with the reason, nobody is blamed, and no engine look is made |
| The engine gone too | `a_switch_and_an_engine_gone_name_the_switch`: the reason is the switch. The one engine look is the claim gate's before the run again (84), and no look follows the run |

### A combination whose run a switch broke is tested again in a new checkout

| Scenario | Test |
|---|---|
| Tested again in a new checkout | `a_switched_run_is_tested_again_in_a_new_checkout_once_its_own_is_released`: the second start's place differs from the first. The first place is not live as the second starts, and `released_places` lists it |
| The broken run's release fails | `a_switched_run_whose_release_fails_is_tested_again_in_a_new_checkout`: `failed_releases` names the broken place with what failed, the place stays live, and the run again starts elsewhere |
| A host that keeps switching | `a_host_that_keeps_switching_blames_with_why_after_three_runs`: after three runs, each switched, the submission is failed. `a:test` has no key and `not_run` holds the switch's words, the delivered failure names `a:test`, and `results.jsonl` holds no `a:test` line |

The design (D6) adds no code for this requirement. It holds through
`read_not_run`, a new place for every claim, and releases before
starts, and these tests pin each of those.

### sluice exec reads a switch of the host's system as a combination's run does

| Scenario | Test |
|---|---|
| The tests | `a_check_that_fails_after_the_hosts_system_was_switched_is_not_run_and_a_pass_is_recorded` (`crates/sluice/tests/exec.rs`, `tools`): output `could not run: a:test: the host's system was switched during the run, from <a> to <b>`; exit 1; only `b:test`'s pass recorded; the worktree, and its checkout in `sluice status`, stay. In the core, `a_session_run_failed_after_a_switch_records_no_fail`'s first half shows the same |
| A command | `a_session_run_failed_after_a_switch_records_no_fail`'s second half: `false`'s fail is its own, and `system_looks` is unchanged |

### A host whose system Nix doesn't manage is read as before

| Scenario | Test |
|---|---|
| No Nix system | `a_host_nix_does_not_manage_reads_nothing_and_blames_as_before` (conformance): one run, no not-run event, `a:test` failed under its key, and the submission blamed. In the profile, `a_host_with_no_system_link_reads_nothing` shows that a link made during a run that began with none reads no switch |
| A run an older sluice started | `profiles::checks::tests::a_run_noted_with_no_system_reads_no_switch`: a `Meta` with no `nix_system` reads none, and the comparison answers `None` whatever the link names now |

### Whether the host's system was switched is read only from the host

| Scenario | Test |
|---|---|
| A test that says the host was switched | `a_check_that_says_the_system_was_switched_while_it_stood_is_recorded_as_failed` (`exec.rs`, `tools`): the check prints the switch's very words while the link stands. It is `failed: a:test`, with no `could not run`, and both results are recorded |

The profile reads the host with `std::fs::read_link` alone
(`nix_system` in `crates/sluice/src/profiles/checks.rs`), and runs no
command (rule core.37). `a_run_whose_system_was_switched_says_from_what_to_what`,
`a_run_whose_system_stood_says_nothing` and
`a_link_gone_after_the_run_reads_no_switch` (`crates/sluice/tests/checks.rs`)
cover the comparison against a real link, design D4's link gone among
them.

## Design

- **D1, D2.** `Checks::start` reads the link after the run's directory
  is prepared and before `Equipping::new`. It keeps the link as
  `Meta::nix_system`, with `serde(default, skip_serializing_if)`.
- **D3.** `Tests::switched` is a required method, forwarded in
  `RecordedChecks`. `after_run` takes the run's handle and asks
  `switched` first, whatever `limits.docker` says. It looks at the
  engine only where no switch was read and `docker = true`, and gives
  the switch's reason unprefixed. All three callers pass the handle,
  `finish_session` among them, and a command after `--` is still not
  read.
- **D4, D5.** The answer is `Some` only where both readings exist and
  differ, in the words the design gives.
- **D8.** The link is read from `with_system_link`, else from
  `SLUICE_SYSTEM_LINK`, else from `/run/current-system`. The fakes
  default to no system and add `nix_system`, `system_looks` and
  `live_at_each_start`.

The configuration reference's new paragraph says what task 4.11 asks.
It names NixOS and nix-darwin as products, and names no host, deployer
or tool of ours (rule core.26). `site:prose` passes.

## Tasks

All 20 tasks in `tasks.md` are ticked (`- [ ]` appears 0 times). Each
task's condition holds:

- 1.1 and 3.1 hold by `root:lint`, which runs clippy over the workspace
  with `-D warnings`.
- 2.1 to 2.5 and 4.1 to 4.10 hold by `cli:test` and `conformance:test`,
  each test named in the log as `ok`.
- 4.11 holds by `site:prose`.
- 5.1 and 5.2 hold by the `tools` run above.

## Findings

None that stop the change.
