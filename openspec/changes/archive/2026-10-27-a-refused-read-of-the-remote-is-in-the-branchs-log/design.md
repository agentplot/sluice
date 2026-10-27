# Design

## Context

A branch whose policy names a `remote` is read by a fetch on every
call of the branch seam's `read` (record 0017): `PushedRefs::fetch`
runs `git fetch` through `remote_git` and, on a failure other than a
missing ref, bails `fetching <branch> from <remote> failed: <stderr>`.
Since the chore `a-fetch-with-sluices-token-is-never-silently-refused`
(c5fca2f8), `remote_git` runs a fetch the remote refused on the
credential once more on a fresh one, and a refusal that stands has
sluice's note on each refused credential, its age among it, appended
to git's stderr (`told`). The round's error carries that text to
whoever ran it: a waiting submit or `sluice wait` prints it. The
branch's log records nothing.

A branch whose records are on its remote reads them through
`RemoteRefs::read_remote`, which fetches `refs/sluice/state/<branch>`
through `PushedRefs::fetch_ref`. When that fetch fails and a fetch made
earlier is on the host, the read answers from it, warns, and marks the
branch stale (80); `State::stale` names the remote, and `sluice status`
says it could not refresh. A round carries on from that stale read
today. `fetch_heads` and `fetch_candidates` fetch the records' other
refs (C.6) and bail on failure.

The log already holds two stalls of the same shape, each logged once
per distinct cause: `held` (121, `decide`) and `copy-behind` (124,
`note_copy`; record 0053). The host already keeps state of its own that
decides nothing: the version of the records it last saw and when
(`State::observed`, 73), under
`<git-dir>/sluice-remote/<remote>/observed/<branch>.json`.

## Goals / Non-Goals

**Goals:**

- The core learns that a read of the remote failed by its type, through
  the seams alone, naming no profile (rules core.1, core.2).
- One line per cause and host while a refusal stands, one when it ends,
  whether the records are on the host or on the refusing remote (133).
- Nothing a round, gate, landing or submission decides reads the kept
  refusal (rule core.14).

**Non-Goals:**

- Telling a refusal of the credential from a network failure or a
  remote's 5xx: every read the round could not make is logged alike,
  with git's words saying which it was.
- A second try of a read or a push: the chore and the unit
  `a-push-the-credential-stopped-is-made-once-more` own those (134).

## Decisions

### D1. A typed error for a read of the remote that could not be made

`sluice-seams` gains `ReadRefused { remote, asked, said, note }` and
`read_refused(&anyhow::Error) -> Option<&ReadRefused>`, beside
`Unanswered`, found in the error's chain as `unanswered` finds its
type. `asked` is the ref fetched, `refs/heads/<branch>` or the records'
ref; `said` is what git printed on stderr, trimmed of trailing
whitespace and otherwise unchanged; `note` is sluice's own words on the
refused credentials, when the chore added any. Its `Display` is today's
text, `fetching <asked> from <remote> failed: <said>`, then the note on
the next line, so the round's word to whoever ran it is unchanged.

`remote_git` keeps its note apart from git's stderr rather than
appending it, so a fetch builds the error from both; a push keeps
today's text, the note appended. `PushedRefs::fetch` and `fetch_ref`,
and through them every read `RemoteRefs` makes, raise it; a missing
ref still reads as `None`. `fetch_heads` and `fetch_candidates` raise
it with the refspec they asked for.

The note stays out of the log because it changes every round, the
credential's age among it, and is sluice's words, not the remote's
(133). Alternative: match the core on the error's text. Rejected: it is
a profile's wording, and a profile-specific branch in the core (rule
core.2).

### D2. The records seam says why a read was answered stale

`State::stale` returns `Option<ReadRefused>` in place of the remote's
name: the refusal that made the branch's last read answer from what
was last fetched. `RemoteRefs` keeps the error it met in place of the
branch's name in its stale set, and the routing binding passes it on.
`sluice status` keeps saying it could not refresh from the remote,
taking the remote's name from it (`ops.rs`, `stale`).

### D3. What a round counts as refused, read again, or neither

The round's classification is made in `Sluice::step_locked`, around
the work it does under the branch's lock, in one place:

- The lock is held elsewhere (`Step::Busy`): the round never reached
  the remote and counts as nothing.
- The round's first read of the records, `read_line` at the top of the
  locked round, was answered stale: the round is refused with that
  `ReadRefused`, and stops there, so it forms, starts and lands
  nothing (133; the remote-records delta). Today it carries on.
- The round's work ended with an error whose chain holds a
  `ReadRefused`: the round is refused with it.
- The round's work ended `Ok`: the host read the branch and its records
  this round, so a standing refusal ends.
- Any other error: the round counts as nothing, and a standing refusal
  stays open.

A refused round returns the error it met, unchanged, to whoever ran it
(133: its word stands beside the log). The classification is the
round's alone: `sluice status`, `overview` and a submit's reads are
not rounds and change nothing of it.

Alternative: catch each `branch.read` call. Rejected: a round reads the
branch in `keeps`, `decide` and `perform_landing`, and records in
`read_line` throughout; one place keeps the count one per round.

