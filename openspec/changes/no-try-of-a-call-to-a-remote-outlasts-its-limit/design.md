## Context

See proposal.md for why. Clause 134 as written, rules core.28 and
core.29, and record 0109 say what holds. Record 0109 was decided on
deneb's `fix/main--a-hung-fetch-holds-no-lock`, built on main at
69bc9934 and never landed: 66ad662f the record and rule core.29's
details, 27ffa26e the code, 5fa9d784 its spec. This design says how it
is built on main as main stands once fix
`a-fetch-and-its-hook-share-no-lock` (record 0110; 411ff0b9 the code,
8ff85162 the record, 756363b7 its archive) has landed. Both change how a
fetch holds the repository's fetch lock, and a trial merge of the two
branches conflicts in `crates/sluice/src/profiles/pushed_refs.rs` and in
core.29's details alone.

**The family.** Flume and the Boomdrive runner make git fetches of their
own: Flume's `Git::fetch_branch` in `crates/flume/src/git.rs` and the
runner's `clone.rs`, through `boomdrive::git`'s `run`. Neither has a
limit, read at flume e2973d57 and boomdrive 8ef47ba6 on 2026-11-10.
Neither holds a lock that another process waits on, so neither can hold
every read of a repository as sluice's fetch did. The cause here is
sluice's own: an unbounded try under sluice's own lock. It is not that
the tools differ, so sluice is fixed alone. The family has no shared
owner of a git call's limit for sluice to call or cite, and record 0109
tells the orchestrator so. Within sluice, `bounded` becomes the one
owner of a process run to a limit, and every call to a remote takes its
try's limit from one constant.

What main has once record 0110 lands:

- `PushedRefs::call` in `crates/sluice/src/profiles/pushed_refs.rs`
  makes every git call to a remote under `Policy::run`. A fetch in a
  process that runs under sluice's own fetch of the repository,
  `under_own_fetch`, returns `fetched_nothing()` before any try (record
  0110). Every other fetch takes the lock in `fetching` for each try,
  with `File::lock`, which has no limit. It gets back the lock and its
  path, and drops the lock after the try. `git_as(speaker, given, args,
  held)` builds git's command, names the held lock in
  `SLUICE_FETCH_HELD` for every process git runs, and waits on
  `Command::output`, which has no limit.
- `Policy::run` in `crates/sluice/src/retry.rs` bounds the call with
  backon's `with_total_delay(self.bound)`. That bound counts the pauses
  alone, not the time the tries took. `COMMAND_FOR` is thirty seconds,
  and a wait gives five minutes and its `until`.
- `App`'s agent in `crates/sluice/src/profiles/github/app.rs` ends each
  API call at `timeout_global(Duration::from_secs(30))`.
- `CredentialCommand` in `crates/sluice/src/identity.rs` runs a host's
  credential command in a group of its own to `COMMAND_LIMIT`, thirty
  seconds. It reads the command's pipes on threads of its own and
  SIGKILLs the group at the limit (record 0108).
- `runs::end_group` asks a group to end, gives it a grace, then kills
  what is left. A check's run is ended through it.

## Goals / Non-Goals

**Goals:**

- No try of a call to a remote takes longer than thirty seconds, and
  nothing it holds or started outlives it.
- No fetch waits for the repository's fetch lock longer than a try may
  take.
- No try starts once the call's time is spent.
- Record 0110's way of a sluice under sluice's own fetch stays as it
  is.

**Non-Goals:**

- A limit on Flume's or the runner's own fetches.
- A limit set per host or per policy.
- Passing Ctrl-C on to a try in flight.

## Decisions

### D1. One limit for every try, held by the policy

`retry::TRY_FOR`, thirty seconds, is the limit of every try of a call
to a remote: each git try, each API call, and each run of a credential
command. `Policy` holds it as `try_for`, read by `Policy::try_for()`
and set by a test's `Policy::trying_for(limit)`. git's tries read it
from the policy they run under. The API agent's `timeout_global` and
`identity::COMMAND_LIMIT` take the constant. Where fetches need longer,
one value changes, in the code and not in a host's settings.

Not taken: git's `http.lowSpeedLimit` and `http.lowSpeedTime` (record
0109). They cover HTTP's transfer alone, not a TLS handshake stalled
for curl's five minutes, a remote reached by SSH, or a fetch waiting
for the lock. Also not taken: a limit on the call alone. Under it, a
wait's five minutes would let one stalled try hold the lock that long.

### D2. `bounded` runs a process to a limit, in a group of its own

`crates/sluice/src/bounded.rs` is taken from 27ffa26e:

- `output(command, limit)` spawns with piped standard output and error
  and `process_group(0)`.
- `finish(child, limit, kept)` reads each pipe on a thread of its own,
  keeping its first `kept` bytes. At the limit it ends the group
  through `runs::end_group`, giving `GRACE` of two seconds, so git,
  asked, removes the lock files it holds. It waits `DRAINED`, 200 ms,
  for a pipe a process outside the group holds open.
- `Ended { status, stopped, stdout, stderr }` says how the process
  ended.
- `span(d)` says a limit as `30s`, or as `300ms` under a second.

`CredentialCommand::get` and `erase` move onto `finish`. Its own
`wait`, `drain` and `span` go. Its group is now asked to end before it
is killed, as a check's is.

Not taken: a second copy of the group's ending in `pushed_refs.rs`,
beside the credential command's. That would give one behaviour two
owners that drift apart, as the credential command's SIGKILL at once
already had from a check's grace.

