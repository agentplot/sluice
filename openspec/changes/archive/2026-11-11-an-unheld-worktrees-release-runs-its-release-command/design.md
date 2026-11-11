# Design

## Context

**The brief.** Releasing a worktree that sluice holds nothing for runs
the policy's release command in it, as releasing one it holds does, and
answers with that command's outcome. So what the worktree's own work
started ends before its driver removes it. `nothing to release` stays
the answer only where the policy names no release command.

**Sources.**
- Signal 2026-11-11-agentplot-planner-b1e1f248/01 (afterthought/crew-state,
  `agentplot/main`), agentplot-operator quoting wldn-ops: "`sluice
  release --worktree <w>` on a worktree sluice holds nothing for runs no
  release command and answers 'nothing to release' with exit 0 (wldn-ops's
  probe on a throwaway switchboard-kit worktree, sluice 0.78.1), though
  sluice's help says 'Release this worktree: run the policy's release
  command, end what its runs left running, free its ports'. crew's tidy
  trusts that answer and removes the place, so a unit's emulator started
  from the worktree's own lease outlives it: four Floci containers on the
  box, places removed 2026-11-07 to -09."
- wldn-design's ruling, switchboard-kit PR 522, commit d53448f57
  (`docs/architecture/testing.md`): "the fix is sluice's, and it is
  passed to sluice's owners: `sluice release --worktree` runs the
  policy's release command in any worktree of the repository, held or
  not. The kit adds no second caller, neither a worktree-removal hook
  nor a sweeper. crew's tidy stays as it is, since it already calls the
  one stop." The user, 2026-11-11, refusing a sweeper on proposal 1128:
  "This is the whole point of having Sleuth with its release command."
- agentplot-operator's relay of that ruling, 2026-11-11: one path in
  sluice's release, and one test. In that test, a release that leaves a
  mark runs and exits 0, and a failing release exits non-zero.
- agentplot/blueprints `design/sluice/proposals/session-runs-1.md`
  (origin/main ad3a1ae): clause 94, clause 95 ("A session's place is
  released only at the asking of its session or its driver: the
  policy's release command run in the place under the host's executor,
  everything its runs started ended, and its ports let go (34)"), C.1's
  session worktree row, C.5's worktrunk row ("before worktrunk removes
  the session's place, `wt submit` runs `sluice release` in it (95)")
  and C.7 ("a session's place takes them from the branch it was taken
  off"). `design/sluice/requirements.md` clauses 25 and 34. The
  glossary's place is a working copy prepared for one session, whether
  or not sluice adopted it.
- switchboard-kit's `infra/ministack/release.sh`: "A caller hands it
  whatever environment it has: `sluice release` and crew's removal of a
  unit worktree run it with no MINISTACK_PORT." It finds each port from
  the checkout and is safe to run any number of times.

**sluice today, read on this branch at 49fcf7d3 on 2026-11-11.**
- `crates/sluice/src/exec.rs:655` `release`: it asks
  `checkout.adopted(host.worktree(dir))`. Where that gives none, it says
  `nothing to release` (`:669`) and returns `released: false`. No
  policy is read and the place seam is not asked to release.
  `Command::Release` (`crates/sluice/src/lib.rs:2257`) exits `LANDED`,
  0, unless `failed` is set.
- `crates/sluice/src/profiles/places.rs:1568` `release_session(name,
  policy)`: where the place has no record, it ends any runs left under
  the name with no release command and returns `Released::Nothing`.
  Where it has one, it builds the release with the check seam's builder
  from `session_place` (its ports, `SLUICE_PLACE`, its worktree and its
  scratch). It runs the release with `runs.remove_all_with(name,
  ReleaseWith::Fresh(request))` and lets the place go (`let_go`: scratch,
  ports block, record). A failure is kept on the record by
  `keep_failure`, so the place stays held (123).
- `crates/sluice/src/profiles/checks.rs:1358` `release_request`: no
  `[tests] release` gives `None` before anything runs. Otherwise it
  enters the toolchain alone (core.46: "the working copy was equipped
  by the run it releases"), chooses the executor from the limits, and
  writes a sandbox profile over the place's path and scratch.
- `crates/sluice-core/src/session.rs:441` `release_policy(branch)`:
  `[tests]` and the limits at the branch's current revision. It is an
  error where the branch cannot be read. A revision with no policy names
  no release command.
- `Host::working_copy(dir)` gives the top of the git worktree that holds
  `dir`, or none (`exec.rs`, 147). `Host::default_branch()` gives the
  branch the repository's `HEAD` names, with its revision here (ADR 0036).
- The callers. crew's tidy runs `sluice release --worktree <path>` and
  then `wt remove` (crew `plan.9`). Flume's teardown of a place is
  `sluice release` (flume `crates/flume/src/profiles/merge_queue/sluice/mod.rs`,
  `for_place`, at e2973d57). `wt submit` runs `sluice release` and bails
  on a non-zero exit before worktrunk removes the worktree
  (`crates/sluice/src/profiles/worktrunk/mod.rs:532`).
- sluice's own `.config/sluice.toml` names no release command.

## Goals / Non-Goals

**Goals:**
- `sluice release` in a worktree sluice holds nothing for runs the
  policy's release command there, by the path a held worktree's release
  takes, and answers with its outcome.
- `nothing to release` only where the policy names no release command,
  or where the folder is in no worktree of the repository.
- The one test the ruling names.

**Non-Goals:**
- Changing any caller: crew's tidy, Flume's teardown and `wt submit`.
- Letting a caller name the branch whose policy releases an unheld
  worktree.
- `--accept-failure` in an unheld worktree, and the held path's
  fallback to the last run's command where its branch cannot be read.
  Both stay as they are.
- Releasing a removed worktree that sluice never held. Nothing of it
  remains to run in, and 123's gone path covers the ones it held.

## Decisions

### D1. One path: an unheld worktree is a place for its release's length

`exec::release` resolves the worktree, the name of its session's place
and the policy, then calls `Checkout::release_session(name, policy)`
for a held and an unheld worktree alike. One tail turns the seam's
answer into words and an exit.

In the place profile, a name with no record is released through the
held path. Given a policy, `release_session` first ends any runs left
under the name, as today. Then, where the name's worktree is a worktree
of the repository, it writes a session record for it under the records
lock. The record holds the worktree, the policy's branch and a scratch
from `new_scratch`, and no ports block. The profile makes the scratch
and goes on as for a held place: `session_place`, the check seam's
builder, `remove_all_with(Fresh(request))`, then `let_go`. A builder
that gives `None` lets the record go and returns `Released::Nothing`.
A release that ran returns `Released::Policy`.

The record exists only for the release. It is why the scratch is named
while it exists (rule core.30): a concurrent round's
`remove_unnamed_scratch` leaves it be. It also gives the release the
same `SLUICE_PLACE` a `sluice exec` in that worktree is given, since
both derive from the record's path. So a release command that finds
what a run started by that label finds it here too.

Not taken:
- **A branch in `exec::release` that runs the release itself.** That is
  a second path beside `release_session`, which the ruling rules out. It
  would also be a second copy of the builder's call.
- **Adopting the worktree with `adopt` and then releasing it.** That
  draws a ports block the release never needed, and gives the release
  ports no run of the worktree ever had.
- **A release with no record and a scratch nothing names.** Rule core.30
  rules that out, and the repository's sweep could remove the scratch
  under the running command.
- **A release with no scratch.** `SLUICE_TMP` and a sandboxed release's
  `TMPDIR` would name nothing.

### D2. The policy read is the default branch's

A held worktree's release reads the policy of the branch its runs were
held to, at its current revision (56). For an unheld worktree sluice
holds nothing that names the branch it was taken off (C.7 of
session-runs-1). It reads the repository's default branch instead, the
branch `HEAD` names (`Host::default_branch`), at its current revision
through `Sluice::release_policy`. The overview reads the same branch
where no branch is named (ADR 0036). In switchboard-kit and in sluice's
own repository, places are cut from main or from a bolt off main, and
the release command is the same on both.

Not taken:
- **The policy at the worktree's own head.** Rule core.15 rules out
  reading policy from a session's place, and the worktree's own commits
  could name any command.
- **The target of the worktree's last submission.** A worktree that
  never submitted has none. crew's units freed before they submit are
  the case that leaked. Finding one would also mean a read of every
  branch's records that no release makes today.
- **A branch argument to `sluice release`.** The ruling keeps crew's
  tidy as it is, and Flume's teardown names no branch. That is a change
  of its own if a branch's release command ever differs from the
  default branch's (proposal, out of scope).
- **The branch the worktree has checked out.** For a unit or a change
  place that branch is no branch sluice lands on, and its revision is
  the worktree's own work.

### D3. What the release runs with

The command runs at the top of the worktree and enters the toolchain
the policy names, alone. Rule core.46 does not rule this out for a held
place, since the run it releases equipped the working copy. For an
unheld one, the work that started what the release ends equipped it.
The executor is chosen from the policy's limits, as for any release.
The environment is a held worktree's: `SLUICE_CHECKOUT` the worktree,
`SLUICE_TMP` the release's scratch, `SLUICE_PLACE`, and no
`SLUICE_PORT_*`, since sluice gave the worktree no port. The kit's
release finds its ports from the checkout and is handed none by crew or
`sluice release` today.

### D4. What it keeps: nothing

Whether the release passes or fails, the profile lets go of the record
and scratch it made for the release, and keeps nothing for the
worktree. A held worktree's failed release keeps its place held, since
its ports and runs are what sluice holds (123). An unheld worktree had
none before the release, and has none after it. The caller sees exit 1
and keeps the worktree. The words say so: `could not release
<worktree>: <what failed>`, then `sluice holds nothing for it: sluice
release runs its release command again`. A later `sluice release` runs
the command again.

Not taken:
- **Keeping the record with `release_failed`, so `sluice status` lists
  it.** A later `sluice exec` would adopt the record as it found it,
  with no ports block, since `adopt` draws a block only for a record it
  writes. `--accept-failure` would then free a hold sluice made only to
  show a failure. Listing it would need `adopt` changed too, and that
  belongs to a change of its own if a person asks to see such failures
  in `sluice status`.

### D5. The answer

The answer is a held worktree's:
- `released <worktree>: its runs ended, the release command run, its
  ports free`, with `{"worktree", "released": true}`, exit 0, when the
  command passed;
- the failure as D4 words it, with `{"worktree", "released": false,
  "failed"}`, exit 1, when it failed;
- `nothing to release`, `{"worktree", "released": false}`, exit 0, when
  the policy names no release command, or when the folder is in no
  worktree of the repository.

The brief's "answers with that command's exit" is read as its outcome,
0 or 1, which is what a held worktree's release and the reference's
exit codes give. A default branch that cannot be read, or one whose
policy cannot be read, is a release that failed. It exits 1 with `could
not release <worktree>: <branch> cannot be read (<words>), so its
release command is not known`. Where `HEAD` names no branch, it exits 1
with `could not release <worktree>: the repository's HEAD names no
branch whose policy names its release command`. Either answer keeps the
caller from removing a worktree whose release never ran. A held
worktree whose branch cannot be read still runs the command its last
run carried.

Not taken:
- **Passing the command's own exit code through.** A held and an
  unheld worktree would answer differently for one failure, and a code
  the command chose could collide with sluice's own exit codes, 2 for a
  refusal and 3 for sluice's failure.

### D6. Which folder is released

The worktree is `--worktree` as `sluice exec` names it, `Host::worktree`,
where sluice holds a place for it, as today. Otherwise it is the top of
the git worktree that holds it, `Host::working_copy`, so `sluice
release` from a folder within a worktree releases the worktree. A
folder in no worktree has nothing to release, and neither has a folder
that has gone. The place profile releases a recordless name only where
its folder is the repository's main worktree or a linked worktree git
lists, detached or not. A worktree of another repository has nothing to
release, and this repository's command never runs in it.

### D7. The seam

- `ReleasePolicy` gains `branch: String`, the branch the policy was read
  from. `Sluice::release_policy` fills it, and the profile writes it on
  the release's record.
- `Checkout::release_session`'s contract names the recordless case:
  given a policy, a worktree of the repository is released by the
  policy's release command with a record and scratch of the release's
  own, let go once it ends. Given none, or a folder that is no worktree,
  the answer is `Nothing`, as today. `Released::Nothing`'s doc says when.
- The fake place seam keeps that contract. For a name it holds nothing
  for, whose folder is one of its world's worktrees, and whose policy
  names `release`, it records the release in `session_releases` and
  answers `Policy`. Otherwise it answers `Nothing`.

Rule core.2 holds: the core and the binary reach the place through the
one interface. Rule core.1 holds: nothing outside a profile names a
tool.

### D8. The family

The release is the one stop for a place in every tool. crew's tidy,
Flume's teardown and `wt submit` each call `sluice release` before
removing a worktree, and none starts or ends a place's services
itself. The cause is that sluice's release did nothing for a worktree
it never adopted. So the fix is sluice's alone, and no other tool
changes (wldn-design's ruling). No statement clause changes: 95 and
C.1's session worktree row already say this release runs the policy's
release command in a session's place. Reading C.7's "branch it was
taken off" as the default branch, where sluice holds nothing that names
it, is this change's own reading (D2), recorded in record 0112.

### D9. The record

The decision is recorded as record 0112, "A worktree sluice holds
nothing for is released by its policy's release command". It is
accepted under clause 95, rule core.10 and wldn-design's ruling, and
its choices, D2 to D6, are the construct's own and cheap to change.
Records 0018 and 0081 gain an "Amended in part by record 0112" line.
Rule core.10's details cite the incident and record 0112. Rule core.46's
details admit the toolchain alone for an unheld worktree.

### Rules relied on

- core.10: a place is never released without running its release
  command.
- core.16: a session's place is released only at its session's or
  driver's asking, the worktree left.
- core.15: policy is read from a branch's current revision, never from
  a place.
- core.30: a place's scratch is named by its record before it is made.
- core.46: a release enters the toolchain alone.
- core.1, core.2: the seam and the profiles as above.
- core.21: each task's check is one test or one project's task, which
  the gate runs.

## Risks / Trade-offs

- [A release that now fails where it once answered 0] → crew's tidy,
  Flume's teardown and `wt submit` keep a worktree whose release command
  fails, where before they removed it and left what it started running.
  That is the point of the change: the words name what failed, and the
  next `sluice release` runs it again.
- [Every unheld release enters the toolchain] → a release in a worktree
  of a repository whose policy names a release command now waits for
  `[tests] toolchain` (switchboard-kit's devenv). A held worktree's
  release has always waited the same.
- [The default branch's release command for a worktree off a bolt] →
  where a bolt's policy names a release command the default branch's
  lacks, an unheld worktree off that bolt answers `nothing to release`.
  A held one, released under the bolt's policy, does not. The proposal
  leaves a caller-named branch out of scope.
- [A `sluice exec` in the worktree during its unheld release] → the
  exec finds the release's record and runs on it, and the release then
  lets it go. A held worktree's release racing an exec does the same
  today, and its release ends the runs it finds.

## Migration Plan

Nothing to migrate: no record or state changes form. A host takes the
behaviour with the sluice that carries it.

## Task notes

**1.1 Record 0112.** Front matter as record 0111's. Number 112, or the
next free one where 112 is taken on main or this bolt. Title "A
worktree sluice holds nothing for is released by its policy's release
command". Status accepted, dated 2026-11-11. Decision-makers:
wldn-design's ruling (switchboard-kit PR 522, d53448f57) relayed by
agentplot-operator, the planner's brief of 2026-11-11, and the
construct of `an-unheld-worktrees-release-runs-its-release-command`.
The status line says clause 95, rule core.10 and the ruling decide it,
and that D2 to D6 are the construct's own, each cheap to change.
- Context: the signal's words, the help's words, the callers, and
  `exec::release`'s early return.
- Considered options: one path through `release_session` (D1); a
  second path in `exec::release`; a sweeper, which the user refused on
  proposal 1128; each caller running the release command itself.
- Decision outcome: D1 to D7 in brief, with the paths not taken.
- Consequences: what each caller now sees.

The line for record 0018, after its status paragraph: "Amended in part
by record 0112, 2026-11-11: in a worktree sluice holds nothing for,
`sluice release` runs the policy's release command all the same. It is
read from the repository's default branch at its current revision, run
with no ports and a scratch of the release's own, and keeps nothing
once it answers. It says `nothing to release` only where that policy
names no release command."

The line for record 0081: "Amended in part by record 0112, 2026-11-11:
`sluice release` after an equip finds nothing of the equip's, and runs
the policy's release command as in any worktree sluice holds nothing
for. It says there is nothing to release only where the policy names no
release command."

**1.2 Rules.** In core.10's details, after "releasing a place without
running its release command", add: "(2026-11-07 to 2026-11-09:
`sluice release --worktree` on worktrees sluice held nothing for
answered `nothing to release`, exit 0, with no release command run,
and a unit's emulators, started by its tests outside `sluice exec`,
outlived the places crew's tidy removed after it, signal
2026-11-11-agentplot-planner-b1e1f248/01; record 0112)". Add to its
"Not ruled out": "`nothing to release` in a worktree sluice holds
nothing for whose policy names no release command, or in a folder in no
worktree of the repository (record 0112)". Add 0112 to its ADRs.

In core.46's details, the "Not ruled out" entry becomes: "the release
command run under the toolchain alone, since the working copy was
equipped by the run it releases (95), or, in a worktree sluice holds
nothing for, by the work that started what the release ends (record
0112), and a gone working copy's release entering none (123)". Add 0112
to its records. Then `node site/scripts/rules.mjs`.

**2.3 The one test.** `an_unheld_worktrees_release_runs_its_release_command`
in `crates/sluice/tests/exec.rs` uses `Setup::new` and a `Scratch` for
the marks, and runs no `sluice exec` in the worktree.
1. `s.policy` sets `main`'s policy to `POLICY` with `release = "sh
   <marks>/release.sh"`. `release.sh` appends `pwd -P` to
   `<marks>/released`. `sluice release` in the worktree exits 0. Its
   output begins `released <canonical worktree>`, and `released` holds
   that path once. `sluice status` in the repository says `no worktree
   holds ports or runs here`.
