# plugin-setup Specification

## Purpose
Installing sluice and setting a repository up for it from Claude Code.

## Requirements

### Requirement: The binary is installed when missing, asking first

The setup skill SHALL first read its own plugin's version, the `version`
in `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json`, then check
`sluice --version`. Where the plugin's version is a release's, anything
but `0.0.0`, and sluice is missing or older than it, the skill SHALL
propose installing that release, `v<version>`, by a way fit for the
host, and SHALL run the install or upgrade only on the user's word.
Where the two are equal, it SHALL go on.

#### Scenario: No sluice on a Mac without nix

- **WHEN** a user asks to set up sluice on an Apple-silicon Mac where
  `sluice` and `nix` are not found, and the plugin's manifest gives
  `0.90.0`
- **THEN** the skill shows the install of
  `https://github.com/agentplot/sluice/releases/download/v0.90.0/sluice-aarch64-darwin.tar.gz`
  into `~/.local/bin`, and asks before running it

#### Scenario: An older sluice where nix is

- **WHEN** `sluice --version` prints `sluice 0.89.1` from a nix profile,
  and the plugin's manifest gives `0.90.0`
- **THEN** the skill shows removing the profile's sluice entry and
  `nix profile install github:agentplot/sluice/v0.90.0`, and asks before
  running them

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

### Requirement: The setup asks whether a branch lands through pull requests today

For every branch it gates whose policy names a remote on GitHub, the
setup skill SHALL ask, before writing `.config/sluice.toml`, whether the
team lands work on that branch through pull requests today (127). Where
the answer is yes, it SHALL write the branch's
`[branches.<pattern>.pull_requests]` table with the conditions the team
names, so a request waits for them from the first day (C.7). It SHALL NOT
write the table where the answer is no, nor ask for a branch whose policy
names no remote.

#### Scenario: A team that reviews every pull request into main

- **WHEN** a user sets up sluice for `main` on GitHub and says the team
  lands on `main` through pull requests with one approving review
- **THEN** the policy the skill proposes has `[branches.main]` with
  `remote = "origin"` and `[branches.main.pull_requests]` with
  `reviews = 1`

#### Scenario: A branch the team does not land by pull request

- **WHEN** the user says agents land on `bolt/*` straight from their
  worktrees
- **THEN** the proposed policy has no `[branches."bolt/*".pull_requests]`
  table

#### Scenario: A branch kept on this host only

- **WHEN** a gated branch's policy names no remote
- **THEN** the skill does not ask about pull requests for it

### Requirement: The setup offers what GitHub requires of the branch now

Where the team lands through pull requests, the setup skill SHALL read
the approving reviews and required status checks GitHub's rulesets and
branch protection name for the branch today, and offer them as the
branch's `reviews` and `checks`, leaving out sluice's own checks (113).

#### Scenario: A ruleset that requires a review and CI

- **WHEN** `main`'s ruleset on GitHub requires one approving review and
  the status check `ci/test`
- **THEN** the skill offers `reviews = 1` and `checks = ["ci/test"]`

#### Scenario: sluice's own check is required

- **WHEN** `main`'s ruleset requires the status check `sluice`
- **THEN** the skill leaves `sluice` out of `checks`

### Requirement: The setup names a requirement the policy cannot hold

The setup skill SHALL say plainly, before writing the policy, of any
requirement GitHub holds on the branch that the policy has no condition
for, such as a review from code owners, that sluice will not wait for
it.

#### Scenario: Code owners must review

- **WHEN** `main`'s ruleset requires a review from code owners
- **THEN** the skill says sluice's policy has no condition for a code
  owner's review and that sluice will not wait for one, before writing
  the policy

### Requirement: The setup moves the conditions out of GitHub, asking first

Once the policy holds a branch's conditions, the setup skill SHALL
propose removing the required reviews and status checks from GitHub's
settings for the branch and turning GitHub's merge queue off, and SHALL
change no GitHub setting without the user's word.

