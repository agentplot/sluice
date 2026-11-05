# Design

## Context

See proposal.md for why, and record 0082 for the decision and the
options weighed. The code reads as follows at main 3f5aed5f, where this
change was cut.

- **The hook.** `crates/sluice/src/protect.rs`, `hook` (line 384): in
  git's `prepared` state, `judge` (line 441) reads the transaction and
  gives each refused write as a `Refused` (line 212): the branch, the
  value its ref holds on the host, the value the write would set or none
  for a deletion, the submissions a deletion would strand, and whether
  the ref is sluice's copy of a branch on a remote. `caller` (line 259)
  reads git's own command line from the kernel. The hook tells the
  caller in `refusal`'s words, then takes the branch's lock, waiting up
  to a second, and calls `Sluice::record_protected`. A refusal whose
  lock does not come within the second is told to the caller and
  recorded nowhere.
- **The line.** `crates/sluice-core/src/ops.rs`, `record_protected`
  (line 1206), appends `Event::Protected { ref, from, to, by }`
  (`crates/sluice-core/src/records.rs`, line 1084) through
  `Sluice::update`. One line per refusal, and no host.
- **The branch's document.** `BranchDoc` (`records.rs`, line 722) is
  `branch.json`. Its `copies` (124) is the nearest kin: one entry per
  host whose copy a round could not level, each host writing only its
  own, read by status for this host.
- **The write.** `Sluice::carry` (`crates/sluice-core/src/lib.rs`, line
  259) writes a line's changes. Where they are not empty, it adds the
  lines 133's kept refusal has not yet written, at the times they
  happened. A write that changes nothing writes nothing.
- **Status.** `Sluice::status` (`ops.rs`, line 1246) builds `Status`
  (line 346) from the live records: `copy` from `copies`, `refused` from
  133's kept refusal. `write_status` (`crates/sluice/src/lib.rs`, line
  4200) shows them above the queue, and `status_json` (line 4338)
  carries them.
- **The overview.** `Sluice::branch_view_of`
  (`crates/sluice-core/src/overview.rs`, line 434) builds a
  `BranchView` (line 54) from the live records through `branch_view`
  (line 212). The host's reader writes the views into its document each
  tick (129), and `sluice overview --json` prints that document.
  `write_overview` (`crates/sluice/src/overview.rs`, line 232) shows each
  branch's line, its hold, its staleness, its held checkouts, its
  combinations under test and its last three recent lines.
- **The log.** `log.jsonl`, one line per event. 132's parts are not
  built yet (`finished-submissions`, group 2). Every reader parses
  every line into `Event`, and an event it does not know fails the
  parse.

### Looking across

- **133, a refused read of the remote** (rule core.28; record 0061),
  is the shape this design keeps: once per distinct cause and host, its
  end logged with how long and how many. It differs in three ways. A
  refused read ends when a read succeeds, but a refused write has no
  such sign, so its end is a day's quiet. A refused read is kept in the
  host's state, since the records may be on the remote that refused it.
  A refused write is kept in the branch's document, which the hook
  already writes. And 133's refusal is shown by status on its host alone,
  where a refused write is shown by the overview too, from the live
  records any host's reader reads.
- **Flume**, clause 108: `flume status` gives each place's
  `refused_acts`, a string per refusal of an agent's act, with no time.
  It reads them from its live start's record and stops showing them
  when that start ends (`crates/flume-core/src/status.rs`,
  `crates/flume-core/src/answer.rs`). A start's life bounds the list, so
  it has no need of a day. It repeats a refusal where sluice counts one.
  That is told to the orchestrator and not changed here.
- **The Boomdrive runner** no longer writes a branch sluice guards (its
  ADR 0019, which turned down passing over the refusal because each
  timer run would log a `protected` line). It prints a line per failure
  per pass, and keeps no count and no doctor.
