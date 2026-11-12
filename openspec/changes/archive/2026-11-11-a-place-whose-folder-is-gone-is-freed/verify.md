# Verify: a-place-whose-folder-is-gone-is-freed

Verified on 2026-11-11 at `aae4bf5b` on
`change/a-place-whose-folder-is-gone-is-freed`, against its base
`bolt/what-a-place-starts-ends-with-it` (`5c4e580e`), with sluice 0.78.1.

**Verdict: the whole checks pass, every requirement and scenario holds,
and every task is ticked. Archived, the change leaves every spec passing
`openspec validate --specs --strict`. The change is archived.**

## The whole checks

`sluice exec bolt/what-a-place-starts-ends-with-it` ran the bolt's gate,
`#check-fast`, once. It ran the checks `aae4bf5` affects against the bolt
at `5c4e580`. It exited 0: `passed: 12 tasks`, `recorded 12 results
under their keys for aae4bf5`, in 39s once it had room under the host's
bound.

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
  carry.
- `site:prose` found 0 errors, 0 warnings and 37 suggestions.
- `root:changes` listed this change's branch as valid.
- moon logged 29 warnings. Twelve said it could not store blobs in the
  shared cache, and twelve said it could not archive a task's outputs
  under the place's `.moon/cache/outputs`. Four said it could not hash an
  input that is no file. One said it skipped the dependency install for
  want of a connection. Each is about caching or setup. Every task passed
  and its result was recorded, so none of them affects the outcome.

The bolt's gate does not run `--test places`, which is `cli:tools`, in
main's `#check-tools`. The tests the specs and tasks 2.1 to 2.3 cite
there passed through sluice at `df50806d`:

```
sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools \
  --test places -- a_checkout_git_no_longer_lists a_deleted_checkout_is_gone_and_freed_with_nothing_run
```

All 3 passed: `a_checkout_git_no_longer_lists_is_gone`,
`a_checkout_git_no_longer_lists_is_freed` and
`a_deleted_checkout_is_gone_and_freed_with_nothing_run`. Between
`df50806d` and `aae4bf5b` only the change's delta spec, tasks and this
report changed, so that result stands for `aae4bf5b`.

## The specs, archived

Under the openspec `devenv.nix` pins, 1.14.3:

- `openspec validate a-place-whose-folder-is-gone-is-freed --strict`
  reports the change valid.
- In a scratch copy of `openspec/`,
  `openspec archive a-place-whose-folder-is-gone-is-freed --yes` applies
  `+ 3 added, ~ 3 modified` to `place-release`. Then
  `openspec validate --specs --strict --no-interactive` reports
  `75 passed, 0 failed`.

The six requirements the change adds or modifies each have text under
openspec's 500 characters, counted up to the first scenario:

| Requirement | Characters |
|---|---|
| The operator may accept a release's failure | 422 |
| A checkout whose working copy is gone is released once and freed | 494 |
| A gone checkout is freed whatever its release did | 299 |
| A checkout or a preview git no longer lists is gone | 437 |
| A worktree's place is gone only once its directory is | 228 |
| sluice removes what is left of a checkout git no longer lists | 459 |

Each modified requirement keeps every scenario the base spec gives it.

## Requirements and scenarios

### place-release, added

| Requirement | Scenario | Where it holds | Met |
|---|---|---|---|
| A checkout or a preview git no longer lists is gone | A checkout git stopped removing part way | `a_checkout_git_no_longer_lists_is_gone` (`crates/sluice/tests/profiles/places.rs`). `drop_entry` removes the entry the folder's `.git` names and leaves the folder, and git no longer lists it. Then `gone(NAME)` is true, through either spelling of the repository. Passed. | yes |
| A checkout or a preview git no longer lists is gone | A checkout git lists under another spelling of its path | The same test makes the place through a symlink to the repository. The record's path starts with the link, git does not list it so spelled, and `gone(NAME)` is false. Passed. | yes |
| A checkout or a preview git no longer lists is gone | A preview git no longer lists | The same test: a preview reads `gone == false`, then `true` in `previews()` once its entry is dropped, and `gone(name)` is true. The next pass's free is `a_preview_whose_checkout_is_gone_is_freed` (`crates/sluice-conformance/tests/previews.rs`), which passed in conformance:test. | yes |
| A worktree's place is gone only once its directory is | A worktree git no longer lists | The same test adopts a worktree and drops its entry. `session_places()` reads it `gone == false`, and so does `gone(name)`. Passed. | yes |
| sluice removes what is left of a checkout git no longer lists | What git left of a checkout | `a_checkout_git_no_longer_lists_is_freed`, its first place. It has a stored release that appends to a mark file, and a `0555` directory. Its entry is dropped. `release_gone` returns `ran == true, failed == None`, and the mark holds one line. The folder, scratch, record and run directory are gone, and git lists nothing there. The next place takes the same port. Passed. | yes |