#### Scenario: The ruleset still requires a review

- **WHEN** the policy holds `reviews = 1` for `main` and `main`'s ruleset
  still requires one approving review
- **THEN** the skill proposes removing the requirement from the ruleset
  and asks before doing so

#### Scenario: GitHub's merge queue is on

- **WHEN** `main`'s ruleset turns on GitHub's merge queue
- **THEN** the skill says sluice is the queue and asks to turn GitHub's
  off before landing is handed to sluice

### Requirement: The setup says what stays the same and who lands

Where a branch keeps its pull requests, the setup skill SHALL tell the
user, before committing the policy, that requests are opened as always,
by a submit or by hand; that a draft waits until marked ready and a ready
one waits for the policy's reviews and checks; that sluice then tests it
with what is ahead and lands it with its own push; and that nobody merges
it on GitHub or by hand (127, S62).

#### Scenario: The team is told what changes

- **WHEN** the skill has written `[branches.main.pull_requests]`
- **THEN** before committing it tells the user that only who lands a
  request changes, from GitHub's merge button to sluice's push

### Requirement: The setup says requests opened by hand need sluice running

Where a branch keeps its pull requests, the setup skill SHALL say that
`sluice run` must be kept going for a request opened by hand to be
taken, since no submit waits on it.

#### Scenario: Keeping sluice running

- **WHEN** the skill has written `[branches.main.pull_requests]`
- **THEN** it tells the user to keep `sluice run` going, and that sluice
  starts no service itself

### Requirement: The setup offers no mode in which the host's button lands

The setup skill SHALL offer no policy, setting or GitHub configuration
in which GitHub's merge button, GitHub's merge queue or a merge by hand
lands on a branch sluice keeps while sluice only tests. Asked for one, it
SHALL say sluice has none, and why: what the button lands was never
tested with what landed just before it (127, I1).

#### Scenario: Asked to keep the merge button

- **WHEN** a user asks the skill to let sluice test pull requests while
  people keep merging them with GitHub's button
- **THEN** the skill says sluice has no such mode, gives the reason, and
  offers the pull request conditions instead

### Requirement: The read-back names a branch's pull request conditions

When a branch has a `pull_requests` table, the setup skill's read-back
SHALL name, in that branch's block, what a pull request into it waits
for, its draft state, the number of approving reviews and each other
system's check by the name GitHub shows, and that sluice lands it once
those hold and its combination passes.

#### Scenario: main with one review and a QA check

- **WHEN** the policy has `[branches.main.pull_requests]` with
  `reviews = 1` and `checks = ["qa/approved"]`
- **THEN** the read-back's block for `main` says a pull request into it
  waits until it is out of draft, has one approving review and the check
  `qa/approved` passes, and then sluice tests and lands it

### Requirement: The setup offers the layout beside other working copies and goes on

When the git directory sits inside one of several working copies of the
repository, the setup skill SHALL offer the same layout of the folder
that holds the git directory, with the costs the offer names for a
plain clone, and SHALL then go on with the repository as it stands,
waiting on no answer (126). It SHALL lay the folder out only on the
operator's word, by the same conversion.

#### Scenario: A clone with a worktree beside it

- **WHEN** a user asks to set sluice up in a clone whose `.git` is a
  directory and which has a worktree outside the clone's folder
- **THEN** the skill shows the layout the clone's folder would take and
  what the move costs, says it goes on as the repository stands and lays
  the folder out whenever the user asks, and goes on to survey the
  repository without waiting for an answer

#### Scenario: Asked from the worktree beside the clone

- **WHEN** a user asks to set sluice up in a worktree whose repository's
  git directory is `.git` inside another folder, a clone's
- **THEN** the offer names that clone's folder, the processes holding
  it and the history kept under its path, and the skill goes on in the
  worktree it was asked in

#### Scenario: The move would be refused now

