## Why

Clause 120, as amended on boomdrivedev/blueprints main at 84c7a01, says
a withdrawal for a hand-over of intake (137) holds no request.
Whichever host holds the mark takes the request in again as its
conditions hold, at the head it holds, the host that withdrew it
included, and nothing marks it a draft for it. Record 0088 as amended
and rule core.33 say the same. A hand-over is told by a field of the
withdrawal's record, which this change chooses, and never by its
words.

What sluice does at main 526380ae:

- `hand_over` in `crates/sluice/src/profiles/github/pulls.rs` withdraws
  as `Requester::Intake("<host> takes <branch>'s pull requests in")`.
  The record keeps `by` and `at` and nothing else, so only those words
  tell a hand-over apart.
- Intake's `latest_of` and the wait's `handed_over` pass a hand-over by
  through those words alone (`by.ends_with(" takes <branch>'s pull
  requests in")`).
- `left_unlanded` counts every withdrawn submission. `is_held` is
  given what `latest_of` left, so it passes a hand-over by only because
  of the words. The submit's own path reads `s.latest_of` unfiltered.
  After a hand-over, the agent's next `sluice submit` marks its pull
  request a draft and then ready, as it does after a blame, to put a
  ready mark past a leaving that holds nothing.
- `sluice status` shows a hand-over's withdrawal with no word of where
  the request went. A wait that follows it says `withdrawn by b takes
  main's pull requests in`.

boomdrivedev/sluice pull 171 shows where deneb's records stand. #345 was
withdrawn at 16:18:13Z on 2026-10-30 by `mac-studio takes main's pull
requests in`. Deneb took it in again as #347 at 16:23:16Z, by those
words. The merge queue's App drafted it at 16:30:08Z and 16:41:49Z.
Those drafts followed the blames of #347 at 16:29:53Z and of #348 at
16:41:45Z, both on `cli:tools`, as clause 120 has a submit's request
held after a blame. The design describes the case.

## What Changes

- **A hand-over's withdrawal names its host.** The withdrawal's record
  gains `handed_over_to`, the host the intake mark named, and so does
  its log line. Every other withdrawal names none.
- **Every reader reads the field.** Intake's latest submission of a
  request, the hold for a submitter's act (`left_unlanded`, `is_held`),
  who withdrew a hand-opened request, the submit's ready mark and the
  wait's following of a handed-over request all tell a hand-over by
  `handed_over_to`. None of them holds the request, drafts it or reads
  its timeline for it.
- **An older hand-over reads as it was.** A record written by sluice
  0.60.0 to 0.70.2 names no host. It is read as those versions read it,
  by the words they gave every hand-over and no other withdrawal. It
  holds no request either. Any other record with no host named reads as
  before. A request a person closed is still withdrawn and held where it
  was (C.5).
- **status and the wait say where it went.** `sluice status` shows a
  hand-over's withdrawal as `handed over to <host>`. A wait that follows
  its pull request says `#<id> handed over to <host>: its pull request
  #<n> carries its work`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `request-intake-host`: a withdrawal for a hand-over names the host it
  went to and holds no request. Whichever host holds the mark takes it
  in again at the head it holds, the withdrawing host included. A
  record from before reads as it was.
- `pull-request-intake`: the hold for a submitter's act follows every
  withdrawal but a hand-over's.

## Impact

- `crates/sluice-core`: `Withdrawal::handed_over_to` and
  `Event::Withdrawn`'s, written by `Sluice::withdraw` from
  `Requester::HandOver { to, words }`, which replaces
  `Requester::Intake`.
- `crates/sluice`: `pulls.rs` `handed_over`, `left_unlanded`,
  `withdrawn_by_another` and `hand_over`. In `lib.rs`, the wait's
  `carried` and the status row's detail.
- Records: a new optional field, written only by a hand-over. Records
  without it read as they did. `sluice/submission@1` and
  `sluice/log@1` stand.
- Readers outside sluice see the field in `sluice status --json`,
  `status <branch> --all --json` and `wait --json`: crew's `decided_of`,
  flume's merge queue profile and Boomdrive's `entry`. The design's
  "Looking across" says what each reads.
- Docs: record 0088 cites 84c7a01 and names the field. Rule core.33
  cites 84c7a01 and says how an older record is read. The site's pull
  requests guide and state reference, and the plugin's pull requests
  reference, say a hand-over holds nothing.
- Out of scope: crew, flume and Boomdrive reading the field. Each is its
  own repository's change, and the design names it.
