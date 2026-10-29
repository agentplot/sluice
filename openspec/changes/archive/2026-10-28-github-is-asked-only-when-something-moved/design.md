## Context

See proposal.md for why; record 0071 for the decision. As built on main
(eddf83c2):

- `Command::Run` in `crates/sluice/src/lib.rs` calls `intake(pulls, ..)`
  for every branch on every pass, then the round. `wait_on` (a `sluice
  wait <n>`, a waiting submit, a group submit) and `submit_pull` (a pull
  request's submit and `sluice wait <branch>`) call `intake` before each
  round, every three seconds; `Command::Step` calls it once.
- `Pulls::take_in` (`profiles/github/pulls.rs`) takes intake's host lock
  (`sluice/.intake/<branch>.lock`, try-lock: the process holding it does
  the pass), then: `forge.pulls` (the list, paged by a hundred); per
  open request of the repository, `consider`: `is_held` and
  `retried_by_hand` read the timeline (`readied`) when its latest entry
  left unlanded; `missing` reads `reviews` and, when the policy names
  checks, `checks` (check runs and the combined status: two reads);
  `fetch_head` fetches `refs/pull/<n>/head` with git when the host's
  copy differs; a forecast; then `show` reads `posted_all` and posts
  when it differs, and `show_report` reads `comment_list` and
  `posted_all` again. `known`, `entries`, `shown` and `log_lines` are
  per-process `RefCell`s. `follow` reads `forge.pull(n)` and, for a
  request sent back, its timeline, every tick of a wait.
- `App` (`profiles/github/app.rs`) buys an installation token per
  repository per process, kept in memory (the forge-identity capability
  forbids writing it), and makes every call through `App::answer` under
  the retry policy: a 5xx or 429 is `Judged::Never` (backed off), a 401
  `Credential`, any other status the answer: a 403 that says the limit
  is spent fails the call. No response header is read.
- `retry::Policy` (`crates/sluice/src/retry.rs`): `Policy::waiting(deadline)`
  for a waiting submit and `sluice wait`, five minutes; `Policy::command()`
  for everything else, rounds included, thirty seconds.

## Goals / Non-Goals

**Goals:**

- No call to GitHub while the host's state says an allowance is spent;
  none to find out.
- A tick that has nothing due asks GitHub nothing.
- A request's reads cost nothing while it and its entry stand.
- The design holds whether or not GitHub answers 304 to an ETag another
  token received.

**Non-Goals:**

- Writing a token to disk, sharing one between processes.
- Webhooks, or any process sluice asks a host to run.
- The cadence of the git fetch a round makes of its branch.

## Decisions

### The host's `forge/` state, in one module

New `crates/sluice/src/profiles/github/kept.rs` owns
`$XDG_STATE_HOME/sluice/forge/` (beside `readers/`, `running/`,
`sessions/`; `instance::state_dir`):

```text
forge/
  answers/<h>.json          one kept answer per address: {url, etag,
                            last_modified, body, at}; <h> the first 32
                            hex of sha256(url)
  pauses/<app>-<installation>-<resource>.json
                            {until, words, streak}; resource is GitHub's
                            x-ratelimit-resource: core, graphql
  requests/<owner>/<repo>/<base>.json
                            each branch's readings: {asked, tip,
                            version, requests: {<n>: Reading}}
  requests/<owner>/<repo>/<base>.moved
                            touched by a submit on the host; newer than
                            `asked` asks at once
```

`<base>` is written as `runs::flat_name` writes a name. Every file is
written to a temporary file and renamed over the last, so a reader sees
a whole one or none; a `.json` that does not parse is read as absent and
written again. `requests/…json` is changed under an exclusive lock on a
sibling `.lock` (the house pattern of `reader.rs`: a lock that goes
with its process). Nothing here holds a token. Answers not used for a
day are removed by the process that writes an answer, at most once an
hour, by modification time.

The directory decides nothing the forge has not just confirmed: an
answer is used only on GitHub's 304 for it (core.11, core.14); a
reading only says what was last read, so that a request that has not
moved is not read again; a pause only says when GitHub said to call
again.

### A spent allowance is the retry policy's fifth class

`retry::Judged` gains `Limited`. `Failed` carries, for it, `until:
DateTime<Utc>`. The policy never backs off a `Limited` try: it sleeps
until `until` (on the same sleeper, so Ctrl-C and the tests' recording
sleeper apply) when the policy may wait that long, and otherwise stops
at once with the error `Limited { until, words }`, whose `Display` is
`GitHub's allowance of calls for <app> is spent until <HH:MM local>:
<GitHub's words>`.

How long a policy may wait for a pause is its own, beside its bound:

| Policy | Built for | Waits a pause |
|---|---|---|
| `Policy::waiting(deadline)` | waiting submit, `sluice wait` | until the deadline (`--timeout`), or without end |
| `Policy::command()` | `--status`, `sluice branch`, `forge check` | until its `--timeout` where it takes one, or without end |
| `Policy::command().unpaused()` | a submit that does not wait (the user, 2026-10-28) | never: it fails at once, exit 3, naming the time the pause ends |
| `Policy::round()` (new) | the rounds of `sluice run` and `sluice step`, intake among them | never |

`main.rs` gives a submit with `--no-wait` `Policy::command().unpaused()`:
its `Limited` error is the command's, exit 3, saying until when (the
user, 2026-10-28). `main.rs` gives `Command::Run` and `Command::Step` `Policy::round()`,
which is `Policy::command()`'s thirty seconds with no wait for a pause.
A command that waits says once on stderr, at the first pause it meets,
`sluice: GitHub's allowance of calls is spent until <HH:MM>; waiting`
(with GitHub's words), and nothing on stdout. A round that meets one
fails intake with `Limited`; `lib.rs`'s `intake` says it once per pause
(by `until`), as `tell_passed` says a passed branch once, and the round
goes on.

### GitHub's word that an allowance is spent

`App::call` returns the response's headers it needs beside the status:
`x-ratelimit-remaining`, `x-ratelimit-reset`, `x-ratelimit-resource`,
`retry-after`. `App::answer` classes:

- `Limited` when the status is 429; or 403 and either
  `x-ratelimit-remaining: 0`, `retry-after` present, or a message
  holding `rate limit` (GitHub's "API rate limit exceeded" and "You have
  exceeded a secondary rate limit"); or a GraphQL answer whose `errors`
  hold `"type": "RATE_LIMITED"` (status 200, `graphql_reply`).
- `until`: `retry-after` seconds from now when present; else
  `x-ratelimit-reset` when remaining is 0; else one minute doubled for
  each `Limited` in a row on that pause file (`streak`), thirty minutes
  at most. GitHub's documentation asks exactly this.
- A success that leaves `x-ratelimit-remaining: 0` returns its answer
  and writes the pause, so the next call, in any process, is not made.

Every pause is written to `pauses/<app>-<installation>-<resource>.json`
(REST calls are `core`; GraphQL's `graphql`; the resource GitHub names
wins), keeping the later `until` of the one there and the new. A
success resets `streak`. Before every call, `App::answer` reads the
pause for the call's installation and resource; a standing one is a
`Limited` try with no call made. The JWT's calls (finding the
installation, buying a token) are the app's own allowance and read no
pause.

A 429 is no longer `Never`, and a rate-limit 403 no longer the answer
(the `remote-retry` delta). A `Make` call refused `Limited` was refused
before GitHub took it, so it is made again after the pause, not read
back.

### Every `GET` is conditional

`App::get` and `App::get_found` read `answers/<h>.json` for the full
URL, page and query included, and send `If-None-Match: <etag>` and
`If-Modified-Since: <last_modified>` when it holds them. A 304 returns
the kept body as the answer, and touches the file; a 200 writes the new
ETag, last-modified time and body. A 404 removes the kept answer. The
paged reads (`rest.rs` `all`) ask each page this way, so an unchanged
list of three pages costs nothing.

The ETag is sent whichever process, and so whichever token, stored it.
GitHub's ETags depend on the token, by every report found (community
discussion 75228; terraform-provider-github 3694) and its documentation
does not say: where it answers 304 to another token's ETag, every
process on the host gains; where it answers 200, the call costs what it
does today and replaces the kept answer. The saving that holds either
way is the discovery interval and each request's readings, kept in the
host's state for every process there (the next sections). A task checks the answer by hand on a
host with the app, since no test reaches GitHub, and writes it into
record 0071.

