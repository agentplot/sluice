# Proposal

## Why

Every `sluice exec` takes a run from the host's bound, the same bound the
queue's combinations are started under (71; session-runs-1 clause 89). So
a host can show "testing 2 at once (up to 3)" with three submissions
queued while an agent's checks hold the third run, and nothing on screen
says why the queue is slow. The board and the herdr plugin also list
branches in name order, where the policy names them in an order of its
own, and herdr's tab-bar strip is gone from the operator's hosts because
its setup line names a path that differs between a Mac and a Linux box.
This change is items 12 and 13 of the shared overview contract.

## What Changes

- While `sluice exec` runs, it keeps its run's tasks, and its command,
  on its record in the host's state, beside the count the bound reads
  (session-runs-1 C.3). They change no branch's records, and no gate,
  round or verdict reads them.
- `sluice overview --json` gives each entry of `sessions[]` whose
  worktree has a run going a `run`, `{started, command?, tasks[]}`,
  and the document a top-level `room`, `{bound, queue, sessions,
  reserve}`: the host's bound, the combinations and the session runs
  counted against it, and the part kept from session runs (91). Both
  are added fields of `sluice/overview@1`.
- A worktree's `last_result` gains `tasks[]`, the finished run's tasks,
  and `reused`: whether its passes are what the gate will take when that
  head is submitted (93).
- `branches[]` comes in the order the default branch's policy names
  the branches.
- When agents' checks hold the queue back, the CLI's counts line, the
  board's band and status line, and herdr's band and tab bar say so:
  `testing 2 at once (up to 3) · 1 slot to an agent's checks`.
- The board's pane and herdr's popup gain a CHECKING section, one row
  per worktree with a check going or just finished, and a line for the
  host's runs; herdr's sidebar shows `checking 3/8` on the agent's own
  row. A check is never shown as a queue entry, and is called a check,
  never testing.
- herdr's setup prints the tab-bar command with `$HOME` in place of the
  home directory, so one herdr config serves every host.
- ADRs 0035 and 0036, and Details lines for rules core.11, core.15 and
  core.18.

Out of scope:

- The swancloud change that puts the tab-bar entry back in the herdr
  config it renders (design.md, "swancloud").
- A remote session run's tasks (session-runs-1 clause 96).

## Capabilities

### New Capabilities
- `exec-checks`: a session run's tasks and command kept while it runs,
  the host's room, and both in the overview.
- `branch-order`: the overview's branches in the policy's order.

### Modified Capabilities
- `worktree-last-result`: the result carries its tasks and whether it
  is reused.
- `plugin-board`: the held-back clause, the CHECKING section and the
  host line.
- `herdr-status`: the same in the popup and the tab bar; setup's
  host-independent tab-bar command.
- `herdr-sidebar`: `checking 3/8` on the agent's own row.

## Impact

- **The seams:** `Tests::session_runs` and `HostRun`; `LastResult`'s
  `tasks` and `reused`.
- **The core:** `Sluice::room`, with `session_capacity` on it;
  `finish_session` fills the result's tasks and `reused`; the overview
  orders its branches by the default branch's policy; the policy keeps
  its `[branches]` tables in file order.
- **The CLI:** `sluice exec` keeps its tasks; `sluice overview`'s
  document and text; the counts line of `sluice`, `sluice status
  <branch>` and `sluice overview`.
- **The profiles:** the host's session store, the checks profile and
  the default branch read from git, moved out of the pull-request
  driver; the fakes.
- **`plugins/sluice-board`, `plugins/herdr`:** rendering, setup and
  tests.
- **Docs:** ADRs, `docs/architecture/core.md` and the site's rules
  page, `reference/commands.md`, `reference/configuration.md`,
  `guides/claude-code.md`, `guides/herdr.md`, both plugins' READMEs.
