## Context

ADR 0062 decides the policy; clause 134 and rule core.29, as amended
for forge writes on batch 35's question, bind it. What
is built on main (981c38b2):

- `PushedRefs::remote_git_noted` (`crates/sluice/src/profiles/pushed_refs.rs`)
  runs every git call to the remote. With sluice's identity, a call that
  `stopped` (refused per `REFUSED`, and no porcelain line with a tab) is
  run once more on `Identity::renew`, and the note names both refused
  credentials. Nothing else is tried again.
- `PushedRefs::leased` already settles a push with no answer by
  fetching the branch; `push_branch` and every push in
  `remote_refs.rs` (`push_leased`, `delete`, `delete_if_present`,
  `publish_head`, `beat`, `publish_candidate`, the logs push) bail on
  any failure.
- `App::issue(repo, fresh)` (`profiles/github/app.rs`) buys a fresh
  token and sleeps until it is `SETTLE` (5 s) old. `App::call` makes one
  ureq call; `failed` turns a 5xx into `sluice_seams::Unanswered`.
  Nothing tries an API call again, and a 401 on the API buys nothing.
- The wait (`wait_on` in `crates/sluice/src/lib.rs`) keeps `Unread`: a
  round or read that fails `Unanswered` is run again after the wait's
  pause until `UNANSWERED_FOR` (5 min) since the first in a row, then
  `could_not_read` exits `INTERNAL`.

## Goals / Non-Goals

**Goals:**
- One policy type, used at the two places that reach a remote: the
  git call in `PushedRefs` and the HTTP call in `App`.
- Each failure classed once, where the call is made: never judged,
  credential refused, answered, or not known.
- A forge write classed by what it does: a state set tried again, a
  thing made read back before it is made again, a dispatch never sent
  twice.
- The bound chosen by the command, never by the call site.

**Non-Goals:**
- Any change to how the branch's log records a refusal (133) or to the
  host's kept refusal state.
- `release_retries`, `ensure_readied`'s ready-mark tries, and the
  waits on local processes (`runs.rs`, `places.rs`): none is a call to a
  remote that failed.

## Decisions

### The policy is `backon`'s blocking retry, in `crates/sluice/src/retry.rs`

`Policy { bound: Duration, until: Option<Instant>, sleep: Box<dyn Fn(Duration) -> bool> }`
wraps `backon::BlockingRetryable` with `ExponentialBuilder::new()
.with_jitter().with_min_delay(1 s).with_max_delay(30 s)
.without_max_times().with_total_delay(Some(bound))`. Its one entry,
`Policy::run(kind, op)`, runs `op` (an `FnMut(Try) -> Outcome`) until it
succeeds, or its class says stop, or the backoff yields no pause, or
`until` passes, or the sleeper returns `false` (Ctrl-C). `when` reads
the class, `notify` traces each try at debug level (`tracing`, never
the branch's log), and `sleep` is the policy's sleeper.

Checked against backon 1.6.0's source: `BlockingRetry` offers `sleep`,
`when`, `notify` and `call`, and no per-error `adjust` (that is async
only); `with_total_delay` caps the sum of pauses; the default
`max_times` is 3, so `without_max_times` is needed; the dependency is
`backon = { version = "1.6", default-features = false, features = ["std", "std-blocking-sleep"] }`,
which pulls no runtime (the defaults pull tokio and gloo).

Alternatives: `retry` 2.2.0 would do but has no jitter builder of the
same shape; `backoff` 0.4.0 is unmaintained since 2021; `tokio-retry`
is async only; a loop of sluice's own is ruled out (core.29).

### Classes, decided at the call site that knows the protocol

```
enum Judged { Never, Credential, Answered, Unknown }
enum Kind   { Read, Push, Write }
```

The policy tries again `Never` and `Credential` for every kind, and
`Unknown` for `Read` only; `Answered` never. A `Credential` failure on
the host's own credentials is classed `Answered` for a push (nothing can
be bought afresh) and stays retried for a read.

git (`pushed_refs.rs`, beside `REFUSED`):
- `Credential`: today's `stopped`: stderr matches `REFUSED`, no
  porcelain line with a tab.
