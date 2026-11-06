## Why

On 2026-11-06 at 22:31:11Z, `wt submit` refused to submit
WilldanGroup/switchboard-kit's pull request #268 again after sluice sent
it back for a failed check: "wt submit: could not read whether a
submission stands, so nothing was submitted" (sluice 0.68.1). Signals:
`signals/2026-11-06-agentplot-planner-944513a6/01-wt-submit-refuses-a-failed-entry`,
from WilldanGroup/crew-state `wldn/main`
`signals/2026-11-06-wldn-ops-ea539623/01-crew-reads-a-pull-request-sluice-sent-back-as-a-stop.md`.
A new submit takes a blamed entry again (120), and a failed submission
stands at no revision (record 0065), so `wt submit` should have
submitted.

What happened, from GitHub and the code at 0b15d250:

- switchboard-kit's `main` takes pull requests, and another host takes
  them in: its intake mark, `refs/sluice/intake/main`, names
  chuck-sluice-alpha's driver. The worktree's host follows #268 on the
  forge, by GitHub's word and sluice's check on its head (137).
- sluice's check on #268's head `977138a9` is completed, a failure,
  completed at 22:24:38Z, and titled `waiting: the next submit`, its
  summary "Its submission left the queue, and sluice marked it a draft".
  sluice's App marked #268 a draft at 22:26:12Z. sluice posts that held
  state queued (pull-request-status), by patching the run it posted on
  the head; GitHub keeps a completed run completed and takes only its new
  words.
- `wt submit` reads what stands from `sluice submit main --status --json`
  (`standing_in`). On a host that does not take the requests in, that
  reads #268 as sent back, since its check is a failure (`follow_across`),
  and prints what a waiting submit prints at an outcome,
  `{"state":"failed","title":…,"summary":…,"pull_request":268,"branch":"main"}`,
  exiting 1. It carries no `head`, `draft` or `sends`, so `wt submit`
  reads it as no answer and submits nothing (`standing`).

Had it submitted, the submit marks #268 ready, and then reads the same
check, still failed on GitHub and still titled `waiting: the next
submit`, as sent back, and exits 1 at once. Once the other host takes it
in again and posts `queued …` and then `testing …`, GitHub still shows
the run failed, and every wait and status on this host reads #268 sent
back until it lands. On one host none of this happens: `follow` reads
the host's own records, and record 0065's reading holds.

## What Changes

- **A request sent back on another host stands nowhere.** `wt submit`
  reads a pull request whose `state` is `failed` or `conflict` as
  standing at no revision, as it reads a failed submission, and submits
  it again; one `landed` at the revision a submit would send stands
  (record 0065).
- **`--status` answers across hosts as it does on one.** `sluice submit
  <branch> --status --json` on a host that does not take the branch's
  requests in prints a request decided there with its `head`, `draft`
  and `sends`, beside `state`, `title` and `summary`, and exits 0.
- **Across hosts, a request's state is what sluice's check says.** A
  host that does not take the requests in reads the request's state from
  the words of sluice's check on its head, never from the conclusion
  GitHub kept. `failed: …` and `conflict: …` are sent back; `waiting:
  the next submit` is sent back while the request is a draft and, once a
  ready mark ends the draft, waiting to be taken in again by the host
  the mark names; any other words are open.
- **A submit takes a request sent back on another host again, draft or
  not.** Where this host's records hold no entry of the request that
  left the queue, and sluice's check on its head says it was sent back
  or held, the submit marks it ready, as it does on one host (120, C.5).
- **The GitHub double keeps a completed check run completed**, taking
  only its new words, as GitHub did on #268, so the suite meets what
  #268 met.
- Record 0093 decides this; rule core.33 says it.

## Out of scope

- What GitHub shows of sluice's check: a held or retaken request's check
  stays completed and failed on GitHub's page, and to crew, against
  pull-request-status's "SHALL be queued". Posting a new run when sluice's
  check reopens would answer it, for every run sluice posts. A change of
  its own.
- A request merged on another host: `find` reads only this host's
  history once a request leaves GitHub's open list, so `--status` finds
  none. A waiting `wt submit` keeps the worktree after such a landing,
  and a run again tries to open the request again. A change of its own.
- The entry's state as a field of sluice's check (137 as amended at
  agentplot/blueprints 4791adf), which
  `change/a-host-without-the-intake-mark-shows-where-each-request-stands`
  builds. When the check carries it, the across reading reads the field
  in place of the words, in the one function this change adds.
- crew's half, reading a sent-back landing as its team's fix: crew's
  `fix--a-sent-back-landing-reads-as-its-teams-fix`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `worktrunk-submit`: a pull request sent back on another host stands
  nowhere, and `wt submit` submits it again.
- `pull-request-submit`: `--status` across hosts names the request's
  head, draft mark and `sends`; a submit marks ready a request another
  host sent back, draft or not.
- `request-intake-host`: a wait that follows GitHub reads the request's
  state from sluice's check's words, and a held request marked ready
  waits to be taken in again.

## Impact

- `crates/sluice/src/profiles/worktrunk/mod.rs`: `standing`, `Standing`,
  `landed`. `crates/sluice/src/profiles/worktrunk/wt_submit.rs`: its
  words for a landed request.
- `crates/sluice/src/lib.rs`: `Followed::Landed` and `Followed::SentBack`
  carry the request's head and draft mark; `submit_pull` under
  `--status`.
- `crates/sluice/src/profiles/github/pulls.rs`: `follow_across`, and the
  ready mark in `opening`.
- `crates/sluice/src/profiles/github/fake.rs`: `post_named` keeps a
  completed run completed.
- Tests: `crates/sluice/tests/profiles/pull_requests.rs`, the unit tests
  of `worktrunk/mod.rs`, `pulls.rs` and `fake.rs`.
- Site: the pull requests guide, the worktrunk guide and the commands
  reference say how a host follows a request another host sent back.
- Statement: none. 50, 67, 120, 124, 137 and C.5's pull request and
  worktrunk rows hold as written; record 0093 and rule core.33 cite them.
