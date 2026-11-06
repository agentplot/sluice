## Why

On 2026-11-06 the gate box that drives WilldanGroup/switchboard-kit's
queue stopped answering, so main's pull requests were taken in by the
waits of whichever host held main's intake mark (137). At 15:07Z
submissions #1406 to #1408 were withdrawn and pull requests 241 to 243
marked drafts. Signal:
`afterthought/crew-state:swancloud/main
signals/2026-11-06-swancloud-ops-012fa188/01-wldn-gate-box-offline-leaves-switchboard-kit-intake-to-waits`.

Where the records and the mark are:

- switchboard-kit's policy at `origin/main` names no signers, so each
  host keeps its own records of `main` (C.6 local files). The host that
  took main's requests in that day is chuck-herdr-alpha, by crew's
  waits: its records are
  `/workspace/switchboard-kit/switchboard-kit.git/sluice/main/` there.
  deneb runs no sluice on switchboard-kit, so its clone holds no
  `sluice/` and no records of it.
- The mark is `refs/sluice/intake/main` on GitHub. A wait pushes it
  and lets it go when it ends, so between a host's waits none stands,
  and an `ls-remote` made then lists no `refs/sluice/*`. At 18:15:04Z
  chuck-herdr-alpha's wait pushed it again, `54dad0a8`,
  `{"host":"chuck-herdr-alpha","holder":"wait","sluice":"0.68.1"}`,
  which `git ls-remote origin 'refs/sluice/*'` on deneb lists after.

What chuck-herdr-alpha's log says:

- 14:56:23 to 14:58:47, requests 241, 242 and 243 taken in as #1406,
  #1407 and #1408; 14:59:31, `[1405, 1406, 1407]` under test.
- 15:07:04, #1406, #1407 and #1408 `withdrawn`, each `by: another host
  takes main's pull requests in`.
- 15:12:12 to 15:12:18, the three taken in again as #1411 to #1413;
  their combination tested again; landed at 15:55:09.

No host but chuck-herdr-alpha took main's requests in, and it withdrew
its own submissions, not another host's: each host's records are its
own, and no host writes another's. "another host" is sluice's words
where a push of the mark, refused at its lease, is followed by a read
finding no mark (`put_mark` in
`crates/sluice/src/profiles/github/pulls.rs`). crew runs one `sluice
wait <id> --timeout 9m` per landing, again while it times out, so
several waits of the one host read main's requests at once. Each held
the mark as its own: one ending let it go between another's fetch of it
and that one's renewal; the renewal was refused at its lease; the read
found none; and the host handed everything it had taken in over to no
host.

The drafts were not sluice's: GitHub's timelines show them made at
15:07:16 to 15:07:18 by the person's own account, where every draft
sluice makes reads `sluice-for-willdan[bot]`. crew's landing drafts a
bolt's request when its landing did not land (`plan.py` `pull_hold`,
`gh pr ready --undo`), and sluice 0.68.1's wait read the hand-over's
withdrawal as withdrawn, exit 4. Since 0.70.0 a wait follows a request
its host handed over (record 0085), so no hand-over is reported
withdrawn.

Against 137, a host withdraws its own submissions of the requests on
finding the mark another host's, naming the host that holds it: this
host found none. Against 137 again, a branch one host reaches by waits
keeps no mark between them; here it kept none while its host still
waited. And against core.48, a host that finds another's mark withdraws
a submission whose head the branch holds, as a gate host coming back
after another host landed its requests would.

## What Changes

- **A refused push of the mark is decided again.** A push of the mark
  refused at its lease is decided once more on the mark the branch's
  fetch then brings: this host's own is held; none standing is pushed
  again leased on its absence; another host's is that host's, which
  this host hands over to. A second refusal leaves the pass taking
  nothing in and handing nothing over. No hand-over names a host the
  mark does not name.
- **Work the branch holds is never handed over.** A host that finds the
  mark another host's withdraws none of its submissions whose head the
  branch, as the pass fetched it, holds; the round lands each as the
  commit that landed it (122; rule core.48).
- **A host's waits hold its mark together.** Each wait that reads a
  branch's requests keeps a hold among its host's waits from its first
  pass, a file of the host's state locked while it lives. A wait that
  ends removes the mark only where no other wait of its host still
  holds one, so the last of them lets it go, whether or not it pushed
  it. The amendment that says so in 137 is proposed on agentplot/blueprints.

What the new holder does with requests already in another host's
queue: nothing of that host's. It takes each open request in as its
conditions hold (137). The other host withdraws its own submissions of
them at its next pass that finds the mark, naming the holder, none
landing and none whose head the branch holds. A host that is gone
advances none of its submissions: they wait in its own files until it
returns and settles them so.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `request-intake-host`: a refused push of the mark is decided again on
  the mark the fetch brings; a host's waits hold its mark together, the
  last to end letting it go; a hand-over passes by work the branch
  holds.

## Impact

- `crates/sluice`: `Pulls::takes`, `decide` and `put_mark`, and
  `Taking::Undecided`; `Pulls::hold` and `let_go_marks`; `hand_over`
  reading the branch at the pass's revision; `Kept::hold` and
  `Kept::let_go`, the holds under
  `forge/intake/<owner>/<repo>/<base>.waits/` in the host's state.
- Records: none change. "another host takes <branch>'s pull requests
  in" is no longer written; a record holding it reads as before.
- Statement: 137's "a wait that holds it lets it go when it ends"
  gains "and a host's waits hold it together: the last of them to end
  lets it go", on agentplot/blueprints's branch
  `design/sluice-a-hosts-waits-hold-its-intake-mark-together`, unpushed,
  for the user's yes (record 0089). 45, 122 and C.5 hold as written.
- Records 0088 and 0089; rule core.33 and its details.
- Site: the pull requests guide's "One host takes the pull requests
  in", and the holds and the kept mark in the state reference's
  `forge/`.