2. `s.policy` sets `release = "sh <marks>/fail.sh"`. `fail.sh` echoes
   `emulator still answers` and exits 3. `sluice release` exits 1 with
   those words, and `sluice status` says the same as above.
3. `s.policy` adds `not_a_key = true` under `[tests]`. `sluice release`
   exits 1 and its output names `main`.

**2.2 The CLI test.** In `release_runs_the_policys_release_command_once`,
the release after the first, and the `--json` one after that, each run
`sh teardown.sh` again. Each says `released unit/a: …` with
`{"worktree":"unit/a","released":true}`, and `session_releases` grows by
one each time. `unit/none`, no worktree of the fake world, still has
nothing to release. The test's doc says "a release runs the policy's
release command each time it is asked".

**3.1 The site.** Each page says it in the reader's words (rule core.26):
- `reference/commands.md`. The commands table's `sluice release` row
  adds that in a worktree sluice holds nothing for it runs the default
  branch's release command all the same. "Worktrees sluice holds" gains
  a paragraph: such a worktree, one where no `sluice exec` ran or whose
  place was released, is released all the same, with no ports, and
  sluice keeps nothing for it after. It exits `0` when the command
  passed and `1` with what it printed when it failed, and says `nothing
  to release` only when the policy names no release command. So a
  driver that removes the worktree after `sluice release` ends what the
  worktree's own commands started. The `sluice equip` paragraph's last
  sentence says `sluice release` there runs the policy's release
  command, as in any worktree sluice holds nothing for. The `--json`
  table's `released` is `false` when the policy names no release
  command or the folder is in no worktree. The exit-code paragraph adds
  a policy that could not be read to what exits `1`.
- `reference/configuration.md`'s `release` row: it runs for a worktree
  when `sluice release` runs in it, whether or not sluice holds anything
  for it, and in one it holds nothing for, the default branch's command
  runs, with no ports.
- `guides/worktrunk.md`'s "What the agent's tests leave running": the
  release command runs even where the agent never ran `sluice exec`, so
  what its own commands started there ends too, and a release that
  fails keeps the worktree.
