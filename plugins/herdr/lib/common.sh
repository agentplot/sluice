# shellcheck shell=sh
# What every script of the plugin shares: where its state is, how it calls
# herdr and sluice, and how it names a repository. Sourced, never run.

: "${HERDR_PLUGIN_ROOT:=$(cd "$(dirname "$0")/.." && pwd -P)}"
: "${HERDR_PLUGIN_STATE_DIR:=${XDG_STATE_HOME:-$HOME/.local/state}/herdr-agentplot.sluice}"
: "${HERDR_PLUGIN_CONFIG_DIR:=${XDG_CONFIG_HOME:-$HOME/.config}/herdr/plugins/config/agentplot.sluice}"
root=$HERDR_PLUGIN_ROOT
# Each herdr server, told apart by its socket, has its own watcher, lease,
# documents and tokens; the tab-bar copy is shared, at a path that lasts.
base=$HERDR_PLUGIN_STATE_DIR
state=$base/servers/$(printf '%s' "${HERDR_SOCKET_PATH:-none}" | cksum | cut -d' ' -f1)
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

# A fixed-length name for a string: SHA-256 by whichever tool the system
# has (sha256sum on Linux and busybox, shasum or openssl on macOS), else
# two checksums, said once in the log.
digest() {
  if command -v sha256sum >/dev/null 2>&1; then
    printf '%s' "$1" | sha256sum | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    printf '%s' "$1" | shasum -a 256 | cut -d' ' -f1
  elif command -v openssl >/dev/null 2>&1; then
    printf '%s' "$1" | openssl dgst -sha256 | sed 's/.*= *//'
  else
    if [ ! -f "$state/no-sha256" ]; then
      : >"$state/no-sha256"
      say "no sha256sum, shasum or openssl: told events are named by checksums, which two may share"
    fi
    echo "$(sum "$1")-$(sum "x$1")-${#1}"
  fi
}

# Replace a file in one step, from stdin; nothing in, nothing replaced.
put() {
  tmp="$1.tmp.$$"
  if cat >"$tmp" && [ -s "$tmp" ]; then mv -f "$tmp" "$1"; else
    rm -f "$tmp"
    return 1
  fi
}

# Why sluice cannot be asked here, or nothing: not on PATH, or, as the
# watcher found, too old for overview.
sluice_problem() {
  if ! command -v sluice >/dev/null 2>&1; then
    echo "sluice is not on PATH: install sluice 0.12 or later."
    return
  fi
  cat "$state/problem" 2>/dev/null
}

# Note a problem with sluice for the popup, logging it once; none clears it.
set_problem() {
  if [ -z "$1" ]; then
    rm -f "$state/problem"
  elif [ "$(cat "$state/problem" 2>/dev/null)" != "$1" ]; then
    printf '%s\n' "$1" >"$state/problem"
    say "$1"
  fi
}

# Every document the watcher holds, as one array.
all_docs() {
  set -- "$state"/docs/*.json
  if [ -e "$1" ]; then jq -s . "$@"; else echo '[]'; fi
}
