# Design

## Context

`sluice overview --json` is built by `overview::branch_view` from one
`BranchRead`: the branch's snapshot (its `branch.json`, live
`submissions/*.json`, `finished.jsonl` and `log.jsonl`) and the `Rules`
its policy gives it at the revision its records last saw (rule core.15;
ADR 0036). `recent` is the last ten `landed`, `blamed` and `conflict`
lines of the log, verbatim. It reads nothing else and writes nothing
(rules core.11, core.14).

A flaky failure is a `FailedTask` whose `flaky` is a `FlakyKey { here,
other: Recorded }`. `verdict::read` builds it from the newest recorded
result line under the key whose outcome differs from this run's (19, 69),
and the step then fills `other.log` with `kept_log` (20). The step
records this run's result lines (`ResultLine::of`, with `at` = the step's
`now` and `host` from `Source::Ran { host }`) before it reads the verdict
(16; rule core.7). `FailedTask` is stored on the candidate and on the
blamed submission, so it reaches both the overview's `candidates[].failed`
and `sluice status <id> --json`.

## Goals / Non-Goals

**Goals:**
- The three fields the board needs, in the overview's document, from
  what is already read or recorded.

**Non-Goals:**
- Any change to the overview's text form, `sluice status`'s text, the
  forge's report of a flaky check, or a verdict.
- A new read: `recent_from` and `remote` cost the overview nothing more
  than it reads today.

## Decisions

### `remote` is the policy's name, read where the gate is read

`BranchView.remote: Option<String>` is `rules.remote` of the `Rules`
`branch_view` already has, serialised with `skip_serializing_if =
"Option::is_none"`. A branch with no policy at its recorded revision has
none, as a branch with no remote has none.

- Not the URL (`git remote get-url`): a URL can carry credentials, and
  resolving it is a git call the overview has never made. The board
  resolves the name itself and decides whether it is a forge it links
  to, so the core names no forge (rule core.1).
- "Kept in one host's files" in the intent is a branch with no remote:
  its revisions live only in this host's repository. A branch with a
  remote whose records are in the host's own files still lands on the
  remote, so it names it.

### `recent_from` from the live submissions and the finished index

`BranchView.recent_from: BTreeMap<u64, String>`, always present
(`{}` when empty), serialised with the ids as JSON object keys. For each
id a `recent` line names (`ids` of `landed`, `id` of `blamed`, `id` and
`with` of `conflict`), the worktree is `line.subs[id].from.worktree`, or
else `line.finished[id].worktree`. `Line::load` already parses both from
the snapshot, and `FinishedLine.worktree` is written from
`s.from.worktree` (`line.rs`, the finished move), so the two agree. An id
found in neither, which only a hand-edited record could give, is left
out.

- A conflict's `with` names submissions of the same branch (the log's
  `Event::Conflict { with: Vec<u64> }`), so the branch's own records hold
  them.
- `recent` stays verbatim (the `overview` change's "The overview's
  document"); the map sits beside it rather than inside its lines.
- Not `from.path` or the whole `Submitter`: the board needs `pull/<n>` to
  find a pull request, and `from.worktree` is the one field every
  submission has, live or finished; the finished index keeps no more.

### Flaky times and hosts copied from recorded lines

`FlakyKey` gains `at: Option<String>` and `host: Option<String>`;
`Recorded` gains `host: Option<String>`. All are `#[serde(default,
skip_serializing_if = "Option::is_none")]`, so a record written before
them reads with them `None` and is written back without them.

- `verdict::read` sets `other.host` from the contradicting
  `ResultLine.host`, beside `at`, `branch` and `combination`. It leaves
  `FlakyKey.at` and `host` `None`: `read` has this run's report, not its
  recorded line.
- The step, in the loop that fills `other.log`, sets `k.at` and `k.host`
  from this run's recorded line for the task: the line in `results` on
  this branch and candidate revision with the task's name and hash, of
  the kind this run reported (`Ran` or `Replayed`), newest by `at`. That
  is the line the step recorded at `now`, or the one already recorded
  when `same_as` kept it from being written twice. A replay's line has no
  host, so `host` stays `None`; a check with no line (a signed branch's
  result the tree did not prove) gets neither.
- Copying from the recorded line, not from `now` and `Source::Ran`,
  keeps the rule that what a verdict shows comes from what was recorded
  before it (rule core.7), and gives the time the results log shows.

## Risks / Trade-offs

- [The newest matching line could be another run's on the same
  combination, after the step re-polled a run it had already recorded]
  → it is then the same run's result recorded once (`same_as`), so its
  time and host are this run's.
- [`recent_from` grows the document every line] → at most ten lines,
  each naming a few ids; the watch's change test compares it with the
  rest, and it changes only when `recent` does.
- [A reader that compared documents field by field sees new fields] →
  fields are only ever added to `sluice/overview@1`; the herdr and board
  plugins ignore fields they do not read.

## Migration Plan

None. Old records read with the new fields left out; new records carry
them from the first verdict after the change lands.