- **WHEN** the clone's folder holds a worktree of its own inside it,
  outside its git directory
- **THEN** the offer says the move would be refused now and why, and
  the skill goes on as the repository stands

#### Scenario: The operator says to lay it out during the setup

- **WHEN** the offer has been made beside other working copies and the
  operator then says to lay the folder out
- **THEN** the skill runs the conversion as it does for a plain clone,
  refusing while anything else holds the folder, and goes on with the
  setup where it was

#### Scenario: No choice is recorded

- **WHEN** the setup made the offer beside other working copies and the
  operator never answered it
- **THEN** the skill writes `.config/sluice.toml` as for any repository,
  and the message of the commit adding it says nothing of the layout

### Requirement: Only a session's working copies count toward the repository's

In telling a repository's only working copy from one of several, the
setup skill SHALL count only the working copies a session could work
from: sluice's own checkouts under the git directory, and a worktree
whose folder is gone, SHALL NOT count.

#### Scenario: Only sluice's own checkouts beside the clone

- **WHEN** a user asks to set sluice up in a clone whose `git worktree
  list` shows the clone and checkouts under `.git/sluice/` and nothing
  else
- **THEN** the skill treats the clone as the repository's only working
  copy: it makes the offer and waits for the operator's choice before
  writing anything

#### Scenario: A worktree whose folder was deleted

- **WHEN** the clone's only other worktree shows as `prunable` in `git
  worktree list`, its folder gone
- **THEN** the skill treats the clone as the repository's only working
  copy and waits for the operator's choice

### Requirement: A sluice newer than its plugin takes the plugin's newer release

Where the plugin's version is a release's and sluice is newer, the setup
skill SHALL propose taking the plugin's newer release with
`/plugin marketplace update sluice` and `/plugin update sluice@sluice`,
and SHALL install no sluice.

#### Scenario: A newer sluice

- **WHEN** `sluice --version` prints `sluice 0.91.0`, and the plugin's
  manifest gives `0.90.0`
- **THEN** the skill proposes `/plugin marketplace update sluice` and
  `/plugin update sluice@sluice`, and installs no sluice

### Requirement: A plugin that is no release takes the newest release

Where the plugin's version is `0.0.0`, the plugin is no release. The
setup skill SHALL say so and, where sluice is missing or older than
0.12, SHALL propose the newest release, and SHALL run the install or
upgrade only on the user's word.

#### Scenario: A plugin that is no release

- **WHEN** the plugin's manifest gives `0.0.0` and `sluice` is not found
  on an Apple-silicon Mac without nix
- **THEN** the skill says the plugin is no release and shows the install
  of
  `https://github.com/agentplot/sluice/releases/latest/download/sluice-aarch64-darwin.tar.gz`,
  and asks before running it

### Requirement: The setup installs sluice from a release alone

The setup skill SHALL name no install source but a release. A sluice
from the user's own flake or Home Manager SHALL be left for the user to
update, at the release's tag.

#### Scenario: A sluice from the user's flake

- **WHEN** `sluice --version` prints `sluice 0.89.1` from a path under
  `/nix/store` or `/etc/profiles`, and the plugin's manifest gives
  `0.90.0`
- **THEN** the skill tells the user to set their input to
  `github:agentplot/sluice/v0.90.0` and rebuild, and edits no file of
  theirs

### Requirement: The setup adds the herdr plugin's lines at the person's asking

Asked by the person to set up sluice's herdr plugin, the setup skill
SHALL add the plugin's lines, its two keys, `$sluice`, `$sluice_q`, the
segment entry and toast delivery, to the file `herdr --help` names on
its `Config:` line. The asking SHALL be the person's word to edit the
file: the skill SHALL ask nothing more before writing it, and SHALL then
say what it added (principle 10; core.57; record 0113).

#### Scenario: A file with none of sluice's lines

