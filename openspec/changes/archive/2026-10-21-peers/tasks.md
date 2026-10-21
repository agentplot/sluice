# Tasks

This change builds clauses 72 to 82, 85 to 88 and 107 to 112, the
amended 27, 28, 29 and 69, Part C but its launched executor's jobs and
C.8, and scenarios S17 to S20, S41 and S42 of the statement in agentplot/blueprints,
`design/sluice/requirements.md`.

Read first: this change's proposal, specs and design; those clauses
and Part C;
ADRs 0004, 0008, 0011, 0015, 0017, 0019, 0021 (pull requests);
`docs/architecture/core.md`. Work inside `devenv shell`. Fast tier:
`moon run ':#check-fast'`; real tools: `moon run ':#check-tools'`;
whole repositories: `moon run ':#check-e2e'`. Every git call a test
makes runs with no credential helper and no prompt.

## 1. Decisions and rules

- [x] 1.1 Confirm the statement holds 72 to 82, 85 to 88, 107 to 113 and the amended 27, 28, 29, 69 as this change reads them, and C.6 holds design D1 and D9; verify `openspec validate peers --strict`.
- [x] 1.2 Write ADRs 0037 (remote refs records, the lead and the lease), 0032 (signed results: DSSE, in-toto, SSH and keyless), 0033 (a landing reads its verdict from signed results) and 0034 (the pool is the check seam's launched executor, GitHub its only profile, core.3); amend 0004, 0008 and 0015 as design D12 says; verify the site's decisions pages build and `site:prose` reports nothing.
- [x] 1.3 In `docs/architecture/core.md` and `site/src/content/docs/architecture/rules.md`: drop core.11's server-profile exception; add the rule that a result counts only through a signer the policy names, never a record's executor field (78); add the rule that an instance claims nothing while it has checkouts to release (81); verify `site:prose` reports nothing.

## 2. The key a signer computes (design D9)

- [x] 2.1 Establish how a signer checks a reported key against the combination's tree without running a task, and write it in design D9; verify `a_key_is_checked_against_the_tree_without_running` in `tests/profiles/moon.rs` (check-tools): every key of a run is the sha256 of its manifest in moon's blob store, whose inputs and dependency keys the tree bears out.

## 3. The policy and the host's config

- [x] 3.1 Parse `[systems]`, `[signers.<name>]` (one identity, kept as opaque settings; `confined`; `systems`) and `[pools.<name>]` (opaque to the core but for `signer` and the systems of its `runners`) in `crates/sluice-core/src/policy.rs`. Refuse each case the run-signing, check-systems and actions-pool specs refuse; verify unit tests per refusal and for the C.7 example parsing whole.
- [x] 3.2 Give `Rules` `systems`, `systems_for` and `confined`; on a branch whose state is on the remote (a `remote`, and `[signers]` in the policy), read `trust` as signer names and refuse kinds and unknown names; read `cache` as absent with a warning `sluice status` shows; count a replay only through a trusted ran pass recorded for its key, and run a combination again with no replay when a replay is unproven (108); refuse `lease` or `alive` in a branch's table; drop `cache` from sluice's own policy and `e2e/e2e.sh`; verify `sluice-core` unit tests, `a_replay_no_trusted_pass_proves_runs_again_with_no_replay` in `sluice-conformance/tests/verdicts.rs`, and `status_warns_of_cache_in_trust` in `tests/cli.rs`.
- [x] 3.3 Read `[signing] signer` and `key` (refused unless `0600`), `[peers] lease` and `alive` (defaults 5m, 2m; `alive` shorter than `lease`) and `[runs] systems` from the host's config in `crates/sluice/src/instance.rs`, with the operating system and architecture the host is; verify unit tests.
- [x] 3.4 Document `[systems]`, `[signers]`, `[pools]`, `systems`, `systems_for`, `confined` and signer `trust`, and the host's `[signing]`, `[peers]` and `[runs] systems`, in `site/src/content/docs/reference/configuration.md`, with the C.7 example; verify `site:prose` reports nothing.

## 4. Systems on one host

- [x] 4.1 Add `system` to `ResultLine` and to the check seam's per-task report; read a line with none as the host's system; match a result by task, hash and system in `verdict::read`; verify `sluice-core` unit tests: one key on two systems, a check with no systems named, an old line.
- [x] 4.2 Open one run per system a combination needs, the any-system checks in the first, in `step.rs`; read the verdict over every required check on every system; a run no executor can take stays open until the branch's `timeout`; verify conformance tests in `crates/sluice-conformance/tests/verdicts.rs` for each check-systems scenario over the fakes.
- [x] 4.3 Name each failure's system, `check (system)`, in `sluice submit`'s report, `sluice status` and the JSON of both and of `sluice overview`; add `system` to the board's `OverviewFailed` and its wake and log rows, and to herdr's log row in `lib/words.jq`; verify `a_failure_names_its_system` in `tests/cli.rs`, `moon run plugin-board:test plugin-board:typecheck plugin-herdr:test-sh plugin-herdr:test-dash`.
- [x] 4.4 Add "Systems multiply runs" to `site/src/content/docs/guides/parallel-and-batch.md`: one run per system per combination, how `systems_for` keeps the multiplication to the checks that need it, and how the window and the host's bound count them; verify `site:prose` reports nothing.

## 5. Signed results

- [x] 5.1 Add the in-toto statement and DSSE envelope types and the pre-authentication encoding in `crates/sluice/src/signing/` (design D5); verify unit tests against the DSSE specification's published test vector.
- [x] 5.2 Sign and verify with SSH keys (the `ssh-key` crate, `SSHSIG`, namespace `sluice-run`) against allowed-signers lines and files read from the branch's revision; verify unit tests with keys generated in the test, never committed: a good signature, a key not listed, a tampered statement, a wrong namespace.
- [x] 5.4 Check a reported key against the combination's tree as design D9 says, `moon::check_key`, and give the check seam `proven`: the tasks of a finished run whose keys the tree bears out (a command runner's always); verify unit tests over manifests built as moon builds them (a key of another combination, an altered manifest, a key named for another task, a dependency the tree does not bear out, a dependency marked `ignored`), and `proven_drops_a_key_the_tree_does_not_bear_out` against real moon.

