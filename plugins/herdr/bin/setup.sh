#!/bin/sh
# shellcheck disable=SC1091,SC2154,SC2016 # jq programs; common.sh, sourced, sets root, lib, base, state and config.
# Show the snippets to add to herdr's config.toml: the key, the sidebar
# rows and the tab-bar segment, with this plugin's paths filled in. It
# edits nothing; on a terminal it waits for q or Escape.
set -u
. "$(dirname "$0")/../lib/common.sh"

# The segment's command as a TOML string. herdr runs it through
# `/bin/sh -lc`, which fills in each host's $HOME, so a state directory
# below the home directory is named from $HOME: one line then serves every
# host whose herdr keeps its state at the same place below it.
home=${HOME:-}
home=${home%/}
if [ -n "$home" ] && [ "${base#"$home"/}" != "$base" ]; then
  segment="'sh \"\$HOME/${base#"$home"/}/segment/bin/segment.sh\"'"
else
  segment="\"sh '$base/segment/bin/segment.sh'\""
fi

cat <<EOF
sluice for herdr: add these to herdr's config.toml yourself (or to
whatever generates it), then run \`herdr server reload-config\`.

# The status popup on prefix+s.
[[keys.command]]
key = "prefix+s"
type = "plugin_action"
command = "agentplot.sluice.status"
description = "sluice: the queue"

# Jump to the pane whose submission was blamed.
[[keys.command]]
key = "prefix+shift+s"
type = "plugin_action"
command = "agentplot.sluice.jump"
description = "sluice: jump to the blamed submission's pane"

# Each agent's entry gains a line when its worktree has something to say
# (only panes running an agent are listed there); a blame or a conflict is
# red. These are herdr's own rows plus the last;
# add only the last to rows of your own.
[ui.sidebar.agents]
rows = [
  ["state_icon", "machine", "workspace", "tab"],
  ["agent"],
  [{ token = "\$sluice", rules = [{ contains = "BLAMED", fg = "#f55", bold = true }, { contains = "CONFLICT", fg = "#f55", bold = true }, { contains = "conflicts", fg = "#fc0" }] }],
]

# The queue on each workspace's row.
[ui.sidebar.spaces]
rows = [
  ["state_icon", "workspace", "\$sluice_q"],
  ["branch", "git_status"],
]

# The focused repository's queue at the right of the tab bar: one entry
# for the tab_bar_right list in your [ui] table.
[ui]
tab_bar_right = [
  { type = "command", command = $segment, interval_seconds = 5, timeout_seconds = 2 },
]

Toasts need herdr's own to be on:
[ui.toast]
delivery = "herdr"
EOF

if [ -t 0 ] && [ -t 1 ]; then
  echo
  echo "q close"
  saved=$(stty -g)
  trap 'stty "$saved"' EXIT
  stty -icanon -echo min 1
  while :; do
    k=$(dd bs=1 count=1 2>/dev/null)
    case $k in q | "$(printf '\033')") exit 0 ;; esac
  done
fi
