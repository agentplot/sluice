# Proposal

## Why

A person reading sluice's queue sees `#217 testing` and `landed #12, #13
as c41e8f2`: numbers, which say nothing of what is being tested or what
landed until each is looked up. The statement now says that wherever
sluice shows an entry to a person, its number and its subject stand
together, the branch where it gave none (26, as amended 2026-10-27 with
23). A submit can give a subject (23; record 0064), so the words a reader
needs are on the record; the views do not show them.

## What Changes

- **`sluice status`**: a branch's rows, and the one row of `sluice status
  <id>`, read `#14 Forecast concurrent units  testing  …`: the number,
  then its subject, or its branch (the worktree's, as `from.worktree`
  names it) where the submit gave none. The submissions a row's detail
  names, in a combination, `after`, `with` or `held for`, carry theirs
  too. The lines naming a combination under test do the same.
- **`sluice overview`**: the lines naming a combination under test, and
  the recent `landed`, `blamed` and `conflict` lines, name each
  submission with its subject: `landed #12 Batch review cards, #13
  unit/forecast as c41e8f2`.
- **`sluice` with no command** (the home view): `#14 Forecast concurrent
  units to bolt/concurrent-units · testing`; its `--json` submission gains
  `subject`, the submit's words or null.
- **`sluice overview --json`**: each branch gains `recent_subjects`, the
  subject of each submission its recent lines name whose submit gave one,
  beside `recent_from`, so a reader can name a landed or blamed
  submission whose record has moved to the finished ones.
- **The herdr popup and the tab-bar segment**: every submission the popup
  draws, under TESTING, QUEUED and RECENT, and the segment's combination,
  waiting list and last event, carry the same words.
- One rule for all of them: a line names each submission once, its
  number and its subject together the first time the line shows it; a
  subject is drawn on one line, its line breaks and other control
  characters each run shown as one space.
- The finished index keeps each submission's subject as it moves there,
  so a submission landed and moved is still named by it. A record or an
  index line written before subjects were kept reads as before, named by
  its branch.

Nothing a script reads is renamed or removed: the JSON documents only
gain fields.

## Capabilities

### New Capabilities

- `submission-names`: how sluice's own text output (status, overview, the
  home view) names a submission to a person: its number with its subject,
  or its branch where it gave none, once per line.

### Modified Capabilities

- `overview`: `sluice overview --json` names the subjects of the
  submissions its recent lines name.
- `herdr-status`: the popup's TESTING, QUEUED and RECENT rows and the
  tab-bar segment name each submission with its subject.

## Impact

- `crates/sluice-core`: the submission record's `subject` (as the bolt
  `a-pull-request-is-titled-by-what-lands` defines it; this change adds
  the same field where the bolt's tree lacks it), the finished index's
  line, the overview's branch view and the branch's status.
- `crates/sluice`: `home.rs`, `overview.rs`, `progress.rs` and the status
  command in `lib.rs`; the CLI tests.
- `plugins/herdr`: `lib/words.jq`, which `bin/status.sh` and
  `bin/segment.sh` draw from, and its tests and README.
- The site's command reference and its herdr and walkthrough guides,
  where they quote these lines.

Out of scope, each waiting on a later change the planner places:

- The submit's and `sluice wait`'s own lines (`testing main + #38 + #40`),
  which this change leaves as they are.
- The Claude Code board (`plugins/sluice-board`), whose `/sluice` pane and
  band the herdr popup and segment are worded after, and the herdr pane
  tokens and toasts (`bin/tokens.sh`, `bin/document.sh`): until a change
  of their own, they name submissions by number alone.
- Giving a subject at submit, which the bolt
  `a-pull-request-is-titled-by-what-lands` builds.

## Touches

```
crates/sluice-core/src/records.rs
crates/sluice-core/src/line.rs
crates/sluice-core/src/overview.rs
crates/sluice-core/src/ops.rs
crates/sluice-core/src/text.rs
crates/sluice/src/home.rs
crates/sluice/src/overview.rs
crates/sluice/src/progress.rs
crates/sluice/src/lib.rs
crates/sluice/tests/**
plugins/herdr/lib/words.jq
plugins/herdr/tests/**
plugins/herdr/README.md
site/src/content/docs/reference/commands.md
site/src/content/docs/guides/herdr.md
site/src/content/docs/guides/walkthrough.md
```
