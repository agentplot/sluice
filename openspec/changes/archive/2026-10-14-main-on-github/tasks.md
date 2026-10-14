# Tasks

Work in a worktree off `main` inside `devenv shell`; commit by pathspec
with Conventional Commits; land with `wt submit main`. Read first:
`proposal.md`, `design.md`, `specs/remote-branch/spec.md`;
`docs/architecture/core.md` (core.1, core.4, core.5, core.14, core.15,
core.17); `crates/sluice/src/profiles/local_refs.rs`; the landing in
`crates/sluice-core/src/step.rs`; the guide
`site/src/content/docs/guides/github.md`.

## 1. The policy key

- [x] 1.1 Add `remote` (a string) to `[branches."<pattern>"]` in `crates/sluice-core/src/policy.rs` and to `Rules`; a policy without it parses as before. Verify with unit tests in `policy.rs` for present, absent and wrong type.
- [x] 1.2 Document `remote` in `site/src/content/docs/reference/configuration.md` under `[branches."<pattern>"]`; verify `devenv shell -- moon run site:prose` passes.

## 2. The pushed refs profile

- [x] 2.1 Add `crates/sluice/src/profiles/pushed_refs.rs`: `PushedRefs` over `LocalRefs` and a remote name, `read` fetching on every call into `refs/remotes/<remote>/<branch>` and returning the fetched revision without moving the local branch, `move_to` pushing with `--force-with-lease=<branch>:<expected>` and setting the local branch to the landing commit on success, returning `Elsewhere` on a lease refusal and an error on any other failure. Verify with acceptance tests in `crates/sluice/tests/` against a bare repository as the remote: a read sees a revision pushed to the bare repository by another clone; a push lands; a push after another clone moved the branch is `Elsewhere`.
- [x] 2.2 Add the router (`Branches`) that sends each branch's calls to `PushedRefs` or `LocalRefs` by the branch's `remote`, refusing a `remote` the repository does not have with a message naming it; wire it where the seams are built for `submit`, `step` and `run`. Verify with an acceptance test that a branch with no `remote` behaves as before and one naming a missing remote lands nothing and names it.

## 3. Rounds and landings end to end

- [x] 3.1 Acceptance test through `sluice submit` and `sluice run` with a bare repository as `origin` and `[branches.main] remote = "origin"`: a submission lands on the bare repository's `main`, local `main` equals it, and no other ref on the remote changed. Verify the test passes in the tools tier (`devenv shell -- moon run cli:tools`).
- [x] 3.2 Acceptance test of a move outside the queue: another clone pushes to `origin/main` after the combination is formed; the landing is refused and reported, and the next round tests the submission on the new revision and lands it. Verify in the tools tier.
- [x] 3.3 Acceptance tests of an unsettled landing: (a) the push accepted and the step stopped before recording, the next round records it landed without a new run; (b) the remote unreachable during the push (the bare repository moved aside), the next round lands it without a new run. Verify in the tools tier.
- [x] 3.4 Acceptance test that a failed fetch lands nothing and the output says the fetch failed. Verify in the tools tier.

## 4. Documentation and the decision

- [x] 4.1 Write the next free-numbered ADR in `docs/adr/` (MADR, as 0016): a branch on a remote is read by fetch and landed by a leased push, through a profile wrapping local refs and a router per branch; options: a profile per branch, a `sync` method in the core, a mirror pushed after each landing. Verify `site/scripts/adrs.mjs` renders it under `/sluice/decisions/`.
- [x] 4.2 Update `site/src/content/docs/guides/github.md` to what was built: drop the "Designed, not built" banner and the `design` badge, and make its console output match the real output. Verify `devenv shell -- moon run site:prose` and `cd site && bun run build` pass.
- [x] 4.3 Mark piece 23 done in `docs/plans/phase-1-pass-2.md` (its row and entry, naming the tests). Verify the row reads done.

## 5. sluice's own `main`

- [x] 5.1 After the code above lands on `main`, reinstall sluice (`devenv shell -- cargo install --path crates/sluice --locked` in the `main` worktree, then `~/.cargo/bin/sluice protect`), then add `remote = "origin"` to `[branches.main]` in `.config/sluice.toml` and land it. Verify `git log origin/main -1` equals `git log main -1` after the landing, with no hand push.

## Workflow follow-up

- Archive the change once it is on `main`.