## 6. The remote refs records profile

- [x] 6.1 Implement `State` over `refs/sluice/` as design D2 lays out, in `crates/sluice/src/profiles/remote_refs.rs`: the state as a root commit pushed with a lease, `ids`, each signer's `runs/<signer>` signed by this host's key and added to on top of what another host added, `logs/` kept for the last `KEPT_LOGS` candidates; `results` as the verified signed runs and this host's own replays; route each branch to it or to `local files` in `profiles/records.rs`; record only keys `proven` keeps and read back what is signed; verify `tests/remote_refs.rs` against a bare remote from two hosts, and the shared scenarios S1, S3, S4, S9 and the records' with records on the remote (`peers_*` in `tests/profiles/conformance.rs`), under `check-tools`.
- [x] 6.2 Route `sluice status`, `sluice overview` and every read through the router; read the state last fetched when the remote is unreachable, and say so in `sluice status` (`stale` in its JSON); verify `peers_push_only_the_branch_and_refs_sluice` (the modified remote-branch requirement) and `peers_read_the_last_fetched_state_when_the_remote_is_unreachable` in `tests/profiles/conformance.rs`.
- [x] 6.3 Move a branch's state to the remote under its lock the first time this host routes it there: its own queue as the starting state when the remote has none, else its queued submissions left behind; submit those again in the round under new ids naming the old (`was`); resolve an old id to the new in `sluice status <id>`, as design D10 says; verify `the_first_hosts_queue_moves_and_the_others_is_left_behind` in `tests/remote_refs.rs` (check-tools) and `a_submission_left_behind_is_submitted_again_naming_its_old_id` in `sluice-conformance/tests/records.rs`.
- [x] 6.4 Describe `refs/sluice/` in `site/src/content/docs/reference/state.md`: each ref, who writes it, how to read it with git; verify `site:prose` reports nothing.

## 7. The lead and the lease

- [x] 7.1 Name the branch's lead in its state; a step on a branch whose records are on its remote proceeds only for the lead, taking the branch when no one holds it, or when this host has itself seen the version stand for the lease and there is work (`Step::Led` otherwise, `Took` logged); keep the version a host saw and when in its own state directory; give the CLI's `Sluice` the host's name and `[peers] lease`; verify `sluice-conformance/tests/peers.rs` over the fakes for each peer-lead scenario, S17 included.
- [x] 7.2 Push a submitted head to `refs/sluice/heads/<oid>` before its submission is written, fetch every submitted head before forming, and remove a head once its submission leaves the queue or moves to the finished records (82); a waiting `sluice submit` keeps running rounds as before, and reads `led` while another host keeps the branch; verify `a_submission_lands_by_another_host` in `tests/peers.rs` (check-tools): a session on one host submits with `--no-wait`, the other host lands it signed, the first reads it landed, and its head is gone.
- [x] 7.3 Document the lead and the lease under `[peers]` in the configuration reference and in `reference/commands.md`'s `sluice submit` and `sluice run`; verify `site:prose` reports nothing.

## 8. Claims, hosts and the standing executor

