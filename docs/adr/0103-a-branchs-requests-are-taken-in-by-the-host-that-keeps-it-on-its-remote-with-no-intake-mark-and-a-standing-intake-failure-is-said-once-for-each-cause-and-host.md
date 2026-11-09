---
number: 103
title: A branch's requests are taken in by the host that keeps it on its remote, with no intake mark, and a standing intake failure is said once for each cause and host
status: accepted
date: 2026-11-08
decision-makers:
- the construct stage of change/requests-are-taken-in-with-no-intake-mark
---

# A branch's requests are taken in by the host that keeps it on its remote, with no intake mark, and a standing intake failure is said once for each cause and host

Status: accepted, D1 to D8 below the construct stage's own, under 137 at agentplot/blueprints 5f05fc6 and record 0093 as amended on 2026-11-08, which the user accepted that day; each is cheap to change later. It amends records 0095 and 0096 in part, as their status says.

Deciders: the construct stage of `change/requests-are-taken-in-with-no-intake-mark`.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 56, 72, 73, 120, 128, 133, 137, 140, S64 and C.5's pull request row at 5f05fc6; records 0088, 0089, 0093 as amended on 2026-11-08, 0095 and 0096; rules core.11, core.14, core.27, core.28, core.33, core.47; `crates/sluice/src/profiles/github/pulls.rs` (`Pulls::takes`, `decide`, `put_mark`, `hold`, `let_go_marks`, `hand_over`, `take_in`, `across`), `crates/sluice/src/profiles/github/kept.rs`, `crates/sluice/src/profiles/pushed_refs.rs` (`intake_ref`, `intake_mark`, `put_intake_mark`, `drop_intake_mark`), `crates/sluice/src/lib.rs` (`Taker`, `Holder`, `TakenInBy`, `crate::intake`) and `crates/sluice/src/host_process/pass.rs` as the bolt `sluice-runs-one-host-process` held them at 752db9a9.

## Context and problem statement

Record 0093 as amended on 2026-11-08 takes the intake mark away, with its renewal, its lease, hand-overs and their withdrawals: whatever executes a branch's queue takes its requests in, nothing does one part of that while another does the rest, and no request is taken in twice. 137 at 5f05fc6 says where: only where the branch's records are on its remote, by the instance that keeps the branch there (72), and a pull request table on a branch whose records each host keeps is refused for that branch alone, naming `[signers]`, "so no request is opened where nothing would take it in". Rule core.33 rules out the mark, its renewal, its lease, a hand-over and a withdrawal for one.

The bolt at 752db9a9 still took in a branch whose records each host keeps by whichever host held `refs/sluice/intake/<branch>` on its remote, pushed and renewed by the host process's pass, and withdrew a host's own submissions as a hand-over when it found another's mark. `taken_in_by` named the mark's host and holder. Intake's warnings were the host process's stderr alone, a standing one said once per process by a `thread_local`, so a `sluice step` never printed why a request was not taken, a host process that restarted said it again, and nothing of it reached the branch's log (rule core.28).

Who takes a branch's requests in, what becomes of a branch whose records each host keeps, of the mark an older sluice left and of its hand-overs, what a host that does not take the requests in shows, and how is a failure of intake said?

## Considered options

**D1. Who takes a branch's requests in.**

1. **The instance that keeps the branch on its remote** (chosen). `Pulls::takes` answers `Here` where the branch's records are on its remote and `Sluice::keeps` is true, and `Elsewhere`, naming the lead the branch's state names, otherwise. Any process of that host takes them in under intake's lock, so the state's one lead and intake's one lock make one taker (72; rule core.14).
2. Every host's process taking in a branch whose records each host keeps, its own copy of the queue: each request taken in once per host, which 137 rules out.
3. The first host that finds no lead in a file of its own or a ref of its own on the remote: a mark by another name, which rule core.33 rules out.

**D2. A pull request table on a branch whose records each host keeps.**

