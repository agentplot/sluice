# Setting up sluice's herdr plugin

sluice's herdr plugin shows nothing of the queue until herdr's own
`config.toml` holds its lines: a key for the status popup and one for the
jump, the `$sluice` row under each agent, `$sluice_q` on each workspace's
row, the tab bar's segment, and toasts delivered. herdr gives a plugin no
way to declare them, so they go in the person's own file.

The person asking you to set up sluice's herdr plugin is their word to
edit that file: ask nothing more before writing it, and say what you
added afterwards. Nothing else of sluice's writes the file, no script,
hook or action of the plugin's. Still ask before installing the plugin,
and before changing a line of sluice's the file already holds.

This needs no repository. Work through these in order, showing each
command as you run it.

## 1. herdr and the plugin

```sh
herdr --version
root=$(herdr plugin list --plugin agentplot.sluice --json | jq -r '.result.plugins[0].plugin_root // empty')
echo "${root:-not installed}"
```

- herdr missing, or older than 0.9.3: say so, and write nothing.
- `root` empty: the plugin is not installed. Offer to install it, run
  the install only on the person's word, and write nothing to herdr's
  file until it is installed:

  ```sh
  herdr plugin install agentplot/sluice/plugins/herdr --ref v<version>
  ```

  `<version>` is the version `sluice --version` prints, so the plugin
  and the sluice it calls are one release.

  Then read `root` again.

## 2. Read the lines from the plugin's setup

The lines are the plugin's setup's, never this page's: run the installed
plugin's `bin/setup.sh` as herdr runs its setup pane, with herdr's state
directory for the plugin, no herdr socket and no terminal:

```sh
st=${XDG_STATE_HOME:-$HOME/.local/state}/herdr/plugins/agentplot.sluice
env -u HERDR_SOCKET_PATH HERDR_PLUGIN_STATE_DIR="$st" sh "$root/bin/setup.sh" </dev/null
[ -f "$st/segment/bin/segment.sh" ] && echo "segment: $st" || echo "segment: no copy yet"
```

- With no socket, the setup serves no server and starts nothing; it only
  prints.
- The state directory is given so the segment's entry names this host's
  copy of the segment.
- `segment: no copy yet`: the plugin has not run on this host. Add the
  entry setup prints all the same, and say the tab bar shows the queue
  once a herdr server has started the plugin.

From what it prints, take:

- the two `[[keys.command]]` entries, for `agentplot.sluice.status` and
  `agentplot.sluice.jump`, with their keys, `type`, `command` and
  `description`;
- under `[ui.sidebar.agents]`, its `rows`, herdr's own rows and, last, the
  row holding the `$sluice` token with its `rules`;
- under `[ui.sidebar.spaces]`, its `rows`, whose first row ends with
  `"$sluice_q"`;
- the one `tab_bar_right` entry, running the segment, with its
  `interval_seconds` and `timeout_seconds`;
- `[ui.toast] delivery = "herdr"`.

Each comment above a line in setup's output says why it is as it is.

## 3. The file, and who writes it

The file is the one herdr names on the `Config:` line of `herdr --help`.
herdr resolves `HERDR_CONFIG_PATH` itself, so read its answer rather than
a rule of your own. Then find out who writes it, as the skill's "The
host's file" does for sluice's own:

```sh
f=$(herdr --help | sed -n 's/^Config: *//p')
echo "${f:-herdr names no file}"
for p in "$f" "${f%/*}"; do [ -L "$p" ] && echo "managed: $p links to $(readlink "$p")"; done
[ ! -e "$f" ] || [ -w "$f" ] || echo "managed: $f is read-only"
[ ! -d "${f%/*}" ] || [ -w "${f%/*}" ] || echo "managed: ${f%/*} is read-only"
```

- Nothing printed `managed`, and the person says nothing generates the
  file: go on.
- `managed`, or the person says a tool generates the file: say the file is
  managed, naming the link's target or the read-only path. Write nothing:
  not the file, not the link's target, not a file put in the link's
  place. Work out steps 4 and 5 all the same, and show the lines as they
  would merge, with the keys you would bind and why, for the person to
  add where the file is generated. Then stop.
