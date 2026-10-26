---
number: 59
title: The protection reads only what guarding needs
status: accepted
date: 2026-10-25
decision-makers:
- Chuck Swanberg
---

# The protection reads only what guarding needs

Deciders: the user, in clause 124 of the statement (agentplot/blueprints 1340786), after a host's settings gained `[runs] cores` a minute before its hooks were moved to the newer sluice, and every git ref update in every guarded repository on that host failed for that minute, 2026-10-25.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 1, 124, C.4, C.6; ADRs 0003, 0053; rules core.2, core.4.

## Context and problem statement

The protection hook, `sluice hook reference-transaction prepared`, was bound by the same `Bound::open` every command uses. That read the host's settings in full, every form refusing a field it did not know, opened the forge's app and the `[signing]` key, and built every seam. The judging then read the policy three ways, the core's gate, the branch seam's routing and the records seam's routing, each in full and as strict. Since 0.41.0 a failure anywhere there passes the transaction with one warning (ADR 0053), which is right for a file that cannot be read and wrong for a field the hook does not know: the hook then guards nothing until it is re-installed, and only `submit`, `wait`, `step`, `run` and `protect` installed it.

Clause 124 says the protection reads of the host's settings and the policy only what guarding needs, the branches guarded and where their records are, and passes over every other field, known to it or not; settings or a policy it cannot read at all pass the transaction, said once; and the hook runs the sluice that installed it, which re-installs the hook at its next run in the repository. What does guarding need, how is the hook bound to read only that, and which commands re-install it?

## Considered options

1. A guarding reading of the host's settings and one of the policy, used by the hook alone through a binding for guarding; every other command reads both in full as before; every command run in a repository re-installs a hook sluice installed for another sluice, and installs none where there is none.
2. Drop the refusal of unknown fields from the settings' and the policy's forms everywhere.
3. A hook of its own that reads the refs and the policy by hand, outside `run` and the seams.
4. As option 1, with every command installing the hook where there is none.

## Decision outcome

Option 1.

**What guarding needs of the host's settings is `[forge]`.** It is how sluice reaches a branch's remote: the identity the branch seam's pushed refs reach it as, where a remote branch's records are and where a refusal's `protected` line goes. `Instance::guarding` reads `app_id` and `key` from `[forge]` through a form that passes over any other field of it, and passes over every other table. Everything else takes its defaults. `[signing]` is not read: the hook signs no run. `[runs]`, `[peers]` and `[[caches]]` bound and share the runs, and the hook starts none.

**What guarding needs of the policy is the branches and where their records are.** `Policy::guarding` reads each `[branches]` pattern, in the order written, with the default gate and its `remote`, and the names `[signers]` gives. The patterns say which branches are guarded, by the same `rules_for` the full reading's gate is matched by, ties included; each `remote` says whether the host's ref is the branch or sluice's copy of it (C.4; ADR 0053); and whether `[signers]` names anyone says whether a remote branch's records are on its remote (C.6). It builds a `Policy` with the rest its defaults, so every seam's routing and the core's matcher take it as they take the full reading. Every other table and field, known or not, is passed over.

**What cannot be read at all passes, said once.** Settings or a policy that are not TOML, a `[forge]` without `app_id` or `key` or with either of the wrong kind, a `[branches]` entry that is not a table, a `remote` that is not a string, a `[signers]` that is not a table, or a branch two patterns match as closely: each is an error, and the error reaches the warning the hook already gives, `warning: sluice's branch protection let this through unread: <why>`, once, and the transaction passes (124). Nothing is refused or recorded for it.

**The hook is bound for guarding; the core gains a reading, not a profile.** `Bound::guarding` binds the repository with `Instance::guarding` and with the branch and records profiles told to read each policy the guarding way, the records profile giving a remote's profile no signer's key and no signing, since the hook verifies and signs no run. `run` sets `Sluice::guarding` for the hook, and the core's `policy_at` then reads the guarding way. Which reading a binding uses is the binding's choice, made in the profiles and in the command's run; the core names no profile and has no branch for one (rule core.2). Every other command keeps `Bound::open` and the full readings, refusing a field it does not know: a command that runs checks or lands needs every field it is given to mean what it says.

**The hook runs the sluice that installed it.** Its shim names the installing executable by its absolute path, never a `sluice` found on the caller's `PATH`, since what the hook refuses may not turn on who ran git (1, 124).

**Every command re-installs a hook that stands for another sluice, and installs none.** Every command sluice runs in a repository, but the hook itself, `supervise`, `credential` and `protect`, puts sluice's hook for the running sluice where the hook in place is sluice's, by its marker line, and names another program, keeping the hook it chains. It writes nothing where there is no hook, where the hook is someone else's, or where the hooks directory is a tracked one, and a failure is said once, `sluice: warning: branch protection not re-installed: …`, changing nothing else the command does. `submit`, `wait`, `step` and `run` keep installing it where there is none, and `protect` installs or removes its own. So a host whose sluice was upgraded is guarded by the newer one from its next command in each repository, and a repository whose hook `sluice protect --remove` took out gets none back from a command that only reads. The hook is a file in the repository's hooks directory; neither the installing nor the re-installing writes a branch's ref (rule core.4).

Option 2 would pass over a mistyped field in a gate, a branch's or the host's, which every command that runs checks or lands must refuse. Option 3 would write the judging, the refusal's record and the branch's lock twice, once for the hook and once for the core, and they would drift. Option 4 would put back a hook the operator took out, at the next `sluice status`.

### Consequences

- A host whose settings or policy a newer sluice wrote is guarded by an older hook as before, as long as `[forge]`, the branches' patterns, their `remote`s and `[signers]` keep their kinds.
- A hook installed before this change still reads in full, and passes such a transaction with its warning, until the next command of this sluice or later in its repository re-installs it.
- A command that only reads, `sluice status` among them, may now write the repository's hook file: only where sluice's own hook already stands, only to name the sluice running, and in one rename, as installing writes it.
- The two readings of the policy must agree on the patterns, their order and each `remote`; a test reads every example policy both ways.
- How a host's configuration manager orders its writes of the settings and its upgrade of sluice is that manager's; either order now leaves every guarded branch guarded.