1. **Refuses that branch alone** (chosen), naming the branch, its remote and `[signers]`, as a `[[checkout.cache]]` does (56, S64). A submit to it exits 2 before anything is pushed or opened; a step, a pass, `status` and the overview show the refusal, every other branch moving; routing reads its remote whatever the table refuses (rule core.27).
2. The whole policy refused: every branch stops while one is moved (S64).
3. The table accepted and its requests left waiting: 137 rules out a request opened that nothing would take in.
4. The table's intake alone refused, the branch's worktree submissions going on: a half-kept branch lands its worktrees' work and leaves its requests open, unsaid, where 137 refuses the branch.

**D3. The mark an older sluice left.**

1. **Read, fetched and deleted by nothing** (chosen). A fetch of the branch brings no `refs/sluice/intake/*`; `refs/sluice-fetched/<remote>/intake/*` and `forge/intake/` on a host are read by nothing; the pull requests guide says how to delete a remote's mark by hand and the state reference that `forge/intake/` may be removed.
2. A sweep that deletes every remote's mark: a push to a ref sluice no longer owns, which an older sluice on another host still reads until it is upgraded.
3. A sweep of the host's `forge/intake/`: a few bytes saved for one more thing to test.

**D4. Hand-overs.**

1. **None written, an older one read as written** (chosen). `Requester::HandOver`, `Pulls::hand_over` and `Sluice::drop_obsolete`, whose one caller it was, go. `withdrawn.handed_over_to` stays in `sluice/submission@1` and `sluice/log@1`, read and never written, holding no request (record 0088 as amended; rule core.48), so a branch whose records move onto its remote when its policy names `[signers]` takes each such request in again at its head with no draft.
2. An older hand-over read as any other withdrawal, holding each such request as a draft until its submitter acts again, for a withdrawal no submitter made (120).

**D5. What a host that does not keep the branch shows.**

1. **Each request it follows whose submission the branch's queue does not hold at its head** (chosen), every host's that does not keep a branch whose records are on its remote, read from sluice's check on its head. `taken_in_by` is `{"host"}`, the host the branch's records name as keeping it, `null` where none does, with no holder. The rows' heading reads `pull requests <host> takes in:`, a wait's words `waiting to be taken in by <host>`, and a reader's rows are read again once the kept file or the branch's lead moved.
2. No rows for any branch, record 0095's reading dropped with the mark: a request a host submitted shows nowhere on it until the keeper takes it in, which 137's last sentence rules out.
3. `taken_in_by` dropped: crew reads it, and a row's `queue.host` names a host only where a check stands.

**D6. What a step prints.**

1. **Every warning its round's intake met, each step** (chosen). `PullRequests::intake` returns its causes, what it asked, `<branch>'s pull requests` or `pull/<n>`, and the words it was refused with, and prints nothing; `sluice step` says each as `sluice: warning: taking in <branch>'s pull requests: <asked>: <said>`, text or `--json`; a submit's own first intake says each on its stderr.
2. A step reading the host process's log for its round's lines, which another round may write between.
3. The host process's stderr captured per round, which reads words, not causes, and leaves D7 to parse them.

**D7. A standing intake failure.**

1. **Kept, logged and said as a refused read is** (chosen, 133; rule core.28). The records seam keeps, beside a branch's refusal of its remote, this host's refusal of its intake, the same `Refusal` under `intake/<branch>.json`. `Sluice::note_intake_refused` writes a `read-refused` line marked `"intake":true` for each cause not yet logged, and `Sluice::note_intake_read` ends it with a `read-again` so marked; a pass says a cause on the host process's log only when its note logged it, read from that document and from no process's memory (rules core.11, core.14); `sluice status` shows it as `intake_refused`.
2. The round's refusal document itself: every round that read the branch would end the refusal intake opened, so a standing intake failure would be logged at every pass.
3. Events of their own, `intake-refused` and `intake-again`: an older sluice reading the branch's log on its remote fails on an `event` it does not know, where it reads a `read-refused` line with a field it does not know as it reads any.
4. Rule core.47's form, a count in the branch's live document at every refusal: a write of the branch's state at every pass while GitHub refuses, where 133's form writes once per cause.