- No file at `$f`: write one holding the plugin's lines alone, as step 2
  printed them, on the keys step 4 gives (make its folder with
  `mkdir -p "${f%/*}"` where it is missing). herdr keeps its own defaults
  for everything else. Then go to step 6, with no copy to make.

## 4. A free key for each action

A key is bound where any of these binds it:

- **The file**: the `key` of each of its `[[keys.command]]` entries, and
  each value of its `[keys]` table. A modifier in `[keys.indexed]` binds
  that modifier with `1` to `9`.
- **herdr's defaults**: each key `herdr --default-config` gives an action
  in its `[keys]` part, for every action the file's `[keys]` does not
  rebind:

  ```sh
  herdr --default-config \
    | awk '/^\[keys\]/ { on = 1; next } on && /^\[/ { exit } on' \
    | sed -n 's/^# *\([a-z_]*\) *= *"\([^"][^"]*\)".*/\1 \2/p' \
    | grep -Ev '^(key|type|command|width|height) '
  ```

  Each line is an action and its key. The `key`, `type`, `command`,
  `width` and `height` the filter drops belong to the example
  `[[keys.command]]`, which binds nothing. A range binds each key in it:
  `prefix+1..9` binds `prefix+1` to `prefix+9`.

A key is free only where none of these binds it. Never read a key as free
because `herdr config check` passes: it reports two commands on one key,
but not a command on a key herdr binds by default.

For `agentplot.sluice.status`, then `agentplot.sluice.jump`:

1. The file already has a `[[keys.command]]` whose `command` is the
   action: that entry stays as it is, on whatever key it names, and no
   other key is added for the action.
2. Setup's key is free: the action takes it.
3. Otherwise pick a free key yourself, near setup's in form, and say
   which, why, and what holds setup's key. Where the file binds
   `prefix+shift+m` to `<plugin>.<action>`, for instance, the jump takes a
   free key and `prefix+shift+m` stays `<plugin>.<action>`'s.

Take the two in turn, so the second never takes the key the first just
took. The person can move either key in their file at any time.

## 5. Merge the lines into the file

### What the file already holds

Read the file first. A line of sluice's is already there where:

- a `[[keys.command]]` has the action's id as its `command`;
- a rows list, `[ui.sidebar.agents] rows`, a list of
  `[ui.sidebar.agents.rows_by_agent]` or `[ui.sidebar.spaces] rows`,
  holds the token, `$sluice` or `$sluice_q`, anywhere in its rows;
- a `tab_bar_right` entry's `command` contains
  `agentplot.sluice/segment/bin/segment.sh`;
- `[ui.toast] delivery` is `"herdr"`, `"terminal"` or `"system"`.

Leave each as it is, and add none of them again. Where one differs from
what setup printed, such as an older path to the segment or
`timeout_seconds = 2` where setup gives 30, say how it differs and why
setup gives what it does, in the words of setup's comment above it, and
change it only on the person's word.

Where every line is there, write nothing, run no reload, and say the
plugin is set up, on which keys. You are done.

### The copy

Before the first edit, copy the file somewhere outside herdr's folder,
never beside it:

```sh
copy=$(mktemp) && cp -p "$f" "$copy" && echo "copy: $copy"
```

### The edit

Edit the file in place with your own edit tool, a few lines at a time.
Never rewrite it through a TOML library or a formatter: that drops the
person's comments and reorders their tables. Every line, comment and
order of the person's stays as it was.

TOML refuses a table opened twice, and a key belongs to the last table
header above it. So a key goes inside the table the file already holds
for it, among that table's own keys, before the next header, and never
in a second copy of the table. A table the file lacks is added whole, at
the end.

- **The keys.** For each action step 4 gave a key, one `[[keys.command]]`
  entry with that key and setup's `type`, `command` and `description`,
  after the file's last `[[keys.command]]` entry, else at the end.
