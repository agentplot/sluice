# Verify: a-place-whose-folder-is-gone-is-freed

Verified on 2026-11-11 at `df50806d` on
`change/a-place-whose-folder-is-gone-is-freed`, against its base
`bolt/what-a-place-starts-ends-with-it` (`5c4e580e`), with sluice 0.78.1.

**Verdict: the whole checks pass and every scenario holds, but the work
falls short. Once archived, the delta spec leaves
`openspec/specs/place-release/spec.md` failing `openspec validate
--strict`. The change is not archived, and it goes back to its code
stage.**

## What falls short

`openspec archive a-place-whose-folder-is-gone-is-freed` applies the
delta cleanly, `~ 3 modified`. Afterwards,
`openspec validate --specs --strict` reports `74 passed, 1 failed`. The
one that fails is `place-release`, which passes on the base `5c4e580e`.
Its three issues are the three requirements this change modifies, each
with the warning `Requirement text is very long (>500 characters)`,
which `--strict` reads as a failure:

| Requirement | Text on the base | Text after the archive |
|---|---|---|
| The operator may accept a release's failure | 421 | 561 |
| A checkout whose working copy is gone is released once and freed | 468 | 1032 |
| A gone checkout is freed whatever its release did | (under 400) | 643 |

The lengths count the requirement's text up to its first scenario.

The gate cannot see this. `root:changes` runs `openspec validate
--changes --strict`, which passes for the change: the delta's
requirements are not held to the length a main spec is. CLAUDE.md asks
that `openspec/specs/` change only through a change validated strictly
and archived so the spec reconciles. An archive that leaves a spec
failing strict validation does not meet that.

What the code stage does: shorten the three requirements in
`specs/place-release/spec.md` under 500 characters each, as the warning
says. Move the edge cases into their scenarios, or split a requirement
so that each states one behavior. Then check that
`openspec validate --specs --strict` still passes once the change is
archived, in a scratch copy, before it comes back here.

## The whole checks

`sluice exec bolt/what-a-place-starts-ends-with-it` ran the bolt's gate,
`#check-fast`, once. It exited 0: `passed: 12 tasks`, `recorded 12
results under their keys for df50806`, in 1m 30s. It ran the checks
`df50806` affects against the bolt at `5c4e580`.

The tasks were cli:test, conformance:test, core:test, fakes:test,
seams:test, nix:typecheck, root:adrs, root:changes, root:format,
root:lint, site:prose and site:rules.

- `conformance:test` ran `a_held_checkout_deleted_since_is_released_once_and_freed`
  and `a_preview_whose_checkout_is_gone_is_freed`, which passed.
- `cli:test` ran `release_accepting_the_failure_frees_a_held_checkout`,
  which passed.
- `root:adrs` said `111 records, each with a number of its own`, record
  0115 among them.
- `site:rules` said `the page matches docs/architecture/core.md`. The
  change edits only core.16's details, which the rules page does not
  carry, so the page is unchanged.
- `site:prose` found 0 errors, 0 warnings and 37 suggestions.
- `root:changes` listed this change's branch as valid.
- moon logged 29 warnings. Twelve said it could not store blobs in the
  shared cache, and twelve said it could not archive a task's outputs
  under the place's `.moon/cache/outputs`. Four said it could not hash an
  input that is no file. One said it skipped the dependency install for
  want of a connection. Each is about caching or setup. Every task passed
  and its result was recorded, so none of them affects the outcome.

The bolt's gate is `#check-fast`, which does not run `--test places`.
That test is `cli:tools`, in main's `#check-tools`. The tests the specs
and tasks 2.1 to 2.3 cite there were run once, through sluice, as the
tasks name them:

```
sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools \
  --test places -- a_checkout_git_no_longer_lists a_deleted_checkout_is_gone_and_freed_with_nothing_run
```

