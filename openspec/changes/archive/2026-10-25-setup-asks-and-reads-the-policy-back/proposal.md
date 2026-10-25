# Proposal

## Why

A newcomer set sluice up from its setup skill and got a policy whose
`dev/*` branch named no check, so every submission to it would have
landed untested, and nothing said so. The policy had `[tests]` filled
in, and neither the newcomer nor the person helping them could say on
the call whether that top-level table runs on every branch (it does
not: it says how checks run, and each branch's `required` says which).
The setup skill asks which branches to gate only in one sentence, never
reads back what it wrote, and never says that per-branch checks are
moon tasks. (Signal `2026-10-23-madswan-operator-bea2f287/02`, from the
onboarding calls of 2026-10-19 and 2026-10-20.)

## What Changes

- The setup skill asks, before it writes anything, which branches agents
  will land on and what must pass on each, and says plainly what the
  answer means for the runner: one command runs in full on every gated
  branch; different checks on different branches are moon tasks.
- After writing or changing `.config/sluice.toml`, and whenever the user
  asks what their policy does, the skill reads the file back in plain
  words: each branch pattern, what branches it matches, and the checks
  that must pass before work lands there.
- The read-back says when the checks need moon: that each host running
  them needs moon 2.6 or later, whether this host and repository have
  it, and that a repository without a moon workspace can't run them.
- The read-back warns plainly, before the policy is committed, of every
  branch on which no test will run: no `required` under the moon runner,
  or a required tag no task carries. The skill asks the user to name a
  check or to confirm that branch is meant to land untested.
- The configuration reference says the same in its `[tests]` and
  `required` sections: nothing in `[tests]` picks what runs, and a moon
  branch with no `required` lands every submission untested.
- The gate checks that no example policy the plugin ships leaves a moon
  branch with no check, so the references never teach the trap.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plugin-setup`: adds the interview before writing, the plain-words
  read-back, the moon notice and the no-check warning; the example
  policies' requirement also holds every moon branch to a check.

## Out of scope

- sluice itself warning of a branch with no check, in `sluice submit`,
  `sluice status` or a command that prints a policy. A gate with no
  checks is a gate sluice must honour as written (27), so a warning from
  sluice is a design question for the statement first. It waits on the
  design agent's ruling; no later change is planned for it yet.
- Refusing `required` under the command runner. The read-back names it;
  making the policy parser refuse it is a change to sluice's policy
  reading, not to setup.
- The README's first-time setup and the bare-repository advice from the
  same call, which belong to the newcomer's page and the worktrees skill.

## Impact

- `plugins/sluice/skills/setting-up-sluice/SKILL.md`: steps 2 and 4,
  and the skill's description.
- `plugins/sluice/skills/setting-up-sluice/references/read-back.md`: new,
  the read-back's rules and wording.
- `crates/sluice-core/src/policy.rs`: the example-policies test.
- `site/src/content/docs/reference/configuration.md`: `[tests]` and
  `required`.
- No change to sluice's behaviour, its policy format or its records.

## Touches

```
plugins/sluice/skills/setting-up-sluice/SKILL.md
plugins/sluice/skills/setting-up-sluice/references/read-back.md
crates/sluice-core/src/policy.rs
site/src/content/docs/reference/configuration.md
```