- **The agent rows.**
  - Where `[ui.sidebar.agents]` holds `rows`, append setup's `$sluice`
    row, with its `rules`, as their last row. The person's rows, another
    plugin's among them, stay where they are.
  - Do the same in each list of `[ui.sidebar.agents.rows_by_agent]`:
    those lists replace `rows` for their agents.
  - Where the file holds no `rows` there, add setup's `rows`, herdr's own
    and sluice's, in `[ui.sidebar.agents]`, made where the file has
    none.
- **The workspace rows.** Where `[ui.sidebar.spaces]` holds `rows`,
  append `"$sluice_q"` to the end of their first row. Otherwise add
  setup's `rows` there, in the table made where the file has none.
- **The tab bar.** Add setup's one entry in the form the file already
  writes `ui.tab_bar_right` in.
  - As `[[ui.tab_bar_right]]` tables: one more such table, after the
    last of them, before the next header. Add no `tab_bar_right` key to
    `[ui]`, and open no second `[ui]`:

    ```toml
    [[ui.tab_bar_right]]
    type = "command"
    command = "<another plugin's segment>"
    interval_seconds = 10

    [[ui.tab_bar_right]]
    type = "command"
    command = 'sh "$HOME/.local/state/herdr/plugins/agentplot.sluice/segment/bin/segment.sh"'
    interval_seconds = 5
    timeout_seconds = 30
    ```

  - As `tab_bar_right = [ … ]` in `[ui]`: one more element of that
    list:

    ```toml
    [ui]
    tab_bar_right = [
      { type = "command", command = "<another plugin's segment>", interval_seconds = 10 },
      { type = "command", command = 'sh "$HOME/.local/state/herdr/plugins/agentplot.sluice/segment/bin/segment.sh"', interval_seconds = 5, timeout_seconds = 30 },
    ]
    ```

  - Neither: the key `tab_bar_right` with setup's entry, among the keys
    of the file's `[ui]`, or in a new `[ui]` table where the file has
    none:

    ```toml
    [ui]
    sidebar_width = 30
    tab_bar_right = [
      { type = "command", command = 'sh "$HOME/.local/state/herdr/plugins/agentplot.sluice/segment/bin/segment.sh"', interval_seconds = 5, timeout_seconds = 30 },
    ]

    [ui.toast]
    delivery = "system"
    ```

  The command is the one step 2 printed, which names this host's state
  directory; the one above is a placeholder for it.
- **Toasts.** Where `[ui.toast] delivery` is unset or `"off"`, set
  `delivery = "herdr"`, in `[ui.toast]`, made where the file has none.
  Leave `"terminal"` and `"system"` as they are.

## 6. herdr's check, and the reload

```sh
herdr config check
```

- **It exits non-zero:** put the file back as it was, show the check's
  words, run no reload, and stop:

  ```sh
  cp -p "$copy" "$f"   # or, where there was no file before: rm "$f"
  ```

- **It passes** (`config: ok`): reload herdr.

  ```sh
  herdr server reload-config
  ```

  Where no herdr server answers, the next server to start reads the
  file. Every other herdr server running reads it at its own reload: the
  `reload_config` key, `prefix+shift+r` unless the file rebinds it, or
  its next start.

## 7. What to say

Once the check has passed, tell the person, in plain words:

- the file you edited, or wrote;
- what you added: the keys, the `$sluice` row and where, `$sluice_q` and
  where, the tab bar's entry, and toasts;
- the key each action is on and why: setup's, a key they already had, or
  one you picked because setup's was taken, naming what holds it;
- any line of sluice's that differs from setup's, how and why, and that
  you change it on their word;
- where the copy of the file as it was is, so undoing is one copy;
- whether herdr reloaded, and that every other herdr server running
  reads the file at its own reload;
- where the segment has no copy yet, that the tab bar shows the queue
  once a herdr server has started the plugin.

Where the file is managed (step 3), say instead that you wrote nothing,
naming the link's target or the read-only path, and show the lines as
they would merge, with the keys you would bind and why.
