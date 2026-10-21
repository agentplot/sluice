---
number: 37
title: A remote branch's records are refs on its remote, kept by writing them
status: accepted
date: 2026-10-19
decision-makers:
- Chuck Swanberg
---

# A remote branch's records are refs on its remote, kept by writing them

Deciders: the operator, ratifying many-hosts-3 and ruling on the `peers` design, 2026-10-19.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 58 to 63, 72 to 77, 80, 82, 109, C.6, C.8; rules core.11, core.14; ADRs 0005, 0008, 0011, 0017; the OpenSpec change `peers`, design D1 to D4 and D10.

## Context and problem statement

Every host that submits to a remote branch keeps its own records today, so two hosts on one `main` run two queues, and a submission moves only while its own host runs sluice. The statement now has sluice on many hosts as peers alone, with no server: the records live on the shared remote, any instance with room advances a branch or takes a run, and a submission reaches its outcome with none of its submitter's hosts running (111). Where do the records live, which branches move there, and how does an instance know when it may take a branch another holds?

## Considered options

1. Move every branch with a `remote` to records on the remote.
2. Move a branch with a `remote` only when the policy names signers; keep every other branch in the host's own files.
3. A key per branch, `records = "remote"`.

## Decision outcome

Option 2. A branch's state is one root commit under `refs/sluice/state/<branch>`, written with a push leased on the commit read (62, 63); beside it, each signer's run records under `refs/sluice/runs/<signer>`, only added to (109); submitted heads, open combinations, kept output and each host's sign of life under `refs/sluice/heads/`, `combinations/`, `logs/` and `hosts/` (80, 82). The state names its lead. A step proceeds only while it can write the state naming itself, and a host with work takes the branch once it has itself seen the state unchanged for the host's `[peers] lease`, by its own clock (72, 73). The lease and the sign-of-life bound are the host's settings, never the policy's (74).

Option 1 would refuse the policy on `main`'s current revision, which lists executor kinds under `trust`, and no landing could fix it, since the gate is read from that revision (core.15). Under option 2 the upgrade changes nothing, and one reviewed landing that adds signers moves a repository to peers. The statement's C.6 says so. Option 3 is a second setting meaning what signers already imply: no gate on a shared remote can count a result without a signer (78).

### Consequences

- The records seam gains the `remote refs` profile; `local files` stays for one host.
- The host-local lock still keeps two processes of one host off one branch (core.14); between hosts, the lead in the state and the leased write do.
- A host's observation of how long a version has stood is kept in its own state directory; it decides only when to try a takeover, and the write decides.
- The first lead after the move pushes its own queued submissions as the starting state; every other host submits its own again under new ids.
