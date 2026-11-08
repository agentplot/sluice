## Why

On deneb at 03:52:46Z on 2026-11-07, crew's landing of
`bolt/sluice-tasks-check-only-what-they-touch` on agentplot/sluice's
`main` printed "sluice exited 4 after its push: #2 withdrawn by
agentplot-ops at 2026-11-07T03:52:10Z", so a landing that succeeded read
as a failure. Signal: `agentplot/crew-state:agentplot/main
signals/2026-11-07-agentplot-ops-d4947942/01-sluice-exits-4-after-its-own-landing`.

deneb keeps two copies of agentplot/sluice, and both took pull request
#204 in. The runner's copy took it as #2 at 03:50:35Z, the person's as
#399 at 03:50:36Z. The person's copy landed #399 at 03:52:03Z as
`ed8474b`, a merge whose second parent is #204's head `daeaf98`. At
03:52:04Z the runner's copy formed `[2]` again on `ed8474b`, and at
03:52:10Z its intake withdrew #2 in its submitter's name. crew's waiting
submit, run by name in the runner's bare clone on sluice 0.68.1, read #2
withdrawn and exited 4. The end it reported was its own submission's,
recorded falsely: #2's work had landed (122).

sluice 0.70.0 already builds what stops that chain: record 0085, rule
core.48 and the change `a-landed-bolt-is-never-reported-withdrawn`. A
round lands work its branch already holds. Intake withdraws no request
whose head the branch holds. A wait reports landed a submission withdrawn
while the branch holds its head. Two things record 0085 left still go
against the unit "A waiting submit and sluice wait report the end of the
very entry the submit made, its landing as landed with exit 0, and never
another entry's end as theirs".

- **A wait can end with another submission's end.** A wait whose
  submission was withdrawn while a later submission, or its request,
  carries its work follows that one, and ends with that one's record and
  exit (record 0085 D3, which named it a risk). A submit's wait can exit
  1 for a later submission's failed check, or print a record whose `id`
  is not the one it submitted, with nothing under `--json` saying so. A
  waiting submit to a branch that takes pull requests reports whichever
  submission of its request was decided last, on whatever head the
  request has by then: a bolt landed onto during its own landing among
  them.
- **A landing can read as a closing.** A waiting submit and `sluice wait
  <branch>` read a closed request against the host's copy of the branch,
  not fetched. When another queue's landing closes the request before
  this host fetches the branch, beside a live driver or in the race
  record 0085 D5 names, the wait says `closed without landing` and exits
  1 for work that landed.

## What Changes

- **A wait reports the submission it was given.** For `sluice wait <id>`
  that is the id. For a waiting submit, the submission the submit made.
  For `sluice wait <branch>`, the worktree's last submission. Its
  document is that submission's record, and its exit is that
  submission's outcome.
- **A carrier is followed, never reported as the wait's own.** Where a
  later submission, or the submission's pull request, carries its work,
  the wait says so and follows it, as now. Once the branch holds the
  submission's head, the wait reports the submission landed, as the
  commit that landed it (`landed_elsewhere`), and exits 0. Once the
  carrier is blamed, in conflict, withdrawn with nothing carrying it on,
  or its request closed, the wait reports the submission withdrawn,
  names how the carrier ended, and exits 4. Out of time, it prints the
  submission's own record and `still waiting: sluice wait <id>`.
- **`carried_by`.** Every document a wait prints after following a
  carrier names it and how it stands: `{"id","state"}` for a submission,
  `{"pull_request","state"}` for a request with none on this host.
- **The wait marks delivered only its own outcome.** The carrier's
  outcome stays pending for its own carrier, the round's notice or
  `sluice inbox` (51).
- **A waiting submit to a branch that takes pull requests reports the
  submission it made.** That is the first submission of its request this
  host's records hold, which its first pass's intake takes in (135).
  From then on the wait waits on it as `sluice wait <id>` does.
- **A closed request is read against the branch as a round reads it.**
  A request read closed, whose submission is not landed, is read against
  the branch as a round reads it, from its remote, before the wait calls
  it closed without landing. Where the branch holds the head, the
  submission is landed, as the commit that landed it, exit 0. A branch
  it cannot read ends the wait as one that could not read, exit 3.
- **The signal's shape is a test.** A waiting submit by name, in a
  repository with no files checked out, whose request another queue
  lands, exits 0 and reports its own submission landed.
- **The decision recorded.** Record 0100, amending record 0085 in part.
  Rule core.48 rules out a wait ending with another submission's end and
  a closing read against the host's copy alone. The site's rules page is
  rewritten, and its `sluice wait` reference with it. The change
  `a-landed-bolt-is-never-reported-withdrawn`, whose tasks are all done,
  is archived first, so the requirements this change amends are in
  `openspec/specs`.

## Out of scope

- **Two copies of one repository on one host, each taking a branch's
  requests in.** That is record 0093's forge queue, which
  `change/pull-requests-are-queued-on-the-forge` builds on the bolt
  `sluice-runs-one-host-process`, and the one-copy design (Boomdrive
  rule core.14). Until it lands, this change keeps each copy's waits
  telling their own submission's true end.
- **A wait that follows sluice's host process.** That is
  `change/a-hosts-queues-move-in-sluices-own-host-process`. The end this
  change reads lives in `carried` and the request's follow, which a wait
  that follows the host process keeps.
- **A request followed across hosts.** Its end is read from sluice's
  check on its head (137; records 0095, 0096), and the forge queue
  rewrites that follow. A request whose head moved after the submit, and
  that another host lands, is reported as that host's check says.
- **A delivery read where no files are checked out.** That is
  `change/a-delivery-read-needs-no-work-tree`, beside this one on the
  bolt.
- **A group submit's waits.** Each reports its submissions as before.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `submission-wait`: a wait reports its own submission's end, following
  what carries its work and naming it in `carried_by`, never another
  submission's end as its own. A waiting submit to a branch that takes
  pull requests reports the submission its request was first taken in
  as. A closed request is read against the branch as a round reads it.

## Impact

- `crates/sluice/src/lib.rs`: `carried`, `wait_on` and
  `await_submission` keep the submission waited on apart from the
  carrier they follow; `Waited` and `Carried` carry both; the report of
  the submission's own end with `carried_by`; `report_delivering` marks
  only it; `submit_pull` keeps the first submission of its request and
  waits on it; `PullMode::Follow` names the submission it follows for.
- `crates/sluice/src/profiles/github/pulls.rs`: `Pulls::follow` reads a
  closed request against `Branch::read`.
- `crates/sluice/src/words.rs`: the words for a carrier that landed, and
  for one that stopped short.
- Tests: `crates/sluice/tests/profiles/worktrunk.rs` and
  `crates/sluice/tests/profiles/pull_requests.rs`.
- Records and docs: record 0100, record 0085's amendment line, rule
  core.48, the site's rules page, and `sluice wait` in the site's
  commands reference.
- JSON: a wait's document gains `carried_by`. Every field it had stays
  (`sluice/submission@1`). crew reads the outcome by exit and `state`,
  which now always describe the submission it waited on.
- Statement: none. 5, 50, 51, 67, 122, 135, 137 and C.5 hold as written.
