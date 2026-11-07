# Design

## Context

The rule: clauses 120 and 137 as amended on agentplot/blueprints main
at 84c7a01, record 0088 as amended and rule core.33. A withdrawal for a
hand-over holds no request. Whichever host holds the mark takes it in
again at the head it holds, as its conditions hold, the host that
withdrew it included. Nothing marks it a draft for it. A field of the
withdrawal's record tells a hand-over, never its words.

The hand-over this builds on: records 0085 and 0089, and the change
`a-host-hands-over-only-to-the-host-its-mark-names` (#388). A host that
finds the mark another's withdraws its own submissions of the requests,
none landing and none whose head the branch holds. A wait on one
follows its request on GitHub.

Each reader of a withdrawal at main 526380ae:

| Reader | Where | Reads a hand-over by |
|---|---|---|
| intake's latest submission of each open request | `Pulls::latest_of` | its words |
| the hold for a submitter's act | `left_unlanded`, `is_held` | `latest_of`, so its words |
| who withdrew a hand-opened request | `withdrawn_by_another` | `latest_of`, so its words |
| a submit's ready mark after a leaving | `submit`'s `left` | nothing: every withdrawal is a leaving |
| a wait on a withdrawn submission | `lib.rs` `carried`, `PullRequests::handed_over` | its words |
| `sluice status` | `detail_of_state` | nothing: a withdrawal shows no detail |
| `sluice inbox`, outcome delivery | `Submission::decided` | nothing needed: no withdrawal is an outcome (51) |
| `sluice overview --json` | its branches' submissions in a chain, recent landings, blames, conflicts | nothing needed: it lists no withdrawal |
| `status --json`, `status <branch> --all --json`, `wait --json` | the record whole | whatever the record holds |

What agentplot/sluice pull 171's records on deneb say about 2026-10-30.
`sluice/main/log.jsonl` and `.finished/345.json`, `347.json` and
`348.json` in `~/Code/github_agentplot/sluice/sluice.git`:

- 16:09:36, #345 submitted at `a006e071`, from `pull/171`, its agent
  madswan-ops.
- 16:18:13, #345 `withdrawn` by `mac-studio takes main's pull requests
  in`, deneb's hand-over to mac-studio.
- 16:23:16, #347 submitted at `a006e071`. Deneb holds the mark again
  and takes the request in again. `latest_of` passed #345 by on its
  words.
- 16:29:53, #347 `blamed`, `cli:tools` failed. 16:30:08, the merge
  queue's App marks pull 171 a draft: the hold after a blame (120,
  S56).
- 16:31:05, #348 submitted on a person's ready mark. 16:41:45, blamed
  on `cli:tools`. 16:41:49, a draft again.
- 2026-10-31 00:33:13, #357, the next submit's, which lands.

The hand-over withdrew #345 and held nothing. The drafts were the hold
after two blames. The 2026-10-30 case is the test "The mark comes
back".

### Looking across

The hand-over and its withdrawal are sluice's. The other tools of the
family read sluice's records of a submission and take `withdrawn` as
final:

- **crew** (`plugin/lib/gather.py` `decided_of`, from `sluice status
  <branch> --all --json`; `plugin/lib/plan.py` `EXITS`, from `sluice
  wait`). A wait follows a hand-over (record 0085), so crew's landing
  is never told it was withdrawn. Its history read sees `withdrawn`
  with `told` from `withdrawn.at`.
- **flume** (`crates/flume/src/profiles/merge_queue/sluice/mod.rs`
  `newest_read`, from `sluice status <base> --all --json`). It takes the
  newest submission whose `from.head` is the place's branch, a pull
  request's included, and maps `withdrawn` to its final
  `SubmissionState::Withdrawn`.
- **Boomdrive** (`crates/boomdrive/src/sluice.rs` `FINAL`, from `sluice
  status <id> --json`). It reads `withdrawn` as a state after which the
  entry no longer changes.

None of them can tell a hand-over today. With this change each can, by
`withdrawn.handed_over_to` in the record it already reads. sluice owns
the reading: its wait follows the request where the work went (record
0085), and the others follow that answer or read the field. The
changes in crew, flume and Boomdrive are theirs. This change only gives
the field, and the orchestrator is told.

## Goals / Non-Goals

**Goals:** a hand-over's withdrawal is told by a field of its record.
Every reader of a withdrawal in sluice reads it. None of them holds a
request for it, drafts it or asks GitHub for its timeline. A record
from before reads as it was.

**Non-goals:** crew, flume and Boomdrive reading the field. Archiving
the changes #386 and #388 left in `openspec/changes/`.

## Decisions

### D1. The withdrawal's record names the host it handed over to

`Withdrawal` gains `handed_over_to: Option<String>`, the host the
intake mark named, and `Event::Withdrawn` gains the same. Both are
skipped when `None`, so every other withdrawal, and every record
before, is written and read unchanged. `Requester::HandOver { to,
words }` replaces `Requester::Intake(words)`. `Sluice::withdraw` writes
`to` into both, and `words` into `by` as before, so the log still says
in words which host takes the requests in.

Every reader tells a hand-over by `handed_over(e)`. That reader holds
for a submission withdrawn whose record names a host:

- `Pulls::latest_of` passes it by, so intake reads its request as one
  it has not taken, which it takes at the head the request holds once
  its conditions hold;
- `left_unlanded` is false for it, so neither `is_held` nor the
  submit's `left` reads it as a leaving: no hold, no draft, no
  timeline read, no draft-and-ready at the next submit;
- `withdrawn_by_another` names nobody for it, so a hand-opened request
  never rests on it;
- the wait's `PullRequests::handed_over` follows its request (record
  0085).

`sluice status` shows its row's detail as `handed over to <host>`. The
wait that follows it says `#<id> handed over to <host>: its pull
request #<n> carries its work, so this wait follows it`.

The name says what it is in the words the site already uses ("hands
its pull requests over"). The value is the host's name among its
peers, as the mark and the log give it.

Options weighed:

- a `for: "hand-over"` marker with no host, which names nothing a
  reader can follow;
- a new state, which `sluice/submission@1` rules out, since its nine
  states are stable;
- the log line alone, which the record's readers do not read.

### D2. A record from before reads as it was

sluice 0.60.0 to 0.70.2 wrote a hand-over's withdrawal with no host
named. Its `by` holds the words those versions gave every hand-over and
no other withdrawal: `<host> takes <branch>'s pull requests in`, or
`another host takes ...` before 0.70.1. Those versions read such a
record as a hand-over by those words, and so does this one, in
`named_no_host`, only where the record names no host. A record with no
host named whose `by` is anything else is any other withdrawal, read as
before: held, and let go by a ready mark (120, S56).

This keeps 120 for those records: a withdrawal for a hand-over holds no
request, whenever it was written. It keeps core.48 too: a wait on such
a submission follows the request that carries its work. Reading such a
record as any other withdrawal would hold and draft a request that a
host of 0.60.0 to 0.70.2 handed over, once the mark came back to it.
Rule core.33 rules out telling a hand-over by its words. This change
keeps that for every record that can carry the field. Words are read
only in a record written before the field existed. Rule core.33's
details say so.

Options weighed: every record with no host named read as any other
withdrawal. It would hold the requests the hosts on 0.68.1 and 0.70.x
handed over on 2026-11-06 (switchboard-kit's 241 to 243, 268 and 269),
and a wait on them would exit 4.

### D3. A request closed is still withdrawn

A request a person closes leaves the queue as before: intake withdraws
its submission as its agent, naming no host (C.5's pull request row,
S24). That withdrawal holds the request: a submit's request opened again
waits for its submitter's act as a draft. This holds after a hand-over
and its return, since the closing is written by whichever host held the
request then.

## Risks / Trade-offs

- **A record with no host named whose `by` ends in the words of a
  hand-over** is read as one. Only an older hand-over writes such words
  (D2). `sluice withdraw --as` names a session's agent, and an agent
  named so would be one no host gives.
- **A repository written by an older sluice and this one in turn**, as
  when a host's own waits run two versions, holds hand-overs with no
  host named beside ones that name it. This version reads both as
  hand-overs (D2).
- **The field shows in every JSON record of a hand-over's withdrawal**
  → optional, as `beside` and `covered_by` are, and skipped where
  absent.
- **Record 0093, proposed on 2026-11-06,** has no host take a branch's
  requests in, so no hand-over is made once it is accepted and built.
  Until then record 0088 as amended governs, and this builds it. Once
  that design is built, the hand-overs the records already hold are
  still read by this field, or by their words where none is named, as
  rule core.33 keeps.
