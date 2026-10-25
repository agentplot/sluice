# plugin-setup Specification

## Purpose
Installing sluice and setting a repository up for it from Claude Code.

## Requirements

### Requirement: The binary is installed when missing, asking first

The setup skill SHALL check `sluice --version` first and, when sluice is
missing or older than 0.12, SHALL propose an install or upgrade fit for the
host and run it only on the user's word. A sluice from the user's own flake
or Home Manager SHALL be left for the user to update.

#### Scenario: No sluice on a Mac without nix

- **WHEN** a user asks to set up sluice on an Apple-silicon Mac where
  `sluice` and `nix` are not found
- **THEN** the skill shows the `sluice-aarch64-darwin.tar.gz` release
  install into `~/.local/bin` and asks before running it

### Requirement: The policy matches the repository's tools

The setup skill SHALL survey the repository and propose a
`.config/sluice.toml` whose runner, toolchain, executor, checkout, driver and
remote match what the repository uses, from the references it ships.

#### Scenario: A moon repository on GitHub with worktrunk

- **WHEN** the repository has `.moon/workspace.yml`, `devenv.nix`,
  `.config/wt.toml` and an `origin` on github.com
- **THEN** the proposed policy uses `runner = "moon"`, the devenv
  toolchain and `remote = "origin"`, and the skill tells agents to use
  `wt submit`

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

### Requirement: The setup offers the layout before going on in a repository's only working copy

When asked to set sluice up in a repository whose git directory sits
inside its one and only working copy, the setup skill SHALL, before it
surveys the repository or writes anything, offer to lay the repository
out with its git directory bare in the folder and the working copy
moved to `main/` beside one folder per branch, and SHALL go on only
once the operator has chosen (126, S61).

#### Scenario: A plain clone with no worktrees

- **WHEN** a user asks to set sluice up in a clone whose `.git` is a
  directory and whose `git worktree list` shows that one working copy
- **THEN** the skill shows the layout the folder would take, with its
  costs, and asks whether to lay it out or keep the plain clone, before
  it writes `.config/sluice.toml` or any other file

#### Scenario: Already in the layout

- **WHEN** the user asks for setup in `main/` of a repository whose git
  directory is bare and beside its worktrees
- **THEN** the skill makes no offer and goes on to survey the
  repository

### Requirement: The offer names what the move costs

The offer SHALL name each process that holds the folder, by its working
directory or an open file in it, and say it must be stopped and started
again from `main/`, and SHALL name none of the setup's own session or
what that session started; SHALL name the Claude Code history kept under the
folder's path and the paths it will be carried to; and SHALL say what
stays under the old path: prompt recall, and the folder's trust and
permission answers, which Claude asks again once for `main/`.

#### Scenario: A dev server and an editor in the folder

- **WHEN** a dev server and an editor each have the folder open, and
  Claude Code holds sessions under the folder's path
- **THEN** the offer lists the dev server and the editor by process and
  command, and names the history to be carried and where it goes

#### Scenario: The setup's own session has helpers in the folder

- **WHEN** the setup's session has started a tool server and a watcher
  whose working directory is the folder, and nothing else holds it
- **THEN** the offer names no process to stop

### Requirement: The setup lays the repository out only once nothing else holds the folder

On the operator's word to lay the repository out, the setup skill SHALL
run the plugin's conversion, naming its own session, and the conversion
refuses while any process outside that session, its ancestors and its
descendants holds the folder (126). On a refusal it SHALL name what
holds the folder and wait for the operator to let it go, moving nothing
until then.

#### Scenario: The editor still has the folder open

- **WHEN** the operator says to lay the repository out while an editor
  still has a file in the folder open
- **THEN** nothing is moved, the skill names the editor's process, and
  it lays the repository out only after the operator says the editor
  has let the folder go and the conversion no longer refuses

#### Scenario: Only the session and what it started

- **WHEN** the operator says to lay the repository out, and the only
  processes in the folder are the setup's session, the shell its command
  runs in and the helpers the session started
- **THEN** the conversion lays the repository out

### Requirement: The setup carries the history across and goes on in main/

The layout SHALL carry the folder's other Claude Code history across,
leaving the setup's own session for the operator to move into `main/`
with Claude Code's `/cd`. The setup skill SHALL then go on with the
survey and the policy in `main/`, never in the folder that now holds
the git directory.

#### Scenario: The setup goes on after the move

- **WHEN** the layout has succeeded and carried the folder's ended
  sessions, and the operator has typed `/cd <folder>/main`
- **THEN** the skill reports what was carried and what was left, and
  writes `.config/sluice.toml` in `main/`

### Requirement: The plain clone is kept only on the operator's word, and the setup's record says so

When the operator chooses to keep the plain clone, the setup skill SHALL
go on there, and the commit that adds `.config/sluice.toml` SHALL say in
its message that the operator chose to keep the plain clone rather than
the layout. Without that choice the setup SHALL NOT write the policy in
a repository's only working copy.

#### Scenario: The operator keeps the plain clone

- **WHEN** the operator answers the offer by keeping the plain clone
- **THEN** the skill goes on in the clone, and the message of the commit
  adding `.config/sluice.toml` says the operator chose to keep the plain
  clone

#### Scenario: No answer yet

- **WHEN** the operator has not answered the offer
- **THEN** the skill writes no `.config/sluice.toml` and commits nothing
