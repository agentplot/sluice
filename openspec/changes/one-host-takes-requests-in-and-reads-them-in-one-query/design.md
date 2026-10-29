## Context

See proposal.md for why; record 0071 for the decision. This change
builds on `github-is-asked-only-when-something-moved` as it lands on
main: the host's `forge/` state (`kept.rs`: answers, pauses, each
branch's readings and `.moved`), `[forge] discovery`, `take_in`'s
decision of whether anything is due, the readings per request, postings
on change, and `follow` reading a wait's own request at most once in a
quarter of `discovery`.

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

- One process across hosts asks GitHub about a branch's open requests.
- No request is tested in two hosts' queues.
- A request out of draft that waits on its conditions costs one query
  per branch per discovery, not reads per request.

**Non-Goals:**

- Changing the same-host follow the bolt builds.
- Changing what `github-is-asked-only-when-something-moved` built.

## Decisions

### The intake mark's observation, in `kept.rs`

`kept.rs` gains `intake/<owner>/<repo>/<base>.json`, the intake mark as
this host last saw it: `{seen, since, renewed}`, written as every file
of `forge/` is.

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
  | absent | a driver | takes the requests in and pushes nothing |
  | absent | a wait, its host running no driver | pushes it (leased on absence) and takes the requests in |
  | its own | any | renews it when a third of `[peers] lease` has passed since `renewed` |
  | a wait's | a driver | takes it at once |
  | another's, id unchanged for `[peers] lease` since first seen | any | takes it |
  | another's | otherwise | takes nothing in |

  **A branch no other host has touched gets no mark** (the user,
  2026-10-28 and 2026-10-29: "No intake ref on the remote yet means
  this host alone"). The remote is all the hosts share, so the mark's
  absence is the one sign, read from the remote alone (core.27), that
  no other host has shown itself. A driver that finds no mark takes the
  requests in and pushes none, and pays nothing on a repository only it
  reaches. A wait on a host where no driver lives may be beside another
  host's driver it cannot see, so where it finds no mark it shows
  itself: it pushes the mark, leased on its absence, holds it as a wait
  and takes the requests in. The driver reads that mark in the fetch of
  the branch its next round makes and takes it at once, by the table's
  third row; from then on the mark stands, renewed by its holder. A
  wait beside its host's driver takes nothing in (the bolt) and so
  never pushes the mark. A wait that holds it and ends drops it, which
  returns the branch to no mark.

  A push refused at the lease is another host's take: this host takes
  nothing in this pass and reads the mark again next discovery. A
  driver is a process holding the host's `readers/<repository>.driver`
  lock (the bolt); anything else holding intake's lock is a wait. A
  wait that held the mark drops it when it ends, by a leased delete
  from its last value, in `submit_pull`'s and `wait_on`'s exits and on
  Ctrl-C; a failed drop leaves it to lapse.

**The hand-over.** On a branch whose records are each host's own, a
pass on a host that finds the mark another host's withdraws every entry of its
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

- [A hand-over withdraws a request mid-test on the old host] → Only on
  a host that lost the mark; the holder tests it again. Rare: a driver
  takes from a wait, or a lease lapsed.
- [The mark and the take race: a wait renews at t, a driver takes at
  t+ε, the wait's pass takes in at t+2ε] → The wait takes in only right
  after its own renewal or read of the mark in the same pass; the
  window is one pass, the two queues are each host's own, and a landing
  is a leased push, so nothing lands twice; the hand-over withdraws the
  wait's entry at its next pass.
- [A branch with no mark is taken in by a driver and a wait on another
  host for one pass, until the driver's next fetch reads the wait's
  mark] → The user's decision: a repository one host uses pays no push;
  the wait's mark is the first sign of another host, the driver takes
  it, and the hand-over settles any request both took in.
- [Two hosts' drivers on a branch with no mark both take its requests
  in, since neither pushes on absence] → Neither shows itself until a
  wait on a host with no driver pushes the mark; the hand-over then
  settles it.

## Migration Plan

The intake mark appears on the remote of a branch that takes requests
and keeps its records in each host's files once a wait on a host where
no driver lives takes its requests in; a
host on an older sluice ignores it and takes requests in as today until
it is upgraded, which the hand-over then settles. Rolling back leaves
the mark unread and harmless; `git push origin
:refs/sluice/intake/<branch>` removes it.
