# Tasks

This change builds clauses 72 to 82, 85 to 88 and 107 to 113, the
amended 27, 28, 29 and 69, Part C and scenarios S17 to S20 and S40 to
S42 of the statement in agentplot/blueprints,
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
- [x] 1.2 Write ADRs 0031 (remote refs records, the lead and the lease), 0032 (signed results: DSSE, in-toto, SSH and keyless), 0033 (a landing reads its verdict from signed results) and 0034 (the pool is the check seam's launched executor, GitHub its only profile, core.3); amend 0004, 0008 and 0015 as design D12 says; verify the site's decisions pages build and `site:prose` reports nothing.
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
- [ ] 5.3 Verify a sigstore bundle from `actions/attest` with the `sigstore` crate, against the public-good and GitHub's trusted roots fetched by TUF and cached on the host, checking the certificate's workflow, ref, repository and issuer against `[signers.<name>] github`; verify unit tests over recorded bundles in `crates/sluice/tests/profiles/forge/`, under `*.json` fixtures excluded from formatting with their reason in `treefmt.nix`; a wrong ref, a wrong workflow, a pull request's ref; and a host with no cached root reading the result as unverified.
- [x] 5.4 Check a reported key against the combination's tree as design D9 says, `moon::check_key`, and give the check seam `proven`: the tasks of a finished run whose keys the tree bears out (a command runner's always); verify unit tests over manifests built as moon builds them (a key of another combination, an altered manifest, a key named for another task, a dependency the tree does not bear out, a dependency marked `ignored`), and `proven_drops_a_key_the_tree_does_not_bear_out` against real moon.

## 6. The remote refs records profile

- [ ] 6.1 Implement `State` over `refs/sluice/` as design D2 lays out, in `crates/sluice/src/profiles/remote_refs.rs`: the state as a root commit pushed with a lease, `runs/<signer>` added to and re-pushed on a refused add, `heads/`, `combinations/`, `logs/` and their removal; `results` as the verified union of the trusted signers' runs; verify the records conformance suite (`crates/sluice-conformance/tests/records.rs`) run against it with a bare repository as the remote, under `check-tools`.
- [ ] 6.2 Choose the profile per branch by design D1 and route `sluice status`, `sluice overview` and every read through it; show the state last fetched when the remote is unreachable; verify `tests/remote_branch.rs` cases for each remote-records scenario, and the modified remote-branch requirement's two scenarios.
- [ ] 6.3 Move a branch's state to the remote on its first round under signers, and re-submit the other hosts' queued submissions under new ids, as design D10 says; verify a `check-tools` test with two hosts' state directories over one bare remote.
- [ ] 6.4 Describe `refs/sluice/` in `site/src/content/docs/reference/state.md`: each ref, who writes it, how to read it with git; verify `site:prose` reports nothing.

## 7. The lead and the lease

- [ ] 7.1 Write `lead.json` and refuse a step whose state names another lead; keep the version a host saw and when, in its own state directory; take the branch after the lease by the host's own clock; verify conformance tests over the fakes with a fake clock for every peer-lead scenario, S17 included.
- [ ] 7.2 Make a waiting `sluice submit` and `wt submit` run rounds as a lead on a peer branch, and a `--no-wait` submit write its entry, push its head and wake; verify a `check-tools` test where the submitting host stops and a second host lands the submission.
- [ ] 7.3 Document the lead and the lease under `[peers]` in the configuration reference and in `reference/commands.md`'s `sluice submit` and `sluice run`; verify `site:prose` reports nothing.

## 8. Claims, hosts and the standing executor

- [ ] 8.1 Write claims to the branch's state, earliest first and only with room; rewrite `refs/sluice/hosts/<host>` at `alive`; lapse a claim by the reader's own observation; refuse a report under a lapsed claim; release a host's leftover checkouts before it claims; verify conformance tests over the fakes for every run-claims scenario and S20.
- [ ] 8.2 Make `sluice run` with `[signing]` a standing executor: refuse to start when `[runs] systems` names a system whose `os` and `arch` the host is not; claim runs of its systems on branches that trust its signer for the claims the gate needs, sign and record on its signer's ref only the results `proven` keeps; verify a `check-tools` test with a box host and a session host over one bare remote, and each standing-executor scenario.
- [ ] 8.3 Show hosts, their signers, systems, runs held, room and lapses in `sluice overview`; show each result's signer; add both to the board's types and pane; verify `cli.rs` cases and `moon run plugin-board:validate plugin-board:test`.
- [ ] 8.4 Write `site/src/content/docs/guides/executors-and-systems.md`: one host as today, a warm executor on a box, signers and what each may claim, a session host's signer for bolts, choosing systems per check, the overview of hosts. Remove `site/src/content/docs/many-hosts/` and its sidebar entry in `site/astro.config.*`, and repoint every link to it. Verify `site:prose` reports nothing and the Pages build's link check passes.

