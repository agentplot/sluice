# shellcheck shell=sh disable=SC2154,SC2016 # jq programs; common.sh sets lib, state and config.
# herdr's panes as the plugin needs them: each with its folder, the top of
# the worktree it is in, that worktree's repository, and the branch its
# `sluice_branch` token names, which any source may set; with --front,
# whether it is the pane in front of the user (it, its tab and its
# workspace all focused). Sourced.

# Prints [{pane_id, tab_id, workspace_id, cwd, top, repo, named}], each
# with `front` too under --front, for the server in the environment; []
# and 1 when herdr does not list the panes. A pane's folder is resolved as
# the panes kept in folder $1, a server's, found it, and git is asked only
# of a folder none of them is in (D6). Only with --front are tabs and
# workspaces listed too, which a landing's toast alone reads.
panes_json() {
  pj_front=false
  if [ "${1:-}" = --front ]; then
    pj_front=true
    shift
  fi
  pj_raw=$(hd pane list 2>/dev/null) || {
    echo '[]'
    return 1
  }
  pj_tabs='{}'
  pj_spaces='{}'
  if [ "$pj_front" = true ]; then
    pj_tabs=$(hd tab list 2>/dev/null) || pj_tabs='{}'
    pj_spaces=$(hd workspace list 2>/dev/null) || pj_spaces='{}'
  fi
  pj_kept=
  if [ -n "${1:-}" ] && [ -f "$1/panes.json" ]; then { IFS= read -r pj_kept <"$1/panes.json"; } 2>/dev/null; fi
  # The folders no kept pane is in, each asked of git once: its worktree's
  # top and its repository, both without links, or nothing outside git.
  pj_new=$(printf '%s' "$pj_raw" | jq -r --arg k "$pj_kept" '
    [.result.panes // [] | .[] | (.foreground_cwd // .cwd // empty) | strings | select(test("\n") | not)]
    - [$k | try fromjson catch [] | arrays | .[] | objects | .cwd] | unique[]' 2>/dev/null)
  pj_nl='
'
  pj_met=
  pj_ifs=$IFS
  IFS=$pj_nl
  set -f
  for pj_d in $pj_new; do
    pj_g=$(git -C "$pj_d" rev-parse --path-format=absolute --show-toplevel --git-common-dir 2>/dev/null) &&
      pj_g=$(cd "${pj_g%%"$pj_nl"*}" 2>/dev/null && pwd -P && cd "${pj_g#*"$pj_nl"}" 2>/dev/null && pwd -P) || pj_g=
    pj_met="$pj_met$pj_d	${pj_g%%"$pj_nl"*}	${pj_g#*"$pj_nl"}$pj_nl"
  done
  set +f
  IFS=$pj_ifs
  jq -n -c --argjson p "$pj_raw" --argjson t "$pj_tabs" --argjson w "$pj_spaces" --arg k "$pj_kept" \
    --arg met "$pj_met" --argjson front "$pj_front" '
    def given: if . == "" then null else . end;
    ([$k | try fromjson catch [] | arrays | .[] | objects | select(.cwd | type == "string") | {(.cwd): {top, repo}}]
      + [$met | splits("\n") | select(length > 0) | split("\t") | {(.[0]): {top: (.[1] | given), repo: (.[2] | given)}}]
      | add // {}) as $where
    | ([$t.result.tabs // [] | .[] | select(.focused) | .tab_id]) as $ft
    | ([$w.result.workspaces // [] | .[] | select(.focused) | .workspace_id]) as $fw
    | [ $p.result.panes // [] | .[] | (.foreground_cwd // .cwd) as $d | .tab_id as $tab | .workspace_id as $ws
        | {pane_id, tab_id, workspace_id, cwd: $d, top: $where[$d].top, repo: $where[$d].repo,
           named: (.tokens.sluice_branch // null | if type == "string" then gsub("[[:cntrl:]]"; "") else null end
             | if . == "" then null else . end)}
        + (if $front then {front: (.focused == true and ($ft | index($tab)) != null and ($fw | index($ws)) != null)}
           else {} end) ]' 2>/dev/null || {
    echo '[]'
    return 1
  }
}

# Read the panes of the server in the environment and keep them in its
# folder $1 (D6), replaced only when they changed; 1, and nothing kept,
# when herdr does not list them.
keep_panes() {
  kp_p=$(panes_json "$1") || return 1
  kp_l=
  if [ -f "$1/panes.json" ]; then { IFS= read -r kp_l <"$1/panes.json"; } 2>/dev/null; fi
  [ "$kp_p" = "$kp_l" ] && return 0
  { printf '%s\n' "$kp_p" >"$1/panes.json.tmp.$$" && mv -f "$1/panes.json.tmp.$$" "$1/panes.json"; } 2>/dev/null
}

# Bring a pane forward: an agent's pane by `agent focus`, which brings its
# tab and workspace with it; any other by its workspace, then its tab.
focus_pane() {
  # $1 pane_id, $2 tab_id, $3 workspace_id
  hd agent focus "$1" >/dev/null 2>&1 && return 0
  hd workspace focus "$3" >/dev/null 2>&1
  hd tab focus "$2" >/dev/null 2>&1
}
