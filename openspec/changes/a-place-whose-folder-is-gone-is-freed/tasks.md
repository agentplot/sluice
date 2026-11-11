# Tasks

Work in this change's place on
`change/a-place-whose-folder-is-gone-is-freed`, cut from
`bolt/what-a-place-starts-ends-with-it`. That bolt is the base every
`sluice exec` below names. A test of the check-tools tier and a moon
task run through `sluice exec bolt/what-a-place-starts-ends-with-it --`,
as CLAUDE.md's Running checks has it.

The change touches the place profile and the seam's docs, adds two
tests in `crates/sluice/tests/profiles/places.rs`, and changes no core,
fake or conformance code (design D4, D6).

Commit by pathspec, with Conventional Commits subjects scoped to the
part of the tree touched, citing record 0115 (or the number it takes)
and clause 123.

## 1. The decision recorded

- [x] 1.1 Write `docs/adr/0115-a-checkout-git-no-longer-lists-is-gone-and-sluice-removes-what-is-left-of-it.md`, as design D8 and Task notes 1.1 give it, taking the next free number where 0115 is taken on main, this bolt or an in-flight branch. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run root:adrs` passes.
- [x] 1.2 In `docs/architecture/core.md`, amend core.16's details with the incident and its "Not ruled out", and add the record to its ADRs, as Task notes 1.2 words them. Rewrite `site/src/content/docs/architecture/rules.md` with `node site/scripts/rules.mjs`. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run site:rules` passes.

## 2. Gone, as git reads it

- [x] 2.1 In `crates/sluice/src/profiles/places.rs`, add `listed(path)`, whether git lists the folder as one of the repository's worktrees: the main worktree or an entry under `<common>/worktrees/`, read through gix as `checked_out` reads them, matched by device and inode, starting no process (design D1). `gone` reads a candidate's or a preview's place as gone where its recorded path is no directory or `listed` is false; `preview_place` reads a preview's `gone` the same way while its command has not failed; `session_places` keeps its reading (D5). A failed read is an error, never gone. `Checkout::gone`'s doc in `crates/sluice-seams/src/lib.rs` says so (D6). Add `a_checkout_git_no_longer_lists_is_gone` in `crates/sluice/tests/profiles/places.rs`, as Task notes 2.1 gives it (spec "A checkout whose working copy is gone is released once and freed"). True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test places a_checkout_git_no_longer_lists_is_gone` passes.
- [x] 2.2 `remove_worktree` in `crates/sluice/src/profiles/places.rs` asks `listed`. A folder git lists goes by `git worktree remove --force`, as today. A folder git does not list goes by sluice's own removal: walk it without following links, make every directory its owner cannot write owner-writable, then remove it; its failure names the folder and what refused it. Remove `registered` (design D2, D3). `Checkout::free`'s doc says so (D6). Add `a_checkout_git_no_longer_lists_is_freed` in the same test file, as Task notes 2.2 gives it (spec "A gone checkout is freed whatever its release did" and "The operator may accept a release's failure"). True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test places a_checkout_git_no_longer_lists_is_freed` passes.
- [x] 2.3 The tests that already free a deleted checkout still pass with `remove_worktree` on `listed`: a deleted folder git still lists goes by `git worktree remove --force` (D2). True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test places a_deleted_checkout_is_gone_and_freed_with_nothing_run` passes.

## 3. The docs

- [x] 3.1 Say it on the site, as Task notes 3.1 gives it: `site/src/content/docs/reference/commands.md` ("Checkouts held because their release failed": the `--accept-failure` paragraph and the last paragraph) and `site/src/content/docs/reference/state.md` (the `gone` paragraph). True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run site:prose` passes.