## 9. Landing from signed results

- [ ] 9.1 Read a combination's checks again from the trusted signers' runs before a landing intent, and land only on a pass, as design D8 says; verify a conformance test where the state holds a passing verdict with no signed results behind it and nothing lands, and S18.
- [ ] 9.2 On a branch whose state is on the remote, prove a replay only through a pass a signer the gate trusts signed for its key and system; verify `verdict.rs` unit tests and a conformance test for S42 over signed results.
- [ ] 9.3 Update the using-sluice skill (`plugins/sluice/skills/using-sluice/SKILL.md`): submitting to a peer branch and closing the laptop, reading a result's system and signer, a host with no signing key; verify `moon run plugin-sluice:validate`.

## 10. The GitHub Actions pool

- [ ] 10.1 Add the check seam's launched side (list pools, launch, alive, wake) to `crates/sluice-seams`, the fakes and the lead's dispatch rule (107); verify conformance tests over the fakes for S41, the pool order, a lost job's lapsed claim, and a submission landing with every host off (S19).
- [ ] 10.2 Implement it in the GitHub profile with `workflow_dispatch` and the Actions jobs API; verify `tests/profiles/forge.rs` cases over the recorded HTTP fakes: dispatch inputs, a job ended with no report, a cancelled job.
- [ ] 10.3 Write the templates of `sluice.yml` (round, run and sign jobs, `schedule`, `workflow_run`) and `sluice-pr.yml` in `crates/sluice/src/profiles/github/workflows/`, with the `sluice` environment on the round and sign jobs only and no `id-token` on run jobs, the run job uploading its report and moon's manifests and the sign job signing only what `proven` keeps, as designs D7 and D9 say; add that directory to actionlint's includes in `treefmt.nix`; verify `root:format` passes on them.
- [ ] 10.4 Add `sluice init actions`: write both workflows, add `[signers.forge]` and `[pools.forge]` when absent, leave a differing file unless `--force`, print what to set on GitHub; add `sluice job round` and `sluice job sign` for the workflows to call; verify `cli.rs` cases over a scratch repository for each actions-pool `init` scenario.
- [ ] 10.5 Extend `sluice forge check` with the workflow, the dispatch permission, the environment's branch rule and the forge signer; verify `tests/profiles/forge.rs` cases for each forge-identity scenario.
- [ ] 10.6 Try it by hand on a scratch repository on GitHub, as the gate cannot reach GitHub: submit, switch the host off, see a round job land the submission, a run on `macos-15`, a cancelled job run again, and a pull request that edits the workflow sign as nothing; record the run links in the PR description.
- [ ] 10.7 Write `site/src/content/docs/guides/github-actions.md`: the app and its environment, `sluice init actions`, the signers, submit and close the laptop, reading the checks on a pull request, the cost of the pool and how standing executors come first; update `reference/commands.md` for `sluice init actions`, `sluice job` and `sluice forge check`; update the setting-up-sluice skill's forge and executor choices; update README's Get started and Quick start for the pool; verify `site:prose` reports nothing and `moon run plugin-sluice:validate`. README is outside every gate (core.21 Details): read it through once by hand.

## 11. Pull requests

- [ ] 11.1 Record a pull request's head runs from `sluice-pr.yml`, signed by `sluice.yml`'s sign job, and read them by key in its combination; refuse `pull_requests.checks` naming sluice's own; verify conformance tests for S40 and each pull-request-intake scenario.
- [ ] 11.2 Post `sluice / <check> (<system>)` per required check per system beside `sluice`, with outcome and signer; verify `tests/profiles/pull_requests.rs` cases for each pull-request-status scenario.
- [ ] 11.3 Update `site/src/content/docs/guides/pull-requests.md` for head runs and the checks per system; verify `site:prose` reports nothing.

## 12. Integration

- [ ] 12.1 Add S17 to S20 and S40 to S42 to `crates/sluice-conformance/tests/scenarios.rs`; run `:#check-fast`, `:#check-tools`, `:#check-e2e`, `openspec validate peers --strict` and the site build; verify every one passes and a one-host repository with no `[signers]` behaves as before (the existing suites unchanged).

## Workflow follow-up

- Land with `sluice submit main` from this worktree.
- Adopting peers on sluice itself is the operator's: a `[signing]` key on each standing host, `sluice init actions`, and one landing that adds `[signers]`, `[systems]` and `[pools]` and rewrites `trust` on `main` and `bolt/*`.
- Archive with `/opsx:archive peers` once landed.
