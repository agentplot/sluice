# Tasks

Starts only after `main-on-github` and `github-app` are on `main`. Work in a worktree off
`main` inside `devenv shell`; commit by pathspec with Conventional
Commits; land with `wt submit main`. Read first: `proposal.md`,
`design.md`, the three specs; `docs/architecture/core.md` (core.1,
core.2, core.4, core.11, core.14, core.15); `crates/sluice-core/src/ops.rs`
(`submit`, `forecast`, `withdraw`); `crates/sluice/src/profiles/worktrunk/`;
the guide `site/src/content/docs/guides/pull-requests.md`.

## 1. The policy table

- [x] 1.1 Add `[branches."<pattern>".pull_requests]` with `reviews` (default 0) and `checks` (default empty) to `crates/sluice-core/src/policy.rs`, refused on a branch with no `remote`. Verify with unit tests: present, absent, defaults, refused without `remote`.
- [x] 1.2 Document the table in `site/src/content/docs/reference/configuration.md`. Verify `devenv shell -- moon run site:prose` passes.

## 2. The forge interface

- [x] 2.1 Add `crates/sluice/src/profiles/github/` with the `Forge` trait (design: Decisions) and its file-backed fake. Verify unit tests of the fake: pull requests listed, reviews on a head, statuses and check runs by name, ready-for-review events, posting a status and a comment.
- [x] 2.2 Add `App`, the `Forge` over GitHub's REST API with the app's installation token: the `sluice` check run created or updated by name, reviews filtered to the head, other systems' check runs and statuses by name, the timeline's ready-for-review events, opening and editing pull requests. Verify with an opt-in live test (`SLUICE_GITHUB_TEST_REPO`, skipped when unset) against a scratch repository, run once by hand and recorded in the ADR; verify `rg -n 'github' crates --glob '!**/profiles/**'` finds nothing (core.1).

## 3. Intake

- [x] 3.1 Add the pull request driver (`session("pull/<n>")` fetching `refs/pull/<n>/head`, the agent from the body marker or the author) and the driver router. Verify with acceptance tests on the fake that a submission from `pull/7` has the pull request's head and agent.
- [x] 3.2 Add the intake pass before each round of a branch with `pull_requests` in `submit`'s wait, `step` and `run`; `run` serves branches whose policy names `pull_requests`. Verify with acceptance tests on the fake, one per scenario of `specs/pull-request-intake/spec.md`: waiting for a check, a draft, opened by hand, a stale review, a fork, a new commit while queued, closed while queued, blamed then retried, blamed and left alone. Run in the tools tier.

## 4. Status and comments

- [x] 4.1 Post the `sluice` check run from intake and from the driver's `notify`, with what is missing and the forecast while waiting, never updating it to what it already reads; comment once on failed and conflict. Verify with acceptance tests on the fake, one per scenario of `specs/pull-request-status/spec.md`, and that a landing leaves the pull request's head as the landing commit's second parent on the bare remote.

## 5. Submit

- [x] 5.1 `sluice submit` to a branch with `pull_requests` pushes the agent's branch, opens or updates the pull request with the body marker, and waits or returns; `wt submit` passes through, removing the worktree only on landing. Verify with acceptance tests on the fake and a bare remote for every scenario of `specs/pull-request-submit/spec.md`, through `wt submit` as the worktrunk tests do.
- [x] 5.2 Update `site/src/content/docs/reference/commands.md` and `state.md` for submitting to a branch with pull requests and the `pull/<n>` submitter. Verify `devenv shell -- moon run site:prose` passes.

## 6. Documentation and the decision

- [x] 6.1 Write the next free-numbered ADR in `docs/adr/` (MADR, as 0016): pull requests are a driver's, waiting is read from GitHub each round and never stored, a check run as the app as the state; options: pull requests in the core's records, commit statuses as the operator, webhooks. Verify `site/scripts/adrs.mjs` renders it.
- [x] 6.2 Update `site/src/content/docs/guides/pull-requests.md` to what was built: drop the banner and badge, make console output and posted states match. Verify `moon run site:prose` and `cd site && bun run build` pass.
- [x] 6.3 Mark piece 24 done in `docs/plans/phase-1-pass-2.md`. Verify the row reads done.

## 7. Integration

- [ ] 7.1 Run all tiers (`devenv shell -- moon run ':#check-fast' ':#check-tools' ':#check-e2e'`) and the sandbox task (`moon run cli:sandbox`); verify all pass before submitting.

## Workflow follow-up

- Turning pull requests on for sluice's own `main` is the operator's
  call, after review of this change; it is not part of it.
- Archive the change once it is on `main`.
