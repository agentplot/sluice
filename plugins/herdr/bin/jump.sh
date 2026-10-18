#!/bin/sh
# shellcheck disable=SC1091,SC2154,SC2016 # jq programs; common.sh, sourced, sets root, lib, state and config.
# Focus the pane whose worktree made the submission most recently blamed or
# found in conflict, in any repository the watcher holds; a toast says so
# when no pane is open for it.
set -u
. "$(dirname "$0")/../lib/common.sh"
. "$lib/panes.sh"

newest=
for f in "$state"/docs/*.json; do
  [ -e "$f" ] || continue
  k=$(basename "$f" .json)
  newest=$(jq -c --arg k "$k" --argjson n "${newest:-null}" '
    [.branches[] | .branch as $b | .recent[] | select(.event == "blamed" or .event == "conflict") | . + {branch: $b, key: $k}]
    + [$n | select(.)] | sort_by(.at) | last' "$f")
done
if [ -z "$newest" ] || [ "$newest" = null ]; then
  hd notification show "sluice: nothing blamed" --body "No submission was blamed or conflicted lately." >/dev/null 2>&1
  exit 0
fi
k=$(printf '%s' "$newest" | jq -r .key)
id=$(printf '%s' "$newest" | jq -r .id)
path=$(jq -r --arg i "$id" '.[$i].path // empty' "$state/docs/$k.paths" 2>/dev/null)
if [ -z "$path" ] && [ -f "$state/docs/$k.dir" ]; then
  path=$(cd "$(cat "$state/docs/$k.dir")" && sluice status "$id" --json 2>/dev/null | jq -r '.from.path // empty')
fi
pane=
[ -n "$path" ] && pane=$(panes_json | jq -r --arg p "$path" '[.[] | select(.top == $p)] | first // empty | [.pane_id, .tab_id, .workspace_id] | @tsv')
if [ -z "$pane" ]; then
  hd notification show "sluice: no pane for #$id" \
    --body "No open pane is in the worktree #$id came from${path:+ ($path)}." >/dev/null 2>&1
  exit 0
fi
IFS='	' read -r p t w <<EOF
$pane
EOF
focus_pane "$p" "$t" "$w"