- **WHEN** the person asks their agent to set up sluice's herdr plugin,
  the plugin is installed, and herdr's file holds `[keys]`, `[theme]`
  and `[ui]` tables and no line naming sluice
- **THEN** the agent adds the two keys, the `$sluice` row, `$sluice_q`,
  the segment entry and `[ui.toast] delivery = "herdr"` to that file
  without asking again, `herdr config check` passes, and the agent says
  what it added and to which file

#### Scenario: No file yet

- **WHEN** the file `herdr --help` names does not exist
- **THEN** the agent writes it with the plugin's lines alone, as the
  setup pane shows them, and herdr keeps its own defaults for
  everything else

### Requirement: The herdr plugin is installed only on the person's word

The setup skill SHALL write nothing to herdr's file where herdr is
missing, older than 0.9.3, or has no plugin `agentplot.sluice`
installed. It SHALL offer the plugin's install and run it only on the
person's word.

#### Scenario: The plugin is not installed

- **WHEN** `herdr plugin list --plugin agentplot.sluice --json` lists
  no plugin
- **THEN** the skill offers `herdr plugin install
  agentplot/sluice/plugins/herdr`, runs it only on the person's word,
  and writes nothing to herdr's file until the plugin is installed

### Requirement: The herdr lines are the plugin's setup's

The setup skill SHALL take the lines from the installed plugin's setup:
`bin/setup.sh` under the root `herdr plugin list --plugin
agentplot.sluice --json` names, run with herdr's state directory for the
plugin, `${XDG_STATE_HOME:-$HOME/.local/state}/herdr/plugins/agentplot.sluice`,
with no herdr socket and no terminal. The skill SHALL keep no copy of the
lines.

#### Scenario: The segment's entry

- **WHEN** herdr keeps the plugin's state at
  `$HOME/.local/state/herdr/plugins/agentplot.sluice`
- **THEN** the entry the agent adds runs `sh
  "$HOME/.local/state/herdr/plugins/agentplot.sluice/segment/bin/segment.sh"`
  every 5 seconds with a limit of 30, as the setup pane prints it

#### Scenario: The plugin has not run on the host

- **WHEN** no copy of the segment is under the plugin's state directory
- **THEN** the agent adds the entry setup prints and says the tab bar
  shows the queue once a herdr server has started the plugin

### Requirement: The herdr plugin's actions take free keys

The setup skill SHALL bind each of the plugin's two actions to the key
the setup pane shows for it where that key is free, and otherwise to
another free key, saying which, why, and what holds the setup pane's
key. A key the file already binds to one of the plugin's actions SHALL
stay as it is, and no other key SHALL be added for that action.

#### Scenario: The jump's key is another plugin's

- **WHEN** the file binds `prefix+shift+m` to another plugin's action,
  and binds `prefix+m` to nothing
- **THEN** the agent binds `agentplot.sluice.status` to `prefix+m` and
  `agentplot.sluice.jump` to a key neither the file nor herdr binds, and
  says it left `prefix+shift+m` to the action that holds it

#### Scenario: The person's own key for the queue

- **WHEN** the file already binds `prefix+u` to `agentplot.sluice.status`
- **THEN** the agent leaves that entry as it is and adds no other key
  for the status popup

### Requirement: A key is free where neither the file nor herdr binds it

The setup skill SHALL read as bound every key of the file's
`[[keys.command]]` entries and its `[keys]` table, and every key
`herdr --default-config` gives an action the file does not rebind. It
SHALL read a key as free only where none of these binds it, whatever
`herdr config check` says.

#### Scenario: A key herdr binds by default

- **WHEN** the setup pane's key for an action is one that
  `herdr --default-config` binds, and the file does not rebind it
- **THEN** the agent binds the action to another free key, although
  `herdr config check` would pass with both

### Requirement: The person's herdr file is edited in place

