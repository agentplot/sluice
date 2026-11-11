## ADDED Requirements

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
