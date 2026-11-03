# Verify

The work on `change/a-places-scratch-is-under-a-short-directory-of-the-host-users-own`
at 7dd7e3a0, cut from main at fb620e26, checked on 2026-11-03 against
this change's proposal, design, spec and tasks.

**The work holds.** Every requirement and scenario of the place-scratch
spec is met by the code and by the test the scenario names, each of
which passed; every task is ticked and its condition holds; the checks
the tasks name pass.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 7dd7e3a0 | 27 of 27 tasks passed, each run, none replayed; 27 results recorded under their keys |
| `devenv shell -- moon run cli:sandbox --force`, outside any sandbox | passed: 8 sandbox tests, `exec_under_the_sandbox_binds_a_socket_in_a_folder_under_its_scratch` among them |
| `openspec validate a-places-scratch-is-under-a-short-directory-of-the-host-users-own --strict` | valid |
| `rg -n '"\.tmp"' crates/sluice/src/profiles/places.rs` (7.2) | two lines, both the former root: 929, `scratch`'s fallback for a record naming none; 980, `list_scratch`'s listing of `<common>/sluice/.tmp/` |
| `/private/tmp/sluice-501` before and after the sandbox run (2.3) | empty both times: the run's repository took its directory with it |

Once synced into `openspec/specs/place-scratch/spec.md`, the three
requirements this change amends ran past openspec's 500 characters, so
`root:specs` failed under `--strict`. That check passes
`openspec validate <change> --strict`, so the change itself never met
it. Earlier archives split such requirements in the main spec, and so
does this one, with no change of meaning. Each requirement keeps its
name and its first behaviour, and what else it said becomes a
requirement of its own, its scenarios moved with it: the path's parts,
a record naming a scratch in the repository, the sweep's reach, and the
user's directory not swept.

moon warned 24 times during the gate that it could not archive a task's
outputs under `.moon/cache/outputs/`. The tasks still passed. This is
the sandbox's known refusal (`docs/plans/phase-1-integrations.md`, the
table of what a sandboxed run cannot write), which the change
`a-checks-key-holds-its-executor` owns. This change did not cause it.

## Requirements and scenarios

| Requirement | Scenario | Where it is met | Test, and the task it passed in |
|---|---|---|---|
| The host user's scratch directory is made the user's own | Made by the first checkout | `make_scratch`, `make_own` (mode `0o700`, `AlreadyExists` no error) in `places.rs` | `the_users_scratch_directory_is_made_its_own`, cli:tools |
| | Emptied by the system | `make_scratch` in `adopt` at every run, and in `release_gone` | `a_scratch_the_system_emptied_is_made_again`, cli:tools |
| A scratch directory that is not the user's own is refused | A directory that is not the user's own | `own_dir`, `is_own` with the design's words; nothing changed or removed | `a_scratch_directory_not_the_users_own_is_refused`, cli:tools; `only_the_users_own_closed_directory_holds_a_scratch` (link, file, another uid, each bit of `0o077`), cli:test |
| A check under the sandbox writes its scratch and binds sockets in it | A tool's session socket in a folder under the scratch | the record names the resolved path (`user_scratch` resolves `/tmp`); `checks.rs` unchanged | `exec_under_the_sandbox_binds_a_socket_in_a_folder_under_its_scratch`, cli:sandbox |
| A checkout's scratch is at a short path whatever its name | A worktree at a long path | `new_scratch`: user's directory, `repository_key`, `scratch_name` | `exec_runs_from_a_worktree_at_a_long_path`, cli:tools; `a_scratch_is_the_users_directory_the_repositorys_key_and_its_name` (22 bytes past the root), `the_users_scratch_directory_is_resolved`, cli:test |
| | Two worktrees, and one worktree twice | as above | `a_worktree_keeps_one_short_scratch`, cli:tools |
| | Two repositories | `repository_key` of the common directory's path | `two_repositories_keep_their_scratch_apart`, cli:tools |
| A checkout from an older sluice is released where it was made | A worktree's checkout with no scratch on its record | `scratch(name, made)` unchanged | `a_place_from_before_keeps_its_scratch_until_released`, cli:tools |
| | A worktree's checkout whose record names a scratch in the repository | `adopt` keeps a record's `scratch`; `make_scratch` makes a path outside the user's directory as before | `a_place_in_the_repository_keeps_its_scratch_until_released`, cli:tools |
| | A candidate's checkout whose record names a scratch in the repository | `let_go` removes the path the record names | `a_place_in_the_repository_keeps_its_scratch_until_released`, cli:tools |
| Scratch no record names is removed with the abandoned checkouts | A stray directory and a held checkout's | `list_scratch` lists both roots; `named_scratch`, `Named::keeps` compare names | `scratch_no_record_names_is_removed`, cli:tools |
| | Another repository's scratch | `list_scratch` lists only `repository_dir()`, never the user's directory | `another_repositorys_scratch_is_left`, cli:tools |
| | A checkout being made as the sweep runs | records read after the listing | `a_scratch_made_after_the_listing_is_kept`, cli:test |
| | A checkout's record that cannot be read | `named_scratch` unchanged, a warning by path | `an_unreadable_record_keeps_its_scratch`, cli:tools |
| | A user's scratch directory that is not the user's own | `list_scratch` warns with `is_own`'s words, lists nothing under it, still lists `.tmp/` | `a_users_directory_not_its_own_is_not_swept`, cli:tools |

