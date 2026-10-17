#!/bin/sh
# shellcheck disable=SC1091,SC2154,SC2016 # jq programs; common.sh, sourced, sets root, lib, state and config.
# The sidebar: the `sluice` token on every pane whose worktree has
# something to say, `sluice_q` on every workspace with a queue. Reports
# only what changed, unless --refresh, which reports every token again so
# its time to live starts over.
set -u
. "$(dirname "$0")/../lib/common.sh"
. "$lib/panes.sh"

refresh=false
[ "${1:-}" = --refresh ] && refresh=true
ttl=${SLUICE_HERDR_TTL_MS:-120000}
t=$(now)
panes=$(panes_json)
docs=$(all_docs)

# Each document's learned submissions and blamed tests, by repository.
extra='{}'
for f in "$state"/docs/*.json; do
  [ -e "$f" ] || continue
  k=$(basename "$f" .json)
  p=$(jq -c 'objects' "$state/docs/$k.paths" 2>/dev/null)
  [ -n "$p" ] || p='{}'
  fl='{}'
  for g in "$state/failed/$k"-*; do
    [ -e "$g" ] || continue
    fl=$(printf '%s' "$fl" | jq -c --arg i "${g##*-}" --slurpfile v "$g" '. + {($i): ($v[0] | arrays)}' 2>/dev/null) || fl='{}'
  done
  extra=$(printf '%s' "$extra" | jq -c --arg r "$(jq -r .repository "$f")" --argjson p "$p" --argjson fl "$fl" \
    '.[$r] = {paths: ((.[$r].paths // {}) + $p), failed: ((.[$r].failed // {}) + $fl)}')
done

checks='{}'
for c in "$state"/checks/*; do
  [ -e "$c" ] || continue
  checks=$(printf '%s' "$checks" | jq -c --slurpfile v "$c" '. + {($v[0].path): $v[0]}')
done

want=$(jqw -n -c --argjson panes "$panes" --argjson docs "$docs" --argjson extra "$extra" \
  --argjson checks "$checks" --argjson now "$t" 'include "words";
  { panes: [ $panes[] | select(.top) | . as $p
      | ($docs | map(select(.repository == $p.repo))) as $d
      | {id: .pane_id, v: (if ($d | length) == 0 then ""
          else $d | pane_token($p.top; $extra[$p.repo].paths // {}; $extra[$p.repo].failed // {}; $checks[$p.top]; $now) end)} ],
    spaces: [ $panes | group_by(.workspace_id)[] | .[0].workspace_id as $w
      | (map(.repo) | unique) as $repos
      | {id: $w, v: ($docs | map(select(.repository as $r | $repos | index($r))) | queue_token)} ] }')

last=$state/tokens.json
[ -f "$last" ] || echo '{"panes":{},"spaces":{}}' >"$last"
seq_file=$state/seq
n=$(($(cat "$seq_file" 2>/dev/null || echo 0) + 1))
echo "$n" >"$seq_file"

report() {
  # $1 pane|workspace, $2 id, $3 token name, $4 value
  if [ -n "$4" ]; then
    hd "$1" report-metadata "$2" --source agentplot.sluice --seq "$n" --ttl-ms "$ttl" --token "$3=$4" >/dev/null 2>&1
  else
    hd "$1" report-metadata "$2" --source agentplot.sluice --seq "$n" --clear-token "$3" >/dev/null 2>&1
  fi
}

changes=$(jq -n -c -r --argjson want "$want" --slurpfile last "$last" --argjson all "$refresh" '
  ($last[0]) as $l
  | ($want.panes[] | select(($l.panes[.id] // "") != .v or ($all and .v != "")) | ["pane", .id, "sluice", .v]),
    ($want.spaces[] | select(($l.spaces[.id] // "") != .v or ($all and .v != "")) | ["workspace", .id, "sluice_q", .v])
  | @tsv')
old_ifs=$IFS
IFS='
'
for line in $changes; do
  IFS='	' read -r kind id name value <<EOF
$line
EOF
  report "$kind" "$id" "$name" "$value"
done
IFS=$old_ifs
jq -n -c --argjson want "$want" '{panes: ($want.panes | map({(.id): .v}) | add // {}), spaces: ($want.spaces | map({(.id): .v}) | add // {})}' | put "$last"
