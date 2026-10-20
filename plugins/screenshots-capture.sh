#!/usr/bin/env bash
# Retakes the plugins' screenshots: the board in Claude Code and the status
# popup in herdr, each in a light and a dark terminal, of the queue that
# screenshots-scene.sh builds under /tmp/sluice-demo.
#
#   screenshots-capture.sh [board] [herdr]   both when neither is named
#
# Writes plugins/sluice-board/screenshots/board-{light,dark}.png and
# plugins/herdr/screenshots/popup-{light,dark}.png, optimized losslessly.
# Each frame is a window of its own Ghostty instance, shot whole with
# `screencapture -l` by another (Ghostty holds the Screen Recording grant;
# the shell this runs in may not), and never edited after.
#
# Run it in `devenv shell` on macOS, with Ghostty in /Applications. The
# gate cannot: it drives a GUI terminal and a logged-in Claude Code. The
# demo's Claude Code keeps its own configuration, and so its own login, in
# $SLUICE_SCREENSHOTS_CLAUDE (default ~/.local/state/sluice-screenshots/claude);
# the first take stops and asks for /login in a window of its own.
set -euo pipefail

W=$(cd "$(dirname "$0")/.." && pwd)
DEMO=/tmp/sluice-demo
CAP=/tmp/sluice-demo-capture
CLAUDE_DIR=${SLUICE_SCREENSHOTS_CLAUDE:-$HOME/.local/state/sluice-screenshots/claude}
GHOSTTY=/Applications/Ghostty.app
# Seconds a window is given to draw before it is shot.
SETTLE=${SLUICE_SCREENSHOTS_SETTLE:-25}
SHOTS=${*:-board herdr}

die() {
  echo "screenshots-capture: $*" >&2
  exit 1
}
[ "$(uname)" = Darwin ] || die "needs macOS"
[ -d "$GHOSTTY" ] || die "needs Ghostty in /Applications"
[ -x /usr/bin/swiftc ] && [ -x /usr/bin/clang ] || die "needs Apple's swiftc and clang: xcode-select --install"
for t in screencapture oxipng claude herdr moon jq cargo; do
  command -v "$t" >/dev/null || die "needs $t on PATH; run it in devenv shell"
done

# Nothing of the session this runs in reaches the demo: not its herdr, its
# Claude Code or its sluice state.
for v in $(env | grep -Eo '^(HERDR_[A-Z_]*|CLAUDE[A-Z_]*|ANTHROPIC_[A-Z_]*|XDG_[A-Z_]*)=' | tr -d =); do unset "$v"; done

# apple <command...>: Apple's compiler and SDK, never devenv's.
apple() { env -u SDKROOT -u DEVELOPER_DIR -u MACOSX_DEPLOYMENT_TARGET -u NIX_CFLAGS_COMPILE -u NIX_LDFLAGS "$@"; }

cargo build -q -p sluice --bins --manifest-path "$W/Cargo.toml"
bash "$W/plugins/screenshots-scene.sh" --teardown "$DEMO"
rm -rf "$CAP"
mkdir -p "$CAP/zdot" "$CAP/bin" "$CLAUDE_DIR"

# --- what every window runs under --------------------------------------------------------------------------------------
# sluice as built here, told its host is "devbox" by a library loaded into it alone that answers gethostname, so no
# frame names this machine.
cat >"$CAP/devbox.c" <<'C'
#include <string.h>
#include <unistd.h>
static int devbox(char *name, size_t len) {
  if (len == 0) return 0;
  strncpy(name, "devbox", len);
  name[len - 1] = 0;
  return 0;
}
__attribute__((used)) static const struct {
  const void *replacement;
  const void *original;
} interposers[] __attribute__((section("__DATA,__interpose"))) = {{(const void *)devbox, (const void *)gethostname}};
C
apple /usr/bin/clang -dynamiclib -o "$CAP/devbox.dylib" "$CAP/devbox.c"
printf '#!/bin/sh\nDYLD_INSERT_LIBRARIES=%s exec %s "$@"\n' "$CAP/devbox.dylib" "$W/target/debug/sluice" >"$CAP/bin/sluice"
chmod +x "$CAP/bin/sluice"
# The pinned Claude Code under its own name: herdr knows an agent by its process's name, and devenv's wrapper runs
# it as .claude. The wrapper's two settings are env.sh's.
real=$(dirname "$(command -v claude)")/.claude
[ -x "$real" ] || real=$(command -v claude)
ln -s "$real" "$CAP/bin/claude"
cat >"$CAP/env.sh" <<EOF
export PATH='$CAP/bin:$W/target/debug:$PATH' DISABLE_AUTOUPDATER=1 DISABLE_INSTALLATION_CHECKS=1
export XDG_STATE_HOME='$DEMO/state' XDG_CONFIG_HOME='$DEMO/config' CLAUDE_CONFIG_DIR='$CLAUDE_DIR'
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 GIT_TERMINAL_PROMPT=0
export ZDOTDIR='$CAP/zdot' LANG=en_US.UTF-8
EOF
# shellcheck disable=SC1091 # written just above
. "$CAP/env.sh"
# A prompt with no user or host in it.
printf '%s\n' "PROMPT='%1~ %# '" >"$CAP/zdot/.zshrc"
for theme in light dark; do
  printf '{"tui": "fullscreen", "theme": "%s", "permissions": {"defaultMode": "default"}}\n' "$theme" >"$CAP/claude-$theme.json"
