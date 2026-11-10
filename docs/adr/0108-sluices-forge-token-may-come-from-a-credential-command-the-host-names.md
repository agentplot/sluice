---
number: 108
title: sluice's forge token may come from a credential command the host names
status: accepted
date: 2026-11-09
decision-makers:
- Chuck Swanberg
- the design agent
---

# sluice's forge token may come from a credential command the host names

Status: accepted by the user on 2026-11-09, on agentplot-design's card: "One Boomdrive App; the runner holds its key and gives every tool its tokens". How sluice takes the token is agentplot-design's own choice, made on record 0019, the statement and agentplot/boomdrive's record 0007 as amended that day, and cheap to change.

Deciders: the user, 2026-11-09, who asked in agentplot-design's pane as the family moved to the boomdrivedev org: "Do we want to mimic the control plane cloud concept here and have a Boomdrive GitHub app that passes the credentials through from the cloud down to Sluice?", and chose the option above on its card.
Sources: record 0019; the statement's clause 134 and C.8's row of who holds the branch's credential; agentplot/boomdrive record 0007 as amended on 2026-11-09, and its rule core.33; agentplot/blueprints `design/principles.md` 6 and 14; git's credential-helper protocol (`gitcredentials(7)`, `git-credential(1)`).

## Context and problem statement

sluice lands as a GitHub App. The host's `[forge]` names the App's id and the path of its key, and sluice signs a JWT with the key and buys an installation token for each repository (record 0019). On a Boomdrive host, the user chose one App for the family, whose key the runner alone holds, or the cloud on a cloud host, and every tool asks the runner for its token by `boomdrive forge-token` (agentplot/boomdrive record 0007 as amended). sluice has no way to take a token it did not mint.

## Considered options

1. `[forge]` names a credential command, which sluice runs as git runs a credential helper, in place of a key.
2. A token in sluice's environment, set by whoever starts it.
3. sluice keeps its own App beside the family's.

## Decision outcome

Option 1.

- **`[forge] credential` names a command,** its program and arguments as a list, run with no shell. A table names `key` or `credential`, never both, and `app_id` with either, so the ruleset check and a refusal still name the App.
- **sluice speaks git's credential-helper protocol to it.** For a token it runs the command with `get`, writes `protocol`, `host` and `path`, the repository as `<owner>/<name>`, on its standard input, and reads `password` and `password_expiry_utc` from its standard output. sluice keeps the token in memory, as it keeps one it minted, and asks again five minutes before it expires.
- **A credential refused is bought afresh from the command.** sluice runs it with `erase` for the token the forge refused, then with `get`, wherever it would mint a fresh token (134, rule core.29). It never tries again with the refused token.
- **Nothing of the token is kept.** sluice writes no token to a file, a log or a git configuration, as with a token it minted (record 0019). A command that exits non-zero, or prints no `password`, fails the call as a key that cannot sign does, naming the command and its words.
- **Which tools take the family's token.** On a Boomdrive host, sluice and the runner take it from `boomdrive forge-token`: sluice to land, read the forge and post its check, the runner for its clones and fetches. crew does not, because it acts for the person with the person's own credentials, on cards and state no drive serves. An agent in a place keeps the person's credentials too, so the ruleset still keeps agents off a guarded branch.
- **sluice names no tool.** The command is the host's setting. sluice's code, site and skills give a placeholder command, never the runner's (rules core.1, core.26).

Not taken:

- **Option 2.** A token in the environment is read by every process it starts, an agent's command among them, and expires under a long run with no way to buy another.
- **Option 3.** Two Apps on one host is two keys to keep, and two names on the forge for one family's work, which the user ruled out.

### Consequences

- Good: on a Boomdrive host, the App's key is in one process, and sluice's identity is the same whether crew or the cloud is the control plane.
- Good: any host may hand sluice a token from its own secrets manager by the same setting, which record 0019 named as a later way to keep the key from agents.
- Neutral: `sluice forge check` verifies a command as it verifies a key: a token comes, the installation may write contents, and the App is every ruleset's only bypass.
- The statement's clause 134 and C.8's row of who holds the branch's credential say it, amended the same day. Rule core.29's and core.51's details read with it.
