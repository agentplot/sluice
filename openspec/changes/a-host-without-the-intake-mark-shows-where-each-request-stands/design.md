# Design

## Context

137 at agentplot/blueprints 4791adf, on a branch whose records each host
keeps (C.6 local files), where the host that holds the branch's intake
mark takes its requests in:

> The same reads answer a status, an overview or a wait of the branch
> on a host that does not take its requests in, for every open request
> that host submitted or reaches (26, 80): each is shown as the taking
> host's, naming the host the mark names and whether its holder is that
> host's driver or a wait, and the request's entry there, taken in or
> waiting and on what, its state, its number on that host and the
> entries ahead of it, each a field of the JSON form as well as words,
> and a withdrawal for the hand-over names the taking host as a field
> (record 0088). sluice's check on the request's head carries those
> fields for any host to read (C.5), and the mark's host is read from
> the mark as the tick's fetch brings it (129); neither is read from the
> other host's records, and a request whose check names no entry yet is
> shown as waiting to be taken in by the mark's host.

C.5's pull request row at the same commit: the check carries "the
entry's state, what it waits for, the host that took it in, its number
there and the entries ahead of it, each a field a program reads as well
as the check's words (137)".

What sluice holds at 2f0b5c98, each in
`crates/sluice/src/profiles/github/pulls.rs` unless named:

- **The check.** `queued` titles a queued request `queued: next on
  main` or `queued behind #10, #11`. Its summary is "In main's queue; it
  lands once its combination passes." `queue_shown` titles one under
  test `testing main + #10 + #12`. `waiting` titles one that lacks its
  conditions `waiting: needs 1 review`, `waiting: a draft`, `waiting:
  the next submit` or `waiting: withdrawn by …`. `outcome_shown` gives
  landed, failed and conflict. None names the host that posted it or
  the request's own number on that host. A `Posting` is a status, a
  title, a summary and an optional text (`forge.rs`), compared whole
  before each post (`post_check`), so a post goes out only when what it
  says changed (core.32).
