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
- `Sluice::keeps` (`crates/sluice-core/src/step.rs`): on a branch whose
  policy is `signed` (records on the remote), the lead of clause 72 by
  this host's own observation of the records' version; a branch in the
  host's own files has no lead. The agentplot repositories name no
  signers: each host keeps its own records of a shared `main`, and
  each host's intake takes the same request into its own queue.
- The bolt `the-driver-pushes-outcomes-and-waits-follow-it`, which lands
  before this change: `sluice run` holds `readers/<repository>.driver`;
  `reader::driver_live` probes it; a wait beside a live driver runs no
  round and, under pull requests, no intake after its first pass.

## Goals / Non-Goals

**Goals:**

- No call to GitHub while the host's state says an allowance is spent;
  none to find out.
- A tick that has nothing due asks GitHub nothing.
- A request's reads cost nothing while it and its entry stand.
- One process across hosts asks GitHub about a branch's open requests.
- The design holds whether or not GitHub answers 304 to an ETag another
  token received.

**Non-Goals:**

- Writing a token to disk, sharing one between processes.
- Webhooks, or any process sluice asks a host to run.
- The cadence of the git fetch a round makes of its branch.
- Changing the same-host follow the bolt builds.

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
  intake/<owner>/<repo>/<base>.json
                            the intake mark as this host last saw it:
                            {seen, since, renewed}
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
| `Policy::command()` | a submit that does not wait, `--status`, `sluice branch`, `forge check` | until its `--timeout` where it takes one, or without end |
| `Policy::round()` (new) | the rounds of `sluice run` and `sluice step`, intake among them | never |

`main.rs` gives `Command::Run` and `Command::Step` `Policy::round()`,
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
way is the long-lived process doing the asking with the token it keeps
for its hour (the next sections). A task checks the answer by hand on a
host with the app, since no test reaches GitHub, and writes it into
record 0071.

GraphQL has no conditional answer; its query is asked only when the
REST list says something moved (below).

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

`Pull` gains `updated_at` (from `updated_at` in REST, `updatedAt` in
GraphQL) and `merged` (`merged_at` present; `merged` in GraphQL).
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

### One host takes a branch's requests in

`take_in` decides first whether this host takes the branch's requests
in this pass:

- **Records on the remote** (`rules.signed`): `Sluice::keeps(branch)`
  as the round calls it. The lead's lease is the deployment's (72, 73);
  a host that does not keep the branch takes nothing in. A wait whose
  round would take the branch (no lead, or a lease passed) takes it and
  takes in, as today.
- **Records in each host's files**: the intake mark,
  `refs/sluice/intake/<branch>` on the branch's remote. Its value is a
  root commit with an empty tree whose message is one JSON line,
  `{"host":"<peer name>","holder":"driver"|"wait","sluice":"<version>"}`,
  a new commit at each renewal so its id moves. `PushedRefs` gains
  `intake_mark(branch) -> Option<(Rev, Mark)>`, read from the ref the
  branch's fetch now also brings (`+refs/sluice/intake/<branch>:refs/sluice/<remote>/intake/<branch>`,
  one fetch, no extra call), and `put_intake_mark(branch, from:
  Option<&Rev>, mark)` and `drop_intake_mark(branch, from: &Rev)`, each
  a leased push through `PushedRefs::push` under the retry policy
  (core.5, core.29).

  At each discovery the host reads the mark and its observation
  (`intake/…json`: the mark's id when first seen and when):

  | The mark | This host | It |
  |---|---|---|
  | absent | any | takes it (leased on absence) |
  | its own | any | renews it when a third of `[peers] lease` has passed since `renewed` |
  | a wait's | a driver | takes it at once |
  | another's, id unchanged for `[peers] lease` since first seen | any | takes it |
  | another's | otherwise | takes nothing in |

  A push refused at the lease is another host's take: this host takes
  nothing in this pass and reads the mark again next discovery. A
  driver is a process holding the host's `readers/<repository>.driver`
  lock (the bolt); anything else holding intake's lock is a wait. A
  wait that held the mark drops it when it ends, by a leased delete
  from its last value, in `submit_pull`'s and `wait_on`'s exits and on
  Ctrl-C; a failed drop leaves it to lapse.

**The hand-over.** On a branch whose records are each host's own, a
pass on a host that does not hold the mark withdraws every entry of its
own records submitted from a `pull/<n>` place and neither out nor
landing (122), with the event naming the host that holds the mark
(`withdrawn: <host> takes main's pull requests in`). It posts nothing
on the request and comments nothing: the holder posts. The holder takes
each in again as its conditions hold, so no request is tested in two
queues. Entries submitted from the host's own places are not touched.

**A wait that follows across hosts.** `submit_pull`'s loop, on a host
that does not take the branch's requests in, calls no intake and no
round. `Pulls::follow` finds no entry of the request in this host's
records, so it reads GitHub: `forge.pull(n)` and sluice's check on the
request's head (`posted` for `sluice`), each conditional and each no
more often than once in a quarter of `discovery` (the loop keeps the
last answer between), and returns:

- `merged`: `Followed::Landed`, reported as a landing is, exit 0, with
  the check's title;
- sluice's check concluded `failure` on the request's current head:
  `Followed::SentBack`, reported as the submission's outcome its title
  names (failed or conflict), with its exit;
- closed and not merged: `Followed::Closed`, as today;
- otherwise `Followed::Open` with the check's title and summary lines,
  in place of `shown`.

`Followed::Landed` and `Followed::SentBack` are new variants of the
enum in `crates/sluice/src/lib.rs`; their words and exits are those of
`report` for the same outcome.

### One GraphQL query per branch per discovery

`Forge` gains `requests(repo, base) -> Result<Vec<Request>>`, where
`Request { pull: Pull, reviews: Vec<Review>, checks: Vec<Check>,
posted: Vec<Posted>, readied: Vec<DateTime<Utc>> }`. `App` answers it
with one query, paged fifty at a time:

```graphql
query($owner: String!, $name: String!, $base: String!, $after: String) {
  repository(owner: $owner, name: $name) {
    pullRequests(states: OPEN, baseRefName: $base, first: 50, after: $after) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number id title body isDraft updatedAt headRefOid headRefName
        author { login } headRepository { nameWithOwner } baseRefName
        latestOpinionatedReviews(first: 30) { nodes { author { login } state commit { oid } } }
        timelineItems(itemTypes: [READY_FOR_REVIEW_EVENT], last: 10) {
          nodes { ... on ReadyForReviewEvent { createdAt } }
        }
        commits(last: 1) { nodes { commit { statusCheckRollup { contexts(first: 100) { nodes {
          __typename
          ... on CheckRun { databaseId name status conclusion title summary text
                            checkSuite { app { databaseId } } }
          ... on StatusContext { context state }
        } } } } } }
      }
    }
  }
}
```

By GitHub's reckoning (connections counted at their `first`/`last`,
divided by a hundred) a page costs about three points of GraphQL's own
allowance. Check runs of sluice's app (`checkSuite.app.databaseId` the
app's id) are `posted`; the rest, with the statuses, are `checks`, the
latest of each name, mapped to `Standing` as `rest.rs` maps them;
`latestOpinionatedReviews` are `reviews`, which `approvals` counts as
today.

