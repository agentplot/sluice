# shellcheck shell=sh disable=SC2154,SC2016 # jq programs; common.sh sets lib, state and config.
# herdr's panes as the plugin needs them: each with the top of the worktree
# it is in, that worktree's repository, and whether it is the pane in front
# of the user (it, its tab and its workspace all focused). Sourced.

# Prints [{pane_id, tab_id, workspace_id, agent, top, repo, front}].
panes_json() {
  raw=$(hd pane list 2>/dev/null) || raw='{}'
  tabs=$(hd tab list 2>/dev/null) || tabs='{}'
  spaces=$(hd workspace list 2>/dev/null) || spaces='{}'
  dirs=$(printf '%s' "$raw" | jq -r '.result.panes // [] | .[] | (.foreground_cwd // .cwd // empty)' | sort -u)
  where='{}'
  old_ifs=$IFS
  IFS='
'
  for d in $dirs; do
    t=$(top_of "$d") || continue
    r=$(repo_of "$d") || continue
    where=$(printf '%s' "$where" | jq -c --arg d "$d" --arg t "$t" --arg r "$r" '. + {($d): {top: $t, repo: $r}}')
  done
  IFS=$old_ifs
  jq -n -c --argjson p "$raw" --argjson t "$tabs" --argjson w "$spaces" --argjson where "$where" '
    ([$t.result.tabs // [] | .[] | select(.focused) | .tab_id]) as $ft
    | ([$w.result.workspaces // [] | .[] | select(.focused) | .workspace_id]) as $fw
    | [ $p.result.panes // [] | .[] | (.foreground_cwd // .cwd) as $d | .tab_id as $tab | .workspace_id as $ws
        | {pane_id, tab_id, workspace_id, agent, top: $where[$d].top, repo: $where[$d].repo,
           front: (.focused == true and ($ft | index($tab)) != null and ($fw | index($ws)) != null)} ]'
}

# Bring a pane forward: an agent's pane by `agent focus`, which brings its
# tab and workspace with it; any other by its workspace, then its tab.
focus_pane() {
  # $1 pane_id, $2 tab_id, $3 workspace_id
  hd agent focus "$1" >/dev/null 2>&1 && return 0
  hd workspace focus "$3" >/dev/null 2>&1
  hd tab focus "$2" >/dev/null 2>&1
}