- **swancloud's doctor**, on swancloud's branch
  `fix/the-views-timer-moves-anchors-only` (7011324e),
  `packages/swancloud-doctor/sluice-refused.sh`, reads
  `<git dir>/sluice/<branch>/log.jsonl`, keeps the `protected` lines of
  the branch's ref since fifteen minutes ago, drops the 40-digit
  revisions from `by`, and names the last. It reads no count. Once parts
  close (132), `log.jsonl` would hold only the current part. This
  design's cause is the same reading of `by`, done once by sluice. The
  doctor's window stays the doctor's, judged on `last`.

## Goals / Non-Goals

**Goals:**

- A refused write shown by sluice, in words and in `--json`: which
  writes, by what command, on which host, how many times, first and last
  (148).
- One line in the log per cause and host, and one at its end, never one
  per refusal (148; rule core.47).
- A count read from the live records alone, which no part's close
  changes (131, 132; rule core.41).
- A fixed caller dropping out a day after its last refused write.
- One place a host's doctor reads: `sluice overview --json`.

**Non-Goals:**

- The plugins' panes and status lines.
- swancloud's doctor's own change.
- Naming the process that ran git.
- Keeping the hook off the network on a branch whose records are on its
  remote.

## Decisions

### D1. A cause is the command, its revisions read as `<revision>`, on its host

`crates/sluice-core/src/refused_writes.rs` (new) gives
`command_of(by: &str) -> String`: every whole word of `by` that is 40 or
64 hexadecimal digits, with no letter, digit or underscore on either side,
written as `<revision>`. Nothing else of `by` changes. A cause is the
triple `(host, ref, command_of(by))`, where `host` is the instance's
name, `Sluice::peer.name`, as on `copy-behind` and `read-refused`.

The timer's `git -C /home/me/app update-ref refs/heads/main 8079db3…
10e4791…` and its next run with other revisions are then one cause,
`git -C /home/me/app update-ref refs/heads/main <revision> <revision>`.
`git reset --hard origin/main` is a cause of its own and names no
revision. An abbreviated revision, `git branch -f main 8079db3`, stays
as written, so two such writes to different revisions are two causes. A
person or an agent writes those by hand, once. A script that runs again
names its revision in full or by a name.

The whole transaction's revisions are not looked up in `by`. A command
can name a revision that is no value in the transaction, such as
`update-ref <ref> <new> <old>` whose `<old>` is not the copy's value,
and every full object name is a revision whatever it names.

### D2. The count is in `branch.json`

`BranchDoc` gains:

```rust
/// 148: each write the protection refused whose cause stands, one entry
/// per host, ref and command; each host writes its own.
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub refused_writes: Vec<RefusedWrite>,
```

```rust
pub struct RefusedWrite {
    pub host: String,
    #[serde(rename = "ref")]
    pub reference: String,
    /// The command, each full object name in it as `<revision>` (D1).
    pub command: String,
    /// How many of its writes were refused while it stood.
    pub times: u64,
    pub first: String,
    pub last: String,
    /// The last refused write's: the value the ref held on the host, and
    /// the one it would have set, `None` for a deletion.
    pub from: Rev,
    pub to: Option<Rev>,
}
```

The JSON a reader sees is the struct as serialized:
`{"host","ref","command","times","first","last","from","to"}`, with `to`
`null` for a deletion. Status and the overview print it in this form, so
one shape stands in the state, in status and in the overview.

### D3. The hook's write counts, and logs only a cause's first

`Line::refuse_write(now, host, reference, by, from, to)` in
`crates/sluice-core/src/line.rs`, in this order:

1. `self.end_refused_writes(now)` (D4), so a cause past its day is ended
   before it is matched.
2. A standing cause of `(host, reference, command_of(by))`: `times += 1`,
   `last = now`, and `from` and `to` set. No line.
3. Otherwise a `protected` line at `now`, with `ref`, `from`, `to`, `by`
   as git ran it, and `host`. The cause is pushed with `times` 1 and
   `first` and `last` at `now`. Where `host` then has more than
   `STANDING` causes on the branch, 20, the one whose `last` is oldest is
   taken out and a `protected-ended` line logged for it at `now`.

`Sluice::record_protected` calls it inside `update`, as it logs the line
today. `Event::Protected` gains `host: Option<String>`, `serde(default,
skip_serializing_if = "Option::is_none")`, so a line written before it
reads as before. The hook's lock, its second and what it tells the
caller are unchanged (`protect.rs`). A refusal the lock does not come to
in its second is counted nowhere, as it is recorded nowhere today.

### D4. A day, then its end in the next write

`refused_writes.rs` gives `SHOWN`, a day (86,400 seconds). A cause
stands at `now` while `now < last + SHOWN`.

`Line::end_refused_writes(now)` takes out every cause, of any host, that
no longer stands, and logs for each, at `last + SHOWN`, in that order:

```rust
/// 148: a cause of refused writes that stood a day with none refused:
/// how many it refused, the first and the last.
#[serde(rename = "protected-ended")]
ProtectedEnded {
    host: String,
    #[serde(rename = "ref")]
    reference: String,
    command: String,
    times: u64,
    first: String,
    last: String,
},
```

`Sluice::carry` calls it on its clone of the line once the line's own
changes are not empty, as it adds 133's kept lines there. So any write
that changes something carries the ends: a round's, a submit's, the
hook's. A write that changes nothing still writes nothing. A branch
nothing writes keeps an ended cause in `branch.json`, unshown (D5, D6),
and its end is logged once something writes. Rounds, gates, landings and
submissions read nothing of `refused_writes` (rule core.14). `in_play`
is unchanged, so a cause gives the driver no reason to step a branch.

Any host ends any host's cause. The line names the host it was refused
on, and ending a cause past its day is the same whoever does it.

### D5. Status shows each standing cause above the queue

`Status` gains `refused_writes: Vec<RefusedWrite>`: the document's causes
standing at `Sluice::now`, newest `last` first. `write_status` writes
them after the copy behind and 133's refusal, before the combinations:

```console
$ sluice status main
main at c43609b · 0 in the queue · testing 0 at once (up to 4)
sluice refused writes to main in the last day, by commands that are not sluice's:
  git -C /home/me/app update-ref refs/heads/main <revision> <revision>
    on build-1: 2024 times, first 2026-10-30T05:50:23Z, last 2026-11-02T16:01:20Z, from 10e4791 to 8079db3