The removal requirement's clause that a directory sluice still cannot
remove is a failure to free, with the checkout still held, has no test
of its own, and no scenario names one. It holds by reading the code. In
`free`, `remove_worktree` returns its error with `?` before `let_go`, so
the record, scratch and ports stay. `remove_leftover`'s error names the
entry and what refused it, and `remove_worktree` wraps it in `removing
what is left of <folder>, which git no longer lists as a worktree`.

The reading requirement's clause that a failed reading is never gone
holds the same way: `listed` returns an error where the path cannot be
read or the worktrees cannot be listed, and `gone` and `preview_place`
propagate it.

### place-release, modified

| Requirement | Scenario | Where it holds | Met |
|---|---|---|---|
| The operator may accept a release's failure | Accepted | `release_accepting_the_failure_frees_a_held_checkout` (`crates/sluice/tests/cli.rs`), unchanged. Passed in cli:test. | yes |
| The operator may accept a release's failure | Accepted where git no longer lists the checkout | `a_checkout_git_no_longer_lists_is_freed`, its third place. The place has a `0555` directory holding a file, and git's entry is dropped. `free` returns `Ok`, the mark file is unchanged so nothing ran, and the run's directory is gone. `left_behind` finds no folder, no scratch and nothing git lists. `made()` is empty, and the next place takes its port. Passed. The command's own path is the cli test above. | yes |
| A checkout whose working copy is gone is released once and freed | A worktree removed by hand | Unchanged by the delta. Its tests are in cli:test and conformance:test, which passed. | yes |
| A gone checkout is freed whatever its release did | A gone checkout's release fails; A combination's checkout deleted; Nothing stored to run | Unchanged by the delta. `a_deleted_checkout_is_gone_and_freed_with_nothing_run` passed in the named run above. The cli and conformance tests of a gone place passed in the gate. | yes |
| A gone checkout is freed whatever its release did | A checkout held because git refused it | `a_checkout_git_no_longer_lists_is_freed`, its second place. Its release runs through `Runs::remove` before its entry is dropped, as on deneb. `release_gone` then returns `ran == false`, the mark is unchanged, and the place is freed as above. Passed. The round's path is `a_held_checkout_deleted_since_is_released_once_and_freed` (`crates/sluice-conformance/tests/places.rs`), which passed. | yes |

The working-copy requirement's added clause, that a stored release which
has run already runs nothing, holds in `Runs::release_in`
(`crates/sluice/src/runs.rs`). It returns `false` where the run's
`released` mark exists. `Places::release_gone` also notes
`gone_release` on the record before it starts, so it never starts twice.
The second place of `a_checkout_git_no_longer_lists_is_freed` covers
this.

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
  `preview_place` returns `Result` and reads `folder_gone` while the
  preview's command has not failed. Its callers propagate the error.
- **D2, one reading, one owner.** `remove_worktree` asks `listed`. A
  folder git lists goes by `git worktree remove --force`, and one it
  does not goes by `remove_leftover`, which does nothing where the folder
  is not there. `registered` is gone, and `release_worktree` calls
  `listed` where record 0112 had `is_worktree`. `free` gives
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
- **D5, a worktree's place keeps its reading.** `session_places` reads
  `gone = !worktree.is_dir()`, and `gone` reads a session's place the
  same way.
- **D6, the seam.** The docs of `Checkout::gone` and `Checkout::free` in
  `crates/sluice-seams/src/lib.rs` say what D6 gives, and the
  signatures are unchanged. Outside the profile, git is named only in
  those docs, as before (rule core.1).
- **D8, the record.** Record 0115 is at
  `docs/adr/0115-a-checkout-git-no-longer-lists-is-gone-and-sluice-removes-what-is-left-of-it.md`.
  It is accepted, and its title and decision-makers are as D8 and task
  note 1.1 give them.

## Tasks

All seven tasks are ticked, and each holds:

- 1.1: record 0115 exists with the context, considered options, outcome
  and consequences that task note 1.1 lists. root:adrs passed.
- 1.2: core.16's details carry the deneb incident in "Rules out" and the
  new "Not ruled out", and they cite ADR 0115. site:rules passed, and the
  rules page needs no change.
- 2.1: `listed`, `folder_gone`, `gone`, `preview_place` and the seam's
  doc are in place. `a_checkout_git_no_longer_lists_is_gone` covers
  steps 1 to 5 of task note 2.1, and passed.
- 2.2: `remove_worktree` on `listed`, `remove_leftover`, `registered`
  removed and the doc of `free` are in place.
  `a_checkout_git_no_longer_lists_is_freed` covers the three places of
  task note 2.2 and the `Unlock` guard of its step 4, and passed.
- 2.3: `a_deleted_checkout_is_gone_and_freed_with_nothing_run` passed.
- 3.1: `reference/commands.md` covers the `--accept-failure` paragraph
  and the gone paragraph, and `reference/state.md` covers the `gone`
  paragraph. site:prose passed.
- 4.1: the delta spec holds the three added requirements and the three
  modified ones as the task words them, with their scenarios. No code,
  test or record changed after `df50806d`. The scratch archive above
  passes every spec under openspec 1.14.3, and the change validates
  strictly.
