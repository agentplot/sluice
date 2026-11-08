# Tasks

This change builds clauses 88, 107, 111 and 113 as they bear on GitHub
Actions jobs, Part C's launched executor and C.8, and scenario S40 of
the statement in agentplot/blueprints, `design/sluice/requirements.md`,
on top of the `peers` change, which lands first.

Read first: this change's proposal, specs and design; the `peers`
change's design (archived once it lands); ADRs 0019, 0021, 0032, 0034;
`docs/architecture/core.md`. Work inside `devenv shell`. Fast tier:
`moon run ':#check-fast'`; real tools: `moon run ':#check-tools'`. Every
test is part of a moon task. Every git call a test makes runs with no
credential helper and no prompt.

## 1. Keyless signatures

- [ ] 1.1 Verify a sigstore bundle from `actions/attest` in the GitHub profile, as design D1 says: a public-good bundle with the `sigstore` crate, a bundle of GitHub's own instance by sluice's checks of the certificate chain, the RFC 3161 timestamp and the envelope's signature; both trusted roots fetched by TUF and cached on the host; the certificate's workflow, ref, repository and issuer checked against `[signers.<name>] github`; hand the verifier to the records profile through the binding; verify unit tests over recorded bundles in `crates/sluice/tests/profiles/forge/`, under `*.json` fixtures excluded from formatting with their reason in `treefmt.nix`; a wrong ref, a wrong workflow, a pull request's ref; and a host with no cached root reading the result as unverified.

## 2. The workflows and `sluice job`

- [ ] 2.1 Add the run and sign jobs and `workflow_run` of `sluice-pr.yml` to the template of `sluice.yml` in `crates/sluice/src/profiles/github/workflows/`, whose round job `forge-rounds` writes, and write `sluice-pr.yml` beside it, with the `sluice` environment on the sign job beside the round job and no `id-token` on run jobs, the run job uploading its report and moon's manifests and the sign job signing only what `proven` keeps, as design D2 says; verify `root:format` passes on them.
- [ ] 2.2 Add `sluice job run`, `sign` and `record` beside `sluice job round`, which `forge-rounds` builds, as design D2 says; verify `cli.rs` cases over a scratch repository and a bare remote: a run job records nothing, a sign job drops a key the tree does not bear out, a record under a lapsed claim is refused.
- [ ] 2.3 Extend `sluice init actions`, whose round part `forge-rounds` builds: write `sluice.yml` with its run and sign jobs and `sluice-pr.yml`, give `[pools.forge]` a runner for each system the policy names, leave a differing file unless `--force`, and print the identity token permission the sign job needs; verify `cli.rs` cases over a scratch repository for each actions-pool `init` scenario.
- [ ] 2.4 Extend `sluice forge check`, whose round part `forge-rounds` builds (the round job on the branch, the dispatch permission, the environment's branch rule), with the run and sign jobs and the forge signer naming the workflow; verify `tests/profiles/forge.rs` cases for each forge-identity scenario.

## 3. Pull requests

- [ ] 3.1 Record a pull request's head runs from `sluice-pr.yml`, signed by `sluice.yml`'s sign job, and read them by key in its combination; refuse `pull_requests.checks` naming sluice's own; verify conformance tests for S40 and each pull-request-intake scenario.

## 4. Guides

- [ ] 4.1 Write `site/src/content/docs/guides/github-actions.md`: the app and its environment, `sluice init actions`, the signers, submit and close the laptop, reading the checks on a pull request, the cost of the pool and how standing executors come first; link it from the executors and systems guide, the configuration reference's `[pools]`, the roadmap and the setting-up skill's peers profile; update `reference/commands.md` for `sluice init actions`, `sluice job` and `sluice forge check`; update `guides/pull-requests.md` for head runs and the checks per system; update the setting-up skill's forge and executor choices; update README's Get started and Quick start for the pool; verify `site:prose` reports nothing, the site build's link check passes and `moon run plugin-sluice:validate`. README is outside every gate (core.21 Details): read it through once by hand.

## 5. Trial and integration

- [ ] 5.1 Try it by hand on a scratch repository on GitHub, as the gate cannot reach GitHub: submit, switch the host off, see a round job land the submission, a run on `macos-15`, a cancelled job run again, and a pull request that edits the workflow sign as nothing; record the run links in the PR description.
- [ ] 5.2 Add S40 beside the peers scenarios in `crates/sluice-conformance/tests/peers.rs`; run `:#check-fast`, `:#check-tools`, `:#check-e2e`, `openspec validate actions-pool --strict` and the site build; verify every one passes.

## Workflow follow-up

- Land with `sluice submit main` from this change's worktree, after `peers`.
- Adopting the pool on sluice itself is the operator's: the app's `sluice` environment, `sluice init actions`, and one landing that adds the forge signer to `trust` and `[pools]`.
- Archive with `/opsx:archive actions-pool` once landed.
