#!/bin/sh
# shellcheck disable=SC1091,SC2154,SC2016 # jq programs; common.sh, sourced, sets root, lib, state and config.
# The status popup, and with --dashboard the same view in its own tab:
# the focused pane's repository's queue, read from the watcher's state and
# redrawn as it changes. Keys: q or Escape closes, j jumps to the culprit's
# pane, a switches between this repository and every one held.
# --once prints one frame and exits.
set -u
. "$(dirname "$0")/../lib/common.sh"

all=false
once=false
for a in "$@"; do
  case $a in
  --dashboard) all=true ;;
  --once) once=true ;;
  esac
done

# The directory of the pane the popup was opened over.
ctx=${HERDR_PLUGIN_CONTEXT_JSON:-}
[ -n "$ctx" ] || ctx='{}'
dir=${SLUICE_HERDR_DIR:-$(printf '%s' "$ctx" |
  jq -r '.focused_pane_cwd // .workspace_cwd // empty' 2>/dev/null)}
repo=
[ -n "$dir" ] && repo=$(repo_of "$dir")
name=
[ -n "$repo" ] && name=$(name_of "$repo")

[ -t 1 ] && tty=true || tty=false

frame() {
  t=$(now)
  docs=$(all_docs)
  shown=$(printf '%s' "$docs" | jqw -c --argjson all "$all" --arg r "$repo" --arg n "$name" '
    include "words";
    if $all then . else map(select(.repository == $r or (.repository | repo_name) == $n)) end
    | sort_by(.repository, .host)')
  if [ "$(printf '%s' "$shown" | jq length)" -gt 0 ]; then
    printf '%s' "$shown" | jqw -r --argjson now "$t" --argjson color "$tty" 'include "words"; .[] | frame($now; $color)[]'
  elif [ -n "$(sluice_problem)" ]; then
    sluice_problem
  elif [ "$all" = true ]; then
    echo "No repository with sluice's records is open in a pane."
  elif [ -z "$repo" ]; then
    echo "No sluice records in this repository."
  elif [ "$(cut -d' ' -f1 "$state/probe/$(sum "$repo")" 2>/dev/null)" = none ]; then
    echo "No sluice records in this repository."
  else
    echo "Waiting for sluice to read $repo..."
  fi
  echo
  if [ "$all" = true ]; then
    echo "q close · j jump to the blamed submission's pane · a this repository"
  else
    echo "q close · j jump to the blamed submission's pane · a every repository"
  fi
}

draw() {
  if [ "$tty" = true ]; then
    printf '\033[H'
    esc=$(printf '\033')
    frame | sed "s/\$/${esc}[K/"
    printf '\033[J'
  else
    frame
    printf '\f\n'
  fi
}

# A repository the watcher has not read yet: ask it to.
if [ -n "$repo" ] && [ "$(all_docs | jq --arg r "$repo" 'any(.repository == $r)')" = false ]; then
  printf '%s\n' "$dir" >"$state/want/$(sum "$repo")"
  sh "$root/bin/watch.sh" --ensure
fi

if [ "$once" = true ]; then
  frame
  exit 0
fi

if [ -t 0 ]; then
  saved=$(stty -g)
  trap 'stty "$saved"; printf "\033[?25h"' EXIT
  trap 'exit 0' INT TERM HUP
  stty -icanon -echo min 0 time 10
  printf '\033[?25l\033[2J'
fi

draw
while :; do
  key=$(dd bs=1 count=1 2>/dev/null)
  if [ -z "$key" ]; then
    # A second without a key on a terminal; the end of input otherwise.
    [ -t 0 ] || exit 0
    draw
    continue
  fi
  case $key in
  q) exit 0 ;;
  "$(printf '\033')")
    # Escape alone closes; an arrow key's bytes follow at once.
    [ -t 0 ] || exit 0
    stty time 1
    more=$(dd bs=8 count=1 2>/dev/null)
    stty time 10
    [ -z "$more" ] && exit 0
    ;;
  j)
    sh "$root/bin/jump.sh"
    exit 0
    ;;
  a)
    if [ "$all" = true ]; then all=false; else all=true; fi
    ;;
  esac
  draw
done
