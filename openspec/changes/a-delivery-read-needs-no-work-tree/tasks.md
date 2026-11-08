# Tasks

The groups land in order, as one landing, from this change's place on
`change/a-delivery-read-needs-no-work-tree`, cut from
`bolt/sluice-tells-each-submitter-its-own-end`, its base. Run suites
through sluice against the base (`sluice exec
bolt/sluice-tells-each-submitter-its-own-end -- cargo test ...`); a
quick `cargo check`, `cargo clippy` or one unit test may run directly.
Tests use scratch repositories only, a checkout, its git directory, a
bare clone and a worktree's own git directory, never a real account or
the host's own sluice state; every git call in a test is isolated from
the user's configuration and credentials. Commit by pathspec,
Conventional Commits scoped to the part of the tree, citing clauses by
number and rules by id.

## 1. The decision recorded

- [x] 1.1 Record 0099, rule core.48's last clause and its details as amended, record 0085's amendment line, and the site's rules page written by `node site/scripts/rules.mjs`, in this change's planning commit. True when `sluice exec bolt/sluice-tells-each-submitter-its-own-end -- moon run root:adrs site:rules` passes.
- [x] 1.2 The open change a-landed-bolt-is-never-reported-withdrawn's outcome-delivery delta keeps its requirement that the driver reads a repository with no files checked out with no `git status`, and adds no longer the one this change replaces (design D5). True when `sluice exec bolt/sluice-tells-each-submitter-its-own-end -- moon run root:changes` passes.

## 2. Who is there, read with no files and no commit

- [x] 2.1 `Driver::recipient(folder) -> Result<Option<Submitter>>` in `crates/sluice-seams`, a default method giving the submitter `session` gives, and `None` where it is detached (design D2); `Session::bare` removed, and every `Session` built without it: `crates/sluice-fakes`, `crates/sluice-core/src/ops.rs` (`named_session` and its test driver), `crates/sluice-conformance/tests/inbox.rs`, and the worktrunk driver's `session`, which keeps its own check of whether files are checked out so it runs no `git status` there. True when `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- [x] 2.2 The worktrunk driver's `recipient` in `crates/sluice/src/profiles/worktrunk/mod.rs`: the branch by `git symbolic-ref -q --short HEAD`, `None` where it names none; the top level by `git rev-parse --show-toplevel` only where `git rev-parse --is-inside-work-tree` says `true`; `SLUICE_AGENT` or the branch as the agent; its host; the herdr session; no `git rev-parse --verify HEAD` and no `git status`. `Drivers` in `crates/sluice/src/profiles/mod.rs` forwards `recipient` to worktrunk's. True when `sluice exec bolt/sluice-tells-each-submitter-its-own-end -- cargo test -p sluice --features tools --test worktrunk -- a_folder_with_no_files_checked_out_is_read_with_no_status_and_no_commit` passes, the test that replaces `a_repository_with_no_work_tree_is_read_with_no_status`: a bare clone, and the checkout's git directory while the checkout holds an untracked file, each read by `recipient` as `main` with no path, and by `session` with nothing held beyond its commits and no path; a bare clone whose `HEAD` names a branch with no commit read by `recipient` as that branch; a bare clone with a detached `HEAD` read as `None`; the checkout read with its path.

## 3. The delivery read

- [x] 3.1 `recipient`, `delivering` and `inbox` in `crates/sluice/src/lib.rs` read who is there by `Driver::recipient`, so a folder with no files checked out is taken by the same `pending_one` and `pending` as the worktree on the branch its `HEAD` names (design D1); the name delivered to is the session, else the path, else the agent (D3); `Recipient::Bare`, `no_work_tree` and their two lines gone, a failed read said as before (D4). True when `sluice exec bolt/sluice-tells-each-submitter-its-own-end -- cargo test -p sluice --features tools --test worktrunk -- a_wait_with_no_files_checked_out_reads_as_a_worktree_on_its_branch` passes, the test that replaces `a_wait_with_no_work_tree_says_plainly_what_it_cannot_deliver`: worktree `a`'s landing, waited on by `sluice wait <n>` in the repository's git directory, whose `HEAD` names `main`, is reported landed with exit 0, nothing said of `pending`, `git status` or files checked out, and nothing marked; `sluice inbox --json` there prints `{"outcomes":[]}` with nothing on stderr and exits 0; `a`'s inbox then collects it.
- [x] 3.2 A git directory delivers its branch's own (outcome-delivery's "The git directory's own landing", inbox's "A git directory collects its branch's own"). True when `sluice exec bolt/sluice-tells-each-submitter-its-own-end -- cargo test -p sluice --features tools --test worktrunk -- a_git_directory_delivers_its_branchs_own` passes: in worktree `a`'s own git directory, `git -C <a> rev-parse --absolute-git-dir`, with `SLUICE_AGENT` `agent-a` and no herdr variables, `sluice wait <n>` on `a`'s landing marks it delivered to `agent-a`, and `sluice inbox --json` there collects `a`'s next submission, blamed, with `delivered_to` `agent-a`.
- [x] 3.3 A `HEAD` whose branch has no commit fails no delivery read (outcome-delivery's and inbox's "A HEAD whose branch has no commit"). True when `sluice exec bolt/sluice-tells-each-submitter-its-own-end -- cargo test -p sluice --features tools --test worktrunk -- a_head_with_no_commit_fails_no_delivery_read` passes: with `a`'s own git directory's `HEAD` pointed at `refs/heads/nothing` after `a`'s submission landed, `sluice wait <n>` there exits 0, says nothing of what it could not read and marks nothing, and `sluice inbox --json` there prints `{"outcomes":[]}` with nothing on stderr and exits 0.

## 4. The site

- [x] 4.1 `site/src/content/docs/reference/commands.md`: the wait's paragraph on what it marks delivered, and the `sluice inbox` section, say that run in a bare repository or a git directory, which has no files checked out, each takes what a worktree on the branch its `HEAD` names would, and that `delivered_to` there, in no herdr pane, names the agent, `SLUICE_AGENT` or the branch; `site/src/content/docs/reference/state.md`'s paragraph on `delivered_to` says the same. True when `sluice exec bolt/sluice-tells-each-submitter-its-own-end -- moon run site:prose` clears every warning.