### D3. A stopped try is a call whose answer was not read

`git_as` runs its command through `bounded::output` at
`self.policy.try_for()`. It returns what git printed and whether the
limit came first. `call` reads a stopped try as `Judged::Unknown`, which
the policy tries again for a read and never for a push. It sets
`last` to git's output with sluice's words after it, through `told`:
`git <fetch|push|ls-remote> with <remote> was still going after
<limit>, the most a try may take, and was stopped`. `told` adds no
blank line where git printed nothing. A push so stopped goes on to the
settling read that `push` already makes for an answer not read (122;
record 0017). A read that is stopped every time fails as a refused
read in those words. A round logs it once per cause and host (133;
rule core.28).

### D4. The fetch lock, with record 0110

Each fetch's try runs in this order:

1. `under_own_fetch` is asked first, before any try, as record 0110
   has it. A sluice under sluice's own fetch takes no lock and runs no
   git.
2. `fetching` opens the lock and tries it every 20 ms until
   `try_for` has passed. It returns
   `Result<Result<(File, PathBuf), PathBuf>>`: the lock held with its
   path, or the path where another process held it all that time.
3. A lock held past the limit sets `waited` and fails the try as
   `Judged::Never`, which is tried again after a pause. `waited` is
   cleared when a later try takes the lock. Where the call ends with
   `waited` set, it fails with `another fetch of this repository held
   <path> past <limit>, the most a try waits for it`. That is no
   `ReadRefused`, so it is logged on no branch, as a race lost is
   (record 0086).
4. `git_as(.., held)` names the held lock's path in `SLUICE_FETCH_HELD`
   on the command, then runs it through `bounded::output`. Every
   process git runs is in git's group and has git's environment. The
   `reference-transaction` hook is one of them. It still finds the
   lock named, as record 0110 needs, and it is ended with the try at
   the limit.
5. The lock is dropped as soon as `git_as` returns, so it is never
   held over a pause.

Not taken: waiting for the lock with no limit, on the grounds that the
holder's own try ends at its limit. A holder that is no try of this
sluice's, an older sluice or a hand-held lock, would hang every read
again. Also not taken: a wait whose limit is the call's time. That is
five minutes under a wait.

### D5. The call's time runs from its first try

`Policy::run` reads `begun` from its clock each time its backoff loop
starts: at the first try, and again after a spent allowance's pause. Its
`when` stops trying, as run out, once the time since `begun` reaches
`bound`, beside the existing check of `until`. A command's call whose
tries each take twenty seconds makes a second try and no third.

Not taken: backon's total delay alone. It counts only the pauses.

### D6. Ctrl-C reaches sluice, not its try

git in a group of its own no longer takes the terminal's Ctrl-C.

- **A wait** has `stop_on_interrupt` in force. Its try in flight runs
  to its end or to its limit, and the policy then stops at its next
  pause. An API call in flight already ends this way, since Ctrl-C
  does not interrupt it either.
- **Any other command** keeps Ctrl-C's default and ends at once. The
  kernel lets its locks go with it: the fetch lock's descriptor is
  closed on exec, so no git holds it. Its git then ends by itself, as
  a credential command and a check already do. Under record 0110, a
  hook of that git finds the lock free and takes it as any fetch does.

Not taken: passing Ctrl-C on to the group of the try in flight. Every
command would need a signal handler, though Ctrl-C ends a wait and
nothing else, and the try holds nothing of sluice's once sluice has
gone. Rule core.29's details say this is not ruled out.

### D7. The record and the rule

Record 0109 is written on main from 66ad662f at the number it holds.
That number is free on main and on every branch: on 2026-11-10 only
`fix/main--a-hung-fetch-holds-no-lock` held a `docs/adr/0109-`. Its
Sources name record 0110 as well. Its decision says how a try's limit
sits beside record 0110: a sluice under sluice's own fetch waits for no
lock at all, and every other fetch waits for it no longer than a try. It
names this change for its spec, and D2, D4 and D6 as built within it.
It stays the code stage's own decision, accepted and cheap to change
later.

Rule core.29's details keep record 0110's case, then add 0109's after
it: a try with no limit; a try that holds or waits for the fetch lock
past `retry::TRY_FOR`; a try left running past its limit, with what it
started; a try started once the call's bound has passed; a stopped push
made again blind. "Not ruled out" gains the try in flight when Ctrl-C
ends a command that is no wait. Its ADR list ends `0108, 0109, 0110`.
The rule's own line is unchanged, so the site's rules page is too.

## Risks / Trade-offs

- **A transfer longer than thirty seconds is stopped.** The largest
  repository on deneb is 21 MiB, and sluice's fetches are incremental.
  A first fetch of a large repository may need more. `TRY_FOR` is one
  value to raise (record 0109).
- **A try is read again after up to thirty seconds rather than at
  once.** Against a remote that never answers, a command's call ends
  after about thirty seconds and one more try. It can never end later
  than the bound plus one try, about a minute.
- **A wait stops up to thirty seconds after Ctrl-C** when a try is in
  flight (D6). Before this change it stopped at once, since git took
  the Ctrl-C too.
- **An orphaned git after Ctrl-C** runs until its remote answers or
  drops it. It holds no lock of sluice's (D6).
- **Polling.** The lock is tried every 20 ms, and a bounded process's
  exit every 10 ms, both only while a try runs. That costs a fetch
  nothing that it would notice.
