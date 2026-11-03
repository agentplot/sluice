# Tasks

Each group is one session's work for an implementer who reads this
change (proposal, design, specs) and its own group. The groups land in
order, as one landing. Work in this change's place on its branch, cut
from main. Run suites through sluice (`sluice exec main -- cargo test
...`); a quick `cargo check`, `cargo clippy` or one unit test may run
directly. No test reaches a credential or the host's own sluice state:
set `XDG_STATE_HOME` to the test's scratch directory, as the tests do
now. The profile and unit tests name a scratch root of the test's own
with `Places::with_scratch_root`; every sluice a test starts is given
`SLUICE_USER_SCRATCH` by `support::isolated`, the tests' own directory
standing for the user's, since the gate runs the tests under the
sandbox, which lets them write there and not in `/tmp/sluice-<uid>`
(design, "Where the user's directory comes from, in tests"); and
`support::Repo` removes its repository's directory there, and under
`/tmp/sluice-<uid>`, when dropped.
Tests of the sandbox run on macOS only, and only in the `cli:sandbox`
task. Commit by pathspec (`git add <paths>; git commit -m "<subject>" --
<paths>`), Conventional Commits, scoped to the part of the tree, citing
clauses by number and rules by id. Every group ends with its check task
passing.

## 1. The decision recorded

- [x] 1.1 Amend record 0063 on clause 35 as amended (agentplot/blueprints f15896b): option 5, the user's directory, the repository's key, the directory made and checked as the user's own, the sweep's reach and a place made before; rule core.30 and its details, the line allowing the scratch under the repository's own `sluice/.tmp/` replaced by the user's directory and a place made before released where its record names; an item for both in `docs/architecture/_open.md`. Done in the commit that writes this change. True when `sluice exec main -- moon run root:adrs root:changes` passes.

## 2. The scratch under the user's own directory

