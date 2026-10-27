## Context

ADR 0062 decides the policy; clause 134 and rule core.29 bind it. What
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
  `Error::Timeout(Timeout::Resolve | Timeout::Connect)`, a status 5xx or
  429.
- `Credential`: 401 on an installation token.
- `Answered`: any other status not 2xx (and 404 where `get_found` reads
  it as absent).
- `Unknown`: any other transport error: a `GET` or a GraphQL query
  (`Kind::Read`) is tried again, a `POST`/`PATCH` (`Kind::Write`) is
  reported with what was not read.
- Buying a token (`POST .../access_tokens`) and finding the installation
  are `Kind::Read`: they change nothing the repository holds.

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

- [A `POST` GitHub answered 5xx may have been applied, and is tried
  again] → clause 134 classes a server unable to answer as never
  judged, and this change follows it. The writes sluice makes are a
  check run created or updated (`rest.rs`), a comment (`comment_once`
  reads the comments before posting), a pull request's draft mark
  (GraphQL, the same state set twice) and a workflow dispatch
  (`actions.rs`). A dispatch tried again after a 5xx GitHub had in fact
  taken would start a second workflow run; this is named in the change's
  report for the design agent rather than decided here.
- [git's wording is the classifier] → the strings are git's own,
  checked against `remote-curl.c` (`RPC failed`, `The requested URL
  returned error`) and curl's messages; a wording sluice does not know
  falls to `Unknown`, which is never pushed again: the safe side.
- [Thirty seconds of tries holds a round up] → a round that cannot
  read logs once and ends; a round runs again on the timer in any case.
- [A five-minute call inside a wait overruns `--timeout`] → the
  policy's `until` is the wait's deadline.

## Migration Plan

None: no state, config or record changes. A host on the new sluice
simply tries more before failing.
