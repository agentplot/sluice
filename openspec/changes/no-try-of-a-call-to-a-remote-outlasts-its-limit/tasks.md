# Tasks

Work in this change's place on
`change/no-try-of-a-call-to-a-remote-outlasts-its-limit`, cut from
`bolt/no-try-of-a-call-to-a-remote-outlasts-its-limit`, which is the
base every `sluice exec` below names. The source of the code is record
0109's branch in this repository, `fix/main--a-hung-fetch-holds-no-lock`:
66ad662f the record, 27ffa26e the code, 5fa9d784 the spec, all on main
at 69bc9934. Take each file or hunk from there by hand onto main as
record 0110 leaves it, as the design says. Take nothing of its
`openspec/changes/a-hung-fetch-holds-no-lock/`, since this change carries
the spec.

A unit test, or `cargo clippy`, runs directly. A test of the `tools`
feature and each moon task run through `sluice exec
bolt/no-try-of-a-call-to-a-remote-outlasts-its-limit -- …`, as
CLAUDE.md's Running checks has it. A remote that never answers is the
tests' own `Silent` listener on 127.0.0.1. No test reaches a real
remote, a credential, or the host's own sluice state. Commit by
pathspec, with Conventional Commits subjects scoped to the part of the
tree touched, citing clause 134, records 0109 and 0110, and rules
core.28 and core.29.

## 1. Main as record 0110 leaves it

- [x] 1.1 Merge main into this branch once main holds fix `a-fetch-and-its-hook-share-no-lock` (record 0110). Use the subject `chore(merge): take main at <rev> into change/no-try-of-a-call-to-a-remote-outlasts-its-limit, a sluice under sluice's own fetch fetches nothing (record 0110; core.29)`. Where main does not hold it yet, stop short with `flume needs`, naming the fix this change waits on. True when `git merge-base --is-ancestor main HEAD` exits 0 and `git grep -q 'fn under_own_fetch' HEAD -- crates/sluice/src/profiles/pushed_refs.rs` exits 0.

## 2. The decision recorded

- [x] 2.1 Write `docs/adr/0109-no-try-of-a-call-to-a-remote-outlasts-its-limit-and-a-fetch-lets-its-lock-go-with-it.md` from 66ad662f's, as design D7 gives it. Its Sources name record 0110 and the code as main holds it after record 0110. Its decision says that a sluice under sluice's own fetch waits for no lock at all (record 0110), and that every other fetch waits for it no longer than a try may take. It holds design D2, D4 and D6 as built within it, each with the path not taken. Where it names an OpenSpec change, it names this one. True when `sluice exec bolt/no-try-of-a-call-to-a-remote-outlasts-its-limit -- moon run root:adrs` passes.
- [x] 2.2 In `docs/architecture/core.md`, add record 0109's case to rule core.29's details after record 0110's, as design D7 gives it. It rules out a try of a call to a remote with no limit, and a try that holds the repository's fetch lock, or waits for it, past `retry::TRY_FOR`, citing deneb's hang of 2026-11-10 and signal 2026-11-10-agentplot-ops-e19d134d/01. It also rules out a try git did not end within its limit left running, or anything it started; a try started once the call's bound has passed since its first try; and a stopped push made again blind. Add to "Not ruled out" a try in flight when Ctrl-C ends a command that is no wait, left to end by itself and holding none of sluice's locks. Make the ADR list end `0108, 0109, 0110`. Leave the rule's own line as it is. True when `sluice exec bolt/no-try-of-a-call-to-a-remote-outlasts-its-limit -- moon run site:rules` passes.

## 3. A try's limit, held by the policy

- [x] 3.1 In `crates/sluice/src/retry.rs`, add `TRY_FOR`, thirty seconds, and a `try_for` field that `Policy::new` sets to it. Add `Policy::try_for()`, and a test's `Policy::trying_for(limit)`. Make the module doc say each try is bounded by `TRY_FOR`. In `Policy::run`, read `begun` at each start of the backoff loop, and stop as run out once the time since `begun` reaches the bound, as design D5 gives it. Add `no_try_starts_once_the_calls_time_is_spent` from 27ffa26e, the remote-retry spec's "Slow tries". True when `cargo test -p sluice --lib retry::` passes.
- [x] 3.2 In `crates/sluice/src/profiles/github/app.rs`, make `http_config` end each call at `crate::retry::TRY_FOR`, its doc saying no try of a call takes longer than a try of any call to a remote may. Drop the `Duration` import it leaves unused. True when `cargo clippy -p sluice --all-targets -- -D warnings` is clean.