### Discovery apart from change

`instance.rs` reads `[forge] discovery`, a duration as `[peers] lease`
is read (`"60s"`, `"2m"`), default sixty seconds, refused below ten
seconds with the key and the least it takes. `Pulls::new` gets it.

`take_in` opens with a decision made from the host alone:

```
due  = now - readings.asked >= discovery
       || moved file newer than readings.asked
moved_branch  = t.tip != readings.tip
moved_records = the records' version (Line) != readings.version
```

- Nothing of the three: the pass returns with no call to GitHub.
- Any of the three: the list is asked for, conditionally, and
  `readings.asked`, `tip` and `version` written. A landing closes
  requests and changes every waiting request's forecast, and an entry
  whose state moved changes what its request shows, so either asks at
  once; an unchanged list is a 304 and costs nothing, and the postings
  of requests whose entry moved are made from the readings (below).

`readings.asked` is in the host's state: a `sluice run` restarted, by
hand or by a supervisor every seventeen seconds, asks nothing until the
interval has passed since any process on the host asked.

`submit_pull` and `submit_pull_group`, after they open, update or mark
a request ready, touch `requests/<owner>/<repo>/<base>.moved`, so the
host's next pass asks at once. A submit on another host is seen at this
host's next discovery.

### A request read again only once it moved