It passed all 3 tests: `a_checkout_git_no_longer_lists_is_gone`,
`a_checkout_git_no_longer_lists_is_freed` and
`a_deleted_checkout_is_gone_and_freed_with_nothing_run`.

`openspec validate a-place-whose-folder-is-gone-is-freed --strict`
reports the change valid.

## Requirements and scenarios

### place-release

| Requirement | Scenario | Where it holds | Met |
|---|---|---|---|
| The operator may accept a release's failure | Accepted | `release_accepting_the_failure_frees_a_held_checkout` (`crates/sluice/tests/cli.rs`), unchanged. Passed in cli:test. | yes |
| The operator may accept a release's failure | Accepted where git no longer lists the checkout | `a_checkout_git_no_longer_lists_is_freed`, its third place. The place has a `0555` directory holding a file, and git's entry is dropped. `free` returns `Ok`, the mark file is unchanged so nothing ran, and the run's directory is gone. `left_behind` finds no folder, no scratch and nothing git lists. `made()` is empty, and the next place takes its port. Passed. The command's own path is the cli test above. | yes |
| A checkout whose working copy is gone is released once and freed | A worktree removed by hand | Unchanged by the delta. Its tests are in cli:test and conformance:test, which passed. | yes |
| A checkout whose working copy is gone is released once and freed | A checkout git stopped removing part way | `a_checkout_git_no_longer_lists_is_gone`. `drop_entry` removes the entry the folder's `.git` names and leaves the folder, and git no longer lists it. Then `gone(NAME)` is true, through either spelling of the repository. Passed. | yes |
| A checkout whose working copy is gone is released once and freed | A checkout git lists under another spelling of its path | The same test makes the place through a symlink to the repository. The record's path starts with the link, git does not list it so spelled, and `gone(NAME)` is false. Passed. | yes |
| A checkout whose working copy is gone is released once and freed | A preview git no longer lists | The same test: a preview reads `gone == false`, then `true` in `previews()` once its entry is dropped, and `gone(name)` is true. The next pass's free is `a_preview_whose_checkout_is_gone_is_freed` (`crates/sluice-conformance/tests/previews.rs`), which passed in conformance:test. | yes |
| A checkout whose working copy is gone is released once and freed | A worktree git no longer lists | The same test adopts a worktree and drops its entry. `session_places()` reads it `gone == false`, and so does `gone(name)`. Passed. | yes |
| A gone checkout is freed whatever its release did | A gone checkout's release fails; A combination's checkout deleted; Nothing stored to run | Unchanged by the delta. `a_deleted_checkout_is_gone_and_freed_with_nothing_run` passed in the named run above. The cli and conformance tests of a gone place passed in the gate. The other places tests of these scenarios are `cli:tools` and were not run, since the delta does not touch them. | yes |
| A gone checkout is freed whatever its release did | What git left of a checkout | `a_checkout_git_no_longer_lists_is_freed`, its first place. It has a stored release that appends to a mark file, and a `0555` directory. Its entry is dropped. `release_gone` returns `ran == true, failed == None`, and the mark holds one line. The folder, scratch, record and run directory are gone, and git lists nothing there. The next place takes the same port. Passed. | yes |
| A gone checkout is freed whatever its release did | A checkout held because git refused it | `a_checkout_git_no_longer_lists_is_freed`, its second place. Its release runs through `Runs::remove` before its entry is dropped, as on deneb. `release_gone` then returns `ran == false`, the mark is unchanged, and the place is freed as above. Passed. The round's path is `a_held_checkout_deleted_since_is_released_once_and_freed` (`crates/sluice-conformance/tests/places.rs`), which passed. | yes |

The requirement's clause that a directory sluice still cannot remove is
a failure to free, with the place still held, has no test of its own,
and no scenario names one. It holds by reading the code. In `free`,
`remove_worktree` returns its error with `?` before `let_go`, so the
record, scratch and ports stay. `remove_leftover`'s error names the
entry and what refused it. `remove_worktree` wraps it in `removing what
is left of <folder>, which git no longer lists as a worktree`.