`take_in` asks it at a discovery whose conditional list came back
changed (a 200), or while a request out of draft waits on its
conditions, or when a request has no reading; its answer fills every
request's reading at once, and `missing`, `is_held` and `show` read the
reading. A query GitHub refuses (other than `Limited`) falls back to the
REST reads per request for that pass, said once. `FileForge` answers
`requests` from its file. The REST list stays, as the one conditional
question.

### What each caller costs, once built

| Caller | Each tick (3 s) | Each discovery (60 s) | Besides |
|---|---|---|---|
| host's driver, a branch with requests | nothing | one REST list (304 free) | one GraphQL query; postings that changed |
| a wait beside its host's driver | nothing (the bolt) | nothing | its own request, conditionally, once in 15 s |
| a wait on a host that does not take in | nothing | nothing | its own request and check, conditionally, once in 15 s |
| any process while a pause stands | nothing | nothing | nothing |

## Risks / Trade-offs

- [A request opened by hand, or a review given, is taken up to a minute
  later] → The user's decision; `[forge] discovery` shortens it per
  host, ten seconds at least. A submit on the host asks at once.
- [ETags turn out per token] → The design holds: the driver's token
  lasts its hour, a 200 costs no more than today, and the other savings
  do not depend on it.
- [A hand-over withdraws a request mid-test on the old host] → Only on
  a host that lost the mark; the holder tests it again. Rare: a driver
  takes from a wait, or a lease lapsed.
- [The mark and the take race: a wait renews at t, a driver takes at
  t+ε, the wait's pass takes in at t+2ε] → The wait takes in only right
  after its own renewal or read of the mark in the same pass; the
  window is one pass, the two queues are each host's own, and a landing
  is a leased push, so nothing lands twice; the hand-over withdraws the
  wait's entry at its next pass.
- [A no-wait submit waits up to an hour for a spent allowance] → The
  user's decision: it says until when, once, on stderr; `--timeout`
  bounds it; Ctrl-C stops it. Failing would only make the agent call
  again.
- [The kept answers hold request titles and bodies on disk] → The
  host's records hold the same subjects; the directory is the user's
  own state, written with the user's mode.
- [Another host keeps calling until its own first refused call] → One
  call per host; GitHub's limit is the installation's, and the pause is
  written from that first answer.

## Migration Plan

Nothing to migrate. `forge/` is made on first use; a host without it
asks as if nothing were kept. `[forge] discovery` is optional. The
intake mark appears on the remote of a branch that takes requests and
keeps its records in each host's files, at its first discovery after
this sluice is installed; a host on an older sluice ignores it and
takes requests in as today until it is upgraded, which the hand-over
then settles. Rolling back leaves `forge/` and the mark unread and
harmless; `git push origin :refs/sluice/intake/<branch>` removes the
mark.

## Open Questions

- Whether a submit that does not wait should wait out a spent allowance
  without end, as decided here, or fail at once with exit 3 naming the
  time it resets.