`Pull` gains `updated_at` (from `updated_at`) and `merged` (`merged_at`
present).
`Reading`, kept per request number:

```
head, updated_at, draft,          as the list last said
entry: (id, state)?,              its latest entry on this host
missing: Vec<String>?,            what it lacked when last read
held: bool?, readied: Vec<time>,  the timeline's ready marks last read
posted: {name: Posting},          what sluice last posted or read back
report: comment id?, said: [notification ids commented]
```

`consider` and `take_groups` read through the reading:

- A request whose `head`, `updated_at`, `draft` and `entry` equal the
  reading's uses the reading's `missing`, `held` and `readied`: no
  `reviews`, `checks` or `readied` call. One out of draft whose
  `missing` is not empty is read again at each discovery
  (conditionally), since another system's check moves no request's
  `updated_at`.
- A draft reads nothing beyond the list: `missing` is `["a draft"]`, as
  today; `is_held` reads its timeline only once it is out of draft.
- `show` and the per-check posts in `show_report` compare with
  `posted`; equal posts nothing, different posts and writes `posted`.
  `posted_all` is read only for a head the reading holds no `posted`
  for, and its answer written there. `show_report` edits the comment
  `report` names without `comment_list`, reading the list only when
  `report` is unset; `comment_once` consults `said` before the list.
- `fetch_head` already fetches only when the host's copy differs.

Readings are written once per pass under the branch's readings lock,
held by the process that holds intake's lock.

### What each caller costs, once built

| Caller | Each tick (3 s) | Each discovery (60 s) | Besides |
|---|---|---|---|
| `sluice run`, a branch with requests | nothing | one REST list (304 free to the process whose token received it); the reads of a request that moved or waits on its conditions | postings that changed |
| a wait or a waiting submit | nothing, unless something moved on the host | the same asking, shared with every process on the host through `asked` | its own request, conditionally, once in 15 s |
| any process while a pause stands | nothing | nothing | nothing |

## Risks / Trade-offs

- [A request opened by hand, or a review given, is taken up to a minute
  later] → The user's decision; `[forge] discovery` shortens it per
  host, ten seconds at least. A submit on the host asks at once.
- [ETags turn out per token] → The design holds: a long-lived `sluice
  run`'s token lasts its hour, a 200 costs no more than today, and the
  other savings do not depend on it.
- [A no-wait submit fails while the allowance is spent, and its agent
  may submit again at once] → The user's decision (2026-10-28): it
  fails at once, exit 3, naming the time the pause ends, so the agent
  knows when to try; a submit again before then makes no call to GitHub,
  since the pause is read first.
- [The kept answers hold request titles and bodies on disk] → The
  host's records hold the same subjects; the directory is the user's
  own state, written with the user's mode.
- [Another host keeps calling until its own first refused call] → One
  call per host; GitHub's limit is the installation's, and the pause is
  written from that first answer.

## Migration Plan

Nothing to migrate. `forge/` is made on first use; a host without it
asks as if nothing were kept. `[forge] discovery` is optional. Rolling
back leaves `forge/` unread and harmless.