## The design, read against the code

- **D1, gone as git lists worktrees.** `Places::listed`
  (`crates/sluice/src/profiles/places.rs`) reads the main worktree,
  where the repository is not bare, and each entry of `repo.worktrees()`
  by its `base()`, through gix. It starts no process. It matches by
  `Folder`, the device and inode of the folder, read with
  `symlink_metadata` for the folder itself. A folder that no longer
  exists is matched by its nearest existing ancestor and the rest of its
  path. An entry whose base or folder cannot be read names nothing. A
  `path` that cannot be read, or worktrees that cannot be listed, are an
  error. `folder_gone` asks `listed` only where the path is a directory.
  `Checkout::gone` reads `folder_gone` for `Kind::Made` and
  `Kind::Preview`, and only `is_dir` for `Kind::Session`.
  `preview_place` now returns `Result` and reads `folder_gone` while the
  preview's command has not failed. Its callers propagate the error.
- **D2, one reading, one owner.** `remove_worktree` asks `listed`. A
  folder git lists goes by `git worktree remove --force`, and one it
  does not goes by `remove_leftover`, which does nothing where the folder
  is not there. `registered` is removed, and `is_worktree` (record 0112)
  becomes `listed`, which `release_worktree` now calls. `free` gives
  `remove_worktree` only `worktree_path(name)` of a place that is no
  session's, so rule core.20 holds.
- **D3, what is left of the folder.** `remove_leftover` walks the folder
  with `symlink_metadata` and removes links and files with
  `remove_file`, never following a link. Before it lists a directory, it
  gives the owner `0o700` where any of those bits is missing. It then
  removes each entry and the directory, with each failure in context.
  An entry that disappears meanwhile is not an error.
- **D4, the release once and the operator's acceptance.** The core is
  untouched. `release_gone` notes its start, runs the stored release
  once, then calls `free`. `free` runs nothing and removes the leftover
  by D2 and D3.
- **D5, a worktree's place keeps its reading.** `session_places` still
  reads `gone = !worktree.is_dir()`, and `gone` reads a session's place
  the same way.
- **D6, the seam.** The docs of `Checkout::gone` and `Checkout::free` in
  `crates/sluice-seams/src/lib.rs` say what D6 gives, and the
  signatures are unchanged. Outside the profile, git is named only in
  those docs, as before (rule core.1).
- **D8, the record.** Record 0115 is at
  `docs/adr/0115-a-checkout-git-no-longer-lists-is-gone-and-sluice-removes-what-is-left-of-it.md`.
  It is accepted, and its title and decision-makers are as D8 and task
  note 1.1 give them.

## Tasks

All six tasks are ticked, and each holds:

- 1.1: record 0115 exists with the context, considered options, outcome
  and consequences that task note 1.1 lists. root:adrs passed.
- 1.2: core.16's details carry the deneb incident in "Rules out" and the
  new "Not ruled out", and they cite ADR 0115. site:rules passed, and the
  rules page needs no change.
- 2.1: `listed`, `folder_gone`, `gone`, `preview_place` and the seam's
  doc are in place. `a_checkout_git_no_longer_lists_is_gone` is added,
  covering steps 1 to 5 of task note 2.1, and passed.
- 2.2: `remove_worktree` on `listed`, `remove_leftover`, `registered`
  removed and the doc of `free` are in place.
  `a_checkout_git_no_longer_lists_is_freed` is added, covering the three
  places of task note 2.2 and the `Unlock` guard of its step 4, and
  passed.
- 2.3: `a_deleted_checkout_is_gone_and_freed_with_nothing_run` passed.
- 3.1: `reference/commands.md` covers the `--accept-failure` paragraph
  and the gone paragraph, and `reference/state.md` covers the `gone`
  paragraph. site:prose passed.
