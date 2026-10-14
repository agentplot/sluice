---
number: 18
title: sluice pushes as a GitHub App whose token reaches only its own git
status: accepted
date: 2026-10-14
decision-makers:
- Chuck Swanberg
---

# sluice pushes as a GitHub App whose token reaches only its own git

Deciders: the user, planning the change `github-app` and ruling on how `sluice forge check` reads a ruleset's bypass list, 2026-10-14.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clause C.4; `design/sluice/proposals/many-hosts-2.md` (C.4: "an identity no session on any host can use"); ADRs 0003, 0017.

## Context and problem statement

Since ADR 0017, GitHub's `main` is the real one, and sluice lands on it by a push with the host's own git credentials. Agents run git as the same user with the same credentials, so an agent's `git push origin HEAD:main` reaches GitHub's `main`: the local `reference-transaction` hook guards only the local branch. C.4 asks for the forge's rules, which only sluice's identity bypasses. Which identity does sluice push as, and how does it keep that identity from the agents beside it?

## Considered options

1. The operator's account, the ruleset's bypass, as now.
2. A machine user's fine-grained personal token, the ruleset's bypass.
3. A GitHub App, the ruleset's only bypass, whose installation token sluice gives only to its own git calls.

## Decision outcome

Option 3. The host's config names the app in `[forge]`: its id and the path of its private key, a file with no group or other permission bits. sluice signs a ten-minute JWT with the key (RS256), finds the app's installation on the repository, and buys an installation token for that repository, good for an hour. A process keeps the token in memory and buys another five minutes before it ends.

The pushed refs profile takes an optional identity, an interface in the sluice crate that the GitHub profile implements (rule core.2), so the profile asks for credentials and does not know they are GitHub's (rule core.1). With an identity, each fetch and push runs as `git -c credential.helper= -c credential.helper='!<sluice> credential' …` over the remote's HTTPS URL, its configured URL before any `insteadOf`. The empty helper clears the user's helpers for that call alone; `sluice credential get` answers git with the token, which it finds in that git call's environment and nowhere else. Nothing is written to a git config, so the git an agent runs never finds the helper, carries the user's own credentials, and is refused by the ruleset. Every fetch and push sluice makes, with an identity or without, runs with no terminal prompt, no askpass program and a credential manager told never to ask: sluice runs unattended, so a credential it cannot find is an error. With no `[forge]`, sluice pushes with the host's credentials, as ADR 0017 does, and `sluice status` says the branch has no identity of its own.

`sluice forge check` verifies the setup: the key reads, a token mints, the installation may write contents, and every ruleset on the branch has the app, always, as its only bypass. GitHub shows a ruleset's bypass list only to a caller with write access to the ruleset, and an app that could edit the ruleset could let anyone through it, so the check reads the rulesets with the operator's own token (`GH_TOKEN`, `GITHUB_TOKEN` or `gh auth token`) and everything else with the app's.

Option 1 is what lets an agent's push through: the agents hold the operator's credentials. Option 2 is a second account to manage, its token a secret like the key, and a personal token cannot create the check runs the `pull-requests` change needs. An `http.extraHeader` carrying the token instead of a helper was rejected: it shows in the process list for the call's lifetime.

### Consequences

- The key is a file the host's user may read, and agents run as that user: the ruleset keeps agents off `main` because their git never uses the key, not because they cannot read it. Agents under another user, or the key in a secrets manager, would close that; both are later.
- A token that cannot be bought, GitHub down or the key revoked, fails the round as a failed fetch does: it lands nothing and says why.
- The tests sign with a key made for them, run git through the helper against a fake HTTP remote, and check the app against a fake of GitHub's API on a port the system picks; no test reaches the user's git config or credentials.
- The live test against a scratch app and repository is the operator's, run once by hand; it has not been run.
