# Design

## Context

After `main-on-github`, `PushedRefs` runs `git fetch` and `git push`
for a branch with a `remote`, with whatever credentials the host's git
holds. The host's config is read by `crates/sluice/src/instance.rs`
(`~/.config/sluice/config.toml`, `[runs] max` today). Agents run git in
their worktrees with the same user and the same credentials as sluice.

## Goals / Non-Goals

**Goals:**
- Only git calls sluice makes can authenticate as the app.
- Everything GitHub-specific stays in `profiles/github/` (core.1); the
  pushed refs profile asks an identity for credentials and does not
  know it is GitHub's.

**Non-Goals:**
- Hiding the key from the host's user: an agent running as the same
  user can read a `0600` file. The ruleset keeps agents off `main`
  because they do not use the key, not because they cannot. A host
  where agents run as another user, or the key in a secrets manager,
  closes that; both are later.

## Decisions

**A GitHub App, its installation token per call.** sluice signs a JWT
(RS256, ten minutes) with the app's key, finds the app's installation
on the repository (`GET /repos/{owner}/{repo}/installation`), and
exchanges the JWT for an installation token
(`POST /app/installations/{id}/access_tokens`), valid an hour. A process
keeps its token in memory and mints a new one five minutes before it
expires. HTTPS calls use a small blocking client (ureq or reqwest
blocking); `gh` is not needed for the app. Alternative: a fine-grained
personal token of a machine user. Rejected: a second account to manage,
and no check runs for the pull requests change.

**The token reaches git by a helper on the command line.** `PushedRefs`
runs `git -c credential.helper= -c credential.helper='!<sluice>
credential' fetch|push ...` with the remote's HTTPS URL, and passes the
token to the helper over an inherited pipe or a one-time environment
variable on that child only. `sluice credential get` answers git's
credential protocol with `username=x-access-token` and the token. The
empty first `credential.helper` clears the user's helpers for that call
only. Nothing is written to `.git/config`, so an agent's git never
finds the helper. A remote whose URL is SSH is pushed over HTTPS for
that call (`https://github.com/<owner>/<repo>.git`), since the app's
token is not an SSH key. Alternative: `http.extraHeader` with the
token. Rejected: it shows in `ps` for the call's lifetime; the helper's
pipe does not.

**An identity behind the pushed refs profile.** `PushedRefs` takes an
optional `Identity` (a trait in the sluice crate: credentials for a
remote URL). `profiles/github/` implements it for the app; with no
`[forge]` table there is none and git uses the host's credentials, as
in `main-on-github`.

**`sluice forge check`.** A generic command in `main.rs` that asks the
forge profile to check the branch's remote; the GitHub profile checks
the key's mode, mints a token, reads the installation's permissions,
and reads the branch's rules
(`GET /repos/{owner}/{repo}/rules/branches/{branch}` and the rulesets
they come from) to list their bypass actors. The command's word is
`forge`, so `main.rs` names no forge (core.1).

**The host config.** `[forge] app_id` (integer) and `key` (path, `~`
expanded) in `instance.rs`'s form, `deny_unknown_fields` kept; a key
file with any group or other permission bit is refused.

## Risks / Trade-offs

- [The key on disk, readable by any process of the user] → see
  Non-Goals; the ruleset still refuses agents' pushes, which carry the
  user's own credentials.
- [GitHub down or the key revoked] → minting fails, the round lands
  nothing and says why, as a failed fetch does.
- [Tests need GitHub] → unit tests sign a JWT with a test key and check
  it against the public half; an acceptance test runs `git` through the
  helper with a fake identity and reads what git was given. A live test against a scratch repository and app,
  opt-in by environment, is run once by hand and recorded in the ADR.

## Migration Plan

1. This change lands; nothing changes on a host with no `[forge]`.
2. The operator follows the new guide: creates the app on the
   organization, installs it on the repository, saves the key with mode
   `0600`, adds `[forge]` to the host config, and changes the ruleset's
   bypass to the app alone.
3. `sluice forge check main` exits 0.
