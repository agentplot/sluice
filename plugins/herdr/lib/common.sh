# shellcheck shell=sh
# What every script of the plugin shares: where its state is, how it calls
# herdr and sluice, and how it names a repository. Sourced, never run.

: "${HERDR_PLUGIN_ROOT:=$(cd "$(dirname "$0")/.." && pwd -P)}"
: "${HERDR_PLUGIN_STATE_DIR:=${XDG_STATE_HOME:-$HOME/.local/state}/herdr-agentplot.sluice}"
: "${HERDR_PLUGIN_CONFIG_DIR:=${XDG_CONFIG_HOME:-$HOME/.config}/herdr/plugins/config/agentplot.sluice}"
root=$HERDR_PLUGIN_ROOT
state=$HERDR_PLUGIN_STATE_DIR
# shellcheck disable=SC2034 # read by the scripts that source this
config=$HERDR_PLUGIN_CONFIG_DIR
lib=$root/lib
mkdir -p "$state/docs" "$state/told" "$state/checks" "$state/probe" "$state/want" "$state/run" "$state/failed"

# The one source of the time, so the tests can fix it.
now() {
  echo "${SLUICE_HERDR_NOW:-$(date +%s)}"
}

say() {
  printf '%s agentplot.sluice: %s\n' "$(date +%H:%M:%S)" "$*" >&2
}

hd() {
  "${HERDR_BIN_PATH:-herdr}" "$@"
}

# jq with the plugin's words and rules loaded.
jqw() {
  jq -L "$lib" "$@"
}

# The checksum of a string: a file name for a path.
sum() {
  printf '%s' "$1" | cksum | cut -d' ' -f1
}

# The repository holding a directory (its git common dir, absolute and
# without links), or nothing outside git.
repo_of() {
  d=$(git -C "$1" rev-parse --path-format=absolute --git-common-dir 2>/dev/null) || return 1
  (cd "$d" 2>/dev/null && pwd -P)
}

# The top of the worktree holding a directory.
top_of() {
  t=$(git -C "$1" rev-parse --show-toplevel 2>/dev/null) || return 1
  (cd "$t" 2>/dev/null && pwd -P)
}

# The state key of a document: its host, its repository's name and a
# checksum of the repository's path.
key_of() {
  echo "$1--$(name_of "$2")-$(sum "$2")"
}

# A repository's name: its directory's, without `.git`, and its worktree's
# when the repository is the `.git` inside one.
name_of() {
  basename "${1%/.git}" .git
}

# Replace a file in one step, from stdin; nothing in, nothing replaced.
put() {
  tmp="$1.tmp.$$"
  if cat >"$tmp" && [ -s "$tmp" ]; then mv -f "$tmp" "$1"; else
    rm -f "$tmp"
    return 1
  fi
}

# Every document the watcher holds, as one array.
all_docs() {
  set -- "$state"/docs/*.json
  if [ -e "$1" ]; then jq -s . "$@"; else echo '[]'; fi
}
