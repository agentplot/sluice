## Why

sluice's branch protection refuses any other hand's write to a branch it
guards, and to its copy of a branch on a remote (124, `protect.rs`). It
logs each refusal as a `protected` line in the branch's log, with the
command that tried and both revisions. Nothing of sluice's shows those
lines. On deneb, swancloud's views timer ran `git -C <repository>
update-ref refs/heads/main <new> <old>` every few minutes. Its writes were
refused 5920 times in swancloud's records, 2787 times in strategy's and
443 in weir's, and the Boomdrive runner's many more. Each refusal was a
line, and no one saw any of it for days. swancloud's fix had its doctor
`grep` sluice's `log.jsonl`.

The user, 2026-11-05, reviewing swancloud's fix, chose that sluice shows
refused writes itself and that swancloud's doctor reads them from sluice,
not from sluice's log file. This change is sluice's part. Its clauses are
148 and S73, and 124's and C.4's amendments, on agentplot/blueprints
main at 539f745, which the user approved on 2026-11-05. Record 0082 sets
the design down, accepted on that approval, and rule core.47 binds it.
Both are written with this change.

## What Changes

- **Once per cause and host.** A refused write's cause is the branch's
  ref, the host whose ref was tried, and the command that tried it, with
  each full object name in it read as `<revision>`. Its first refusal is
  logged as `protected` today is, now naming the host. Each later refusal
  of a standing cause writes no line: it counts one more (148).
- **Counted in the branch's state.** `branch.json` gains
  `refused_writes`: for each standing cause its `host`, `ref`,
  `command`, `times`, `first`, `last`, and the last refused write's
  `from` and `to`. The hook writes it in the write it makes today, under
  the branch's lock. Nothing is counted from the log, so a part's close
  (132) leaves the count whole (131; rule core.41).
- **A day, then its end.** A cause stands until a day passes with no
  write of it refused. Then it is shown no more, and the first write of
  the branch's records after logs `protected-ended`, with the host, the
  ref, the command, the count, the first and the last, at the time it
  ended. A write of it refused later opens it anew. A host keeps at most
  twenty causes standing per branch, and a twenty-first ends at once the
  one whose last refusal is oldest.
- **`sluice status <branch>`** says above the queue, for each standing
  cause, newest first, the command, its host, how many times, the first
  and the last, and the last write's revisions, then what to do. `--json`
  carries the same under `refused_writes`, `[]` when none stand.
- **`sluice overview`** gives each branch, under its line, a line per
  standing cause, at most three, then how many more and `sluice status
  <branch>`. `--json` gives each branch `refused_writes`, as status lists
  them. The host's reader writes it into its document, so every watch and
  follower reads it there (129).
- **The doctor reads sluice.** The site and the setup skill tell an
  operator, and a tool that watches the host, to read
  `branches[].refused_writes` from `sluice overview --json`, never
  `log.jsonl`.

## Capabilities

### New Capabilities

- `refused-writes`: how a write the protection refused is counted once
  per cause and host in the branch's state, shown by `sluice status` and
  `sluice overview` until a day passes with none, and ended in the log.

### Modified Capabilities

- `branch-protection`: "A refusal is recorded with the command and both
  revisions" records the first refusal of a cause, naming the host, and
  counts the rest.

## Out of scope

- **swancloud's doctor reading `sluice overview --json`.** Its
  `sluice-refused.sh`, on swancloud's `fix/the-views-timer-moves-anchors-only`
  branch, reads `log.jsonl`. Moving it to `branches[].refused_writes` is
  swancloud's change. It waits on this change's release, and is told to
  the orchestrator.
- **The plugins.** The Claude Code board and the herdr status popup show
  nothing of refused writes. Showing them is a later change that reads
  `refused_writes` from the overview's document, as `held_checkouts` is
  read.
- **Who ran git.** The cause is git's command. The process that ran git,
  a timer's script or an agent's shell, is not recorded. Naming it waits
  on a caller that the command alone does not tell apart.
- **The hook's write on a branch whose records are on its remote.** It is
  a push, as today. Keeping the count in the host's state and carrying it
  into the next write, as 133 keeps a refused read, waits on a host that
  meets the push's cost.
- **The log in parts.** 132 is not built yet (`finished-submissions`,
  group 2). This change counts nothing from the log, so it holds before
  and after that group.
- **Flume's `refused_acts`.** Flume lists each refused act once per
  refusal, for as long as its start lives. That is told to the
  orchestrator. It is not changed here.

## Impact

- `crates/sluice-core/src/refused_writes.rs` (new): the cause, the
  command read with `<revision>`, standing, ending and the bound.
- `crates/sluice-core/src/records.rs`: `BranchDoc::refused_writes`,
  `RefusedWrite`, `Event::Protected`'s `host`, `Event::ProtectedEnded`.
- `crates/sluice-core/src/line.rs`: `Line::refuse_write` and
  `Line::end_refused_writes`.
- `crates/sluice-core/src/lib.rs`: `Sluice::carry` ends the causes past
  their day in any write that changes something.
- `crates/sluice-core/src/ops.rs`: `record_protected` through the cause;
  `Status::refused_writes`.
- `crates/sluice-core/src/overview.rs`: `BranchView::refused_writes`,
  kept to the causes standing when the view is read.
- `crates/sluice/src/lib.rs`: `write_status` and `status_json`.
- `crates/sluice/src/overview.rs`: `write_overview`.
- `crates/sluice/src/protect.rs`: the module's doc comment.
- Tests: `crates/sluice-core/src/refused_writes.rs`'s own;
  `crates/sluice-conformance/tests/refused_writes.rs` (new) and S73 in
  `crates/sluice-conformance/tests/scenarios.rs`;
  `crates/sluice/tests/cli.rs`; `crates/sluice/tests/protect.rs`.
- Docs: `site/src/content/docs/reference/commands.md`,
  `site/src/content/docs/guides/worktrees.md`,
  `site/src/content/docs/reference/state.md` where it lists
  `branch.json` and the log's events,
  `plugins/sluice/skills/setting-up-sluice/references/profiles/remote-branches.md`'s
  traps.
- Records and rules: record 0082, rule core.47, the site's rules page.
- Sources: clauses 26, 124, 131, 132, 133, 148, C.4, S73; records 0053,
  0059, 0061, 0078, 0082; rules core.11, core.14, core.25, core.28,
  core.41, core.47.

## Touches

```
crates/sluice-core/src/refused_writes.rs
crates/sluice-core/src/records.rs
crates/sluice-core/src/line.rs
crates/sluice-core/src/lib.rs
crates/sluice-core/src/ops.rs
crates/sluice-core/src/overview.rs
crates/sluice/src/lib.rs
crates/sluice/src/overview.rs
crates/sluice/src/protect.rs
crates/sluice-conformance/tests/refused_writes.rs
crates/sluice-conformance/tests/scenarios.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/protect.rs
site/src/content/docs/reference/commands.md
site/src/content/docs/guides/worktrees.md
site/src/content/docs/reference/state.md
plugins/sluice/skills/setting-up-sluice/references/profiles/remote-branches.md
docs/adr/0082-a-write-the-protection-refused-is-shown-once-per-cause-counted-in-the-branchs-document-until-a-day-passes-with-none.md
docs/architecture/core.md
site/src/content/docs/architecture/rules.md
openspec/changes/sluice-shows-the-writes-its-protection-refused/**
```
