# Tasks

## 1. The host's registry of running checks

- [x] 1.1 Add `crates/sluice/src/host_count.rs` (design D1): `running/` under `instance::state_dir()`, its `.lock`, one entry and one `.hold` per admitted run named by `flat_name` of the run directory, an entry live while its hold is locked or its run directory is supervised, dead entries removed on admission; `admit(kind, limit)` returning a held admission or none, `started(run_dir)` letting the hold go, and `running()` counting live entries by kind. Verify with unit tests in the module, each on a scratch state directory: an admission under the limit is granted and one at it is refused; an entry whose hold and run directory are both free counts nothing and is removed by the next admission; two threads admitting against a limit of one leave exactly one admitted; the decision reads only the entries and the limit (`cargo test -p sluice host_count`, part of `cli:test`). The directory is given to it, as `Places` is given its blocks of ports, so tests never touch the host's own state.
- [x] 1.2 Document the registry in the module's docs and in `instance.rs`'s, saying the bound is the host's, shared by every repository on it (71), and that each run's share is the cores divided by it. Verify `cargo doc -p sluice --no-deps` builds with no warning, and the `the_bound_and_each_runs_share` test gains the S60 case: 18 cores, `max = 3`, `jobs() == 6` (`cli:test`).

## 2. The check seam admits every run

- [x] 2.1 In `crates/sluice-seams/src/lib.rs`, add `AtBound` (an error type the core downcasts to), replace `Tests::sessions()` with `Tests::running() -> Result<Running>` (`queue`, `sessions`, the host's), and state on `Tests::start` that it admits the run before running anything and fails with `AtBound` at the host's bound (design D2). Update every implementation: `crates/sluice/src/profiles/checks.rs`, `crates/sluice-fakes/src/lib.rs`, and the conformance double in `crates/sluice/tests/profiles/conformance.rs`. Verify `cargo check --workspace --all-targets` and the fakes' existing sessions tests, renamed to `running`, pass (`fakes:test`).
- [x] 2.2 In `checks.rs`'s `start`, admit through `host_count` before the toolchain, `ready` and the runner's setup: a `session/` or `preview/` place against the bound less `reserved`, any other against the bound; refuse with `AtBound`; let the hold go once `Runs::start` returns, or when the start fails. Count `running()` from `host_count`. Verify with a test in `crates/sluice/tests/checks.rs` that, with `max = 1` and one run going, a second `start` from a second repository's checkout fails with `AtBound` having run nothing (its run directory has no `request.json`), and starts once the first has ended, each on a registry of its own test's (`cli:test`).
- [x] 2.3 Give the fakes a host count other repositories add to (`World::other_runs(n)`) and make the fake `start` refuse with `AtBound` when its runs going plus that count reach its bound. Verify with a unit test in `crates/sluice-fakes` (`fakes:test`).

## 3. The core decides on the host's count

- [x] 3.1 In `crates/sluice-core/src/step.rs`, make `capacity` the bound less `running.queue + running.sessions` less the runs this instance has marked `Starting` and not yet started on the branches it serves, and `room`'s `queue` and `sessions` the host's (design D3). Verify with a conformance test in `crates/sluice-conformance/tests/queue.rs`: bound 2, the fake host already running one other repository's run, three submissions on `main`, and one round starts exactly one run (`conformance:test`).
- [x] 3.2 In `perform_starts` and the claimed-run start in `execute`, put a run back to `Open` without a place when the host is at its bound before its place is made, and release its place and put it back (clearing its claim on a remote branch) when `start` refuses with `AtBound`; start no further run that step. Verify with conformance tests: a fake `start` refusing once leaves the run open, the place released, no run recorded and no blame, and the next round starts it (`queue.rs`); on a remote branch a refused claim is open to another host (`peers.rs`) (`conformance:test`).
- [x] 3.3 Make the sign of life's `held` the host's `running.queue + running.sessions`. Verify with a conformance test in `crates/sluice-conformance/tests/peers.rs` for the *Another repository's runs leave less room* scenario: bound 4, one run of `main`, one other repository's run, and the beat says held 2 (`conformance:test`).

## 4. Agents' runs and previews wait on the whole host

- [x] 4.1 In `crates/sluice/src/exec.rs` and `crates/sluice/src/preview.rs`, treat a start refused with `AtBound` as the host being at its bound: print the waiting line once and go round the wait loop (design D4). Verify with a test in `crates/sluice/tests/exec.rs`: bound 1, a second repository's `sluice exec` running a command that holds until told, and this repository's `sluice exec` prints `waiting: the host is at its bound`, then runs once the first ends (`cli:tools`).
- [x] 4.2 Verify `sluice overview --json`'s `room` across repositories with a test in `crates/sluice/tests/exec.rs` for the *Another repository's submission* scenario: another repository's combination running, and this repository's `room.queue` is 1 while none of its branches shows it under test (`cli:tools`).

## 5. S60 on real repositories

- [x] 5.1 Add `three_repositories_share_the_hosts_bound` to the `bound` module of `crates/sluice/tests/instance.rs`: three repositories sharing one `XDG_STATE_HOME` and a host config of `max = 3`, four submissions across them (two in one), a `sluice run` going in each at once, the check script recording runs alive and `SLUICE_JOBS` as the existing test's does. Assert at most three alive at once, three reached, all four ran and landed, and every run's `SLUICE_JOBS` is the test host's cores divided by three, never below one (`cli:tools`).
- [x] 5.2 Assert the moon runner hands the share on: in `crates/sluice/tests/profiles/moon.rs`, a run's `request.json` `argv` holds `--concurrency` followed by the `SLUICE_JOBS` its environment carries (`cli:tools`).

## 6. Docs

- [x] 6.1 Write the ADR (design D6) as the next free number in `docs/adr/`, add it to `adrs.toml`, and mark ADR 0012 superseded in part by it. Verify `root:format` passes over `adrs.toml`; no gate task reads the ADR's Markdown (core.21), so it is checked by review against D1 and D2.
- [x] 6.2 Say on the site that `[runs] max` caps every repository on the host together, and that with no `max` the quarter of the cores is shared by them all: `site/src/content/docs/guides/parallel-and-batch.md` (*The host's bound*) and `site/src/content/docs/reference/configuration.md` (`[runs]`, `SLUICE_JOBS`). Verify `site:prose` passes with no warning.
- [ ] 6.3 Say the same in the plugin's skills: `plugins/sluice/skills/setting-up-sluice/SKILL.md` and `plugins/sluice/skills/using-sluice/SKILL.md` and `references/exec.md`. Verify the plugin docs test passes (`crates/sluice/tests/plugin_docs.rs`, `cli:test`).

## 7. The gate

- [ ] 7.1 Run the gate's checks on the committed work with `sluice exec main` and verify every check-fast and check-tools task passes.