- [x] 8.1 Publish a combination open to claims to `refs/sluice/combinations/<branch>/<id>` before it is opened, fetch them on every host that may claim or lead, and remove one once it has a result or is dropped (82); name the host that claims each run in the branch's state, earliest first and only with room; let a host that does not lead claim, run and report its own runs, signing what `proven` keeps, and release its own checkout; rewrite `refs/sluice/hosts/<host>` with each round of a host that does not lead; lapse a claim by the reader's own observation of the host's beat; refuse a report under a lapsed claim; release a host's leftover checkouts before it claims (`made` on the checkout seam); name a host by `[peers] name`, its host name by default; verify conformance tests over the fakes for every run-claims scenario and S20 (`sluice-conformance/tests/peers.rs`), and `a_run_is_claimed_and_run_by_the_other_host` in `tests/peers.rs` (check-tools).
- [x] 8.2 Make `sluice run` with `[signing]` a standing executor: it serves every branch with records, a remote's included, and on a branch another host keeps it claims, runs and signs runs as 8.1 does; it refuses to start when `[runs] systems` names a system whose `os` and `arch` the host is not, and claims only runs of those systems; verify `a_host_claims_only_runs_of_its_systems` and `a_host_naming_a_system_it_is_not_is_refused` over the fakes, `a_host_the_gate_does_not_trust_claims_nothing`, and `a_run_is_claimed_and_run_by_the_other_host` in `tests/peers.rs` with `sluice run` as the other host.
- [x] 8.3 Show the hosts serving a remote in `sluice overview`, by their last sign of life: signer, system, runs held, room and a lapse, in its text and its JSON `hosts`; add `OverviewHost` and a hosts line to the board; verify `the_overview_shows_hosts_their_room_and_a_lapse` over the fakes and the board's `hosts` test, with `moon run plugin-board:test plugin-board:typecheck plugin-board:validate`.
- [x] 8.4 Write `site/src/content/docs/guides/executors-and-systems.md`: one host as today, a warm executor on a box, signers and what each may claim, a session host's signer for bolts, choosing systems per check, the overview of hosts. Remove `site/src/content/docs/many-hosts/` and its sidebar entry in `site/astro.config.*`, and repoint every link to it. Verify `site:prose` reports no error; and the site build passes its link check.

## 9. Landing from signed results

- [x] 9.1 Before a landing on a branch whose records are on its remote, read the combination's checks again from the signed results, as design D8 says: every run has results a trusted signer signed for this combination, each passing, a replay only through a trusted pass signed for its key; a verdict the state alone holds lands nothing, is logged `unsigned`, and is tested again; sign a replay with its run, as a replay; verify `a_verdict_the_state_alone_holds_lands_nothing` over the fakes and `signed_runs_are_read_verified_on_another_host` in `tests/remote_refs.rs`, through `moon run ':#check-fast' ':#check-tools'`.
- [x] 9.2 On a branch whose state is on the remote, prove a replay only through a pass a signer the gate trusts signed for its key and system, and drop a reopened run's claim so it runs again; verify `verdict.rs` unit tests and `s42_a_replay_no_trusted_signer_proved_runs_again` over the fakes.
- [x] 9.3 Update the using-sluice skill (`plugins/sluice/skills/using-sluice/SKILL.md`): submitting to a peer branch and closing the laptop, reading a result's system and signer, a host with no signing key; verify `moon run plugin-sluice:validate`.

## 10. Pools

- [x] 10.1 Add the check seam's launched side (`launch`, `job_alive`, `wake`) to `crates/sluice-seams` and the fakes; the lead starts on a pool, the policy's in order, each open run no live instance able to take it has room for by its last sign of life (107), its claim the job's; a pool's claim lapses when its pool says its job no longer runs (88); a submission to a branch whose records are on its remote wakes the first pool (111); verify `s41_a_box_with_room_comes_before_the_pool` and `s19_a_pools_jobs_land_the_work_with_no_box` over the fakes.
- [x] 10.2 Implement the pool in the GitHub profile, `github/actions.rs`, handed to the check profile by the binding: a run is a `workflow_dispatch` of the pool's workflow on its signer's branch with the run's runner and claim; a job is alive while a queued or running run of the workflow is titled `sluice <claim>`; a wake dispatches a round; verify `the_pools_calls` in `tests/profiles/forge.rs` against the fake API.

## 11. Integration

- [x] 11.1 Keep S17 to S20, S41 and S42 in `crates/sluice-conformance/tests/peers.rs`, each test named for its scenario; run `:#check-fast`, `:#check-tools`, `:#check-e2e`, `openspec validate peers --strict` and the site build; verify every one passes and a one-host repository with no `[signers]` behaves as before (the existing suites unchanged).

## Workflow follow-up

- Land with `sluice submit main` from this worktree.
- Adopting peers on sluice itself is the operator's: a `[signing]` key on each standing host, and one landing that adds `[signers]` and `[systems]` and rewrites `trust` on `main` and `bolt/*`.
- GitHub Actions as a pool follows in the `actions-pool` change.
- Archive with `/opsx:archive peers` once landed.
