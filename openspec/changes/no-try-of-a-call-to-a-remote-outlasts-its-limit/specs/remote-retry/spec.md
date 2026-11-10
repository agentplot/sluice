## ADDED Requirements

### Requirement: No try of a call to a remote outlasts its limit

Each try of a call sluice makes to a remote, by git or to the forge's
API, SHALL end within thirty seconds, whatever called it. A git try
SHALL run in a process group of its own, ended at the limit with all it
started, the remote's helper and any hook among them, so nothing the
try held, the repository's fetch lock among them, outlives it (134;
rules core.28, core.29; record 0109).

#### Scenario: A fetch the remote never answers

- **WHEN** a read of a branch meets a remote that takes every
  connection and never answers
- **THEN** each try is stopped at its limit with the remote's helper,
  the read is tried again, and it fails as a refused read in sluice's
  words, `git fetch with origin was still going after 30s, the most a
  try may take, and was stopped`, with the repository's fetch lock free
  (`a_fetch_the_remote_never_answers_is_stopped_and_lets_its_lock_go`
  in `crates/sluice/tests/pushed_refs.rs`)

### Requirement: A try stopped at its limit is a call whose answer was not read

A try stopped at its limit SHALL be read as one whose answer was not
read. A read so stopped SHALL be tried again. A push so stopped SHALL
NOT be made again: the ref it set SHALL be read, and what the remote
holds decides (122, 134; record 0109).

#### Scenario: A push the remote never answers

- **WHEN** a landing's push meets a remote that takes the connection
  and never answers
- **THEN** one push was made, the ref is read, and the move fails
  saying the push went unanswered and that the read failed
  (`a_push_the_remote_never_answers_is_stopped_and_not_pushed_again`
  in `crates/sluice/tests/pushed_refs.rs`)

### Requirement: A fetch waits for the fetch lock no longer than a try may take

A fetch SHALL wait for the repository's fetch lock no longer than a try
may take. A lock held past that is a try the remote never judged, tried
again after a pause; a call that finds it held every time SHALL fail
naming the lock, as no refusal of the remote's. A sluice under
sluice's own fetch waits for it not at all (rules core.28, core.29;
records 0109, 0110).

#### Scenario: A lock held past a try's limit

- **WHEN** another process holds the repository's fetch lock past the
  limit of every try of a read
- **THEN** the read fails within its call's time, saying another fetch
  of the repository held the lock, as no refused read, and once the
  lock is let go the branch reads
  (`a_fetch_waits_for_the_lock_no_longer_than_a_try` in
  `crates/sluice/tests/pushed_refs.rs`)

## MODIFIED Requirements

### Requirement: The policy's time is set by what called it
A waiting submit and `sluice wait` SHALL give each call five minutes of
tries, ending sooner at the wait's `--timeout` or at Ctrl-C; every other
round, command and submit SHALL give each call thirty seconds. The time
SHALL run from the call's first try, or from the end of a spent
allowance's pause, and no try SHALL start once it is spent, however long
the tries took. Past that time the call fails as its last try did (134;
record 0109).

#### Scenario: Five minutes of 5xx in a wait
- **WHEN** `sluice wait 77` reads and GitHub answers every try with `503` for five minutes
- **THEN** the wait exits `3`, saying on stderr `sluice: could not read #77 for` how long, then that whether it landed is not known, then GitHub's words, and on stdout `could not read #77: sluice wait 77`

#### Scenario: Ctrl-C between tries
- **WHEN** a wait's read is between tries and the operator presses Ctrl-C
- **THEN** the wait stops at once, as a wait stopped by Ctrl-C does, and the submission stays queued

#### Scenario: A round outside a wait
- **WHEN** `sluice run` fetches a branch whose remote answers every try with `503`
- **THEN** the fetch fails within thirty seconds of pauses, and the refusal is logged on the branch once (133)

#### Scenario: Slow tries
- **WHEN** a command's call fails on tries that each take twenty seconds
- **THEN** a second try is made after the first, and no third, since
  the call's thirty seconds are spent
  (`no_try_starts_once_the_calls_time_is_spent` in
  `crates/sluice/src/retry.rs`)
