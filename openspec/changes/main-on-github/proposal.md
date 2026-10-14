# Proposal

## Why

sluice lands on a branch in one host's repository, and the branch most
teams share lives on a forge: today an operator pushes sluice's `main`
to GitHub by hand after every landing, and a push from anyone else goes
unnoticed until that hand push fails. The branch's pushed refs profile
(C.4, standing as written) closes this: the round reads the branch from
its remote and lands with a leased push. The design is in
agentplot/blueprints, `design/sluice/proposals/pull-requests-1.md` (C.4
pushed refs, C.7 `remote`), and in the site guide
`site/src/content/docs/guides/github.md`.

## What Changes

- A branch's policy may name the remote it lives on: `[branches."<pattern>"]
  remote = "origin"`. A branch with no `remote` stays a local branch,
  unchanged.
- For such a branch, every round fetches it from the remote before
  reading it, and forms every combination on the remote's revision. The
  local branch moves only when a landing's push is accepted (core.4).
- A landing is a push of the landing commit to the remote's branch,
  leased on the revision the combination was formed on (24, 45, 46). Only
  once the remote accepts it does the local branch move and the
  submission read landed.
- A push the remote refuses because the branch moved is reported, never
  retried blind; the next round forms the chain again on the new
  revision (S8). A push that fails for any other reason lands nothing
  and is settled by the next round's fetch, through the landing intent
  the step already writes.
- A round that cannot fetch lands nothing and says why; checks already
  running keep running.
- Only the branch is pushed. The records stay local files (C.6).
- Site: the configuration reference gains `remote`; the guide
  `guides/github.md` loses its "designed, not built" banner and matches
  what was built. An ADR records the decisions.

Out of scope, each its own later change:
- Pull requests: the `pull-requests` change, which builds on this one.
- A GitHub App as sluice's identity: sluice pushes with the credentials
  `git push` already uses on the host.
- Many hosts advancing one branch (`many-hosts-2.md`): one host leads.
- Setting up the forge's ruleset: the guide tells the operator how; no
  code does it.

## Capabilities

### New Capabilities
- `remote-branch`: a branch whose shared copy lives on a remote: fetched
  every round, landed by a leased push, and formed again when it moves
  outside the queue.

### Modified Capabilities

## Impact

- `crates/sluice/src/profiles/`: a pushed refs profile of the branch seam
  beside `local_refs.rs`, using the git command for fetch and push (core.1
  allows git); the instance chooses it per branch from the policy.
- `crates/sluice-core/src/policy.rs`: the `remote` key, with the parser
  and `.config/sluice.toml` landing together; reinstall the installed
  `sluice` before committing a policy that uses it.
- Tests: acceptance tests against a bare repository on the same host as
  the remote, in the tools tier.
- `site/src/content/docs/reference/configuration.md`,
  `site/src/content/docs/guides/github.md`, `docs/adr/`,
  `docs/plans/phase-1-pass-2.md` (piece 23's row).