The setup skill SHALL edit herdr's file in place, leaving every line,
comment and order of the person's as it was. It SHALL add each key
inside the table the file already holds for it, SHALL never open a
table the file holds a second time, SHALL add the segment's entry in
the form the file already writes `ui.tab_bar_right` in, and SHALL add
nothing the file already holds.

#### Scenario: The tab bar written as tables

- **WHEN** the file holds a `[ui]` table and gives its tab bar entries
  as `[[ui.tab_bar_right]]` tables
- **THEN** the agent adds the segment as one more `[[ui.tab_bar_right]]`
  table, adds no `tab_bar_right` key to `[ui]`, and opens no second
  `[ui]`

#### Scenario: Everything already there

- **WHEN** the file already binds both actions, holds `$sluice`,
  `$sluice_q` and an entry running the plugin's segment as the setup
  pane gives it, and delivers toasts
- **THEN** the agent writes nothing, runs no reload, and says the
  plugin is set up and on which keys

### Requirement: sluice's rows go beside the person's own

Where herdr's file holds agent rows of the person's own, the setup
skill SHALL append only the `$sluice` row to them, in `rows` and in each
list of `rows_by_agent`. Where it holds workspace rows of their own, the
skill SHALL append only `$sluice_q`, at the end of their first row.
Where it holds neither, the skill SHALL add the setup pane's rows.

#### Scenario: Agent rows of the person's own

- **WHEN** the file's `[ui.sidebar.agents] rows` hold herdr's rows and
  a row of another plugin's token
- **THEN** the agent appends the `$sluice` row with its rules after
  them, and the other plugin's row stays where it was

### Requirement: Toasts are turned on, and the person's delivery kept

The setup skill SHALL set `[ui.toast] delivery = "herdr"` where herdr's
file leaves delivery unset or `"off"`, and SHALL keep `"terminal"` or
`"system"`.

#### Scenario: Toasts already delivered by the system

- **WHEN** the file holds `[ui.toast] delivery = "system"`
- **THEN** the agent leaves it as it is

### Requirement: A line of sluice's the herdr file holds changes only on the person's word

Where a line of sluice's that herdr's file holds differs from the setup
pane's, the setup skill SHALL say how and why the pane gives what it
does, and SHALL change it only on the person's word.

#### Scenario: An older segment entry

- **WHEN** the file's entry runs the plugin's segment with
  `timeout_seconds = 2`
- **THEN** the agent says the setup pane gives 30 and why, and changes
  it only on the person's word

### Requirement: A herdr file a tool writes is shown its lines, never written

Where herdr's file or its folder is a link or read-only, or the person
says a tool generates the file, the setup skill SHALL say so, naming the
link's target or the read-only path. It SHALL write nothing, neither the
file, the link's target nor a file in the link's place, and SHALL show
the lines merged, for the person to add where the file is generated.

#### Scenario: A link into the Nix store

- **WHEN** herdr's `config.toml` is a link into `/nix/store`
- **THEN** the skill says the file is managed and names the link's
  target, writes nothing, and shows the lines merged, with the keys it
  would bind

### Requirement: herdr's check gates the write and herdr is reloaded

Before editing, the setup skill SHALL copy herdr's file to a path
outside herdr's folder. After editing, it SHALL run `herdr config check`.
Where the check fails, it SHALL put the file back from the copy, show the
check's words, and reload nothing. Where the check passes, it SHALL run
`herdr server reload-config`. Where no herdr server answers, it SHALL say
the next server to start reads the file. It SHALL say that every other
running server reads the file at its own reload.

#### Scenario: The check fails

- **WHEN** `herdr config check` exits non-zero after the edit
- **THEN** the file is as it was before the edit, the agent shows the
  check's words, and it runs no reload

#### Scenario: The check passes

- **WHEN** `herdr config check` prints `config: ok`
- **THEN** the agent runs `herdr server reload-config` and says what it
  added, the keys it bound and why, and where its copy of the file as it
  was is
