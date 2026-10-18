#!/bin/sh
# shellcheck disable=SC1091,SC2154,SC2016 # jq programs; common.sh, sourced, sets root, lib, base, state and config.
# A row's link, from herdr's Ctrl-click (HERDR_PLUGIN_CLICKED_URL) or the
# popup's number key ($1): sluice-herdr:focus?path=<worktree> focuses that
# worktree's pane; sluice-herdr:log?path=<log>&task=<test> opens the log in
# a split. Any pane can print a link, so a link is acted on only when the
# documents the watcher holds name its path; anything else is logged.
set -u
. "$(dirname "$0")/../lib/common.sh"
. "$lib/panes.sh"

url=${1:-${HERDR_PLUGIN_CLICKED_URL:-}}

# One parameter of the link's query, percent-decoded.
param() {
  # In the C locale, so a byte decoded is that byte, whatever the user's
  # locale: gawk in UTF-8 would make %C3%A9 two characters.
  printf '%s' "${url#*\?}" | tr '&' '\n' | sed -n "s/^$1=//p" | head -1 | LC_ALL=C awk '
    BEGIN { for (i = 0; i < 256; i++) hex[sprintf("%02x", i)] = sprintf("%c", i) }
    { s = $0; out = ""
      while ((i = index(s, "%")) > 0) {
        out = out substr(s, 1, i - 1)
        out = out hex[tolower(substr(s, i + 1, 2))]
        s = substr(s, i + 3)
      }
      printf "%s", out s }'
}

case $url in
sluice-herdr:focus\?*) kind=focus ;;
sluice-herdr:log\?*) kind=log ;;
*)
  say "not a link of this plugin's: $url"
  exit 0
  ;;
esac
path=$(param path)
docs=$(all_docs)

case $kind in
focus)
  learned='{}'
  for f in "$state"/docs/*.paths; do
    [ -e "$f" ] && learned=$(jq -c --argjson a "$learned" '$a + (objects // {})' "$f" 2>/dev/null || echo "$learned")
  done
  named=$(printf '%s' "$docs" | jq -r --arg p "$path" --argjson l "$learned" '
    [.[].branches[].submissions[].from.path, ($l[] | .path)] | any(. == $p)')
  if [ "$named" != true ]; then
    say "a link to a worktree no submission came from, left alone: $path"
    exit 0
  fi
  pane=$(panes_json | jq -r --arg p "$path" '[.[] | select(.top == $p)] | first // empty | [.pane_id, .tab_id, .workspace_id] | @tsv')
  if [ -z "$pane" ]; then
    hd notification show "sluice: no pane for $path" --body "No open pane is in that worktree." >/dev/null 2>&1
    exit 0
  fi
  IFS='	' read -r p t w <<EOF2
$pane
EOF2
  focus_pane "$p" "$t" "$w"
  ;;
log)
  named=$(printf '%s' "$docs" | jq -r --arg p "$path" '[.[].branches[].candidates[].failed[].log] | any(. == $p)')
  if [ "$named" != true ]; then
    say "a link to a log no combination's failed tests name, left alone: $path"
    exit 0
  fi
  ctx=${HERDR_PLUGIN_CONTEXT_JSON:-}
  [ -n "$ctx" ] || ctx='{}'
  target=$(printf '%s' "$ctx" | jq -r '.focused_pane_id // empty' 2>/dev/null)
  [ -n "$target" ] || target=${HERDR_PANE_ID:-}
  if [ -n "$target" ]; then
    hd plugin pane open --plugin agentplot.sluice --entrypoint log --placement split --direction right \
      --target-pane "$target" --env "SLUICE_HERDR_LOG=$path" --env "SLUICE_HERDR_TASK=$(param task)" >/dev/null 2>&1
  else
    hd plugin pane open --plugin agentplot.sluice --entrypoint log --placement split --direction right \
      --env "SLUICE_HERDR_LOG=$path" --env "SLUICE_HERDR_TASK=$(param task)" >/dev/null 2>&1
  fi
  ;;
esac