## 4. A process run to a limit

- [x] 4.1 Add `crates/sluice/src/bounded.rs` from 27ffa26e, as design D2 gives it: `output`, `finish`, `Ended`, `GRACE`, `DRAINED` and `span`, ending a group through `runs::end_group`, with its three tests. Add `pub mod bounded;` to `crates/sluice/src/lib.rs`. True when `cargo test -p sluice --lib bounded::` passes.
- [x] 4.2 In `crates/sluice/src/identity.rs`, run `CredentialCommand::get` and `erase` on `bounded::finish`, taking `span` and `Ended` from `bounded`. Make `COMMAND_LIMIT` `crate::retry::TRY_FOR`, its doc saying it is as long as a try of any call to a remote has. Remove the module's own `wait`, `drain` and `span`. A command that gives nothing within its limit still fails, saying it gave nothing and was stopped, as forge-identity's "The command hangs" has it. True when `cargo test -p sluice --lib identity::` passes.

## 5. A git try ends at its limit, and the lock with it

- [x] 5.1 In `crates/sluice/src/profiles/pushed_refs.rs`, build design D3 and D4 beside record 0110's code. Keep `under_own_fetch`, `fetched_nothing`, `fetch_lock` and `FETCH_HELD_VAR` as main holds them. `git_as` keeps its `held` argument, names it in `SLUICE_FETCH_HELD`, and runs its command through `bounded::output` at `self.policy.try_for()`, returning `(Output, bool)`. `fetching` waits for the lock no longer than `try_for`. It returns `Result<Result<(File, PathBuf), PathBuf>>`. `call` reads a stopped try as `Judged::Unknown`, in sluice's words through `told`. It reads a lock held past the limit as `Judged::Never` and records it in `waited`. Where the call ends with `waited` set, it fails naming the lock. `told` adds no blank line where git printed nothing. Make the module doc say both: one fetch at a time holds the lock, and a process its git runs waits on no lock of its ancestor's (record 0110); no try outlasts the policy's limit for one, and a fetch's lock goes with it (record 0109). True when `sluice exec bolt/no-try-of-a-call-to-a-remote-outlasts-its-limit -- cargo test -p sluice --features tools --test protect -- a_hook_under_sluices_own_fetch_fetches_nothing a_fetch_whose_packing_runs_the_hook_ends` passes.
- [ ] 5.2 In `crates/sluice/tests/support/http.rs`, add `Silent` from 27ffa26e: a remote that reads each request and never answers, with `start`, `url`, `requests` and `all_let_go`. In `crates/sluice/tests/pushed_refs.rs`, add the helper `trying(limit)` and the test `a_fetch_the_remote_never_answers_is_stopped_and_lets_its_lock_go`, the remote-retry spec's "A fetch the remote never answers". True when `sluice exec bolt/no-try-of-a-call-to-a-remote-outlasts-its-limit -- cargo test -p sluice --features tools --test pushed_refs a_fetch_the_remote_never_answers_is_stopped_and_lets_its_lock_go` passes.
- [ ] 5.3 In `crates/sluice/tests/pushed_refs.rs`, add `a_fetch_waits_for_the_lock_no_longer_than_a_try` from 27ffa26e, the remote-retry spec's "A lock held past a try's limit". True when `sluice exec bolt/no-try-of-a-call-to-a-remote-outlasts-its-limit -- cargo test -p sluice --features tools --test pushed_refs a_fetch_waits_for_the_lock_no_longer_than_a_try` passes.
- [ ] 5.4 In `crates/sluice/tests/pushed_refs.rs`, add `a_push_the_remote_never_answers_is_stopped_and_not_pushed_again` from 27ffa26e, the remote-retry spec's "A push the remote never answers". True when `sluice exec bolt/no-try-of-a-call-to-a-remote-outlasts-its-limit -- cargo test -p sluice --features tools --test pushed_refs a_push_the_remote_never_answers_is_stopped_and_not_pushed_again` passes.
- [ ] 5.5 In `site/src/content/docs/guides/github-app.md`, where it gives the pauses between tries, say that no single try runs past thirty seconds. Say that sluice stops a fetch or push GitHub has not answered by then, tries a fetch again, and reads the branch after a push. Name no host or tool of ours (rule core.26). True when `sluice exec bolt/no-try-of-a-call-to-a-remote-outlasts-its-limit -- moon run site:prose` passes with no warning on the page.
