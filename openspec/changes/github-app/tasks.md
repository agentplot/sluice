# Tasks

Starts only after `main-on-github` is on `main`. Work in a worktree off
`main` inside `devenv shell`; commit by pathspec with Conventional
Commits; land with `wt submit main`. Read first: `proposal.md`,
`design.md`, `specs/forge-identity/spec.md`; `docs/architecture/core.md`
(core.1, core.2); `crates/sluice/src/instance.rs`;
`crates/sluice/src/profiles/pushed_refs.rs`.

## 1. The host config

- [x] 1.1 Add `[forge] app_id` and `key` to the host config in `crates/sluice/src/instance.rs`, refusing a key file with group or other permission bits. Verify with unit tests: absent, present, a `0644` key refused with the file and `0600` named.
- [x] 1.2 Document `[forge]` in the host config section of `site/src/content/docs/reference/configuration.md`. Verify `devenv shell -- moon run site:prose` passes.

## 2. Tokens

- [x] 2.1 Add the app's JWT and installation token to `crates/sluice/src/profiles/github/`, with a blocking HTTPS client and an RS256 crate, refreshing five minutes before expiry and holding the token in memory only. Verify with unit tests: a JWT signed with a test key verifies with its public half and carries the app id and a ten-minute expiry; a fake token endpoint is called once per hour of tokens.

## 3. git with the app's token

- [x] 3.1 Add the `Identity` interface and give `PushedRefs` an optional one; with it, fetch and push run with `-c credential.helper=` and sluice's helper, over the remote's HTTPS URL. Add the hidden `sluice credential get`. Verify with an acceptance test that a git call made through `PushedRefs` with a fake identity receives the fake token through the helper, that `.git/config` and the environment of the parent hold no token afterwards, and that `git push` run directly in a worktree gets no credential from sluice.
- [x] 3.2 Wire the app's identity from `[forge]` where the seams are built, and make `sluice status` say when a branch with a `remote` has no identity of its own. Verify with an acceptance test of the status line with and without `[forge]`.

## 4. forge check

- [ ] 4.1 Add `sluice forge check [<branch>]` with the GitHub profile's checks (key mode, token, installation permissions, rules' bypass actors), exiting non-zero and naming each failure. Verify with tests against a fake API for each scenario of the spec, and `rg -n 'github' crates --glob '!**/profiles/**'` finds nothing (core.1).
- [ ] 4.2 Document `sluice forge check` in `site/src/content/docs/reference/commands.md`. Verify `moon run site:prose` passes.

## 5. Guide and decision

- [ ] 5.1 Write `site/src/content/docs/guides/github-app.md`, "Give sluice its own GitHub identity": create the app (permissions: contents read and write, commit statuses, checks and pull requests read and write, metadata read), install it on the repository, save the key with mode `0600`, add `[forge]`, set the ruleset's only bypass to the app, run `sluice forge check main`; link it from `guides/github.md` in place of "sluice pushes as you". Verify `moon run site:prose` and `cd site && bun run build` pass.
- [ ] 5.2 Write the next free-numbered ADR in `docs/adr/` (MADR, as 0016): sluice's identity on a forge is an app whose token reaches only sluice's own git calls; options: the operator's account, a machine user's token, an app. Record the live test's run if it was made. Verify `site/scripts/adrs.mjs` renders it.
- [ ] 5.3 Note the app in piece 23's entry of `docs/plans/phase-1-pass-2.md` and add the change's row. Verify the plan reads current.

## 6. Integration

- [ ] 6.1 Run all tiers (`devenv shell -- moon run ':#check-fast' ':#check-tools' ':#check-e2e'`) and `moon run cli:sandbox`; verify all pass before submitting.

## Workflow follow-up

- The operator creates the app and the ruleset following the guide, and
  `sluice forge check main` exits 0; until then the host's credentials
  push, as before.
- An opt-in live test against a scratch app and repository, run once by
  hand.
- Archive the change once it is on `main`.
