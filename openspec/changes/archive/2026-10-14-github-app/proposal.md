# Proposal

## Why

Once `main-on-github` lands, GitHub's `main` is the real one, and the
only bypass of its ruleset is the operator's own account. Agents push
with the operator's credentials, so `git push origin HEAD:main` from an
agent's worktree reaches GitHub's `main`: the local
`reference-transaction` hook guards only the local branch, and the push
touches none. C.4 (pushed refs) asks for "the forge's rules, which only
sluice's identity bypasses"; a GitHub App gives sluice an identity of
its own, the ruleset's one bypass, which no session on the host holds
(`design/sluice/proposals/many-hosts-2.md`, C.4: "an identity no session
on any host can use").

## What Changes

- The host's config, `~/.config/sluice/config.toml`, gains a
  `[forge]` table naming a GitHub App: its id and the path of its
  private key on this host. The key never enters a repository.
- sluice mints a short-lived installation token for the repository from
  the key when it needs one, holds it in memory only, and mints a new
  one before it expires.
- sluice's own `git fetch` and `git push` for a branch with a `remote`
  on GitHub carry the app's token through a credential helper given on
  the git command line, so only sluice's git calls hold it; agents' git
  is unchanged and their pushes to `main` are refused by the ruleset.
- With no `[forge]` table, sluice pushes with the host's own credentials
  as `main-on-github` does, and `sluice status` says the branch has no
  identity of its own.
- `sluice forge check` reports whether the app is set up for the
  branch's remote: the key reads, a token mints, the app is installed on
  the repository with the permissions it needs, and the branch's
  ruleset lets only the app through.
- Site: a guide, "Give sluice its own GitHub identity", for creating the
  app, installing it, keeping its key, and the ruleset with the app as
  its only bypass; the GitHub guide and the configuration and commands
  references point to it. An ADR records the decisions.

Out of scope:
- Pull requests, statuses and check runs: the `pull-requests` change,
  which follows this one and uses the app's token.
- Creating the app or the ruleset by code: the operator does both in
  GitHub's settings, following the guide; `sluice forge check` verifies
  them.
- Keeping the key in a secrets manager (1Password, secretspec): a later
  change; the key is a file with mode `0600`.
- Many hosts sharing one app.

## Capabilities

### New Capabilities
- `forge-identity`: sluice's own identity on the forge: the app's key
  on the host, the tokens minted from it, the git calls that carry
  them, and the check that the forge lets only that identity move the
  branch.

### Modified Capabilities

## Impact

- `crates/sluice/src/profiles/github/` (core.1: GitHub named only in a
  profile): the app's JWT, the installation token, the credential
  helper, and the check.
- `crates/sluice/src/instance.rs`: the host config's `[forge]` table.
- `crates/sluice/src/profiles/pushed_refs.rs` (from `main-on-github`):
  its git calls take the credential helper when an identity is set.
- `crates/sluice/src/main.rs`: `sluice forge check`, and the hidden
  `sluice credential` that git calls as its helper.
- A JWT crate (RS256) in `crates/sluice/Cargo.toml`.
- Depends on `main-on-github`.
- `site/src/content/docs/guides/` (new guide), `reference/configuration.md`,
  `reference/commands.md`, `guides/github.md`, `docs/adr/`,
  `docs/plans/phase-1-pass-2.md`.
