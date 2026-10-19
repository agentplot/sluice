---
number: 32
title: A run counts through a signer the policy names, in a signed in-toto statement
status: accepted
date: 2026-10-19
decision-makers:
- Chuck Swanberg
---

# A run counts through a signer the policy names, in a signed in-toto statement

Deciders: the operator, ratifying many-hosts-3 and ruling on the `peers` design, 2026-10-19.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 27 to 29, 69, 78, 79, 85, 108, 109, C.6, C.7; rules core.1, core.7, core.9; ADRs 0004, 0015; the OpenSpec change `peers`, design D5, D6 and D9.

## Context and problem statement

A gate trusts executors (27, 28), and a run's record names its executor. On one host only sluice writes the records, so the name is true. Under peers every host writes them, and any host can write any name. A run's system is now part of what it proves (85), and a pass on a key satisfies every branch that trusts whoever proved it (69). How does a run prove who made it, on which system, and whether it was confined, so that every host can check it from the records alone?

## Considered options

1. Trust the record's executor field, and protect the records' refs on the forge.
2. Signers named in the policy, each with the claims it may make; every run record an in-toto statement in a DSSE envelope, signed with an SSH key the policy lists or, on a forge's job, keylessly with the job's workload identity.
3. GPG signatures over the records.

## Decision outcome

Option 2. `[signers.<name>]` names one identity and the most it may claim: `confined`, and the `systems` it runs. A remote branch's `trust` lists signer names, and `confined = true` asks for confined runs. A result counts only when a trusted signer signed it and every claim it relies on is one that signer may make (78, 79). A replay counts only when a trusted signer has a pass for the same key on the same system; the cache supplies outputs and decides nothing (108). A one-host branch in the host's own files signs nothing, and its `trust` keeps listing executor kinds (78).

A run's results are one statement: a subject per check and system, digest the task runner's hash, and a predicate with each outcome, the claims, the branch, the combination, the claim and the time. An SSH signer signs the DSSE pre-authentication encoding with `SSHSIG`, namespace `sluice-run`, through the `ssh-key` crate. A GitHub job signs through `actions/attest`, which uses the sigstore instance GitHub keeps for the repository, public for a public one and GitHub's own for a private one; sluice verifies the bundle with the `sigstore` crate and checks the certificate's workflow, ref, repository and issuer against the signer. A signer signs only keys it computed from the combination itself, in a step that runs none of its tasks (C.6), so a check cannot sign a pass under another combination's key.

Option 1 fails because GitHub's rules protect only branches and tags: any host that can push can write `refs/sluice/`. Option 3 adds a second key system for operators who already hold SSH keys, and does nothing for a forge's job.

### Consequences

- A rule under "Verdicts and attribution": a result counts only through a signer the policy names, never through a record's executor field alone.
- `cache` in `trust` is read as absent, with a warning; refusing it would refuse `bolt/*`'s current policy (core.15).
- DSSE and in-toto are a few types written in the `sluice` crate; verification belongs to the records seam's `remote refs` profile, which gets each signer's identity as settings the core does not read (core.1).
- Supersedes ADR 0004's trust by the cache's write credential.
