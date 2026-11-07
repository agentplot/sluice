## Why

Clause 137, as amended on agentplot/blueprints main at 4791adf, has
status, the overview and a wait on a host that does not take a branch's
requests in show each open request that host submitted or reaches as
the taking host's. Each names the host the intake mark names and
whether its holder is that host's driver or a wait. Each also names the
request's entry there: taken in or waiting and on what, its state, its
number on that host and the entries ahead of it. Every one of these is
a field of the JSON form as well as words. They are read from the mark,
as the host's last fetch brought it, and from sluice's check on the
request's head, and never from the other host's records. C.5's pull
request row has the check carry those fields for any host to read.

crew's fix `fix--a-landing-in-another-hosts-queue-reads-as-in-the-queue`
looked for such a read and found none at sluice main 0b15d250. What
sluice does at 2f0b5c98:

- `sluice status <branch>` and `sluice overview` read this host's
  records alone (`Sluice::status`, `Sluice::overview`). A request taken
  in on another host shows nowhere. One this host handed over shows
  only as a withdrawn submission, under `--all`.
- A wait whose request another host takes in (`follow_across` in
  `crates/sluice/src/profiles/github/pulls.rs`) prints the check's title
  and summary. Under `--json` it prints `{"pull_request","branch",
  "state"}`, with `head` and `draft` for `--status`. Nothing names the
  host, the number there or the entries ahead as fields.
- sluice's check (`queued`, `queue_shown`, `waiting`, `outcome_shown`)
  says `queued behind #10, #11` or `testing main + #10 + #12`. It names
  no host and no number of the request's own, and nothing in it is a
  field a program reads.
- What a wait last read of a request is kept in its own process
  (`Pulls::seen`). Two waits on one host each read the request on their
  own bound, and a status could share neither.

## What Changes

- **The check carries the request's submission as fields.** Wherever
  sluice posts its `sluice` check on a request's head, the check names
  the host that posts it. Where that host took the request in, it also
  names the submission's number there, its state and the submissions
  ahead of it. Where the request waits to be taken in, it names what
  the request waits for. The words say the same, and a hidden block in
  the check's summary carries them for a program.
- **A host keeps the requests it follows.** A host that does not take
  a branch's requests in keeps, in its forge state, each request a
  submit on it opened, updated or marked ready, each request its
  hand-over withdrew, and each request a wait or status on it was asked
  about. With each it keeps what was last read of it. Every process on
  the host reads a request from GitHub at most once in a quarter of
  `[forge] discovery`, and shares what that read found.
- **status, the overview and a wait show where each request stands.**
  Each followed request that this host's queue does not hold at its head
  is shown as the taking host's. `taken_in_by` names the host the mark
  names and whether its `sluice run` or a wait holds it. `queue`
  carries the check's fields. Each is in the JSON form and in the words.
  A request whose check names no submission is shown waiting to be
  taken in by the mark's host.
- **The fields decide over the words.** A host reads a request's state
  from the check's fields where the check carries them, and from its
  words only where it does not, as a check an older sluice posted.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-status`: sluice's check names the host that posts it
  and, as fields as well as words, the request's submission there or
  what it waits for.
- `request-intake-host`: a host that does not take a branch's requests
  in keeps the requests it follows. It reads each at most once in a
  quarter of the interval for the whole host, and shows each as the
  taking host's.
- `submission-wait`: a wait on a request another host takes in prints
  `taken_in_by` and `queue` under `--json`, and names the host and the
  submission there in its words.
- `overview`: a branch whose requests another host takes in lists the
  requests this host follows, each as that host's.

## Impact

- `crates/sluice/src/profiles/github/pulls.rs`: the check's fields and
  words; `follow_across` reads through the kept follows; the hand-over,
  the submit and the wait add to them; a seam method answers status and
  the overview.
- `crates/sluice/src/profiles/github/kept.rs`: `forge/follows/`.
- `crates/sluice/src/lib.rs` and `crates/sluice/src/overview.rs`:
  status, overview and wait words and documents, reached through the
  `PullRequests` seam (core.1, core.2).
- `crates/sluice/tests/profiles/pull_requests.rs` and the GitHub double:
  the scenarios.
- The site's pull requests guide, the commands reference and the state
  reference; the sluice plugin's references where they say what a wait
  prints across hosts.
- docs/adr: a record of this decision and its construction. Rule
  core.33's details: the reads and the fields.

## Out of scope

- A branch whose records are on its remote. Every host reads the
  request's submission from those records (26, 80), and status shows
  it as a submission, as it does now.
- The hand-over's withdrawal naming the taking host as a field
  (`withdrawn.handed_over_to`). The change
  `a-hand-overs-withdrawal-holds-no-request` builds it, and this change
  reads it where it is.
- What GitHub shows of a completed check run whose words sluice
  changes, and a request merged on another host that `find` no longer
  lists. `a-failed-entry-is-submitted-again` names both as changes of
  their own.
- A request no submit, hand-over, wait or status on this host has
  touched. A host that does not take the requests in never lists the
  branch's open requests (137; rule core.33).
