# Tasks

Every verification below is a test the gate's moon tasks run; run them
while iterating with `sluice exec main -- cargo test -p <crate> --test
<file> <name>`, and the whole gate with `sluice exec main` once
committed. Commits cite record 0074 and rule core.38.

## 1. The seam's question and the engine look

- [x] 1.1 Add `Tests::unmet(&self, limits: &Limits) -> Result<Option<String>>` to `crates/sluice-seams/src/lib.rs`, documented with 84, 114, 139 and record 0074 (design D1); give the submit-only seam in `crates/sluice-core/src/ops.rs` its `unreachable!`, and the fake check seam in `crates/sluice-fakes` a setter for the reason it answers, `None` by default; verify with `cargo check --workspace` and the fakes' existing tests passing.
- [x] 1.2 In `crates/sluice/src/profiles/checks.rs`, add the engine look (design D2): `GET /_ping` over each existing path of the sockets the profile opens, two-second connect and read timeouts, `200` an engine, the reason naming each path and what it said or the paths looked at; `unmet` answers `None` at once when `limits.docker` is false; add `Checks::with_engine_sockets` for tests. Verify with tests in `crates/sluice/tests/checks.rs` using a unix-socket listener in the test's temporary directory: one answering `HTTP/1.0 200 OK` reads as met; one answering `500`, a socket file nothing listens on, and no socket at all each read as unmet with the path in the reason; `docker = false` reads as met with no socket.

## 2. Turning a run down before the claim

- [x] 2.1 In `crates/sluice-core/src/step.rs`, read `unmet(&policy.limits)` at most once a round per branch, lazily, and make `claimable` refuse every run of the branch while it is `Some` (design D3). Verify with conformance tests in `crates/sluice-conformance/tests/peers.rs` (or a new `engine.rs`) on the fakes: a host whose fake answers a reason claims no run of a `docker = true` branch, makes no place and starts nothing; the same host claims once the fake answers `None`; a branch without `docker = true` is claimed with the fake answering a reason, and the fake is never asked.
- [x] 2.2 Add `declined` to `SystemRun` and `Event::Declined` in `crates/sluice-core/src/records.rs`, written by the round that turned a run down only where its host's reason changed, and the host's entry removed in the write that claims it (design D4). Verify with conformance tests: ten rounds with one reason write one `Declined` line and one entry; a new reason replaces the entry with a second line; a later claim by that host clears its entry; a record without `declined` reads as before.
- [x] 2.3 With two peers on one branch, one whose fake answers a reason and one whose fake answers `None`, verify by a conformance test that the second claims the run and the first never does, and that the lead's standing-hosts-before-pools pass offers the run to the second host rather than counting itself (107).
- [x] 2.4 Make the lead's time-limit pass read an open run this host declined as timed out, and the session's notice name the hosts and reasons it waited on in place of the system wording (design D7). Verify with a conformance test: a lone host without an engine, a 30-minute limit and the fake clock moved past it reads the combination not passed, and the notice says no host whose container engine answers took its tests.

## 3. The look after a failed run

- [x] 3.1 In `Step::finish_run`, for a branch asking the engine, a run not ended by its time limit and with a fail or a missing test, ask `unmet` again once; on a reason, move every fail and missing test into `not_run` with "the container engine no longer answers after the run: <why>" and leave passes as they are (design D5). Verify with conformance tests in `crates/sluice-conformance/tests/could_not_run.rs`: a pass and a fail with the fake answering a reason after the run records the pass under its key, lists the fail as not run, blames nobody and tests the combination again; with the fake answering `None` the fail is recorded and blamed; a run whose tests all passed, and one the time limit ended, never ask.
- [x] 3.2 Apply the same look in `Sluice::finish_session` for a session run of the tests, never for a `--` command (design D5, D6). Verify with a test in `crates/sluice-core/src/session.rs`'s tests or the conformance suite: the session's fail is recorded under no key when the fake answers a reason after the run; a command's fail is reported as the command's whatever the fake answers.
- [x] 3.3 End to end in `crates/sluice/tests/checks.rs` (or `exec.rs`), with the open executor and a fake engine socket the test serves: a check that removes the socket and exits 1, beside one that exits 0, leaves the first not run with the engine reason and the second's pass recorded under its key; a check that prints "Cannot connect to the Docker daemon" and exits 1 while the socket still answers is recorded as a fail.

## 4. What the person sees

- [ ] 4.1 `sluice exec <branch>` asks `unmet` before starting the branch's tests and refuses with the reason, running and recording nothing; a command after `--` runs as before (design D6). Verify with tests in `crates/sluice/tests/exec.rs`: with the engine sockets pointed at an empty directory and `docker = true`, `sluice exec main` prints the refusal naming the sockets and leaves no result; `sluice exec main -- true` passes.
- [ ] 4.2 Carry each open run's `declined` into `CandidateView` (`crates/sluice-core/src/overview.rs`) and print it from `progress::testing_line`, so `sluice status` and `sluice wait` say `waiting for a host whose container engine answers: <host>: <why>`, and add `waiting` to their JSON (design D4). Verify with tests in `crates/sluice/tests/cli.rs` (or `progress.rs`'s unit tests) reading a record with a declined run: the line and the JSON name the host and reason; a combination with no declined run prints as before.
- [ ] 4.3 Document it where the person reads it: the `docker` row of `site/src/content/docs/reference/configuration.md` says a host whose container engine does not answer at `DOCKER_HOST` or the usual sockets takes none of the branch's runs and leaves them for one whose engine does, and that a test failed after the engine went has no result; `plugins/sluice/skills/setting-up-sluice/references/profiles/sandbox.md` says the same beside `docker = true`; `plugins/sluice/skills/using-sluice/references/verdicts.md` names the waiting line and the not-run reason. Use the user words only (rule core.26), leave every `ready` passage to the unit removing it, and verify with `site:prose` (Vale, no warnings) and `root:format` passing under `sluice exec main`.

## 5. The gate

- [ ] 5.1 Commit and run `sluice exec main`; every gate task passes (format, lint, type check, tests, the conformance suite, `site:prose`).