**D8. The suite and the unfinished changes built on the mark.**

1. **The suite keeps main's records on its remote** (chosen): `OnGitHub` gives `main` `[signers.a]` with a key made for each fixture and the host's `[signing]`, so host `a` keeps it; a test across hosts lets `b`'s pass run first, so `b` keeps it. The tests of the mark, its lease, its holds and hand-overs are taken out, or rewritten to the host that keeps the branch where something of them stands. `a-host-hands-over-only-to-the-host-its-mark-names` and `a-hand-overs-withdrawal-holds-no-request` are taken out of `openspec/changes/`: their deltas describe the mark and were never archived, and what of them stands is this change's `request-intake-host` delta.
2. The suite left on records each host keeps, its tests of pull requests reading D2's refusal alone: nothing would test intake.

## Decision outcome

Option 1 of each of D1 to D8. Rules core.33 and core.28 hold what they rule out.

### Consequences

- One host takes a branch's requests in, the one that keeps it on its remote, and nothing is pushed to or read from `refs/sluice/intake/<branch>`.
- sluice's own `main` and WilldanGroup/switchboard-kit's `main` take pull requests with no `[signers]`, so each is refused on the release that carries this until its policy names signers. The release notes say so first, and the orchestrator is told.
- Naming `[signers]` keeps every branch with a `remote` on its remote (C.6): a repository whose `main` names signers moves its other branches' records there too, each run signed by a key the policy lists, which its operators plan with the hosts' keys before the policy changes.
- A host of an older sluice beside one of this still pushes and reads the mark for a branch whose records each host keeps, which this one refuses; on a branch whose records are on its remote both take requests in only where they keep it.
- crew reads `taken_in_by` from a wait's document and loses `holder`.
- One write of the branch's records for each new intake cause, never one per pass; an older sluice reads an intake's `read-refused` and `read-again` lines as 133's, its `intake` field passed over.

## Amendment, 2026-11-08: what the build chose

The code stage of `change/requests-are-taken-in-with-no-intake-mark` made these choices within D1 to D8, each its own and cheap to change later.

1. **An intake that asks nothing between discoveries says what stands.** The causes the intake that last asked met are kept with the host's readings of the branch, `forge/requests/`, and an intake that asks GitHub nothing, its discovery not due, gives them to its caller as standing, so a step beside a pass that just asked prints them (D6). The path not taken: a step's intake asking at once whatever the discovery, an ask of the forge that nothing on the host moved (136; rule core.32).
2. **A cause is what it asked and the forge's words, whatever the tries.** The retry policy's `tried 5 times over 28s: ` is left out where two intakes' causes are compared, so a standing failure is one cause however often it is tried (D7). The path not taken: comparing the words whole, which logs the same failure at every pass.
3. **The intake's refusal is kept in the repository's common directory.** Under `sluice-remote/<remote>/intake/` there for a branch on its remote, and `.intake/` beside `.refused/` for one in the host's own files, so a submit from any worktree reads the one refusal its host's intake met (D7). The path not taken: the git directory of the worktree a command runs in, where the round's own refusal of its remote is kept today, so a worktree's submit logged a standing cause again.
4. **An intake writes its lines under the branch's lock where it is free.** Where a round holds it, that round's next write carries them, as 133's are carried (D7; rule core.14). The path not taken: waiting for the lock, a pass's intake held behind a round's checks.
5. **The overview names a refused default branch with no records.** A branch the default branch's policy refuses for its pull request table is listed with the refusal where it has no records yet, as it is where it has some (D2). The path not taken: naming only branches with records, so a branch refused before its first submission shows nowhere but in its status.
6. **A submit to a refused branch refuses every form.** `--check` and `--status` exit 2 with the refusal as a plain submit does, before anything is read of the worktree (D2). The path not taken: forecasting or answering a status for a branch whose policy is refused, which reads its gate.
7. **The refusal's words say "state".** `name [signers] in .config/sluice.toml so its state is kept on origin`, the word the pages use for the branch's records (rule core.26). The path not taken: "records", as D2 sketched them.
