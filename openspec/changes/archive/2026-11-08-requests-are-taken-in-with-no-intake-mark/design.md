# Design

## Context

The rule. Record 0093 as amended on 2026-11-08, its "No intake mark"
and points 3 to 5 of the amendment: whatever executes a branch's queue
takes its requests in, nothing does one part of that while another does
the rest, no request is taken in twice, and the mark, its renewal, its
lease and its hand-overs go. 137 at agentplot/blueprints 5f05fc6 says
where: only where the branch's records are on its remote, by the
instance that keeps the branch there (72), and a pull request table on a
branch whose records each host keeps is refused for that branch alone,
naming `[signers]`, "so no request is opened where nothing would take it
in". 120, 140 and C.5's pull request row say the same of who takes a
request in, lands one beside the chain and finds a request opened by
hand. Rule core.33 rules out the mark, its renewal, its lease, a
hand-over and a withdrawal for one; rule core.28 a read of a remote
reported only to the process that met it; rules core.11 and core.14 a
decision kept in a process's memory.

What the bolt holds at 752db9a9. #29 took the forge's round job out
whole (record 0093's amendment, point 4) and put intake back as
`a-hosts-queues-move-in-sluices-own-host-process` built it beside the
mark:

| What | Where | What it does |
|---|---|---|
| who takes in | `Pulls::takes` | a branch whose records are on its remote: `Sluice::keeps`, the lead (72, 73); any other: the mark |
| the mark | `pushed_refs.rs` `intake_ref`, `intake_mark`, `put_intake_mark`, `drop_intake_mark`, the refspec in `PushedRefs::read` | `refs/sluice/intake/<branch>`, `{"host","holder","sluice"}`, fetched to `refs/sluice-fetched/<remote>/intake/<branch>` |
| its lease and renewal | `Pulls::decide`, `put_mark`; `Kept::observed`, `observe` | `forge/intake/<owner>/<repo>/<base>.json` |
| the waits' holds | `Pulls::hold`, `let_go_marks`; `Kept::hold`, `let_go` | `forge/intake/.../<base>.waits/<pid>.hold`; no live path calls them, since no wait runs intake (135) |
| the takers | `lib.rs` `Taker` | a pass is `Driver`, a step and a submit's own first intake `Step`; nothing passes `Wait` |
| a hand-over | `Pulls::hand_over`, `Requester::HandOver` | withdraws the host's request submissions, `withdrawn.handed_over_to` naming the mark's host |
| who takes in, shown | `TakenInBy`, `Holder`; `Across`, `Followed::Open`, `There` | `taken_in_by` `{"host","holder"}` in `status`, `overview`, `wait` and `--status` |
| the rows of a host that does not take in | `PullRequests::across`, `kept_across`, `forge/follows/` | only a branch whose records each host keeps |
| intake's warnings | `telemetry::warn` in `take_in`, `across`; `crate::intake`'s `thread_local` `FAILED` | the host process's stderr; said once per process |

Who reads what intake meets today. A pass runs in the host process,
whose stderr is its log. `sluice step` asks the host process for its
round (rule core.14; record 0093's build, 3) and prints the answer's
`said`, which holds what intake wrote on its `out`, the spent
allowance's line alone. A submit that does not wait runs its own first
intake in its own process, whose stderr is its caller's. So a step's
caller never sees why a request was not taken, a pass says a standing
failure once for each start of the host process, and the branch's log
holds nothing of it unless the round's fetch was refused too
(`a_repository_github_no_longer_finds_is_said_once_with_what_to_do`).

Where the family stands. Flume, Boomdrive, weir and timber keep no
mark of their own on a remote: the intake mark was sluice's alone, so
nothing is reconciled across the tools for it. crew reads `taken_in_by`
from a wait's document (record 0095's follow-up) and drafts a landing's
request when its wait exits 4; neither reading changes but for the
`holder` that goes. Flume reads a submission's state from `sluice
status <base> --all --json` and takes `withdrawn` as final; it never
read the mark. Rule core.28's refusal of a remote, kept on the host and
logged once per cause and host, is sluice's alone too, and this change
gives intake that one design rather than a second.

## Goals / Non-Goals

**Goals:** no mark kept, read or written anywhere; the instance that
keeps a branch on its remote takes its requests in, and no other; a
pull request table on a branch whose records each host keeps refuses
that branch alone; no hand-over written, an older one read as written;
a host that does not keep the branch shows the requests it follows; a
step prints every warning its round met; a standing intake failure
logged and said once for each cause and host from what the records seam
keeps; the suite and the unfinished changes built on the mark rewritten
or taken out.

**Non-goals:** a machine whose signature the branch does not accept
leading none of its rounds (137 as amended at 5f05fc6, 78), which
`keeps` does not ask today and which is its own unit from record
0093's amendment; `sluice daemon`; the GitHub Action way, which the
change `actions-pool` builds and whose workflow the refusal names once
it exists; the pass's words for a branch it passes by, kept per host
process as the host process's build chose (record 0093's build, 7);
the spent allowance's line, which is the host's pause (134); moving
sluice's own `main` or WilldanGroup/switchboard-kit's onto signers.

## Decisions

### D1. The instance that keeps the branch takes its requests in

`Pulls::takes(s, t, branch)` answers `Here` where the branch's records
are on its remote and `Sluice::keeps(branch)` is true, and otherwise
`Elsewhere` with the host the branch's records name as its lead, which
`keeps` read, or none where none leads yet. Any process of that host
takes them in under intake's lock, a pass, a step and a submit's own
first intake alike, so the records' one lead and intake's one lock make
one taker (72; rule core.14). `Taker` goes, and `takes` no longer reads
a mark, decides twice or answers `Undecided`. A host that does not keep
the branch asks GitHub nothing of its open requests, withdraws nothing,
and follows its own requests by GitHub's word and sluice's check on
their heads, as it does today for such a branch (137; records 0095,
0096). `Sluice::keeper(branch)`, or `keeps` returning the lead it read,
gives the name; the build chooses which.

Options weighed:

- every host's process taking in a branch whose records each host
  keeps, its own copy of the queue: each request taken in once per
  host, which 137 rules out ("no request is taken in twice");
- the first host that finds no lead in a file of the host's own state,
  or a ref of its own on the remote: a mark by another name, which
  core.33 rules out.

### D2. A pull request table on a branch whose records each host keeps refuses that branch alone

`Rules` gains `unkept: bool`, set where the table has `pull_requests`
and the branch is not signed (its policy names no `[signers]`, so its
records are each host's; C.6). `Policy::rules_for(<b>)` refuses `<b>`
alone with `NoIntake { branch, remote }`, in words such as `main takes
pull requests, which only the host that keeps it on origin takes in,
and its records are each host's own: name [signers] so they are kept on
origin (137)`. Every other branch reads as before (56, S64). Routing
reads the branch's remote by `Policy::remote_of`, whatever its table
refuses (rule core.27). The refusal is a policy's, so:

- a submit to it exits 2 before anything is pushed or opened, as one to
  a branch whose policy refuses `[[checkout.cache]]` does;
- a step, a pass and the host process's look pass it by with its words,
  every other branch moving (26; rule core.14);
- `sluice status <b>` and the overview show it in those words;
- `PullRequests::branches` lists every other branch that takes pull
  requests, the refused one passed by, never failing the list.

The forge's workflow is named in the refusal by the change that builds
the GitHub Action way, which no command of this sluice writes yet: a
refusal naming `sluice init actions` would send its reader to a command
that does not exist (rule core.26).

Options weighed:

- the whole policy refused: every branch stops while one is moved (S64);
- the table accepted, its requests waiting: 137 rules out a request
  opened that nothing would take in;
- the table's intake alone refused, the branch's worktree submissions
  going on: 137 refuses the branch, "as a word sluice no longer keeps
  is", and a half-kept branch would land its worktrees' work and leave
  its requests open, unsaid.

This reverses no decision of #29's. #29 took out the refusal of
d837d194 with the forge's round job, whose `[pools]` half it needed;
137 kept the refusal through 47ddb48, its revert and 5f05fc6.

### D3. Nothing of the mark is kept, and nothing reads what an older sluice left

`PushedRefs` loses the mark's refspec, the `--prune` it alone needed,
and every function of it, `leased_ref` and `at_the_lease` among them, so
a fetch of the branch brings no `refs/sluice/intake/*`. `Kept` loses
`intake/` and its `.waits/`. A mark an older sluice left on a remote is
fetched, read and deleted by nothing; `refs/sluice-fetched/<remote>/
intake/*` and `forge/intake/` an older sluice left on a host are read
by nothing either. The pull requests guide says how to delete a
remote's mark by hand, `git push <remote> :refs/sluice/intake/<branch>`,
and the state reference that `forge/intake/` may be removed.

Options weighed: a sweep that deletes every remote's mark, a push to a
ref sluice no longer owns, which an older sluice on another host still
reads until it is upgraded; a sweep of the host's `forge/intake/`,
which saves a few bytes and is one more thing to test.

### D4. No hand-over is written, and an older one is read as written

`Requester::HandOver` goes, with `Pulls::hand_over` and
`Sluice::drop_obsolete`, whose one caller it was, and nothing withdraws
a submission for a mark; a round still drops the combinations every
other withdrawal makes obsolete before it reads any run. `Withdrawal::handed_over_to` and
`Event::Withdrawn`'s stay in `sluice/submission@1` and `sluice/log@1`,
read and never written (26). `handed_over` and `named_no_host` stay as
`a-hand-overs-withdrawal-holds-no-request` built them: intake's latest
submission of a request, the hold for a submitter's act, who withdrew a
hand-opened request and the wait's following each read such a
withdrawal as holding no request (record 0088 as amended; rule core.33,
"Not ruled out"; rule core.48).

A host's own records of a branch go to its remote the first time it
routes the branch there (`Records::migrate`), so a branch whose
records each host kept, and that names `[signers]` to take pull
requests again under D2, carries the hand-overs its hosts wrote. Read as
written, each such request is taken in again at its head by the host
that keeps the branch, with no draft.

Options weighed: reading an older hand-over as any other withdrawal,
which holds each such request as a draft until its submitter acts again,
for a withdrawal no submitter made (120).

### D5. A host that does not keep the branch shows the requests it follows

Record 0095's rows were for a host without the mark. They become every
host's that does not take the branch's requests in, which under D1 is a
host that does not keep a branch whose records are on its remote:

- `opening` keeps each request a submit on the host opens, updates or
  marks ready under `forge/follows/`, and `follow_across` each a wait or
  a status follows, as now; the hand-over adds none, since there is
  none;
- `PullRequests::across` and `kept_across` read every such branch's
  follows where this host does not keep it, rather than only a branch
  whose records each host keeps, each row a request whose submission
  the branch's queue does not hold at its head, so a request the
  keeper took in shows in the queue and nowhere else;
- `taken_in_by` is `{"host": <lead>}`, the host the branch's records
  name as keeping it, `null` where none does; `holder` goes, since no
  mark names one; the rows' heading reads `pull requests <host> takes
  in:`, or `pull requests this host follows:` where none leads;
- `kept_across` reads the rows again once the kept file moved or the
  branch's lead did, in place of the mark's commit (129; rules core.25,
  core.31).

A wait and `--status` give `taken_in_by` the same way. Its words name
the host that keeps the branch where one does: `waiting to be taken in
by b`.

Options weighed:

- no rows for any branch, record 0095's reading dropped with the mark:
  a request a host submitted shows nowhere on that host until the
  keeper takes it in, which 137's last sentence rules out ("each
  request it submitted or reaches with the request's entry there,
  taken in or waiting and on what");
- `taken_in_by` dropped: crew reads it, and each row's `queue.host`
  names a host only where a check stands.

### D6. A step prints what its round met

`PullRequests::intake` returns what its pass met beside whether the
branch takes pull requests: each warning a cause, what intake asked,
`main's pull requests` or `pull/<n>`, and the words it was refused
with, which is today's `taking in main's pull requests: pull/12: <e>`
split in two. Nothing in `pulls.rs`'s intake path prints. `crate::intake`
gives its caller the causes:

- `pass::step` puts each in the answer's `warnings`, and `sluice step`
  prints each on stderr as `sluice: warning: taking in <branch>'s pull
  requests: <asked>: <said>`, text or `--json`, every step, a standing
  cause among them, since a step is a person asking what its round met;
- a submit's own first intake prints each on its stderr, as it does
  today;
- a pass hands them to D7, and says only what D7 logs anew.

The spent allowance's line is not a warning of these: it stays on
`out`, or stderr under `--json`, said once for its pause (134).

Options weighed: a step reading the host process's log for its round's
lines, which another round may write between; the host process's stderr
captured per round, which reads words, not causes, and leaves D7 to
parse them.

### D7. A standing intake failure is kept, logged and said as a refused read is

Intake's failures are reads of the forge the core could not make (rule
core.28's deneb case of 2026-11-05 was intake's), so they take 133's one
design rather than a second (rule core.12):

- the records seam keeps, beside a branch's refusal of its remote, this
  host's refusal of its intake: the same `Refusal` document, under
  `intake/<branch>.json` where the first is `refused/<branch>.json`,
  its `remote` the branch's remote, its causes the D6 causes, its
  `rounds` the intakes it refused;
- `Sluice::note_intake_refused(branch, causes)` keeps it and writes a
  `read-refused` line, `{"host","remote","asked","said","intake":true}`,
  for each cause not yet logged, in a write of its own or the next write
  of the branch's log, as 133's lines are (`Sluice::carry`);
  `Sluice::note_intake_read(branch)`, at an intake that met no failure,
  ends it with a `read-again` line, `{"host","remote","since","stood",
  "rounds","intake":true}`; `intake` is skipped where false, so the
  round's own lines are written as before; a cause logged while the
  refusal stands is not logged again when it comes back;
- a pass says a cause on the host process's log only when its note
  logged it, which the kept document's `logged` mark tells, never a
  `thread_local` or anything else a process holds: a host process that
  restarts, a step and a submit beside it say nothing again (rules
  core.11, core.14);
- `sluice status <branch>` shows the standing refusal above the queue,
  `this host could not take in main's pull requests since <t>, <n>
  rounds: <asked>: <said>`, and `--json` carries it as `intake_refused`
  beside `refused`; nothing a round, gate, landing or submission
  decides reads it.

A request intake could not read is still read again each round, and
holds no other (128; rule core.42); what changes is only how often it
is said and where it is written. A branch's intake that fails whole,
its list refused, still lets the round land what is queued.

Options weighed:

- the round's refusal document itself: every round that read the branch
  would end the refusal intake opened, so a standing intake failure
  would be logged at every pass;
- events of their own, `intake-refused` and `intake-again`: an older
  sluice reading a branch's log on its remote fails on a line whose
  `event` it does not know (`parse_lines`), where it reads a
  `read-refused` line with a field it does not know as it reads any;
- core.47's form, a count in the branch's live document at every
  refusal: a write of the records at every pass while GitHub refuses,
  where 133's form writes once per cause.

### D8. The suite and the unfinished changes

`OnGitHub` gives `main` `[signers.a]` with a key made for the test,
`trust = ["a"]`, and the host's `[signing]` before `[runs]`, so its
records are on the bare remote and host `a` keeps it; `other(name)`
gives the policy that host's signer too where a test asks, and a test
across hosts lets `b`'s pass run first, so `b` keeps `main` and `a`
does not. The tests of the mark, its lease, its holds, its renewal and
the hand-over are taken out where nothing of them stands, or rewritten
to the host that keeps the branch where something does
(`records_on_each_host_one_host_asks` becomes the refusal's test; `a_
host_that_stopped_loses_the_mark` the lead's lease). The mark's helpers
(`mark`, `push_mark`, `record_marks`, `marks_pushed`,
`moved_under_the_next_push`, `renewal_due`, `mark_comes_back`,
`handed_over_mid_test`) go. The unit tests of an older hand-over's
reading stay.

`a-host-hands-over-only-to-the-host-its-mark-names` and
`a-hand-overs-withdrawal-holds-no-request` are taken out of
`openspec/changes/`: their deltas describe the mark and were never
archived, and what of them stands, an older hand-over read as holding
no request, is this change's `request-intake-host` delta.

## Risks / Trade-offs

- **sluice's own `main` and WilldanGroup/switchboard-kit's take pull
  requests and name no `[signers]`** → each is refused on the release
  that carries this. The release notes say so first, the orchestrator is
  told, and ops hold those hosts' installs until each names signers.
  `sluices_own_policy_reads` says so of sluice's own: it finds `main`
  refused until `.config/sluice.toml` names signers, which is not this
  change's work.
- **Naming `[signers]` keeps every branch with a `remote` on its remote
  (C.6)** → a repository whose `main` names signers moves its bolts'
  records there too, each run signed by a key the policy lists; ops
  plan that with the hosts' keys before the policy changes.
- **A host of an older sluice beside one of this** → the older one
  still pushes and reads the mark for a branch whose records each host
  keeps, and this one refuses that branch; on a branch whose records
  are on its remote both take requests in only where they keep it.
- **A remote holds an older mark until someone deletes it** → read by
  nothing; the guide says how.
- **Two copies of one repository on one host share its name among its
  peers, so both keep the branch** → each takes its requests in under
  its own copy's intake lock, the records' leased write refusing the
  second of two writes at once, until the one-copy design decides which
  copy takes them in (137; rule core.48's details, "Not ruled out").
- **One write of the branch's records for each new intake cause** → as
  133's, once per cause and host, never per pass.
- **Rows across hosts read the branch's lead from its records** → the
  branch's document, which `status` and the reader read already; no
  other host's record of a request (rule core.33).

## Migration Plan

1. Before the release that carries this is installed on a host serving
   a branch that takes pull requests with no `[signers]`, the branch's
   policy names `[signers]` for the hosts that run its checks and
   `trust` names them; the first host to route the branch there carries
   its records onto its remote (`Records::migrate`).
2. After, a remote's `refs/sluice/intake/<branch>` and a host's
   `forge/intake/` may be removed by hand.
3. Rollback: an older sluice reads the records this one writes, an
   intake's `read-refused` and `read-again` lines among them, as lines
   of 133's, its `intake` field passed over.