sluice sets each host's main to each landing itself: stop what runs these
```

A deletion's line reads `from 10e4791, deleting it`. The command is its
first line, cut after 100 characters with `…`. `--json` carries it
whole. The last line is the copy's, `sluice sets each host's <branch> to
each landing itself: stop what runs these`, where the branch's policy
names a remote. Otherwise it is `only sluice moves <branch>: submit the
change to sluice instead`, the refusal's own words (124). `status_json`
adds `"refused_writes"`, `[]` when none stand.

### D6. The overview shows them under the branch's line

`BranchView` gains `refused_writes: Vec<RefusedWrite>`, `serde(default)`,
so a reader's document from an older sluice reads with none.
`branch_view` takes the document's causes. `Sluice::branch_view_of` keeps
those standing at `Sluice::now`, newest `last` first. The reader builds
the views at every tick, so a cause drops out of its document at the
first tick past its day. `BranchView::unread` has none.

`write_overview` writes, after the branch's held places and before its
combinations under test, a line per standing cause, at most three:

```console
  sluice refused 2024 writes on build-1, last 2026-11-02T16:01:20Z: git -C /home/me/app update-ref refs/heads/main <revision> <revision>
  and 2 more: sluice status main
```

The command is cut as in D5. `--json` is the reader's document, which
gives each branch `refused_writes` whole.

### D7. What a host's doctor reads

A tool that watches a host runs `sluice overview --json` in each
repository and reads `branches[].refused_writes[]`. Where the host runs
`sluice run`, that is its reader's document, and otherwise a one-shot
read of the live records (rule core.25). For one branch it runs
`sluice status <branch> --json` and reads `refused_writes`. It judges
its own window on `last`, and names the caller by `command` and
`host`. It reads neither `log.jsonl`, which holds one line per cause
and, once 132 is built, only the current part, nor `branch.json`, whose
form is sluice's. The site's commands page says so under the overview's
fields, and `remote-branches.md`'s traps tell the operator to.

### D8. The log's two events on the state page

`site/src/content/docs/reference/state.md` gains, after `read-again`:
`protected`, logged once for each command a host's protection refused,
with `host`, `ref`, `from`, `to` and `by` as git ran it; and
`protected-ended`, logged a day after the last of them, with `times`,
`first` and `last`. Under `branch.json`, it gains `refused_writes`. Both
events are shown as JSON in its usual form.

## Tests, and where each scenario is held

- **Unit**, in `refused_writes.rs`: `command_of` writes a 40-digit and a
  64-digit word as `<revision>`, leaves a 7-digit one, a 41-digit one and
  one inside a longer word, and changes nothing else; a cause stands one
  second before its day and not at it.
- **Conformance**, `crates/sluice-conformance/tests/refused_writes.rs`
  (new), over the fakes with `World::advance` and `Sluice::as_peer`, calling
  `Sluice::record_protected` as the hook does: one line for a cause
  refused three times, with `times` 3, `first`, `last` and the last
  revisions; the same command with other full revisions is one cause;
  two hosts are two causes; the log has no `protected` line from the
  second refusal on; a cause refused again 23 hours after its last still
  stands; a day after its last, status shows none, and the next write
  logs `protected-ended` at `last` and a day, with its `times`; a
  refusal after its end logs `protected` anew; a twenty-first cause ends
  the oldest at once; the count and status read the same with the log's
  lines taken away, as a part's close takes them (132); no round, gate
  or landing differs with causes standing (rule core.14).
- **S73**, `crates/sluice-conformance/tests/scenarios.rs`,
  `s73_a_timers_refused_writes_are_one_command_counted_and_shown_until_a_day_after_the_last`.
- **The CLI**, `crates/sluice/tests/cli.rs`, over the fakes: status's
  text and `--json`, a deletion's words, the copy's last line and the
  local branch's; the overview's lines, the three and `and n more`, and
  `--json`; nothing shown a day after the last.
- **The hook against git**, `crates/sluice/tests/protect.rs`: the
  timer's `git -C <repository> update-ref refs/heads/bolt/x <new> <old>`
  run twice with different revisions gives one `protected` line naming
  the host, `branch.json` one cause with `times` 2, and `sluice status
  bolt/x --json` and `sluice overview --json` the same. `log.jsonl`
  moved aside to `log/1.jsonl`, as 132's close moves it, changes neither.
  `every_way_git_moves_a_protected_branch_is_refused` still asserts a
  line for each git command, since each is a cause of its own.

## Risks / Trade-offs

- [A caller run once a day ends and opens again each day] → two lines a
  day, and it is shown all the while. A day keeps a caller of every few
  minutes or every hour one cause, and drops a fixed one the next day.
- [A command that names something new each time, a commit's message, is
  a new cause each time] → it opens a line each time, as today. The
  bound of twenty keeps `branch.json` small, and the cause put out early
  logs its count.
- [An older sluice reads a log holding `protected-ended`] → it fails to
  parse the line, as it fails on any event it does not know. The hook
  runs the sluice that installed it, and the next command in each
  repository re-installs it from the newer one (124). On one host the
  release is one binary. On a branch whose records are on its remote,
  every host that reads it runs this release before an end is written,
  a day after the first refusal at the earliest.
- [The hook cannot have the lock within its second] → the refusal is
  told to its caller and not counted, as it is not recorded today. A
  caller refused every few minutes is counted on the next.
- [Clocks differ between hosts on a branch whose records are on its
  remote] → a cause may end a few seconds early or late by another
  host's write. The day dwarfs it.
- [The hook's write on such a branch is a push] → as today. Out of
  scope.

## Migration Plan

Nothing moves. `branch.json` from an older sluice has no
`refused_writes` and reads with none. Its `protected` lines have no
`host` and read as before. They are counted nowhere, since the count
starts with this release, so a caller refused before the upgrade is
shown from its first refusal after. An older sluice reads a `branch.json`
holding `refused_writes` and ignores it. Rolling back leaves the field
unread and the `protected-ended` lines unparseable to the older sluice,
the risk above.

## Open Questions

None. The day and the twenty are this stage's own, set down in record
0082 with the paths not taken.
