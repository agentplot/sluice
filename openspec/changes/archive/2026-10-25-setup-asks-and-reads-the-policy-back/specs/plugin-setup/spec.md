# Spec Delta

## ADDED Requirements

### Requirement: The setup asks which branches agents land on and what each checks

Before writing `.config/sluice.toml`, the setup skill SHALL ask which
branches agents will submit to and, for each, what must pass before work
lands there, offering the moon tags the repository's tasks carry when it
uses moon. It SHALL say that one command runs in full on every gated
branch and that different checks per branch need moon.

#### Scenario: A moon repository with two tiers

- **WHEN** a user sets up sluice in a repository with `.moon/workspace.yml`
  whose tasks carry the tags `check-fast` and `e2e`
- **THEN** the skill asks which branches agents land on and offers
  `#check-fast` and `#e2e` for each before writing the policy

#### Scenario: Different checks per branch without moon

- **WHEN** a user of a repository with no moon workspace asks for a quick
  suite on `dev/*` and the full one on `main`
- **THEN** the skill says one command runs in full on every gated branch,
  and that checks differing per branch are moon tasks, before writing

### Requirement: The setup reads the policy back in plain words

After writing or changing `.config/sluice.toml`, and whenever the user
asks what their policy does, the setup skill SHALL read the file back in
plain words: each branch pattern in the order the file lists it, which
branches it matches, and the checks that must pass before work lands
there. It SHALL say that `[tests]` picks how checks run and never which.

#### Scenario: Two patterns under moon

- **WHEN** the skill has written a policy with `runner = "moon"`,
  `[branches."dev/*"] required = ["#check-fast"]` and `[branches.main]
  required = ["#check-fast", "#e2e"]`
- **THEN** its read-back names `dev/*` as every branch whose name starts
  `dev/` gated by the tasks tagged `check-fast`, and `main` gated by the
  tasks tagged `check-fast` and `e2e`

#### Scenario: The command runner

- **WHEN** the policy has `runner = "command"`, `command = "make test"`
  and the branches `dev/*` and `main`
- **THEN** the read-back says `make test` runs in full on every
  submission to both

### Requirement: The read-back says when the checks need moon

When a branch's checks are moon tasks, the read-back SHALL say that every
host running them needs moon 2.6 or later, whether this host has moon
and the repository has a moon workspace, and SHALL say plainly that the
checks cannot run when the repository has no moon workspace.

#### Scenario: moon is not installed

- **WHEN** the policy has `runner = "moon"` and neither the host's
  `PATH`, the policy's `toolchain` shell nor a `.prototools` gives moon
- **THEN** the read-back says the checks are moon tasks and moon is not
  installed on this host

#### Scenario: Required checks under the command runner

- **WHEN** the policy has `runner = "command"` and a branch with
  `required = ["#test"]`
- **THEN** the read-back says `#test` is a moon target that the command
  runner never reads, and that only the command runs

### Requirement: The read-back warns when a branch runs no check

The read-back SHALL warn, before the policy is committed, of every
branch on which no test will run: under the moon runner, one with no
`required` or an empty one, or one whose every required tag no task
carries. The warning SHALL say every submission there lands untested,
and the skill SHALL ask the user to name a check or confirm it.

#### Scenario: A branch with no required

- **WHEN** the policy has `runner = "moon"` and `[branches."dev/*"]` with
  `parallel = 2` and no `required`
- **THEN** the read-back warns that no tests will run on `dev/*` and
  every submission to it lands untested, and asks before committing

#### Scenario: A tag no task carries

- **WHEN** a branch requires `#suite` and `moon query tasks` lists no
  task tagged `suite`
- **THEN** the read-back warns that `#suite` matches no task, so nothing
  runs for it on that branch

## MODIFIED Requirements

### Requirement: The example policies read

Every example policy the plugin ships SHALL parse as a policy sluice
accepts, and every branch an example with `runner = "moon"` lists SHALL
require at least one check.

#### Scenario: The examples parse

- **WHEN** `cargo test -p sluice-core` runs
- **THEN** `the_plugins_example_policies_read` passes

#### Scenario: An example teaches a branch with no check

- **WHEN** an example policy with `runner = "moon"` lists a branch with
  no `required`
- **THEN** `the_plugins_example_policies_read` fails naming the example
  and the branch