- `Never`: stderr says the connection was not made (`Could not resolve
  host`, `Could not resolve hostname`, `Failed to connect`,
  `Connection refused`, `Connection timed out`, `Operation timed out`),
  or the remote answered git's ref discovery with a 5xx or 429 (`The
  requested URL returned error: 5xx` / `429`): nothing of a push was
  sent then.
- `Answered`: any porcelain line with a tab (a ref's answer); a fetch's
  `couldn't find remote ref` (which `fetch_spec` turns into `Ok(false)`
  before classing).
- `Unknown`: everything else, `RPC failed; HTTP 5xx`, `the remote end
  hung up unexpectedly` and `send-pack: unexpected disconnect` among
  them: for a push, sent but unanswered.

HTTP (`app.rs`, from ureq 3.4.2's `Error` and `Timeout`):
- `Never`: `Error::HostNotFound`, `Error::ConnectionFailed`,
  `Error::Timeout(Timeout::Resolve | Timeout::Connect)`: the connection
  was never made. A status 5xx or 429 is `Never` too, except on a call
  that makes something (below).
- `Credential`: 401 on an installation token.
- `Answered`: any other status not 2xx (and 404 where `get_found` reads
  it as absent).
- `Unknown`: any other transport error, and a 5xx or 429 on a call that
  makes something.
- Buying a token (`POST .../access_tokens`) and finding the installation
  are reads: they change nothing the repository holds.

### A forge write is classed by what it does

134 as amended (blueprints e524de8) and core.29 (sluice b33c8ca9) split
the forge's writes in two, and the `App`'s calls carry which, as
`Call { Read, Set, Make }`, mapped onto the policy's `Kind` in `app.rs`
alone, so `retry.rs` keeps its three kinds:

| Call | What it is | Policy | Its writes |
|---|---|---|---|
| `Read` | a `GET`, a GraphQL query, a token bought | `Kind::Read` | every read |
| `Set` | sets a state; twice is once | `Kind::Read` | the draft mark (`set_draft`, GraphQL); a check run updated (`PATCH /check-runs/{id}`); a comment edited; a pull request's body edited |
| `Make` | makes or starts something | `Kind::Write`, a 5xx or 429 classed `Unknown` | a check run created (`POST /check-runs`); a comment posted; a pull request opened (`POST /pulls`); a workflow dispatched |

So a `Set` is tried again on any failure but GitHub's answer, as a read
is; a `Make` is tried again blind only on `Never`, a connection not
made, or `Credential`, a token GitHub refused before taking the call.
A `Make` the policy ends on `Unknown` fails with
`sluice_seams::Unanswered` in its chain and the words `GitHub's answer
to <method> <path> was not read`, then GitHub's own words when it gave
any, so the caller knows to read before making it again.

A pull request opened is not named in 134's list; it makes something,
as a comment does, and is held to the same rule.

### A thing made whose answer was not read is read back first

Each `Make` but the dispatch is settled where it is made, in
`rest.rs`, on an `Unanswered` failure, by one read under the policy:

- a check run created: `posted_all(repo, rev)` holds a run of the
  posting's name: it is carried on, the post done; none: posted again,
  once, under the policy as a `Make`.
- a comment posted: `comment_list(repo, number)` holds a comment whose
  body is the body posted: carried on; none: posted again.
- a pull request opened: `pulls(repo, base)` holds one from `head_ref`:
  it is the pull request opened; none: opened again.

A second `Unanswered` is the call's failure, as today's.
`comment_once`'s read before posting and `show`'s read of the posted run
stay; they decide whether to post at all, before any call.

### A dispatch whose answer was not read is claimed, and never sent again

`Actions::dispatch` makes its call as a `Make` and never reads or sends
it again: nothing reads a dispatch back by its call (the run it starts
is read later by its title). On `Unanswered`:

- **launch** (`step.rs`, `launch_on_pools`): the core reads
  `sluice_seams::unanswered(&e)` and claims the run as if the dispatch
  had been answered: the run `Starting`, its host the pool's claim, and
  the `Claimed` event carrying `unanswered: Some(<the error's words,
  whole>)`, a field skipped when `None`, so records written before read
  as today. The next round's `lapsed` reads, through `job_alive`,
  whether a job of that claim runs; a dispatch GitHub never took lapses
  the claim and the run is open again (77, 88), dispatched then under a
  new claim. Any other failure fails the round, as now. This is the
  event 134 names: the branch's log holds the dispatch and that its
  answer was not read, once, in the claim's own event (133).
- **wake** (`ops.rs`): unchanged; a failed wake is warned and loses
  nothing (59, 111), and the warning now carries the unanswered words.

### A refused credential is bought afresh at the start of the next try

The op is handed a `Try` with whether the last failure was `Credential`;
when it was, the op calls `Identity::renew` (git) or `App::issue(repo,
true)` (HTTP) before calling, and appends the refused credential's
`about` to the list the final note is built from. `issue` keeps its
settle, so the fresh token is five seconds old when it is handed over
(core.29 leaves this to the token). A renew that fails ends the policy
with the refusal and why none could be bought, as today. The note keeps
today's form, extended to N: `origin refused <first>, then <second>,
then <third>`.

### A push not known is settled by reading the ref it set

`PushedRefs` gets one push entry, `push(args, ref, to: Option<&Rev>)`,
which every push in `pushed_refs.rs` and `remote_refs.rs` uses. It runs
the push under the policy as `Kind::Push`; on `Unknown` it fetches (or
`ls-remote`s) `ref` once, under the policy as a read, and returns
success when the remote holds `to` (or no ref when `to` is `None`),
otherwise the failure, with `the push went unanswered, and <what the
remote holds>`. `leased` keeps its `Moved` mapping on top of it; the
records' pushes keep their `Ok(false)` on a stale lease.

### The bound is the command's, handed down at construction

`main.rs` builds one `Policy` per command: `Policy::waiting(deadline)`
(five minutes, `UNANSWERED_FOR`, ending at `--timeout`'s deadline and on
Ctrl-C through the same `INTERRUPTED` flag `Sleep` reads) for a waiting
submit and `sluice wait`; `Policy::command()` (thirty seconds) for
everything else. It is passed, `Rc`-shared, to `App::open` and to the
`identity`/`PushedRefs::open_as` path in `profiles/branches.rs` and
`profiles/github/pulls.rs`, so a round a wait runs gets the wait's five
minutes, as the wait reads through an outage today. Thirty seconds is
"seconds" (134) with room for two credential renewals (five seconds of
settle each) and four or five pauses.

Alternative: a process-wide bound in a static, set at command start.
Rejected: hidden state the tests would have to reset, and a profile
reading a global the core set is a seam crossed sideways (core.2).

### The wait's `Unread` is gone; its words stay

`wait_on` no longer reads again on `Unanswered`: the policy has tried
for five minutes by the time an `Unanswered` reaches it. It goes
straight to `could_not_read`, whose duration comes from the policy's
spent error: the policy wraps its last error in a `Spent { tries,
lasted }` context, and `could_not_read` reads `lasted` from it (zero
when absent, as from a fake seam). `UNANSWERED_FOR` stays as the
policy's wait bound. The message, the exit `3`, `--json`'s error
document and its `next` are unchanged.

### Tests never sleep

`Policy` takes its sleeper; tests give one that records each pause and
returns at once, and a seed for the jitter. The integration tests that
run the binary (`cli.rs`, `pushed_refs.rs`, `profiles/forge.rs`) set
`SLUICE_RETRY_SLEEP=record` (read in `main.rs` alone, as `API_VAR` is
read in `App::open`) so pauses are skipped and the binary's tries run at
once; the fake servers count the calls.

## Risks / Trade-offs

- [A dispatch GitHub refused 429 is reported unanswered, not tried
  again] → 134 lets a dispatch go again only while the connection was
  never made; the claim lapses on the next round and the run is
  dispatched then, so a refused dispatch costs one round.
- [A check run found on reading back may be another process's of the
  same name] → `posted_all` reads only the app's own runs on the
  revision, and a run of the posting's name there is the one sluice
  posts; the next `show` sets it to the posting in any case.
- [git's wording is the classifier] → the strings are git's own,
  checked against `remote-curl.c` (`RPC failed`, `The requested URL
  returned error`) and curl's messages; a wording sluice does not know
  falls to `Unknown`, which is never pushed again: the safe side.
- [Thirty seconds of tries holds a round up] → a round that cannot
  read logs once and ends; a round runs again on the timer in any case.
- [A five-minute call inside a wait overruns `--timeout`] → the
  policy's `until` is the wait's deadline.

## Migration Plan

No state or config changes. The branch's log gains one optional field,
`unanswered` on a `Claimed` event, written only when a dispatch's answer
was not read and skipped otherwise: a log written before reads as it
did, and an older sluice reading a newer log ignores the field. A host
on the new sluice simply tries more before failing.
