## MODIFIED Requirements

### Requirement: Setup shows, never edits

The `setup` action SHALL show the snippets to add to herdr's config for the
key, the sidebar rows and the tab-bar segment, and SHALL change no file
outside the plugin's own directories. Its first lines SHALL name both
ways to add them: an agent with sluice's Claude Code plugin adds them to
herdr's file at the person's asking, or the person adds them, or adds
them to whatever generates the file. They SHALL name `herdr config
check` and `herdr server reload-config` to run after either. The
segment's command SHALL name
the copy in the plugin's state directory with `$HOME` in place of the
home directory, so the same line serves every host whose herdr keeps its
state at the same place below the home directory, and SHALL name the
absolute path only when the state directory is not under `$HOME`.

#### Scenario: Setup

- **WHEN** the user runs `setup`
- **THEN** a popup shows a `[[keys.command]]` binding `prefix+m` to
  `agentplot.sluice.status`, the sidebar rows, and the segment, and herdr's
  config is unchanged

#### Scenario: The two ways to add the lines

- **WHEN** the user runs `setup`
- **THEN** its first lines say to ask an agent to set up sluice's herdr
  plugin or to add the lines yourself, and name `herdr config check`
  and `herdr server reload-config`

#### Scenario: One line for every host

- **WHEN** setup runs with the state directory at
  `$HOME/.local/state/herdr/plugins/agentplot.sluice`
- **THEN** the segment's command is `sh
  "$HOME/.local/state/herdr/plugins/agentplot.sluice/segment/bin/segment.sh"`,
  and run through `/bin/sh -lc` under two different home directories,
  each holding the copy, it prints that home's queue
