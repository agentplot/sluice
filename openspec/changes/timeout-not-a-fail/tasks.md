# Tasks

Read first: this change's proposal, design and spec; time-limit-1 in
agentplot/blueprints; `docs/architecture/core.md`. Run suites through
`sluice exec main -- <cmd>`, the gate's checks with `sluice exec main`.

## 1. Statement

- [x] 1.1 Draft `design/sluice/proposals/time-limit-1.md` in
  agentplot/blueprints, branch `time-limit`: 66 amended, S55, a row in the
  proposals README. Verify with Vale (no warnings) and show it to the
  operator for a ruling.

## 2. Reproduce and fix

- [x] 2.1 Add `a_task_the_time_limit_cut_off_has_no_outcome` in
  `crates/sluice/tests/profiles/moon.rs`. A real moon run ends at a 15 s
  limit, with a pass and a fail that ended before it and two tasks
  running: one dies of the signal, one catches it and exits 143. Verify
  it fails on main, then passes, with each cut-off task's own log, via
  `sluice exec main -- cargo test -p sluice --features tools --test moon`.
- [x] 2.2 `Moon::cut_off` and its use in `Checks::progress`: on a
  timed-out moon run, a task without a run record from before the limit
  leaves `tasks` for `missing`. Add the unit test
  `a_task_the_limit_cut_off_is_named_however_moon_read_it` for moon's
  `failed` reading, the box's case. Verify
  `cargo test -p sluice --lib cut_off`.
- [x] 2.3 `sluice-fakes`: a hung task is named missing; `ends_in_time`.
  Add the conformance test
  `a_check_the_time_limit_cut_off_is_recorded_under_no_key_and_the_same_head_lands`.
  Verify `sluice exec main -- cargo test -p sluice-conformance`.

## 3. Words

- [ ] 3.1 `crates/sluice/src/lib.rs` and `words.rs`:
  - a timed-out submission names the tests the limit ended, says nothing
    was recorded for them, and gives the way on;
  - a flaky failure's closing line says the same head lands once the
    operator sets the fail aside, or once a real fix changes what the test
    reads;
  - the set-aside hint names the host or a time limit.

  Verify `a_flaky_failure_says_so` and a timed-out test in
  `crates/sluice/tests/cli.rs`.

## 4. Docs

- [ ] 4.1 Change `plugins/sluice/skills/using-sluice/references/verdicts.md`:
  - the `timed out` bullet;
  - the bullet on a test with no result;
  - the flaky section: an agent's one move is to stop and tell the user
    what it found and the runs' times.

  Change any SKILL.md that mentions set-aside to match. Verify the
  plugin's moon tasks.
- [ ] 4.2 Change the site:
  - `reference/commands.md`: submit's timed-out and flaky output, and
    `sluice set-aside` listing what sluice tells apart itself;
  - `reference/configuration.md`: `timeout` and `flaky`;
  - `reference/state.md` if it says otherwise;
  - `guides/walkthrough.md` on time limits.

  Verify with `site:prose`.
- [ ] 4.3 Add time-limit-1 to core.7's details in
  `docs/architecture/core.md`, and add ADR 0041. Verify `root:format`.
- [ ] 4.4 Verify `openspec validate timeout-not-a-fail --strict`.

## 5. Land

- [ ] 5.1 Run `sluice exec main` and `sluice submit main`.
