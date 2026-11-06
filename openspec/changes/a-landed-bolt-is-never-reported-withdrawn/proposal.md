## Why

On deneb, since 2026-11-05, every crew bolt landing on swancloud's
`main` lands and is pushed, and then sluice exits 4 calling the landing
withdrawn, after saying it could not read whether the submission was
delivered because `git status` ran in a bare repository ("this operation
must be run in a work tree"). Signal:
`afterthought/crew-state:swancloud/main
signals/2026-11-06-swancloud-ops-41b81e17/01-sluice-reports-a-landed-bolt-withdrawn`.

deneb keeps two copies of swancloud, the person's
`~/Code/github_afterthought/swancloud/swancloud.git` and the runner's
`~/.local/share/flywheel/repositories/afterthought/swancloud/swancloud.git`,
each with its own records and queue of GitHub's `main`, and each reads
`main`'s intake mark as its own, since the mark names the host (137,
rule core.33). Both took pull request #330 in: the person's copy as
#425, the runner's as #20. The records show what happened:

- 12:39:12, the person's copy takes #330 in as #425; 13:01:05 its
  combination passes; 13:01:08 it lands as `562606d`, a merge whose
  second parent is #330's head `68a1b49a`, pushed to GitHub, which then
  calls #330 merged.
- 12:38:39, the runner's copy takes #330 in as #20. At 13:01:10 its
  round reads `main` moved to `562606d`, by a landing not its own, and
  forms `[20]` on it again: a combination of work the branch already
  holds, which it would have tested and landed as an empty landing
  commit.
- 13:01:33, the runner's intake reads #330 gone from GitHub's open
  requests and withdraws #20 in its submitter's name, `by:
  swancloud-ops`, the request's agent: "A submitted pull request closed,
  back in draft or on another head leaves the queue"
  (`crates/sluice/src/profiles/github/pulls.rs`, intake's pass, with
  `Requester::Session(sub.from.agent)`). #19 went the same way at
  12:55:18, after the person's copy landed #329.
- crew's waiter, `sluice wait 20` in the kit's main checkout, which on
  deneb is the runner's bare clone, reads #20 withdrawn and exits 4. Its
  delivery read asks the worktrunk driver for the session working
  there, which runs `git status` in the bare repository and fails.

So sluice wrote "withdrawn by swancloud-ops" for work its branch held,
against 122 (the outcome is landed once the branch moved to a revision
holding the work, and nothing after the move changes it) and 5 (sluice
never drops an entry on its own, and its submitter withdrew nothing).
It formed work the branch held as a new combination (6, 22). Its wait
reported the withdrawal though the work landed (50, 51). Its driver ran
`git status` where no work tree is (2).

## What Changes

- **A round lands work the branch already holds.** Before it forms
  anything, a round reads each submission in the queue that no
  combination formed on the branch's revision holds; one whose head
  that revision holds is landed as the commit that landed it there, the
  oldest commit of the branch's first-parent line holding it, with
  `landed_elsewhere` on its record and a `found_landed` line in the log.
  Its outcome is delivered as any landing's. Nothing of it is formed,
  run or landed, and a combination under test that holds it is dropped,
  its checkout released.
- **Intake withdraws no request whose work the branch holds.** A
  submitted pull request gone from the forge's open requests, while the
  branch as that pass fetched it holds its head, has landed: it is not
  withdrawn, and the round lands it. One closed, or merged by hand
  without its head reaching the branch, is withdrawn as before.
- **A wait whose submission's work goes on is never told it was
  withdrawn.** A wait, a waiting submit and `sluice wait <id>` reading
  their submission withdrawn report it landed, as the commit that landed
  it, where the branch holds its head; follow a later submission from
  its place whose head holds it, saying so; and, for a pull request's
  submission, follow the request while it is open and ready on a head
  that holds the work, taken in again on this host or on another (67,
  137). A withdrawal no work goes on from is reported withdrawn and
  exits 4 as before.
- **A repository with no work tree is read with no `git status`.** The
  worktrunk driver reads a bare repository or a checkout's git directory
  as holding no uncommitted work, with no path, and runs no `git status`
  there. A delivery read there, by a wait, a waiting submit or `sluice
  inbox`, delivers nothing and says plainly why: no session works in a
  repository with no work tree, and the outcome stays pending for its
  session's `sluice inbox`.
- **Two copies of one repository on one host.** The design says what
  sluice should do when a second copy reads the intake mark as its own,
  with the readings, and names what waits for the one-copy design
  (Boomdrive rule core.14). This change builds what keeps each copy's
  own records true when the other lands.

## Out of scope

- One copy of a repository per host, and what a second copy does with
  the intake mark: the user's design, Boomdrive rule core.14. Until it
  lands, two copies still both take a branch's requests in, both test
  them, and both push a notice of one landing to its session, each under
  its own submission's number.
- A landing pushed by another copy between one pass's fetch and its list
  of open requests: that pass reads the request closed and withdraws it.
  A wait on it still answers landed. With one copy taking the requests
  in, nothing but its own queue lands them.
- A submission whose head another submission of this queue carried in
  a combination formed before it landed, found again only when a round
  forms it next.

## Capabilities

### New Capabilities

- `landed-elsewhere`: a submission whose work its branch already holds
  is landed as the commit that landed it, never formed, run, landed
  again or withdrawn.

### Modified Capabilities

- `submission-wait`: a wait whose submission's work goes on is never
  told it was withdrawn.
- `pull-request-intake`: a request whose head the branch holds leaves
  the queue landed, not withdrawn.
- `request-intake-host`: a wait on a submission handed over follows its
  request to the host that takes it in.
- `outcome-delivery`: a delivery read where no work tree is says so and
  delivers nothing.

## Impact

- `crates/sluice-seams`: `Branch::landed_by`; `Session::bare`.
- `crates/sluice-core`: `Submission::landed_elsewhere`,
  `Event::FoundLanded`, the round's `land_found`.
- `crates/sluice`: `LocalRefs::landed_by` (`git rev-list --first-parent
  --ancestry-path`); the worktrunk driver's session read; intake's
  closing of requests; the wait's `carried`, `PullMode::Follow`,
  `PullRequests::request_of`; the delivery read's `Recipient`.
- Records: `landed_elsewhere` on a submission and `found_landed` in the
  log are new; a record written before reads as it was.
- Statement: none. 2, 5, 6, 22, 50, 51, 67, 122, 137 and C.5 hold as
  written; record 0085 and rule core.48 cite them.