### D4. The host keeps the refusal in its own state

`State` gains `refusal(branch) -> Result<Option<Vec<u8>>>` and
`keep_refusal(branch, Option<&[u8]>) -> Result<()>`: this host's
document for the branch, replaced or removed whole, as `beat` keeps its
document as bytes. Default methods keep nothing, as `observe` does.
`RemoteRefs` keeps it at
`<git-dir>/sluice-remote/<remote>/refused/<branch>.json`, beside
`observed`; `LocalFiles` at `<common>/sluice/.refused/<branch>.json`,
whose dot no branch name carries, so `branches()` never reads it as a
branch; the routing binding sends each to the branch's route; the
fakes keep it in memory. Each is written atomically, as `observe`
writes.

The document is the core's, in a new `sluice-core/src/refused.rs`:

```json
{
  "remote": "origin",
  "since": "2026-10-26T09:16:40Z",
  "rounds": 12,
  "causes": [
    {"asked": "refs/heads/main", "said": "remote: Repository not found.\n…",
     "at": "2026-10-26T09:16:40Z", "logged": true}
  ],
  "ended": null
}
```

A refused round adds one to `rounds` and, when its `(asked, said)` is
in no cause of the document, adds a cause at the round's time with
`logged: false`; the first refused round sets `since`. A round that
reads again sets `ended` to its time. Nothing but the round writes the
document, and nothing a round, gate or landing decides reads it (rule
core.14); it holds what the host observed, as 73's observation does.

### D5. The one write path carries what is still unwritten

`Sluice::write`, the one path every write of a branch's records takes,
reads the host's document first and appends, to the line it writes, a
`read-refused` line for each cause with `logged: false`, at the cause's
own `at`, and a `read-again` line when `ended` is set, at `ended`, with
`since`, `stood` (seconds from `since` to `ended`) and `rounds`. Once
the write is `Written`, it marks those causes logged, and removes the
document when it had ended. A refused write leaves the document as it
was; the update loop's next try appends them again to its fresh read.

So a line is written in the host's first write to the branch's records
after it happened, whoever makes that write: the refused round itself
when the records are on the host or their fetch was answered, a later
round when they are on the refusing remote, a submit's write when one
comes first (133). After it records a refused round, the round makes a
write of its own, `update` with no change but the lines, when the
document has a cause unlogged and the round's refusal was not of the
records; a failure of that write leaves the lines for the next, and the
round still returns the read's error. A round that reads again makes
the same write, so `read-again` is written that round, its write
otherwise holding nothing.

Each line's `at` is when it happened, not when it was written: a line
written late may read earlier than the lines before it. The log is
read in order (130 to 132), and the order of lines is the order of
writes.

Alternative: put an entry per host in `branch.json`, as `copies`
holds a copy behind. Rejected: on a branch whose records are on the
refusing remote the entry cannot be written while it would mean
anything, and the count would be a write of the records every round,
read by other hosts as the lead's sign of life (73).

### D6. Two events in `sluice/log@1`

```rust
/// 133: a read of the branch's remote a round on `host` could not make,
/// once per distinct ref asked and words said while it stands.
#[serde(rename = "read-refused")]
ReadRefused { host: String, remote: String, asked: String, said: String },
/// 133: the first round on `host` that read the remote again.
#[serde(rename = "read-again")]
ReadAgain { host: String, remote: String, since: String, stood: u64, rounds: u64 },
```

`host` is the instance's name, as on `copy-behind`. An older sluice and
the plugins pass over an event they do not know, as they pass over
`copy-behind`; the schema's name is unchanged.

### D7. Status shows a standing refusal

`Status` gains `refused`, read from this host's document when it has
no `ended`: `remote`, `asked` and `said` of its latest cause, `since`
and `rounds`. Text shows one line above the queue, beside the copy
behind:
`this host could not read refs/heads/main from origin since
2026-10-26T09:16:40Z, 12 rounds: remote: Repository not found.`, git's
words on as many lines as git gave them. `--json` carries it under
`refused`, absent when there is none.

## Risks / Trade-offs

- [The host stops between a write that carried a line and marking it
  logged] → that line is written again by the next write. The window is
  one file's rename; a duplicate says the same thing twice and decides
  nothing.
- [A refusal stands on a host that never runs another round] → its
  document stays, status keeps showing it, and the log holds the
  opening line alone. That is what the host saw.
- [git's words carry a value that changes each round, a request id] →
  each round would be a new cause. GitHub's and git's refusals carry
  none today; the tests pin the words of a refused fetch from a
  repository that is not there.
- [A round refused mid-way, after its first reads passed] → it is
  refused all the same; checks it already started keep running, and
  the round's own outcomes are settled by the next round, as record
  0017 has it.
- [The unit `a-push-the-credential-stopped-is-made-once-more` edits
  `remote_git` too] → both keep `told`'s text for a push; whichever
  merges second keeps the note apart for a fetch, as D1 says.

## Migration Plan

Nothing moves: no existing record changes shape. A host running an
older sluice writes neither event and passes over both. Rolling back
leaves the host's documents unread under `.refused/` and `refused/`;
they decide nothing.
