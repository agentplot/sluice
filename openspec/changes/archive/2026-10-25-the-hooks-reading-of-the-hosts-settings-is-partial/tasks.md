# Tasks

Verification runs through sluice: `sluice exec main -- <command>` while
iterating, and `sluice exec main` for the gate once the work is
committed. `core:test` is `cargo test -p sluice-core`; `sluice:test` and
`sluice:tools` are the sluice crate's moon tasks (`protect.rs` and
`instance.rs` are in `sluice:tools`).

## 1. The guarding reading of the policy (core)

- [x] 1.1 Add `Policy::guarding(text)` in `crates/sluice-core/src/policy.rs` (design D3): `[branches]` patterns in written order with `Rules::default()` and each `remote`, `[signers]` names only, every other table and field passed over; a `[branches]` entry that is not a table, a `remote` that is not a string or `[signers]` that is not a table is an error naming it. Verify with unit tests in `policy.rs`: a policy with an unknown top-level table, an unknown field in `[branches."bolt/*"]` and an unknown field in `[signers.box]` reads, while `Policy::parse` of the same text still fails; `remote = 1` fails naming `remote`; and every example policy the existing example test reads gives the same patterns, `remote`s and `signers` names both ways (`sluice exec main -- moon run core:test`).
- [x] 1.2 Give `Sluice` a reading set by `Sluice::guarding(self)`, beside `as_peer` and `serving`, and make `policy_at` parse with `Policy::guarding` under it (`crates/sluice-core/src/lib.rs`). Verify with a core unit test that `protects` on a branch whose policy holds an unknown field answers `true` guarding and errs otherwise (`sluice exec main -- moon run core:test`).

## 2. The guarding reading of the host's settings and the hook's binding

- [x] 2.1 Add `Instance::guarding()` and `Instance::guarding_from(path)` in `crates/sluice/src/instance.rs` (design D2): `[forge]`'s `app_id` and `key` through a form without `deny_unknown_fields`, every other table and field passed over, the defaults for the rest and no `[signing]`; a file that is not TOML or a `[forge]` missing `app_id` or `key` or with either of the wrong kind is an error naming the file. Update the module's doc comment to say the hook reads only `[forge]`. Verify in `crates/sluice/tests/instance.rs`: settings with `[runs] cores = 12`, `[later] x = 1` and `[forge] later = 1` beside a valid `app_id` and `key` read guarding with that forge and fail `from_file`; a non-TOML file fails both (`sluice exec main -- moon run sluice:tools`).
- [x] 2.2 Give `Branches` and `Records` a guarding switch (`profiles/branches.rs`, `profiles/records.rs`) that routes each of their policy reads (`named`, `policy_of`) through one function choosing `Policy::guarding` or `Policy::parse`; in guarding, `Records::open` gives the remote refs profile no signer keys and no signing (design D3). Add `Bound::guarding(dir)` in `profiles/mod.rs` binding with `Instance::guarding()` and both switches set. Verify by the protect tests of group 3.
- [x] 2.3 Bind the hook with `Bound::guarding` in `crates/sluice/src/main.rs`'s `fn hook`, and set `Sluice::guarding` for `Command::Hook` in `run` (`crates/sluice/src/lib.rs`); keep every error passing through `protect::unread` with exit 0 (design D1, D4). Verify by the protect tests of group 3, and that `sluice status` with `[runs] later = 1` in the host's settings still fails naming the file and the field (a test in `crates/sluice/tests/protect.rs`, `sluice exec main -- moon run sluice:tools`).

## 3. The hook guards as before under settings and a policy it does not fully know

- [x] 3.1 In `crates/sluice/tests/protect.rs`, with the host's settings (under the test's `XDG_CONFIG_HOME`) holding `[runs] cores = 12` and `[later] x = 1`: `git update-ref refs/heads/bolt/x <unit/a>` is refused with one `protected` line and no sluice warning, and `git reset --hard` in the worktree on `bolt/x` passes saying nothing (the spec's first two scenarios). Verify with `sluice exec main -- moon run sluice:tools`.
- [x] 3.2 In `protect.rs`, with the policy at `bolt/x`'s revision giving `[branches."bolt/*"]` `later = true` and a `[later]` table: `git branch -f bolt/x unit/a` is refused with one `protected` line. And on the copy-apart repository (`copy_apart`), with an unknown field in both the policy and the settings, `git update-ref refs/heads/bolt/x <origin's revision>` is refused and says sluice levels this host's `bolt/x` at each landing. Verify with `sluice exec main -- moon run sluice:tools`.
- [x] 3.3 In `protect.rs`: host settings that are not TOML, and separately a policy whose `[branches."bolt/*"]` has `remote = 1`, each let `git update-ref refs/heads/bolt/x <unit/a>` through with exactly one `let this through unread` warning naming the settings file or `remote`, and no `protected` line; the existing unparsable-policy test still passes. Verify with `sluice exec main -- moon run sluice:tools`.

## 4. The hook is the sluice that installed it, re-installed at the next run

- [x] 4.1 Add `protect::reinstall(dir, program)` in `crates/sluice/src/protect.rs` (design D5): writes the shim for `program` only where the hook in place carries sluice's marker and names another program, keeping the chained hook; nothing otherwise, a tracked hooks directory included. Call it in `main.rs` before binding for every command but `hook`, `supervise`, `credential` and `protect`, saying a failure once as `sluice: warning: branch protection not re-installed: …` on stderr. Verify in `protect.rs`: a hook installed by a copied sluice (`copy_executable`, `run_copied`) is rewritten to name the test's sluice by `sluice status`, its chained hook unchanged and nothing said of the hook; after `sluice protect --remove`, `sluice status` leaves no hook (`sluice exec main -- moon run sluice:tools`).
- [x] 4.2 In `protect.rs`, with the hook installed and a directory first on git's `PATH` holding an executable `sluice` that exits 0 having read nothing, `git update-ref refs/heads/bolt/x <unit/a>` is still refused by the installed sluice. Verify with `sluice exec main -- moon run sluice:tools`.

## 5. Record the decision

- [x] 5.1 Write `docs/adr/0059-the-protection-reads-only-what-guarding-needs.md` (MADR, as its neighbours are written; design D6): the hook's partial reading and every other command's strict one, why `[forge]`, the branches and the signers are what guarding needs, and why a command other than `submit`, `wait`, `step`, `run` and `protect` re-installs only a hook already there; cite 124 and rules core.2 and core.4. Point `protect.rs`'s module comment and `instance.rs`'s at it. Verify the ADR is listed where its neighbours are and that `sluice exec main` passes (`root:format` checks the Markdown).

## 6. The gate

- [x] 6.1 Commit the work and run the gate's checks with `sluice exec main`; verify every task it runs passes.
