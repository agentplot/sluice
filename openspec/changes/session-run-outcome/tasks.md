# Tasks

Read first: this change's proposal, specs and design; session-runs-1
clauses 89 to 97; `docs/architecture/core.md` (core.2, core.7, core.11,
core.14, core.18, core.19). Work inside `devenv shell`. Fast tier:
`moon run ':#check-fast'`; real tools: `moon run ':#check-tools'`.
Every git call in a test goes through `tests/support`'s isolated
helpers (`Repo`, `Scratch`): no prompt, no credential helper, no global
config.

## 1. The seams, the fakes and the conformance suite

- [ ] 1.1 Add `LastResult{at, head, kind: RunKind, outcome: RunOutcome, recorded}`, `RunKind{Checks, Command}` and `RunOutcome{Pass, Fail, TimedOut, Lost, Stopped}` to `sluice-seams`, serialised lowercase and snake case; add `SessionPlace.last_result: Option<LastResult>` and `Checkout::ran(name, &LastResult)`; verify `cargo check --workspace --all-targets --all-features`.
- [ ] 1.2 Implement `ran` in `sluice-fakes`: kept per session place, replaced by the next, dropped on `release_session`, an error for a name no session's; given back by `session_places`; verify a unit test in `sluice-fakes` for each of the four.
- [ ] 1.3 Add the place seam's scenario to `sluice-conformance` (`shared.rs`): `ran` after `adopt` is listed by `session_places`, a second `ran` replaces it, `release_session` removes it, `ran` on a candidate's place is an error; verify it passes against the fakes and, in the tools tier, against `profiles::places`.

## 2. The core and the place profile

- [ ] 2.1 `Sluice::finish_session` writes `last_result` through `Checkout::ran` after `record_session` and the cache save: `recorded` true exactly for `Recorded::Written(_)`, `kind` from `command`, `outcome` `timed_out` when the report says so, else `pass` by the predicate `exec` exits 0 on, else `fail`; add `Sluice::session_ended(place, head, kind, outcome)` for a lost or stopped run, writing `recorded` false; verify unit tests in `sluice-core/src/session.rs` over `sluice-fakes`: a clean pass (recorded), a clean fail (recorded), an unclean pass (not recorded, no results line), a command exiting 0 (not recorded), a timed-out run, a rerun whose results are all under their keys (`Written(0)`, recorded), and the results log written before `ran` is called.
- [ ] 2.2 Keep `last_result` on the session place's record in `profiles::places` (serde default, skipped when absent), written whole by rename as `adopt` writes it; give it back from `session_places`; refuse `ran` on a record that is not a session's; verify the conformance scenario of 1.3 in `:#check-tools`.
- [ ] 2.3 Verify the result decides nothing: a core unit test over `sluice-fakes` in which a clean worktree's recorded pass is followed by an unclean failing run, and a submission of its head replays the recorded results unchanged.

## 3. The CLI

- [ ] 3.1 In `exec.rs`, call `Sluice::session_ended` on the lost and stopped paths; leave a refused run's result as it was; verify `crates/sluice/tests/exec.rs` (tools tier, real place and supervisor): `last_result` after a clean pass, an unclean pass (`recorded` false), `-- true` (`kind` `command`, `recorded` false), a timed-out run, a second run replacing the first, and none after `sluice release`.
- [ ] 3.2 Add `last_result` to `worktree_json`, so `status --json`'s `worktrees[]` and `overview --json`'s `sessions[]` carry it, absent when `None`; add it to `write_worktrees`'s line (`last result passed on <short head>, recorded`, or `not recorded`, or `command exited 0`); verify in `crates/sluice/tests/cli.rs` over the fakes: both documents carry the same `last_result` with `last_run`, `ports` and `removed` unchanged, the text line, and `overview --json --watch` printing again when a run ends.

## 4. Decisions and docs

- [ ] 4.1 Write the next free-numbered ADR: a session run's last result is kept on its place's record, not derived from the results log (the `same_as` dedupe, commands and unclean runs), and decides nothing; add it to `docs/adr/adrs.toml`; verify `bun run build` renders it.
- [ ] 4.2 Document `last_result` on `site/src/content/docs/reference/commands.md` (the worktrees list, its console example and the overview's `sessions[]` row, with what "ready" needs: `kind` `checks`, `outcome` `pass`, `recorded`, and `head` the worktree's head) and the session place's record on `reference/state.md`; verify Vale reports no warning and `bun run build` passes.
- [ ] 4.3 Run `:#check-fast`, `:#check-tools`, `openspec validate session-run-outcome --strict` and `bun run build`; verify all pass.
