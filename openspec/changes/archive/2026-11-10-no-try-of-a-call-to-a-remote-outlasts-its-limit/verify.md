# Verify: no-try-of-a-call-to-a-remote-outlasts-its-limit

Verified on 2026-11-10 at `612a07f1` on
`change/no-try-of-a-call-to-a-remote-outlasts-its-limit`, against its
base `bolt/no-try-of-a-call-to-a-remote-outlasts-its-limit`, with sluice
0.78.1.

**Verdict: the whole checks pass, and the work holds. The change can be
archived.**

## The whole checks

`sluice exec bolt/no-try-of-a-call-to-a-remote-outlasts-its-limit` ran
the bolt's gate, `#check-fast`, once. It exited 0: `passed: 28 tasks`,
`recorded 28 results under their keys for 612a07f`, in 5m 59s.

The tasks were cli:test, conformance:test, core:test, fakes:test,
seams:test, nix:lint, nix:test, nix:typecheck, plugin-board:engine-types,
plugin-board:test, plugin-board:typecheck, plugin-board:validate,
plugin-herdr:manifest, plugin-herdr:test-dash, plugin-herdr:test-sh,
plugin-sluice:hooks, plugin-sluice:lay-out, plugin-sluice:validate,
root:adrs, root:changes, root:format, root:lint, root:marketplace,
root:openspec-claude, root:specs, root:toolchain, site:prose and
site:rules.

- `cli:test` ran the sluice crate's lib tests, 261 passed. They include
  `retry::tests::no_try_starts_once_the_calls_time_is_spent`, the four
  `bounded::tests`, and every `identity::tests` test.
- `site:prose` had 0 errors and 0 warnings. None of its 37 suggestions is
  on `guides/github-app.md`.
- moon warned that it could not archive two plugin-herdr outputs and
  could not hash some directory inputs. Neither affects the result.

The `tools` feature's tests are main's gate, `#check-tools`, not the
bolt's. The tasks name five of them, so they were run once through
sluice:
`sluice exec bolt/no-try-of-a-call-to-a-remote-outlasts-its-limit -- cargo test -p sluice --features tools --test pushed_refs --test protect -- <the five>`.
It exited 0.

- protect: `a_hook_under_sluices_own_fetch_fetches_nothing` and
  `a_fetch_whose_packing_runs_the_hook_ends` passed.
- pushed_refs: `a_fetch_the_remote_never_answers_is_stopped_and_lets_its_lock_go`,
  `a_fetch_waits_for_the_lock_no_longer_than_a_try` and
  `a_push_the_remote_never_answers_is_stopped_and_not_pushed_again`
  passed.

`openspec validate no-try-of-a-call-to-a-remote-outlasts-its-limit --strict`
reports the change valid.

## Requirements and scenarios

| Requirement | Scenario | Where it holds | Met |
|---|---|---|---|
| No try of a call to a remote outlasts its limit | A fetch the remote never answers | `retry::TRY_FOR` is 30s. `git_as` runs each try through `bounded::output` at `policy.try_for()`, in a group of its own, ended through `runs::end_group`. The API agent's `timeout_global` and `identity::COMMAND_LIMIT` are `TRY_FOR`. The test asserts sluice's exact words, more than one try, every connection let go, and the lock free. | yes |
| A try stopped at its limit is a call whose answer was not read | A push the remote never answers | `call` reads a stopped try as `Judged::Unknown`, so the policy tries a read again and never a push. `push` then reads the ref through `on_remote`, and `Pushed::Unanswered` carries the read's error. The test asserts one `git-receive-pack`, sluice's words and `the push went unanswered`. | yes |
| A fetch waits for the fetch lock no longer than a try may take | A lock held past a try's limit | `fetching` tries the lock every 20 ms until `try_for`, and returns `Ok(Err(path))` when it stays held. `call` reads that as `Judged::Never` and sets `waited`, which a later try clears once it takes the lock. A call that ends with `waited` set fails naming the lock, as no `ReadRefused`. `under_own_fetch` is still asked before any try (record 0110). The test asserts no refused read, the lock named with `past 300ms`, and a read once the lock is let go. | yes |
| The policy's time is set by what called it (modified) | Slow tries | `Policy::run` reads `begun` each time its backoff loop starts. Its `when` stops as run out once the time since `begun` reaches `bound`. `no_try_starts_once_the_calls_time_is_spent` asserts two tries and one pause. | yes |
| The policy's time is set by what called it (modified) | Five minutes of 5xx in a wait; Ctrl-C between tries; A round outside a wait | These scenarios are unchanged from the main spec. The existing retry tests and the wait tests still pass in `cli:test`. | yes |

The push test does not assert the words of the failed settling read
itself; it asserts only what comes before them. The code gives that
read's error as `holds` after `the push went unanswered, and`, so the
scenario holds. This is a gap in the test, not in the behaviour, and it
does not block the change.

## Tasks

All 14 tasks in `tasks.md` are ticked. Each one's own "True when" was
checked:

- 1.1: `git merge-base --is-ancestor main HEAD` exits 0 with main at
  `5554f215`, and `fn under_own_fetch` is in `pushed_refs.rs`.
- 2.1: record 0109 is in `docs/adr/`. Its Sources name record 0110 and
  main at 5554f215, and it names this change. It holds D2, D4 and D6,
  each with the path not taken. `root:adrs` passed.
- 2.2: core.29's details add record 0109's case after record 0110's, as
  well as the Ctrl-C "Not ruled out". The ADR list ends
  `0108, 0109, 0110`, and the rule's own line is unchanged.
  `site:rules` passed.
- 3.1 and 3.2: `TRY_FOR`, `try_for`, `Policy::try_for`,
  `Policy::trying_for`, `begun` and the test are in `retry.rs`. `app.rs`
  takes `TRY_FOR`, and its `Duration` import is gone. `root:lint` passed.
- 4.1 and 4.2: `bounded.rs` has `output`, `finish`, `Ended`, `GRACE`,
  `DRAINED`, `span` and its tests, and `pub mod bounded` is in `lib.rs`.
  `identity.rs` runs `get` and `erase` on `bounded::finish`, and its own
  `wait`, `drain` and `span` are gone. A stopped command still fails
  with `gave nothing in … and was stopped`.
- 5.1 to 5.4: `git_as` returns `(Output, bool)` and names `held` in
  `SLUICE_FETCH_HELD`. `fetching` returns
  `Result<Result<(File, PathBuf), PathBuf>>`. `told` adds no blank line
  where git printed nothing. The module doc says both records. `Silent`
  and `trying` are in the tests, and all five named tests passed.
- 5.5: `guides/github-app.md` says no single try runs past thirty
  seconds, that sluice tries a fetch again, and that it reads the branch
  after a push. It names no host or tool of ours, and `site:prose`
  passed with no warning on the page.

## Rules kept

- core.10: a try's group is ended with its time limit, through
  `runs::end_group`.
- core.28: a fetch stopped every time is a refused read, logged once per
  cause, and a lock held every time is logged on no branch.
- core.29: there is one try limit, held by the policy. No try starts
  once the call's time is spent, and a stopped push is never made again
  blind.
- core.1: `bounded.rs` names no task runner, worktree tool or forge.
