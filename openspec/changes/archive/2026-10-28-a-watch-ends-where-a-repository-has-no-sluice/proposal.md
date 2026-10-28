# Proposal

## Why

Since the board's session-start read went, it opens a watch in every git
repository a Claude Code session is in, whether sluice is set up there or
not. Where it isn't, that watch never ends: it reads an empty overview
once a second for the session's life, one process per repository per
host, and the board shows sluice's words where it used to stay silent.
Clause 129, as amended (blueprints db0c70c), says a watch ends when its
repository has no sluice, meaning no policy at any revision the host holds
and no records. That is no refused read: the reader's last document says
the repository has no sluice and the watch ends, so no process of
sluice's stays in a repository sluice isn't set up in. A session there
shows nothing of sluice's, neither a queue nor a refusal, and a watch is
opened there again only when asked.

## What Changes

- Each time `sluice overview --json --watch` reads a repository whose
  records list no branch, it asks whether any revision the host holds has
  sluice's policy. That means every local branch and every remote's
  tracking branch, as last fetched, with nothing fetched. Where none has
  it, the watch writes its last document and ends with exit 0. The last
  document is the overview it read, with `"set_up": false` added, and it
  is written as the reader's document and printed as the watch's last
  line. Where a revision does hold the policy, as in a repository set up
  with nothing submitted yet, the watch goes on as it does today.
- A watch that follows a reader whose document says the repository has
  no sluice prints that document and ends the same way.
- A watch that becomes the reader removes a standing "no sluice" document
  left by an earlier reader before it reads, so no watch that starts
  after sluice is set up ends on what an earlier one found.
- A check of the revisions that fails is a read the reader could not
  make: it is written and printed as the error document, and the watch
  goes on (133).
- The board: when its watch prints a document saying the repository has
  no sluice, it drops any queue, error or reason it held. The band and the
  status line are empty, no forecast runs, and it starts no further watch
  on its timer. Only `/sluice` starts the next watch. If that watch also
  finds no sluice, the pane says `sluice isn't set up in this
  repository.` and `Ask Claude to set sluice up in this repository.`
  Otherwise the board follows the queue from then on, as it does today.
- `sluice overview`'s reference, the state reference, the board's README
  and the Claude Code guide say all this.

Out of scope:
- A one-shot `sluice overview`, which ends anyway and gains no
  `set_up` field here.
- `sluice wait` and a waiting submit. A wait that is the reader always has
  its own submission's records, so it never finds a repository with no
  sluice. The removal of a stale "no sluice" document covers a wait that
  takes the lead too, since it shares the reader.
- The herdr plugin. It already starts no watch where a one-shot overview
  finds no records, and probes again after five minutes or when asked. A
  watch of its that now ends on the `set_up: false` document is not
  started again while its probe finds none.
- Telling the session of untold outcomes at its start (record 0068),
  which is another unit's.

None of these waits on a later change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `overview-reader`: a watch ends, with a last document saying so, where
  its repository has no sluice; a follower ends on that document; a new
  reader removes such a document before it reads.
- `plugin-board`: where the watch says the repository has no sluice, the
  board shows nothing and opens no further watch until `/sluice`, whose
  pane says sluice isn't set up there.

## Impact

- `crates/sluice/src/overview.rs`: the watch's loop ends on a repository
  with no sluice, as a reader and as a follower.
- `crates/sluice/src/reader.rs`: a lead removes a standing "no sluice"
  document when it takes the lock; the test for a no-sluice document.
- `crates/sluice/src/exec.rs` (`Host`), `crates/sluice/src/profiles/mod.rs`:
  the host says whether any revision it holds has the policy, read with
  git and never fetched.
- `crates/sluice-fakes/src/lib.rs`, `crates/sluice/tests/support/mod.rs`:
  the fakes say whether any revision they hold has the policy.
- `crates/sluice/tests/cli.rs`: tests over the fakes;
  `crates/sluice/tests/overview_watch.rs`, a new check-tools test of the
  binary on scratch repositories, and its line in `crates/sluice/moon.yml`'s
  `tools` task.
- `plugins/sluice-board/hooks/register.tsx`,
  `plugins/sluice-board/hooks/board.ts`,
  `plugins/sluice-board/types/index.d.ts`, and their tests.
- `plugins/sluice-board/README.md`,
  `site/src/content/docs/guides/claude-code.md`,
  `site/src/content/docs/reference/commands.md`,
  `site/src/content/docs/reference/state.md`.
- Design: clause 129 as amended in blueprints db0c70c; clause 133; ADR
  0056 (one reader per host, its document written whole); ADR 0023 (one
  overview document whose fields are only ever added, the plugins outside
  the core); rules core.1, core.15, core.25, core.27.

## Touches

```
crates/sluice/src/overview.rs
crates/sluice/src/reader.rs
crates/sluice/src/exec.rs
crates/sluice/src/profiles/mod.rs
crates/sluice-fakes/src/lib.rs
crates/sluice/tests/support/mod.rs
crates/sluice/tests/cli.rs
crates/sluice/tests/overview_watch.rs
crates/sluice/moon.yml
plugins/sluice-board/hooks/register.tsx
plugins/sluice-board/hooks/board.ts
plugins/sluice-board/types/index.d.ts
plugins/sluice-board/tests/register.test.tsx
plugins/sluice-board/tests/board.test.ts
plugins/sluice-board/README.md
site/src/content/docs/guides/claude-code.md
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/state.md
```
