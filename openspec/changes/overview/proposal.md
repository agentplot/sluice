# Proposal

## Why

Two plugins, one for Claude Code and one for herdr, show sluice's queues
the way a CI server shows its tests. Each needs every branch's
combinations under test, its queued submissions, its latest landings,
blames and conflicts, and the worktrees and previews held, and needs to
redraw when they change. `sluice status <branch> --json` gives one
branch's submissions alone, and its branch form fetches a remote branch
on every read. The rest is in the records' files, which a plugin may not
read past the records seam (rule core.11). A submission's
`from.worktree` holds the branch it came from, so neither plugin can
find its own session's submissions. The operator settled one shared
document for both plugins; this change builds it, and lands before
either plugin.

## What Changes

- `sluice overview` prints every branch of the repository that has
  records, read from the records alone (26, 61): a line per branch,
  its combinations under test, its latest landings, blames and
  conflicts, and a count of the worktrees and previews held.
- `--json` prints one document, `sluice/overview@1`, with `schema`,
  `host`, `repository`, `at`, `branches[]` (each with `at`, `version`,
  `window`, `parallel`, `depth`, `candidates[]`, `submissions[]` and
  `recent[]`), `sessions[]` and `previews[]`. Fields are only ever
  added.
- `--json --watch` prints the document as one line at once and again
  each time anything in it but `at` changes, reading about once a
  second until stopped.
- A submission's `from` gains `path`, the absolute path of the worktree
  it was submitted from, written by the worktrunk driver.
- It writes, locks and releases nothing (rule core.14). A branch is read
  at the revision its records last saw, never fetched.
- ADR 0023, and rule core.1's details naming `plugins/`, where the
  Claude Code and herdr plugins will live.
- Docs: the commands page (`sluice overview`), the state page
  (`from.path`), the README's command table.

Out of scope: a host-wide `--all` and the repository index behind it,
and a preview's `host` with the `{host}` address placeholder (clause 99
amendment). Both wait on the herdr previews panel. The plugins
themselves are later changes.

## Capabilities

### New Capabilities
- `overview`: every branch's queue at a glance, as text, one document,
  or a line per change; and the worktree a submission came from.

### Modified Capabilities
<!-- none -->

## Impact

- The seams: `Submitter` gains an optional `path`.
- The core: `overview::branch_view`, pure over a branch's records, and
  `Sluice::overview`.
- The CLI: `sluice overview`; `exec::Host` gains `name` and
  `repository`; `preview_json` and `worktree_json` are shared.
- The worktrunk driver sets `path`; the pull-request driver leaves it
  out; the fakes leave it out.
- Docs: ADR 0023, `docs/architecture/core.md` and the site's rules page,
  the commands and state pages, the README.
