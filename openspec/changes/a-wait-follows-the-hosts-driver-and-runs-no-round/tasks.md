# Tasks

Run each test through sluice while you work, for example `sluice exec
main -- cargo test -p sluice --test cli <name>`. Once your work is
committed, run the gate with `sluice exec main`.

## 1. The driver's lock and the reader's step

- [ ] 1.1 In `crates/sluice/src/reader.rs`, add the driver's file,
  `readers/<repository>.driver`, beside the reader's files (design: "The
  driver holds a lock of its own"):
  - `driver_live(dir, repository)` probes with a shared lock and drops
    it. A probe that fails reads as no driver.
  - A `Driver` handle, taken by trying the exclusive lock again over
    about a second, holds the lock until it is dropped.
  - Update the module's comment and its file map to name the file.

  Verify: unit tests in `reader.rs` pass. They check that a held driver
  reads as live; that a dropped one reads as not live; that a probe does
  not block a driver taking the lock; and that a second `Driver` taken
  while one is held reads as another's.
- [ ] 1.2 Replace `Reader::lead_once_free` with `Reader::step(others_drive:
  bool)`:
  - A reader lets go and follows when `others_drive` is true.
  - A follower takes a free lock only when `others_drive` is false.
  - `Reader::take` takes no free lock while a driver lives.

  Verify: unit tests in `reader.rs` pass. They check that a reader lets
  go to a live driver and leaves its last document in place; that a
  follower leaves a free lock while a driver lives and takes it once the
  driver is gone; and that a driver's own step (`false`) takes a free
  lock.

## 2. `sluice run` drives and reads

- [ ] 2.1 In `Command::Run` (`crates/sluice/src/lib.rs`), keep or take
  the `Driver` at the start of each pass:
  - Without it, say once on stderr that another `sluice run` drives this
    repository on this host. Run nothing that pass. Count the pass toward
    `--rounds`.
  - With it, take part as a reader with `others_drive = false`, and run
    the pass as today.

  Verify: tests in `crates/sluice/tests/cli.rs` pass for these
  `host-driver` scenarios:
  - "Two runs at once": the test holds the driver's lock.
  - "One pass from cron beside a driver": `--rounds 1` exits 0, the
    submission is still `queued`, and nothing is started.
  - "The first run stops": the lock is released and the next pass runs
    the round.
- [ ] 2.2 After each pass's rounds, where `sluice run` holds the reader's
  place, write the reader's document from the function the watch's
  reader uses. Make `overview::read` reachable from `lib.rs`. A read that
  fails writes the failure document and the pass goes on.

  Verify: cli tests pass for "A driver with nothing queued", where the
  reader's document holds the whole overview after `sluice run --rounds
  1`, and for a run whose overview read fails, where the reader's
  document holds the error document and the run's exit is unchanged.

## 3. Watches give way to the driver

- [ ] 3.1 In `overview::run` (`crates/sluice/src/overview.rs`), make the
  per-tick step `step(driver_live(..))`, with the error ignored as
  today.

  Verify: cli tests pass for these scenarios:
  - "A watch was the reader": with the driver's lock taken by the test,
    the watch's reader lock is free within one tick, and the watch
    prints the document then written by the test's reader.
  - `overview-reader`'s "A watch with no reader beside a driver": the
    watch reads no records while the test holds the driver's lock with
    the reader's place free.
  - The existing reader tests in `cli.rs` and `overview_watch.rs`, which
    must pass unchanged.

## 4. Waits follow the driver and run no round

- [ ] 4.1 In `wait_on` (`crates/sluice/src/lib.rs`), probe the driver
  once per tick, after the pause:
  - While it lives, skip the intake and the round, and step the reader
    with `others_drive = true`.
  - Otherwise, run as today.

  Update the doc comments of `wait_on`, `Reading` and `Command::Wait`'s
  help, which say a wait always runs rounds.

  Verify: cli tests pass for these scenarios, the driver's lock held by
  the test:
  - "A wait beside a driver that has not reached it": exit 5,
    `queued`, no run recorded.
  - `overview-reader`'s "A follower wait beside a driver".
  - "A wait was the reader".
  - "The driver stops while a wait follows it": the lock is released and
    the wait's own round lands the submission.
  - A waiting `sluice submit` and a group submit beside a held driver,
    which run no round.
  - "A wait follows a watch": no driver, so rounds run as before.
- [ ] 4.2 In `submit_pull` and `submit_pull_group`, probe once per pass:
  - Keep the first pass's intake for a submit and `--status`.
  - Skip every later intake, and every round, while a driver lives.
  - Under `PullMode::Wait`, take nothing in at all while a driver lives.

  Verify: tests in `crates/sluice/tests/profiles/pull_requests.rs` pass
  for "A wait on a pull request beside a driver", with the lock held,
  `sluice wait main --timeout 2s --json` exits 5 with the pull request
  still not taken in; for "A pull request's submit beside a driver",
  where `sluice submit main --no-wait --json` beside a held driver
  answers with the submission intake took; and for a pull request's
  waiting submit beside a held driver, which takes the request in at its
  first pass and runs no round after. The existing pull-request tests
  must pass unchanged.
- [ ] 4.3 With a real `sluice run` started by the test in a fakes
  repository, check that `sluice wait <n>` on a queued submission
  reports `passed · landed as <rev>` and exits 0 once the run's round
  lands it.

  Verify: the cli test for `host-driver`'s "A wait beside a driver that
  lands it" passes.

## 5. Documentation

- [ ] 5.1 Update `site/src/content/docs/reference/commands.md`:
  - The `sluice wait` row: it follows a running `sluice run` and runs
    no rounds of its own there.
  - The `sluice run` row: one per repository on a host; a second one
    waits.
  - "Who runs the rounds": where `sluice run` is running on a host, it
    runs every round there and every wait and watch follows it; where
    nothing runs, waits run the rounds as before. Nothing has to be
    left running.
  - Each other passage that says a waiting submit or wait runs rounds,
    at the waiting act, at `sluice submit` and at `sluice wait`.

  Verify: `moon run site:prose` passes with no Vale warning, and no
  passage of the page says a wait always runs rounds.
- [ ] 5.2 Update `site/src/content/docs/reference/state.md`:
  - Add `readers/<repository>.driver` to the host's state map.
  - In the paragraph on the one reader, say that a running `sluice
    run` is the reader and every wait follows it and runs no rounds of
    its own, and that a watch or wait that held the place lets it go
    within its next look.

  Verify: `moon run site:prose` passes with no Vale warning.

## 6. Gate

- [ ] 6.1 Commit, then run the gate's checks with `sluice exec main`.

  Verify: every task passes. These include `cli:test`, `site:prose` and
  `root:format`.