done

# --- windows -------------------------------------------------------------------------------------------------------------
# The window ids and owners of every on-screen window, as "<id> <pid>".
cat >"$CAP/windows.swift" <<'EOF'
import CoreGraphics
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as! [[String: Any]]
for w in list where (w[kCGWindowLayer as String] as? Int) == 0 {
  print(w[kCGWindowNumber as String] as! Int, w[kCGWindowOwnerPID as String] as! Int)
}
EOF
apple /usr/bin/swiftc -O -o "$CAP/windows" "$CAP/windows.swift"

# open_window <name> <theme> <columns> <rows> <command>: a Ghostty instance of its own running the command under env.sh;
# prints "<window id> <pid>". The command holds no single quote, and no path here a space.
# launch <name> <theme> <columns> <rows> <command>: the Ghostty instance started, nothing waited for.
launch() {
  local theme=$2 cols=$3 rows=$4 cmd=$5 conf=$CAP/$1-$2.conf
  case $theme in light) theme="GitHub Light Default" ;; dark) theme="GitHub Dark Default" ;; esac
  cat >"$conf" <<EOF
theme = $theme
font-size = 13
window-width = $cols
window-height = $rows
window-padding-x = 12
window-padding-y = 10
macos-titlebar-style = hidden
confirm-close-surface = false
window-save-state = never
quit-after-last-window-closed = true
shell-integration = none
command = /bin/zsh -c '. $CAP/env.sh; $cmd'
EOF
  open -na "$GHOSTTY" --args --config-default-files=false --config-file="$conf"
}

open_window() {
  local before pid wid
  before=$(pgrep -x ghostty | sort || true)
  launch "$@"
  for _ in $(seq 1 150); do
    pid=$(comm -13 <(echo "$before") <(pgrep -x ghostty | sort) | head -1)
    if [ -n "$pid" ]; then
      wid=$("$CAP/windows" | awk -v p="$pid" '$2 == p { print $1; exit }')
      [ -n "$wid" ] && break
    fi
    sleep 0.2
  done
  [ -n "${wid:-}" ] || die "the $1 window did not open"
  echo "$wid $pid"
}

# shoot <window id> <file>: the window as it is drawn, taken by a Ghostty instance of its own.
shoot() {
  local wid=$1 out=$2 rc=$CAP/shot.rc
  rm -f "$rc"
  # The camera quits once its command ends, often before its window could be found: only its result is waited for.
  launch camera dark 40 4 "screencapture -o -x -l$wid $out; echo \$? >$rc"
  for _ in $(seq 1 150); do
    [ -f "$rc" ] && break
    sleep 0.2
  done
  [ "$(cat "$rc" 2>/dev/null)" = 0 ] || die "screencapture could not take window $wid; give Ghostty Screen Recording in System Settings"
}

# trust <dir>: Claude Code asks nothing about the folder.
trust() {
  local f=$CLAUDE_DIR/.claude.json
  [ -f "$f" ] || echo '{}' >"$f"
  jq --arg d "$1" '.projects[$d] = ((.projects[$d] // {}) + {hasTrustDialogAccepted: true, hasCompletedProjectOnboarding: true})' "$f" >"$f.new"
  mv "$f.new" "$f"
}

# --- the scene -----------------------------------------------------------------------------------------------------------
# A first take: Claude Code is logged in before the scene starts, which moves on its own from then.
if ! claude auth status 2>/dev/null | jq -e .loggedIn >/dev/null; then
  login=$(open_window login dark 100 30 "cd $CAP && claude --settings $CAP/claude-dark.json")
  echo "screenshots-capture: log the demo's Claude Code in: run /login in the window that opened, then /exit"
  while kill -0 "${login#* }" 2>/dev/null; do sleep 1; done
  claude auth status 2>/dev/null | jq -e .loggedIn >/dev/null || die "the demo's Claude Code is not logged in"
fi
bash "$W/plugins/screenshots-scene.sh" "$DEMO" >"$CAP/scene.log" 2>&1 &
scene=$!
until grep -qx ready "$CAP/scene.log"; do
  kill -0 "$scene" 2>/dev/null || die "the scene failed: $(tail -5 "$CAP/scene.log")"
  sleep 2