The requirement this change leaves as it is, "The checkout's record names
its scratch", still holds: `a_place_names_its_scratch_on_its_record` and
the gone worktree's scenario in `crates/sluice/tests/exec.rs` passed in
cli:tools.

## Tasks

All 17 tasks in the 7 groups are ticked, and each one's condition holds:

- **1.1** Record 0063 takes option 5 and gives the reasons for it. It
  weighs `XDG_RUNTIME_DIR`, `TMPDIR` and a key kept in the repository's
  state, and takes none of them. Rule core.30 and its details are
  amended. `root:adrs` and `root:changes` passed in the gate.
- **2.1 to 2.6** `user_scratch`, `repository_key` (in a `OnceCell`),
  `with_scratch_root`, `repository_scratch`, `own_dir`, `make_scratch`
  and `USER_SCRATCH_VAR` are added. The binding in `profiles/mod.rs`
  hands `SLUICE_USER_SCRATCH` to the place seam where it is set and not
  empty. `support::isolated` sets it, and `Repo`'s drop removes the
  repository's directory both there and under `/tmp/sluice-<uid>`. The
  stack's and the conformance suite's `Places` use
  `test_user_scratch()`. The doc comments in `places.rs` and
  `sluice-seams` are rewritten, citing 35 as amended and core.30.
  nix's `user` feature is in `Cargo.toml`.
- **2.7, 3.3** cli:test, cli:tools, seams:test, core:test and
  conformance:test passed in the gate.
- **3.1, 3.2** The sweep's two roots and its two new tests, as the spec
  table shows.
- **4.1** Passed with no change to the code, as the design said it
  would.
- **5.1** The test is macOS-only. `crates/sluice/moon.yml`'s `sandbox`
  task runs `--test exec`, and its comment says why. The test's name
  holds `sandbox`, so `cli:test` and `cli:tools` skip it with
  `--skip sandbox`.
- **6.1** `state.md` gives the scratch its own block, and its `.tmp/`
  line now covers only checkouts an older sluice made. `configuration.md`
  extends the `SLUICE_TMP` row and adds the line about `/tmp` beside
  `docker`. Neither page names a host, a tool or a team (core.26).
  `site:prose` passed.
- **7.1** The item for core.30 and record 0063 is gone from
  `docs/architecture/_open.md`. The file stays, since other items remain.
  The same commit says "combinations" in the core.39 item, for
  `site:prose`, and gives that reason.
- **7.2, 7.3** As the checks table shows.

## Notes

These are things I noticed. None of them is a failure:

- The proposal's "Touches" list leaves out three paths the work changed:
  `e2e/e2e.sh` and `plugins/screenshots-scene.sh`, which the design names
  (they set `SLUICE_USER_SCRATCH` beside `XDG_STATE_HOME`), and
  `site/src/content/docs/architecture/rules.md`, written again from
  `core.md` for `site:rules`.
- When the user's directory is refused, `adopt` and `materialize` have
  already written the checkout's record, so the record stays, as it did
  when `create_dir_all` failed before this change. No place is returned
  and nothing is made under the refused directory, which is what the
  spec asks. The test releases the record afterwards.
- `make_scratch` checks ownership only for a path under the user's
  directory, or the directory standing for it. Any other path a record
  names, a place's under `sluice/.tmp/` among them, is made with
  `create_dir_all` as before, which is what the design says.
