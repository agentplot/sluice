# Tasks

Read first: this change's proposal, specs and design;
`agentplot/blueprints:design/sluice/proposals/session-runs-1.md`;
`docs/architecture/core.md`; ADRs 0012, 0013, 0016. Work inside `devenv
shell`. Fast tier: `moon run ':#check-fast'`; real tools:
`moon run ':#check-tools'`; e2e: `moon run ':#check-e2e'`. Commit by
pathspec after each task group, Conventional Commits, scope the crate or
`site`/`adr`/`docs`.

## 1. Host config and the count

- [x] 1.1 Add `[runs] reserved` (default 0) to `instance::Instance`, refusing `reserved >= max` with the setting named; verify with tests in `crates/sluice/tests/instance.rs` for default, a value, and the refusal text.
- [x] 1.2 Add `crate::sessions::Sessions` and `SessionRun`: session runs at `$XDG_STATE_HOME/sluice/sessions/<flat worktree>.json` with the flock `sessions/.lock` held across count-and-start, live while a supervisor holds the run directory, dead ones dropped on `begin`; add `Tests::sessions() -> usize` and implement it on `Checks` (`with_sessions(dir)` for tests); verify with `crates/sluice/tests/sessions.rs` (write, read, end, dead run dropped, lock held across processes, the seam's count).
- [x] 1.3 Implement `sessions` on `FakeTests` with scripting helpers to add a running or ended session run; verify the fakes crate tests pass.
- [x] 1.4 Make `Sluice::capacity` subtract `tests.sessions()` under both `Serves` variants and add `Sluice::session_capacity(reserved)`; verify with a core test over fakes: bound 2 with two sessions starts no run; reserve 1 of 4 leaves 3 for sessions.
- [x] 1.5 Document `[runs] reserved` beside `[runs] max`, and the host's state directory (`slots/`, `sessions/`) beside the host config, in `site/src/content/docs/reference/configuration.md`; verify `moon run 'site:prose'` is clean.

## 2. The session place

- [x] 2.1 Add `kind: Made | Session` and `branch` to the place record in `profiles/places.rs` (serde default `Made`), and `Places::adopt(worktree, branch, settings)` that writes the record, holds a slot, makes tmp and returns a `Place` with `SLUICE_PLACE` and `SLUICE_PORT_*`; add `adopt` to the `Checkout` trait and to `FakeCheckout`; verify with `crates/sluice/tests/profiles/places.rs`: same ports on a second adopt, two worktrees never share a port, no `git worktree` is created.
- [x] 2.2 Make `Checkout::release` on a session record end its runs, run the release command once, remove tmp, free the slot and remove the record, leaving the worktree; make it exit clean when no record exists; verify with places.rs tests: worktree still listed after release, release twice is a no-op, slot free after release.
- [x] 2.3 Teach `runs::Runs` per-run directories `.runs/<session flat>/<n>` that never trigger the release command on reuse, and a `remove_all` for release; verify with `crates/sluice/tests/runs.rs`: two runs under one session name, release command runs once at remove_all and not between.
- [x] 2.4 Extend `crates/sluice/tests/stack.rs`: a run that starts a container leaves it running after the run; `release` removes it and frees the ports; verify under `moon run ':#check-tools'` (docker available).

## 3. `sluice exec`

- [x] 3.1 Add `Command::Exec{branch, args after --, --worktree ".", --json}` to the clap CLI with dispatch in `lib.rs`; `branch` is required, as `sluice submit`'s is; verify `crates/sluice/tests/cli.rs` over fakes: usage, a missing branch refused by the parser, refusal outside a worktree.
- [x] 3.2 Implement the checks path: `Driver::session` for head and dirty, policy and gate at the branch's revision (core.15), `Checkout::adopt`, slot wait (branch lock + the host's sessions lock, capacity less reserve, pause 3 s, Ctrl-C exits 130 with nothing recorded), `Tests::start` with `since = branch rev` then `Sessions::begin`, progress loop streaming `out.log` to the terminal, `Sessions::end`, exit = checks' outcome or timed out; verify over fakes in cli.rs: waits while no slot, starts when one frees, never takes the reserve, time limit reported.
- [x] 3.3 Implement the command path (`-- <command>`): the `command` runner with the given argv, `record: false`, same executor, limits and env; verify over fakes that nothing is recorded and the exit is the command's, and with a real-tool test that `SLUICE_PORT_<NAME>` and `SLUICE_CHECKOUT` reach the command.
- [x] 3.4 Move the `ResultLine` builder out of `Sluice::poll` into a shared function, add the optional `worktree` field, and have `exec` write `Ran` results when the worktree is clean, with `combination = head`; print "not recorded: worktree is not clean" otherwise; verify over fakes: clean run writes one line per task with executor and host, dirty run writes none, command run writes none, `Replayed` never written.
- [x] 3.5 Verify the replay end to end over fakes: a clean session run passes, the same head is submitted to a branch trusting open runs, the candidate's verdict reads those keys as replays (69, 93); add this as conformance scenario S26 in `sluice-conformance/src/shared.rs` with S25 (reserve) and S28 (dirty records nothing).
- [x] 3.6 `--json`: one document on stdout (tasks with key, outcome, recorded; ports; exit), human text on stderr; verify in cli.rs.
- [x] 3.7 Document `sluice exec` and `sluice release` in `reference/commands.md` (table rows, a console example, exit codes), the ports, release command, `[executor] docker` and `SLUICE_PLACE`/`SLUICE_PORT_*`/`SLUICE_JOBS` gaps in `reference/configuration.md`, and a paragraph in the walkthrough's services section that points a crew at `sluice exec`; verify `site:prose` clean and `bun run build` passes.
- [x] 3.8 Add `Checkout::save_caches(name, rev, checkout)`: a session's place copies the `[[checkout.cache]]` paths out of the worktree under their key at `rev`, never moving them, saved only from a recorded run that passed; call it from `Sluice::finish_session`; verify in places.rs that the worktree keeps its paths and a later candidate's place restores them, and run S26 on the real profiles with a declared cache in the test's policy.