done
WT=$DEMO/shop-wt
for d in "$WT"/*; do trust "$d"; done

out_board=$W/plugins/sluice-board/screenshots
out_herdr=$W/plugins/herdr/screenshots
mkdir -p "$out_board" "$out_herdr"

# --- the board: this session's worktree is order-emails, queued behind checkout-flow ---------------------------------------
board() {
  local theme=$1 win
  win=$(open_window board "$theme" 150 42 \
    "cd $WT/order-emails && exec claude --plugin-dir $W/plugins/sluice-board --settings $CAP/claude-$theme.json --strict-mcp-config /sluice")
  sleep "$SETTLE"
  shoot "${win% *}" "$out_board/board-$theme.png"
  kill "${win#* }" 2>/dev/null || true
}

# --- herdr: a workspace of agents, one per worktree, and the status popup over it ----------------------------------------
# herdr_config <theme>: a fresh herdr, with no session from an earlier take to restore.
herdr_config() {
  local name=catppuccin
  [ "$1" = light ] && name=catppuccin-latte
  rm -rf "$DEMO/config/herdr"
  mkdir -p "$DEMO/config/herdr"
  # The plugin's setup snippets (bin/setup.sh), with herdr's machine column left out of the agents' rows, no
  # first-run welcome, and a theme to match the terminal's.
  {
    echo "onboarding = false"
    echo
    echo "[theme]"
    echo "name = \"$name\""
  } >"$DEMO/config/herdr/config.toml"
  cat >>"$DEMO/config/herdr/config.toml" <<'EOF'

[[keys.command]]
key = "prefix+s"
type = "plugin_action"
command = "agentplot.sluice.status"
description = "sluice: the queue"

[ui.sidebar.agents]
rows = [
  ["state_icon", "workspace", "tab"],
  ["agent"],
  [{ token = "$sluice", rules = [{ contains = "BLAMED", fg = "#f55", bold = true }, { contains = "CONFLICT", fg = "#f55", bold = true }, { contains = "conflicts", fg = "#fc0" }] }],
]

[ui.sidebar.spaces]
rows = [
  ["state_icon", "workspace", "$sluice_q"],
  ["branch", "git_status"],
]

[ui.toast]
delivery = "herdr"
EOF
}

# split <pane> <direction> <worktree>: a new pane in the worktree, its Claude Code started as an agent named for it;
# prints the pane's id.
split() {
  local p
  p=$(herdr pane split "$1" --direction "$2" --cwd "$WT/$3" --no-focus | jq -r '.result.pane.pane_id')
  # Its shell at its prompt first, or the agent has no shell to start in.
  herdr pane wait-output "$p" --match "$3 %" --timeout 30000 >/dev/null
  herdr agent start "$3" --kind claude --pane "$p" -- --settings "$CAP/claude-$theme.json" --strict-mcp-config >/dev/null
  echo "$p"
}

popup() {
  local theme=$1 win root a b
  herdr_config "$theme"
  win=$(open_window herdr "$theme" 170 48 "cd $DEMO/shop && exec herdr")
  for _ in $(seq 1 150); do
    herdr workspace list >/dev/null 2>&1 && break
    sleep 0.2
  done
  herdr plugin link "$W/plugins/herdr" >/dev/null
  # herdr knows a pane's Claude Code by its hooks, in the demo's own Claude Code configuration.
  herdr integration install claude >/dev/null
  herdr workspace rename "$(herdr workspace list | jq -r '.result.workspaces[0].workspace_id')" shop >/dev/null
  # Four panes, two by two, one per worktree: the agent running its check among them.
  root=$(herdr pane list | jq -r '.result.panes[0].pane_id')
  a=$(split "$root" right checkout-flow)
  herdr pane close "$root" >/dev/null
  b=$(split "$a" right order-emails)
  split "$a" down inventory-sync >/dev/null
  split "$b" down coupon-codes >/dev/null
  sleep "$SETTLE"
  herdr plugin action invoke agentplot.sluice.status >/dev/null
  sleep 5
  shoot "${win% *}" "$out_herdr/popup-$theme.png"
  herdr server stop >/dev/null 2>&1 || true
  kill "${win#* }" 2>/dev/null || true
}

for theme in light dark; do
  for s in $SHOTS; do
    case $s in board) board "$theme" ;; herdr) popup "$theme" ;; *) die "no shot named $s" ;; esac
  done
done
bash "$W/plugins/screenshots-scene.sh" --teardown "$DEMO"
for f in "$out_board"/*.png "$out_herdr"/*.png; do
  [ -f "$f" ] && oxipng -q -o max --strip safe "$f"
done
ls -l "$out_board" "$out_herdr"