- **The wait across hosts.** `follow` sends a request whose branch
  another host takes in (`away`) to `follow_across`. That reads the
  pull and the `sluice` check on its head, each no more often than once
  in a quarter of `discovery`, from `Pulls::seen`, which lives in the
  process. It answers `Followed::Open { title, lines, submission: None,
  … }` with the check's title and summary lines. The wait prints those
  as they change. Under `--json` it prints `{"pull_request", "branch",
  "state"}`, with `head` and `draft` for `--status`
  (`crates/sluice/src/lib.rs`, the wait's loop).
- **status and the overview.** `Command::Status` for a branch prints
  `s.status(branch)`'s queue, and `overview::document_held` prints
  `s.overview(...)`. Both read this host's records alone. A request
  another host took in is in neither. A submission this host handed over
  is a withdrawn row under `--all`.
- **The mark.** `PushedRefs::intake_mark(branch)` reads
  `refs/sluice/intake/<branch>` as the host's last fetch of the remote
  brought it: `{"host", "holder": "driver" | "wait", "sluice"}`
  (`crates/sluice/src/profiles/pushed_refs.rs`, `Mark`). Reading it
  makes no fetch.
- **The host's forge state** (`kept.rs`): answers with their ETags,
  pauses, each branch's readings for the taking host's intake, the
  `.moved` touch a submit leaves, the observed mark, and the waits'
  holds. Nothing keeps what a non-taking host last read of a request.

### Looking across

- **crew** waits with `sluice wait <id> --timeout 9m --json` and reads
  `sluice submit --status --json`. Its fix
  `fix--a-landing-in-another-hosts-queue-reads-as-in-the-queue` needs
  to tell a landing queued on another host from one queued here. It gets
  the fields from sluice's JSON and never reads GitHub's check itself.
- **flume**'s merge-queue profile reads `sluice status <base> --all
  --json` for each branch the overview names, taking the newest entry
  submitted from the place's branch (flume
  `crates/flume/src/profiles/merge_queue/sluice/mod.rs`). On a host that
  does not take the base's requests in, it finds no entry and reads the
  request as held. This change's `pull_requests` rows give each request
  the branch it was opened from, `from`, so flume's read can find its
  place's request there. That is flume's change to make, and it is told
  to the orchestrator.
- One owner: the host that takes a request in posts its fields on the
  request's head. Every other host, and every tool that reads sluice,
  reads them through sluice's commands. Nothing reads another host's
  records.

## Goals / Non-Goals

**Goals:** a reader on any host knows, from sluice alone, which host
takes a request in, whether that host's `sluice run` or a wait holds the
mark, and where the request stands in that host's queue, as fields and
as words. A wait, a status and the overview on one host read each
followed request from GitHub at most once in a quarter of the interval
between them.

**Non-Goals:** the branch on its remote (its records show it, 26, 80);
the hand-over's `handed_over_to`, which
`a-hand-overs-withdrawal-holds-no-request` builds; listing a branch's
open requests on a host that does not take them in (137; rule core.33);
the conclusion GitHub keeps on a completed run.

## Decisions

### D1. The check carries the request's queue as fields, in a hidden block in its summary

Every `sluice` check intake posts on a request's head ends its summary
with one HTML comment, which GitHub's Markdown hides:

```
<!-- sluice:queue {"host":"b","taken_in":true,"submission":1406,"state":"queued","ahead":[{"submission":1405,"pull_request":10}],"waits_for":[]} -->
```

- `host`: the host that posts the check, `Peer::name`.
- `taken_in`: whether that host's records hold a submission of the
  request at the head the check is on.
- `submission`, `state`: that submission's number and its state, as its
  record names it (`queued`, `waiting`, `testing`, `passed`, `landing`,
  `landed`, `failed`, `conflict`, `withdrawn`). Both are `null` while
  the request is not taken in.
- `ahead`: the submissions ahead of it in that host's queue and not out
  of it, each with its request's number or `null` for a worktree's, in
  queue order.
- `waits_for`: while not taken in, what it lacks, in the words the
  check's title gives (`1 review`, `qa/approved`, `a draft`, `the next
  submit`, `withdrawn by the operator`). Otherwise it is empty.

The words say the same for a person: the summary gains a first line,
"Submission 1406 on b." or "Waiting to be taken in on b.", and the title
stays as it is. The block is part of the `Posting`'s summary, so
`post_check`'s comparison posts again only when a field changed
(core.32). GitHub replaces a run's output even where it keeps the run
completed (`a-failed-entry-is-submitted-again`, #268), so the fields
move with every post.

A reader takes the last `sluice:queue` block of the summary. A check
with none, as an older sluice posted it, carries no fields.

*Not taken:* the check run's `external_id`, which GitHub defines as the
integrator's own reference for the run and documents no length for. A
second check run named for the queue would be one more post per change
and one more read per follow. The check's `text` holds the report
(ADR 0044) and is cut to fit, and a cut could take the block. A comment
on the request would be a write per change in the conversation.

### D2. The taking host is read from the mark as last fetched, and its queue from the check

`taken_in_by` is `{"host", "holder"}` from `PushedRefs::intake_mark` as
the host's last fetch brought it, with no fetch made for it (129). It
is `null` where no mark stands. `queue` is the check's block on the
request's current head, as D1 gives it, or `null`. Neither is read from
the other host's records, which this host cannot read anyway (C.6).

Where the mark names one host and the check another, as in the pass
between a hand-over and the new holder's first post, both are shown as
they read. The words say the request is submission N on the check's
host and taken in by the mark's host. A request whose check names no
submission, because none is on its head, it carries no block, or
`taken_in` is false, is shown "waiting to be taken in by <mark host>",
with what the block's `waits_for` names where it names anything.

*Not taken:* the host the check names as the taking host. The mark
decides intake (137), and the check can lag a hand-over by a pass. A
fetch at each status, which 129 gives to the tick alone.

### D3. A host keeps the requests it follows, in `forge/follows/`

`forge/follows/<owner>/<repo>/<base>.json`, changed under the `.lock`
beside it as the readings are (`Kept::change_readings`), keeps each
request by number with what was last read of it: when, its head, open,
merged and draft, its title and the branch it is opened from, and the
`sluice` check's posting on that head. A request enters it when:

- a submit on this host opens, updates or marks ready the request, in
  `opening`, beside the `.moved` touch it already leaves;
- this host's hand-over withdraws its submission (`hand_over`);
- a wait or `submit --status` on this host follows it across hosts
  (`follow_across`).

It leaves when a read finds it merged or closed, once the wait that
read it has reported that, or when this host's records take it in at its
head. That makes 137's "every open request that host submitted or
reaches" the host's own submits, its hand-overs and its own asks.

*Not taken:* asking GitHub for the branch's open requests, which a host
that does not take them in never does (137; rule core.33). Reading
every worktree on the host for a request opened from its branch, which
is one `find` per worktree and still misses a hand-over. Keeping the
list in the branch's records, which hold decisions and would change for
reads alone (core.14, core.41).

### D4. One bound for the host: whichever process asks, a request is read once in a quarter of the interval

`follow_across` reads a followed request through the kept file. Where
its last read is younger than a quarter of `[forge] discovery`, every
process uses it. Otherwise the process reads the pull and the check on
its head, each conditional on the answer kept for its address (136,
core.32), and writes what it found. Two waits and a status on one host
read a request together no more often than one wait did. `Pulls::seen`
keeps only the timeline a wait reads for itself.

*Not taken:* the per-process bound kept as it is. Each process reads on
its own clock, and a one-shot status would read at every call.

### D5. status, the overview and a wait show each followed request as the taking host's

A new `PullRequests` method answers, for a branch, the followed
requests this host's queue does not hold at their heads, each read as
D4 allows. The trait's default answers none, so the core names no forge
(core.1, core.2).

- **`sluice status <branch>`** prints, after its queue, `pull requests
  b takes in, by its sluice run:` (or `by a wait on b`, or `no host
  holds main's intake mark`), then a row per request: its number and
  title (26), and either `submission 1406 there · queued behind #10`
  or `waiting to be taken in: needs 1 review`. Under `--json` the
  document gains `taken_in_by` and `pull_requests`, each row
  `{"pull_request", "title", "from", "head", "draft", "state", "queue",
  "read_at"}`, `state` being the check's title as the wait prints it
  today. A row its read found merged or closed is shown once, `landed`
  or `closed without landing`, and then leaves the kept file (D3).
- **`sluice overview`** gives the same `taken_in_by` and
  `pull_requests` on each such branch, and the same rows under it. The
  one-shot overview reads as status does. A reader's document carries
  the file as it stands whenever its modified time moved, and its tick
  reads GitHub for none of it (core.31; rule core.25). `read_at` says
  how old each row is.
- **A wait** across hosts answers `Followed::Open` with `taken_in_by`
  and `queue` beside its title and lines. Its words gain the line `#12
  is submission 1406 on b, taken in by b's sluice run`, printed again
  only when it changes. Its `--json` documents, `--status`'s and the
  one at its timeout, gain both fields.

*Not taken:* a section of its own for the overview's watch to read
GitHub, which core.31 rules out for a reader's tick.

### D6. The fields decide over the words

Where the check carries a block, a host reads the request's state from
it: `taken_in` with `state` `failed` or `conflict` is sent back, and
`landed` is landed. Where it carries none, the host reads the words, as
`follow_across` does today and as `a-failed-entry-is-submitted-again`
proposes. An older sluice's check reads as it did.

## Risks / Trade-offs

- **Record 0093, proposed on agentplot/sluice main at 958c0750,** has no
  host take a branch's requests in and the intake mark go (137 as the
  statement at 7c2fbf0 proposes it). The check's fields (D1), the kept
  follows (D3, D4) and `queue` (D5) are what 0093's 137 keeps: "each
  request it submitted or reaches with the request's entry there, …
  read from sluice's check on the request's head". `taken_in_by`
  (D2) and the mark's words go with the mark once 0093 is accepted and
  built.
- **A summary's block is only as fresh as the taking host's last post.**
  A host whose driver stopped leaves its last check standing. `read_at`
  and the mark's lease (73) tell the reader how much to trust it.
- **The kept file grows only with this host's own requests**, and each
  leaves once merged, closed or taken in here. A request nobody on the
  host asks about again stays until a submit, a status or a wait reads
  it closed. The overview's one-shot read and status read each due row,
  so a closed request leaves at the next of either.
- **Two changes touch `follow_across`:** this one and
  `a-failed-entry-is-submitted-again`. Whichever lands second takes the
  other's reading. Where both are in, the fields decide (D6).

## Migration

Nothing moves. A check an older sluice posted carries no block and reads
as before. A host without `forge/follows/` follows nothing until its
next submit, hand-over or wait adds a request. The overview's schema
stays `sluice/overview@1`, gaining fields only.