## 4. `sluice release` and the worktrunk driver

- [x] 4.1 Add `Checkout::release_session(name, tests)`: the place profile builds the release request from the `[tests]` settings passed with the place's saved environment and ports, falling back to the last run's saved command only when given none; add `Command::Release{--worktree "."}` that reads the policy through the core (`policy_at` of the record's branch at its current revision, 56), calls `release_session` on the session record, says in its output when the branch could not be read and the saved command ran, and ends the worktree's session run in `Sessions`, exit 0 "nothing to release" without a record; `Command::Exec` gains `--release`, which calls it after the run; verify in cli.rs over fakes and in places.rs with real git, including that a release command changed in the policy after the last run is the one that runs.
- [x] 4.2 Make `wt-submit` run `sluice release --worktree <wt>` before `wt remove`, and not under `--no-remove`, `--no-wait` or `--check`; verify in `crates/sluice/tests/profiles/worktrunk.rs`: release observed before removal, skipped under `--no-remove`.
- [ ] 4.3 Add conformance scenario S27 (stack stays between runs, gone after release) to shared.rs over fakes, and an e2e step in `e2e/e2e.sh`: `sluice exec` twice with the same ports, `wt submit` lands and the release ran; verify `moon run ':#check-e2e'`.
- [ ] 4.4 Update `guides/worktrunk.md`: `wt submit` releases before worktrunk removes; `post-remove` keeps only what is not a run's; the plain-git path runs `sluice release` before `git worktree remove`; verify `site:prose` clean.

## 5. Decisions and rules

- [ ] 5.1 Write ADR 0017 "An agent's worktree is a place sluice equips and never removes" in `docs/adr/` (MADR bare front matter as 0016; sources: blueprint proposal session-runs-1 clauses 89 to 95, clauses 25, 31, 34, 71, 83, 84; ADRs 0012, 0013, 0016; options: nothing, a separate semaphore tool, the adopted place), naming the one-over-the-bound race; verify `site/scripts/adrs.mjs` renders it at `/sluice/decisions/0017/`.
- [ ] 5.2 Update `docs/architecture/core.md` and its mirror `site/src/content/docs/architecture/rules.md`: core.10's "Not ruled out" gains a session's run under the executor; core.16 becomes "The core releases only places it made; a session's place is released only by `sluice release` at its session's or driver's asking"; add core.18 "A session run counts toward the bound from the host's state and is recorded under its keys only when its worktree holds exactly its head"; verify every code comment that cites core.16 still reads true.
- [ ] 5.3 Run the full `:#check-fast`, `:#check-tools` and `:#check-e2e` tiers and `openspec validate session-runs --strict`; verify all pass and the change's tasks are all checked.

## Workflow follow-up

- Land with `devenv shell -- wt submit main` from the worktree branch.
- Archive the change with `/opsx:archive session-runs` after landing.
- Tell the blueprint's author which clauses the build departed from, if any, so the proposal is corrected before ratification.
