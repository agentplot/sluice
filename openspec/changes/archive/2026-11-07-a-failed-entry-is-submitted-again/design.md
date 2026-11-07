## Context

`wt submit <branch>` reads what stands of its worktree's last submission
before it submits: `sluice submit <branch> --status --json --worktree
<w>` (`standing_in`), read by `standing` in
`crates/sluice/src/profiles/worktrunk/mod.rs`. Record 0065 has a
submission stand when it is undecided or landed, at the revision a submit
from the worktree would send now, the document's `sends` (124, rule
core.27). A failed, conflicting or withdrawn one stands nowhere, and the
worktree submits again (67, 120). An answer `standing` cannot read is no
answer: the run submits nothing and exits 3, since a submit at the
revision standing would withdraw it (worktrunk-submit, "A run that cannot
read what stands submits nothing").

On a branch that takes pull requests, `--status` is `submit_pull` in
`PullMode::Status` (`crates/sluice/src/lib.rs`). It finds the worktree's
request (`Pulls::find`) and reads it with `Pulls::follow`. On the host
that takes the branch's requests in, `follow` reads the host's records:
a submission decided on the request's head is `Followed::Done`, printed
as its record with `sends` and exit 0. On any other host (137), `follow`
is `follow_across`: GitHub's word on the request and sluice's check on
its head. Merged is `Followed::Landed`, closed is `Followed::Closed`, and
open with sluice's check a failure is `Followed::SentBack`. `submit_pull`
answers `Landed` and `SentBack` by `across` in every mode, `--status`
included. That is a waiting submit's outcome, `{"state","title",
"summary","pull_request","branch"}` with its exit, 0, 1 or 2, and no
`head`, `draft` or `sends`. `standing` finds no `sends` and returns its
words as an error.

The check `follow_across` reads is one sluice posts and patches as the
request's state changes (`post_named` in
`crates/sluice/src/profiles/github/rest.rs`, a `PATCH` of the run it
posted). `posting_json` sends `status: queued` or `in_progress` for an
undecided state, and `completed` with a conclusion for a decided one.
GitHub keeps a completed run completed. switchboard-kit #268's check on
`977138a9` reads completed, a failure, completed at 22:24:38Z, titled
`waiting: the next submit`, which sluice posts queued
(`waiting` in `pulls.rs`; pull-request-status, "A pull request held for
its submitter says so"). So once sluice's check on a head has said a
request failed, GitHub says failed on that head for good, whatever sluice
posts after. The forge double, `Fake::post_named`, behind the GitHub
double of `crates/sluice/tests/profiles/forge/github.rs`, replaces a
run's status on a `PATCH`, so no test met this.

## Goals / Non-Goals

**Goals:**

- `wt submit` run again in a worktree whose pull request another host
  sent back submits it again, as it does where its host takes the
  requests in.
- Any host reads a request another host takes in as what sluice's check
  says of it: sent back, held for the next submit, waiting to be taken in
  again, open, or landed. Its submit, wait and status agree on it.
- A submit from any host takes a sent-back request again, draft or not.

**Non-Goals:**

- What GitHub's page, crew or any other reader shows of sluice's check.
- A request merged on another host found by `--status` (`Pulls::find`).
- The entry's state, number and host as fields of sluice's check (137 as
  amended at agentplot/blueprints 4791adf).

## Decisions

### D1. A request sent back stands nowhere, and one landed stands

`standing` reads a pull request document (`pull_request`, no `id`) by its
`state`. `closed`, `failed` and `conflict` stand at no revision. `landed`
stands where `head` is `sends`. Any other state is open, and stands where
`head` is `sends` and `draft` is false, as now. A document with no
`sends` stays no answer.

`Standing::PullRequest` becomes `PullRequest { number, landed }`.
`landed` (the removal's reading) is true for a landed pull request as for
a landed submission. `wt submit` says `following pull request #<n>,
landed at this revision` for a landed one, and `already open` for the
rest, as now.

- *Read any document without `sends` as standing nowhere.* An answer
  sluice did not give would then submit, and withdraw what stands (67).
- *Read the state without `sends`.* The driver never decides the revision
  itself (rule core.27, record 0065), and one document shape for every
  host keeps one reading.

### D2. `--status` across hosts answers as it does on one

In `PullMode::Status`, `Landed` and `SentBack` print the request with
`state` `landed`, `failed` or `conflict`, `title` and `summary` from
sluice's check, `head` and `draft` as GitHub holds them, and `sends` (with
`copy` where the host's copy differs, as now). The run exits 0, since
`--status` exits 0 wherever it finds the worktree's request and 1 only
for none, or closed. The words form prints the check's title and summary
as `across_lines` does, and exits 0 too. A wait and a waiting submit keep
`across` and its exits.

`Followed::Landed` and `Followed::SentBack` carry `head: String` and
`draft: bool`, read from the pull request `follow_across` read.

- *Keep the outcome's exit under `--status`.* `standing` would then read
  exits as outcomes on one host and as answers on the other, and
  `--status` is a reading, not a wait.

### D3. Across hosts, a request's state is what sluice's check says

`follow_across` reads an open request's state from the title of sluice's
check on its head and the request's draft mark, never from the conclusion
GitHub kept. One function, beside `follow_across`, reads them:

| sluice's check says | the request | read as |
|---|---|---|
| `failed: …` | draft or not | `SentBack`, not a conflict |
| `conflict: …` | draft or not | `SentBack`, a conflict |
| `waiting: the next submit` | a draft | `SentBack`, as its check says |
| `waiting: the next submit` | out of draft | `Open`, `waiting to be taken in by <host>` |
| anything else, or no check | draft or not | `Open`, as the check says, or `waiting` |

`<host>` is the host the branch's intake mark names, which `away` holds
for the branch. Merged is `Landed` and closed is `Closed`, before any of
this, as now.

The held row follows C.5 and 120. sluice marks a request a submit opened
a draft when its submission leaves the queue, and only a ready mark ends
that draft, the next submit's or a person's. On the host that takes it
in, `is_held` reads the same: a draft is held with no read, and a ready
mark since its leaving takes it again. So a held request out of draft has
had its submitter's act and waits for intake there. A wait on it goes on,
and `--status` shows it open and ready at its head, so `wt submit`
follows it rather than marking it again.

- *GitHub's conclusion, as today.* A held request reads sent back after a
  submit marked it ready, and a request taken in again reads sent back
  until it lands, since GitHub never reopens the run.
- *The request's ready marks against when the check completed.* GitHub's
  `completed_at` on #268 is the first failure's, so a request blamed
  twice on one head would read as taken again after its second blame, and
  the reading costs a timeline read per pass.
- *A new check run whenever sluice's check reopens, so GitHub's
  conclusion is right again.* That changes what GitHub's page and every
  other reader sees, for every run sluice posts, and wants a design of
  its own: which run a reader takes, and the per-check runs. A held
  request is queued by design, so it would still be read by its words and
  draft mark. Out of scope.
- *Read the fields 137 gives the check.* The check carries none yet. When
  `change/a-host-without-the-intake-mark-shows-where-each-request-stands`
  makes it carry them, this function reads them in place of the title,
  and no other reading changes.

### D4. A submit marks ready a request another host sent back, draft or not

`opening` marks an existing request ready today when it is a draft, or
when this host's records hold its latest submission left the queue
(`left`). A host that does not take the requests in holds no such entry,
so a sent-back request that is not a draft stays held. That is a request
opened by hand, or one sent back before the other host's pass marks it a
draft. So where `left` is none and the request is open and not a draft,
the submit reads sluice's check on the request's head before it pushes.
Where the check says `failed: …`, `conflict: …` or `waiting: the next
submit` (D3's sent-back and held rows, draft or not), it marks the
request ready, a draft first, as `mark_ready` does with no `left`. One
read, on the submitter's own act, of the request it acts on (rule
core.32).

- *Only drafts, as today.* A new submit takes a blamed entry again (120),
  whichever host takes it in.
- *Mark every existing request ready.* A draft mark withdraws a request
  the other host has queued or under test (C.5).

### D5. The GitHub double keeps a completed run completed

`Fake::post_named`, given an existing run that is completed (`Failure` or
`Success`) and a posting that is `Queued` or `InProgress`, keeps the
run's status and takes the posting's title, summary and text, as GitHub
did on #268. A completed posting replaces the run whole, as now: #268
shows what GitHub does with an undecided posting on a completed run, and
nothing here reads a decided one's conclusion across hosts. Tests that
read a reopened check's status read its title instead, as any host reads
it now (D3).

## Risks / Trade-offs

- The across reading rests on sluice's own titles: `failed: `,
  `conflict: ` and `waiting: the next submit`. They are sluice's words on
  its own check, and the unit test of D3's function holds them to the
  postings `pulls.rs` makes. The fields of 137 replace them.
- A submit reads a stale check (D4): the other host took the request in
  again and has not yet posted, and the submit's draft mark withdraws
  that entry. The other host takes it in again at the ready mark, a run
  lost.
- A ready mark GitHub stamps before the second the other host removed the
  entry, with the clocks apart by more than the time since, leaves the
  request held. `mark_ready` checks that only where `left` is known. The
  next submit marks it again.
- A held request a person marked a draft again after a submit's ready
  mark reads sent back across hosts. On the taking host it is held too.

## Migration Plan

None. The `--status` document gains fields and exits 0 where it exited 1
or 2, only on a host that does not take the branch's requests in, and
only for a request decided there. `wt-submit` ships with the sluice that
prints it.

## Open Questions

None.
