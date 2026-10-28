# Design

## Context

The statement's clause 26, amended on 2026-10-27 with 23, says that
wherever sluice shows an entry to a person, its number and its subject
stand together, the branch where it gave none. Record 0064 gives a
submit a subject, and the bolt `a-pull-request-is-titled-by-what-lands`
builds it: `Submission.subject: Option<String>` in
`crates/sluice-core/src/records.rs`, the submitter's words as written,
and, where there is none, the submitted branch, which the landing commit
reads as `sub.from.worktree` (`crates/sluice-core/src/text.rs` on that
bolt). That bolt is not on `main` as this change is written, and this
bolt was cut from `main`.

What the views do now, on `main`:

- `sluice status` (`crates/sluice/src/lib.rs`, `write_status` and
  `detail`): a row is `#<id>  <state>  <agent>  <worktree>   <detail>`;
  the detail names other submissions through `combination()` and
  `hashes()`; the combination-under-test lines come from
  `progress::lines`/`progress::testing_line`, which `sluice wait` and a
  waiting submit share.
- `sluice overview` (`crates/sluice/src/overview.rs`): testing lines
  through `progress::testing_line`; recent lines through `recent_line`,
  from the log's `landed`, `blamed` and `conflict` lines, which carry
  numbers only. `BranchView` (`crates/sluice-core/src/overview.rs`)
  carries the in-chain `submissions` whole and `recent_from`, id to
  worktree, read from the live records or the finished index.
- The home view (`crates/sluice/src/home.rs`): `#<id> to <branch> ·
  <state>`; its JSON's `submission` is a hand-built object.
- The herdr popup and segment (`plugins/herdr/bin/status.sh`,
  `bin/segment.sh`) draw from `lib/words.jq`: `rows`, `branch_line`,
  `combination`, `waiting` and `recent_words`, over the overview's JSON.
- The finished index (`FinishedLine`: id, state, worktree) is what a
  reader holds of a submission moved to the finished records; the
  overview's snapshot reads the index, never the finished documents.

## Goals / Non-Goals

**Goals:**

- One way of naming a submission in the four views, held in one place in
  Rust and one in jq.
- A submission landed and moved to the finished records is still named
  by its subject.
- Records, index lines and documents from before subjects read as
  before.

**Non-Goals:**

- The wait's and the submit's lines, the Claude Code board, the herdr
  pane tokens and toasts (see the proposal's out-of-scope list).
- Giving a subject at submit, which the other bolt builds.
- Rewriting the log's lines: `recent` stays as written.

## Decisions

### The record's field is the other bolt's, added here only where absent

This change reads `Submission.subject` exactly as the bolt
`a-pull-request-is-titled-by-what-lands` defines it: `Option<String>`,
`#[serde(default, skip_serializing_if = "Option::is_none")]`, its doc
comment `23: the submitter's words for what lands, which its landing
commit names it by.`, after `held`. Where the tree the code stage works
on lacks it, the first task adds those same lines, so the two bolts'
hunks are one at merge and nothing else of that bolt is built here.
Tests set the field on a record directly; no submit gives one in this
bolt until the other lands.

Alternative: wait for the other bolt to land first. The views need
nothing of it but the field, and a submission with no subject is named
by its branch, which every record already has.

### One name, read through one function

`Submission::name(&self) -> &str` in `records.rs`: the subject where
there is one, else `from.worktree`. `FinishedLine` gains `subject:
Option<String>` (serde default, skipped when none) written when a
submission moves to the finished records (`Line::changes`), and the same
`name()`. A `shown(name) -> String` in `crates/sluice-core/src/text.rs`
replaces each run of control characters with one space and trims; every
Rust view calls it on what it prints, never on what it records or
prints as JSON.

The fallback is the worktree's branch, `from.worktree`, as the landing
commit's `{from}` is, never the target branch `s.branch`: clause 23
names "its branch", the branch submitted.

Alternative: put the fallback in each view. Four places would each
decide what "no subject" reads as; the landing commit already decided.

### A line names each submission once

Each line keeps a set of the numbers it has named. The first time a
number appears it is written `#<id> <name>`; again on the same line,
`#<id>`. A status row starts its set with its own number, so its detail's
`main + #12` stands alone; a combination's other submissions are named.

In Rust: a small `Names` lookup, number to name, built per view, and a
`named(ids, names, said)` that returns `#12 Batch review cards, #13
unit/forecast` and records what it said. `hashes()` and `combination()`
in `lib.rs` take it; `combination()` writes `main + #12 <name> + #13
<name>`. `progress::testing_line` and `progress::lines` take an
`Option<&Names>`: status and overview pass theirs, `sluice wait` and the
waiting submit pass `None` and print as before.

Alternative: subjects only on rows, numbers alone in combinations. The
overview's text and the segment have no rows; under that rule they would
name nothing, which is what clause 26 rules out.

### Where each view finds its names

- **`sluice status <branch>`/`<id>`**: the live records' submissions
  (`Status.submissions`), and the finished ones a row's detail names.
  `Status` (`crates/sluice-core/src/ops.rs`) gains `named: BTreeMap<u64,
  String>`: each finished submission's name from the finished index,
  which the status already reads to count them. With `--all`, the
  history's records give theirs directly.
- **`sluice overview`, text**: `BranchView.submissions` for the chain;
  `recent_subjects` then `recent_from` for the recent lines.
- **`sluice overview --json`**: `BranchView` gains `recent_subjects:
  BTreeMap<u64, String>`, built beside `recent_from` from the same ids,
  holding only the submissions with a subject. A separate map keeps
  `recent_from` meaning what it means and lets a reader tell "no
  subject" from "a sluice that does not say".
- **The home view**: its own submission; `submission_json` gains
  `"subject": s.subject`.
- **herdr**: `words.jq` gains `name_of($b)`: for a number, the in-chain
  submission's `.subject // .from.worktree`, else `$b.recent_subjects`,
  else `$b.recent_from`, else nothing, and `named($b)` writing `#<id>
  <name>`, control characters replaced as in Rust. `combination`,
  `waiting`, `recent_words` and the QUEUED rows take `$b`. A document
  without `recent_subjects` or `recent_from` (an older sluice) names
  recent submissions by number alone.

### The QUEUED and status rows pad their names

The name is a column: padded to the widest name the list shows, so the
columns after it stay aligned. A long subject makes the row long; the
popup already cuts a row at the pane's edge, and the status prints it
whole.

### The herdr words leave the board's behind

`words.jq` says its words are the board's (`plugins/sluice-board`). This
change moves the popup and the segment ahead of the board; the header
comment says so until the board's change catches up.

## Risks / Trade-offs

- [The other bolt defines the field differently by the time both land] →
  The task copies its definition from that bolt's branch at the time of
  coding; the verify stage compares them.
- [A subject carries terminal escapes] → Control characters are replaced
  in every text view and in the popup before painting; tests cover it.
- [Long lines] → The text views print a long subject whole; the popup
  and segment are cut by herdr at their edge, as any long line is now.
- [Tests asserting today's lines] → The CLI's status, overview and home
  tests and the herdr frame fixture (`tests/docs/board.frame`) change
  with the views; the site's examples change with them.

## Migration Plan

None: index lines and records without a subject read as having none, and
are named by their branch. The JSON documents only gain fields.
