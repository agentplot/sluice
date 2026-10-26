# Design

## Context

See proposal.md for why. The ruling is clause 123 of
`agentplot/blueprints:design/sluice/requirements.md`, read with 26 and
61. This builds on the code of
`a-failing-release-holds-its-place-not-its-branch`, on
`bolt/sluice-lands-only-what-its-records-say`, which must be on this
bolt before it is built. What that code gives:

- A combination's checkout whose release fails is kept in the branch's
  state, `BranchDoc.failed_releases`, as `FailedRelease { place, failed,
  tries, at, stopped, host? }` (`crates/sluice-core/src/records.rs`).
  `stopped` is set once the branch's `release_retries` are spent: from
  then on it is held. `sluice status <branch>` prints them through
  `write_failed_releases` and `--json` as `held_checkouts`
  (`crates/sluice/src/lib.rs`).
- A worktree's and a preview's failed release is kept on the place's own
  record on the host, `ReleaseFailure { failed, tries, at, stopped }`
  (`crates/sluice-seams/src/lib.rs`). A worktree's is `stopped` at once,
  since no round tries it again; a preview's once its tries are spent.
  `sluice overview --json` already carries both, as
  `sessions[].release_failed` and `previews[].release_failed`, through
  `worktree_json` and `preview_json`.
- `sluice release <checkout or preview> [--accept-failure]` and `sluice
  release --worktree <path> [--accept-failure]` release one.

Every view the operator watches the queue in reads one document,
`sluice overview --json` (`sluice/overview@1`): the text overview
(`crates/sluice/src/overview.rs`), herdr's popup, dashboard, segment and
tokens (`plugins/herdr/lib/words.jq`, read through `bin/document.sh`),
and the Claude Code board (`plugins/sluice-board/hooks/board.ts`,
`register.tsx`). None of them shows a held place today, and the
document carries no branch's `failed_releases`.

## Goals / Non-Goals

**Goals:**

- One rule for what is shown as held, applied the same in the text
  overview, the herdr plugin and the board: a place whose release failed
  and is `stopped`.
- Every host's overview shows a branch's held checkouts, read from the
  branch's state (rule core.11).
- The views stay readers: nothing they do runs, counts or records a
  release (rule core.14).

**Non-Goals:**

- Changing what is held, when it is tried, or how it is released.
- A view that releases: the views show the command.

## Decisions

### D1. The document carries each branch's held checkouts

`BranchView` (`crates/sluice-core/src/overview.rs`) gains
`held_checkouts: Vec<FailedRelease>`, cloned from
`line.doc.failed_releases` in `branch_view`, always serialized (an empty
list when none), named as `sluice status --json` names them. It carries
the ones still being tried as well, with `stopped` false, so a tool sees
what `sluice status` sees; the views pick the stopped ones (D2).

`sessions[]` and `previews[]` are left as they are: they already carry
`release_failed` with `stopped`, and each names its `branch`.

Alternative rejected: a top-level `held[]` list in the document, every
kind of held place with its commands ready-made. It would repeat what
`sessions[]` and `previews[]` already say, and the plugins already build
sluice's commands in their own words (`sluice submit --check`, `sluice
exec`).

### D2. What a view shows, and how it words it

Each view gathers, for a branch `b`, its held places:

- each `b.held_checkouts[]` with `stopped`: kind checkout, named by its
  `place`, released with `sluice release <place>`;
- each `sessions[]` with `branch == b` and `release_failed.stopped`:
  kind worktree, named by its path (its last segment in the popup and
  the pane, as CHECKING names it), released with `sluice release
  --worktree <path>`;
- each `previews[]` with `branch == b` and `release_failed.stopped`:
  kind preview, named by its `name`, released with `sluice release
  <name>`.

Each says how many times its release failed (`failed once`, `failed 4
times`), the last non-empty line of `failed`, and the command, with
`--accept-failure` to free it without. A checkout whose `host` is set
and is not the document's `host` is said held on that host, and its
command is given as `on <host>: …`.

The words, one form each:

- Text overview, under the branch's line (after `held_line`, the 121
  hold), two lines per place, as the round's own output says a hold:

  ```text
    sluice/try/main/48 is held: its release failed 4 times: usage: scripts/preview up
      sluice release sluice/try/main/48 runs it again; sluice release sluice/try/main/48 --accept-failure frees it without
  ```

  A worktree reads `worktree /work/app/unit-a is held: …` and a preview
  `preview <name> is held: …`. Places whose branch the document does not
  list follow the branches, under a line `<branch> · no records`.
- Popup, dashboard and pane: a `RELEASE FAILED` heading first under the
  branch's heading, above TESTING, since it is what waits on the
  operator; then per place
  `✗ sluice/try/main/48 held · failed 4 times · usage: scripts/preview up`
  in red and, beneath it, dimmed,
  `sluice release sluice/try/main/48 · --accept-failure frees it without`.
- One-line views (segment, band, status line, workspace token): `N held:
  release failed`, last on the line.

The heading and the one-line words say "release failed" beside "held"
because "held" alone is taken: the footer's `2 worktrees held` counts the
worktrees sluice holds ports for.

### D3. A branch with a held place is shown

The popup's and pane's `shown` and the band's and segment's `busy` take
a branch with a held place as having something to show. In the band and
segment, its line with nothing queued reads `sluice main · 0
submissions · 1 held: release failed`. The status line, which today
returns nothing while nothing is queued and the session has nothing,
returns `sluice: N held: release failed` when the repository has held
places. The workspace token is set for them in the same way.

### D4. Reading an older or newer document

herdr's `bin/document.sh` normalizes each document before anything reads
it: it makes `branches[].held_checkouts` an empty list when absent,
keeps only objects with a string `place`, and, before it strips control
characters (which joins a multi-line `failed` into one), replaces each
`failed`, the branch's and each `release_failed.failed`, by its last
non-empty line. The board's types mark `held_checkouts` and
`release_failed` optional, and its functions read them with `?? []`, so a
sluice older than this change draws as before.

### D5. Counting

A branch's count is its held places by D2. The repository's (status
line, workspace token) is their sum over its branches and the places
whose branch has no records. herdr may hold documents of one repository
from two hosts, each listing the same branch's checkouts from the shared
state: the workspace token counts a checkout once by its `place`.
Worktrees and previews are each host's own and are counted per document.

## Risks / Trade-offs

- [The last line of `failed` is not the cause] → it is what the release
  command printed last, usually its error; the full text stays in
  `sluice status <branch>` and `--json`, which the commands page points
  to.
- [Two units edit the overview at once] →
  `the-overview-names-remotes-sources-and-flaky-runs` adds fields to the
  same document and table; both only add, so a settle merges them
  side by side.
- [A long checkout name in the segment] → the segment says a count, never
  a name.

## Migration Plan

None: `held_checkouts` is added to `sluice/overview@1`, whose fields are
only ever added, and each plugin reads a document without it as before.
