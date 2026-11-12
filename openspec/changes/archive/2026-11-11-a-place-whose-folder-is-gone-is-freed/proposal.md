# A place whose folder git no longer lists is freed

## Why

On deneb, sluice holds three of agentplot/sluice's checkouts on `main`
for good: `sluice/try/main/63`, `71` and `76`. Each release failed four
times, and the operator's `--accept-failure` would fail the same way
(signal 2026-11-10-agentplot-ops-8ac92f49/01):

> "sluice/try/main/63 is held: its release failed 4 times: git worktree
> remove …/sluice.git/sluice/.places/sluice+2Ftry+2Fmain+2F63: fatal:
> '…/sluice+2Ftry+2Fmain+2F63' is not a working tree"

`main`'s log on deneb shows how it began. The first try, at 14:22:47Z,
failed inside git: `git worktree remove … error: failed to delete
'…/sluice+2Ftry+2Fmain+2F63': Directory not empty`. git stops deleting
the folder at its first failure, then drops its own entry for the
worktree all the same. What is left is the folder, still holding about
2000 files and its `.git` file, which names an entry git no longer has.
Every later try asks `git worktree remove` again, and git refuses a
folder it does not list.

Clause 123 already says what sluice does with such a place: "A place
whose working copy is already gone is no one's … sluice runs its
release once, from what it itself holds for the place and with the
place's ports, records how that went, frees what it holds at once
whatever the outcome, records the place gone, and never retries it."
The place profile never reads it as gone. `Checkout::gone` asks only
whether the recorded path is a directory
(`crates/sluice/src/profiles/places.rs`, `gone`), and the folder is
still there. So the round keeps calling `release`, which asks git again.
`free`, behind `--accept-failure`, asks git too, so it fails the same
way.

## What Changes

- **Gone, as git reads it.** A combination's checkout or a preview,
  each a place sluice made, is gone when git no longer lists its folder
  as a worktree of the repository, whether or not the folder is still
  there. A folder that is no directory is gone, as today. The profile
  reads git's own list of worktrees from the repository's files. It
  starts no process and matches a folder by its identity on disk, not
  by how its path is spelled (123; rule core.31).
- **Released once and freed.** The next round finds the place gone and
  takes 123's path, which the core already has. It runs the release
  command the place's runs stored, once, from its scratch directory, or
  nothing where that command already ran. It lets go of the place's
  runs, scratch, ports and record. Its branch's log gets `gone`, and its
  branch's state no longer lists it. A place held before this change,
  as deneb's three are, is freed by the next round in the same way.
- **What is left of the folder.** git no longer lists the folder, so
  sluice removes it itself. It is sluice's own: a checkout or preview
  folder under the repository's `sluice/` directory. A directory a check
  left unwritable is made writable by its owner first. A folder git
  still lists is removed by `git worktree remove`, as today.
- **The operator's acceptance frees it too.** `sluice release <name>
  --accept-failure` removes what is left of the folder in the same way,
  with nothing run, so the operator can free a held place whatever git
  says of it (123).
- **A leftover sluice cannot remove** is a failure to free, as any
  failure to free is today. The place stays held with what failed, and
  its words name the folder.
- The decision is recorded as record 0115, or the next free number.
  Rule core.16's details cite the incident and the record. The site's
  commands and state reference pages say it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `place-release`: a checkout or a preview git no longer lists is gone
  and freed as 123 frees a gone place, what is left of its folder
  removed by sluice; the operator's `--accept-failure` frees it the
  same way; a worktree's place keeps its reading, gone only once its
  folder is.

## Impact

- `crates/sluice/src/profiles/places.rs`: one reading of whether git
  lists a folder. It is used by `gone`, by `preview_place` and by
  `remove_worktree`, and replaces `registered`, which started `git
  worktree list`. `remove_worktree` removes a leftover folder git no
  longer lists.
- `crates/sluice-seams/src/lib.rs`: the docs of `Checkout::gone` and
  `Checkout::free` say the same.
- Tests: two new tests in `crates/sluice/tests/profiles/places.rs`.
  `a_checkout_git_no_longer_lists_is_gone` covers the reading.
  `a_checkout_git_no_longer_lists_is_freed` covers the release once,
  the free and the leftover.
- No change to the core, the fakes or the conformance scenarios. Each
  already frees a place the seam says is gone: `release_made`,
  `perform_releases`'s held places that are gone since,
  `release_unusable_previews` and `release_held`.
- `docs/adr/0115-…md` (new), `docs/architecture/core.md`,
  `site/src/content/docs/architecture/rules.md`,
  `site/src/content/docs/reference/commands.md` and
  `site/src/content/docs/reference/state.md`.
- No record or state changes form, and no command's words change.

### Out of scope

- **A worktree's place whose folder git no longer lists.** sluice
  neither made nor removes a session's worktree (rules core.16,
  core.20). Its place stays read as gone only once its folder is.
  Reading a live agent's worktree as gone would run its release and
  give away its ports while the agent works. This becomes a change of
  its own if a report shows such a worktree left held.
- **A sweep of folders under `sluice/.places/` that no record names.**
  Today no path leaves one, since a leftover sluice cannot remove keeps
  its place held. A sweep belongs with a change that frees a place while
  its folder stays.
- **Flume's own removal of its places**, which also runs `git worktree
  remove` and could meet the same leftover. That is Flume's, and this
  stage reports it to the orchestrator.