- [x] 2.1 In `crates/sluice/src/profiles/places.rs`, add `user_scratch()` (the resolved `/tmp` joined with `sluice-<geteuid>`), `repository_key(common)` (10 hex digits of the SHA-1 of the common directory's path bytes, by `gix::hash::hasher`, kept in a `OnceCell` on `Places`), `Places::with_scratch_root(dir)` beside `with_blocks`, and `pub fn repository_scratch(common) -> Result<PathBuf>`; make `new_scratch(name)` `<user's directory>/<key>/<scratch_name(name)>`, as the design's "The user's directory is `/tmp/sluice-<uid>`, resolved" and "The repository's key is a digest of where the repository is" say. Unit tests in the file's `tests` module: the key is 10 lower-case hex digits, the same for one path twice and different for two; `new_scratch` is the root given plus 22 bytes; `user_scratch` is resolved (on macOS it starts `/private/tmp/sluice-`). True when `sluice exec main -- cargo test -p sluice --lib profiles::places` passes.
- [x] 2.2 Add `own_dir(kind, owner, mode, uid)` and `Places::make_scratch(path)` as the design's "The directory is made and checked as the user's own" says, and make the scratch through it in `materialize`, `adopt`, `preview` and `release_gone`, in place of `create_dir_all`; a path under the repository's `sluice/.tmp/` is made as now. The refusal uses the design's words. Unit tests of `own_dir`: a link, a file, another uid's directory, a directory with any bit of `0o077`, each refused with its reason; the user's own `0o700` directory taken. True when `sluice exec main -- cargo test -p sluice --lib profiles::places` passes and `cargo clippy -p sluice --all-targets -- -D warnings` is clean.
- [x] 2.3 In `crates/sluice/tests/profiles/places.rs`, give the file's `places(repo)` helper a scratch root of the test's own, a path not yet made inside a short `Scratch` so the scratch path stays short. In `places.rs`, add `pub const USER_SCRATCH_VAR` (`SLUICE_USER_SCRATCH`), documented as the tests' way to name the directory standing for the user's, as `ENGINE_SOCKETS_VAR` is, and make `repository_key` public; in `crates/sluice/src/profiles/mod.rs`, hand it, where set and not empty, to the binding's `Places` by `with_scratch_root`. In `crates/sluice/tests/support/mod.rs`, add `test_user_scratch()`, `sluice-test-<uid>` in the tests' temporary directory, resolved; make `isolated` set `SLUICE_USER_SCRATCH` to it; make `Repo`'s drop remove its repository's directory there and `repository_scratch(<its .git, resolved>)`, an error ignored as `Scratch`'s drop ignores one; and give the `Places` that `crates/sluice/tests/stack.rs` and `crates/sluice/tests/profiles/conformance.rs` build `with_scratch_root(test_user_scratch())`. True when `sluice exec main -- cargo test -p sluice --features tools --test places --test exec` passes and, after it, no directory under `/tmp/sluice-<uid>` is named by a key of a test's repository (list it before and after the run).
- [x] 2.4 In `crates/sluice/tests/profiles/places.rs`, add, each with a doc comment citing 35 as amended and rule core.30: `the_users_scratch_directory_is_made_its_own`, `a_scratch_directory_not_the_users_own_is_refused` (a link and a directory of mode `0o755`, each left as it was), `a_scratch_the_system_emptied_is_made_again` and `two_repositories_keep_their_scratch_apart`, as the place-scratch spec's scenarios say; update `a_place_names_its_scratch_on_its_record` and `a_worktree_keeps_one_short_scratch` to find the scratch under the root the helper gives. True when `sluice exec main -- cargo test -p sluice --features tools --test places` passes.
- [x] 2.5 In `crates/sluice/tests/exec.rs`, make `exec_runs_from_a_worktree_at_a_long_path` assert the printed `SLUICE_TMP` is `test_user_scratch()` and two runs of 10 hex digits joined by `/`, holding no part of the worktree's path or the repository's, and make `a_clean_run_of_the_checks_is_recorded_and_a_dirty_one_is_not` find its scratch on the worktree's record; read the gone-worktree scenario's scratch from the record as now. True when `sluice exec main -- cargo test -p sluice --features tools --test exec` passes.
- [x] 2.6 Rewrite the doc comments that say where the scratch is: the module's (lines 70 to 77 today) and `Made::scratch`'s in `places.rs`, `Place::tmp`'s and `Checkout::remove_unnamed_scratch`'s in `crates/sluice-seams/src/lib.rs`, citing 35 as amended and rule core.30. True when `cargo clippy -p sluice -p sluice-seams --all-targets -- -D warnings` is clean.
- [x] 2.7 Check the group: commit, then run `sluice exec main -- moon run cli:test cli:tools seams:test`. True when every task passes.

## 3. The sweep reaches only the repository's own

- [x] 3.1 In `places.rs`, make `list_scratch` list the repository's directory under the user's and the former `root()/.tmp/`, checking the user's directory with `own_dir` first and, where it is refused, warning with the refusal's words and listing nothing under it; never list the user's directory itself; return early with no record read when both listings are empty (129). Keep `named_scratch` and `Named::keeps` comparing names alone, with the design's "The sweep lists two roots and compares names" in the doc comment. True when `sluice exec main -- cargo test -p sluice --lib profiles::places` passes, `a_scratch_made_after_the_listing_is_kept` among them.
- [x] 3.2 In `crates/sluice/tests/profiles/places.rs`, update `scratch_no_record_names_is_removed` and `an_unreadable_record_keeps_its_scratch` to the spec's scenarios, strays in both roots; add `another_repositorys_scratch_is_left` (two repositories' `Places` given one scratch root) and `a_users_directory_not_its_own_is_not_swept`, each citing 35 as amended, 123 and rule core.30. True when `sluice exec main -- cargo test -p sluice --features tools --test places` passes.
- [x] 3.3 Check the group: commit, then run `sluice exec main -- moon run cli:test cli:tools core:test conformance:test`. True when every task passes.

## 4. A checkout made before keeps its scratch

- [x] 4.1 In `crates/sluice/tests/profiles/places.rs`, add `a_place_in_the_repository_keeps_its_scratch_until_released`: a worktree's record and a candidate's, written by hand naming `<common>/sluice/.tmp/<10 hex>` with the directory made; the worktree's run is given that path, its release removes it and makes nothing under the root the helper gives, and its next run is given a path under that root; the candidate's release removes its directory and record. Update `a_place_from_before_keeps_its_scratch_until_released` so the next run's path is under that root. No code should need to change for it (design, "A place made before keeps its scratch"); if one does, change the design first. True when `sluice exec main -- cargo test -p sluice --features tools --test places` passes.

## 5. Under the sandbox

- [x] 5.1 In `crates/sluice/tests/exec.rs`, add `#[cfg(target_os = "macos")]` `exec_under_the_sandbox_binds_a_socket_in_a_folder_under_its_scratch`, citing 35 as amended and the C.4 sandbox row: a worktree at a path longer than 255 bytes, its branch's policy naming `[executor] kind = "sandbox"` and `network = "localhost"`, runs `sluice exec` with a command that makes `$SLUICE_TMP/x/sessions/work/`, binds a Unix socket at `x-client.sock` there with `/usr/bin/perl -MIO::Socket::UNIX` (`Listen => 1`, exiting non-zero on failure), and prints the socket's path, the command given no `SLUICE_USER_SCRATCH`, so the binary makes its scratch under the real `/tmp/sluice-<uid>`; assert the command passed, the printed path starts with `user_scratch()` and is shorter than 104 bytes. In `crates/sluice/moon.yml`, add `--test exec` to the `sandbox` task's command, and say in its comment that the sandbox scenarios of `sluice exec` are there; the name holds `sandbox`, so `cli:test` and `cli:tools` already skip it. True when `devenv shell -- moon run cli:sandbox` passes, run outside any sandbox as the pre-merge hook runs it: the gate cannot run it, since macOS refuses a sandbox inside another and sluice's own gate runs under the sandbox.

## 6. Say it to the operator

- [x] 6.1 In `site/src/content/docs/reference/state.md`, give a checkout's scratch its own block after the repository's, as the design's "The words a reader sees" says, and reduce the repository's `.tmp/` line to the scratch of checkouts an older sluice made, removed when they are released, and of strays removed by the next round or `sluice exec`. In `site/src/content/docs/reference/configuration.md`, extend the `SLUICE_TMP` row, and say beside `docker` that a container engine must share `/tmp` (`/private/tmp` on macOS) with its machine for a container to see a folder under `SLUICE_TMP`. Name no host, tool or team of ours (rule core.26). True when `sluice exec main -- moon run site:prose` passes with no warning on the pages this group touched.

## 7. The gate

- [x] 7.1 Remove the item for core.30 and record 0063 from `docs/architecture/_open.md`, and the file once it is empty. True when nothing there names this change.
- [x] 7.2 Confirm no place made now is given a scratch under the repository's state: `rg -n '"\.tmp"' crates/sluice/src/profiles/places.rs` finds only the former root, in `scratch`'s fallback and `list_scratch`. True when it holds and the commit message says so.
- [x] 7.3 Run the gate's checks on the committed work with `sluice exec main`, and `devenv shell -- moon run cli:sandbox` outside any sandbox. True when every task passes and `openspec validate a-places-scratch-is-under-a-short-directory-of-the-host-users-own --strict` passes.
