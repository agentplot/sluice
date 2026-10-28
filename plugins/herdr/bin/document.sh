#!/bin/sh
# shellcheck disable=SC1091,SC2154,SC2016 # jq programs; common.sh, sourced, sets root, lib, state and config.
# One sluice/overview@1 document in, on stdin, from the source named $1
# run in the directory $2 (empty for a source run once): keep it as its
# repository's latest, learn its submissions' worktrees, and raise a toast
# for each landing, blame or conflict not yet told.
set -u
. "$(dirname "$0")/../lib/common.sh"
. "$lib/panes.sh"

source_name=${1:-local}
dir=${2:-}
# Read as sluice/overview@1 with every list it may leave out made empty;
# anything else is logged and the last good state kept. What a held
# place's release printed is kept to its last line with anything on it,
# and each run of control characters in a subject made one space (26),
# before control characters go, which would join its lines into one.
doc=$(jq -c -e 'def last_line: if type == "string"
      then ([splits("\n") | select(test("\\S"))] | last // "") | sub("^\\s+"; "") | sub("\\s+$"; "")
      else "" end;
  def spaced: if type == "string" then gsub("\\p{Cc}+"; " ") else . end;
  def release_failed: if (.release_failed | type) == "object" then .release_failed.failed |= last_line else del(.release_failed) end;
  select(.schema == "sluice/overview@1" and (.host | type) == "string"
    and (.repository | type) == "string" and ((.branches // []) | type) == "array")
  | .branches = [(.branches // [])[] | select(type == "object" and (.branch | type) == "string")
      | .depth //= 0 | .recent = [(.recent // [])[] | objects]
      | .submissions = [(.submissions // [])[] | objects | select(.id != null) | .from //= {}
          | if has("subject") then .subject |= spaced else . end]
      | if (.recent_subjects | type) == "object" then .recent_subjects |= map_values(spaced) else . end
      | .candidates = [(.candidates // [])[] | objects | .submissions //= [] | .targets //= [] | .failed //= []
          | .tasks = [(.tasks // [])[] | objects | select((.target | type) == "string")]]
      | .held_checkouts = [(.held_checkouts // [])[] | objects | select((.place | type) == "string") | .failed |= last_line]]
  | .sessions = [(.sessions // [])[] | objects | select((.worktree | type) == "string")
      | if (.run | type) == "object" then .run.tasks = [(.run.tasks // [])[] | objects | select((.target | type) == "string")] else del(.run) end
      | if (.last_result | type) == "object" then .last_result.tasks = [(.last_result.tasks // [])[] | objects | select((.target | type) == "string")] else del(.last_result) end
      | release_failed]
  | if (.room | type) == "object" and (.room.bound | type) == "number" then . else del(.room) end
  | .previews = [(.previews // [])[] | if type == "object" then release_failed else . end]
  # Nothing a document says is shown with its control characters: a task or
  # path holding an escape would reach the terminal as one.
  | walk(if type == "string" then gsub("[[:cntrl:]]"; "") else . end)' 2>/dev/null) || {
  say "$source_name printed a line it cannot read as an overview document; keeping the last good one"
  exit 0
}
host=$(printf '%s' "$doc" | jq -r .host)
repository=$(printf '%s' "$doc" | jq -r .repository)
key=$(key_of "$host" "$repository")
printf '%s\n' "$doc" | put "$state/docs/$key.json"
[ -n "$dir" ] && printf '%s\n' "$dir" | put "$state/docs/$key.dir"

# Each submission's worktree and branch, kept while its id is in the
# document; a finished one leaves submissions[] but stays in recent[].
paths=$state/docs/$key.paths
jq -e 'type == "object"' "$paths" >/dev/null 2>&1 || echo '{}' >"$paths"
learned=$(jq -c -e --argjson d "$doc" '
  ([$d.branches[].recent[] | (.id // empty), (.ids // [])[]] | map(tostring)) as $recent
  | (. + ([$d.branches[] | .branch as $b | .submissions[] | select(.from.path)
          | {(.id | tostring): {path: .from.path, branch: $b}}] | add // {}))
  | with_entries(select(.key as $k | ($recent | index($k)) or
      ([$d.branches[].submissions[].id | tostring] | index($k))))' "$paths") &&
  printf '%s\n' "$learned" | put "$paths"

# A submission's worktree and blamed tests, read once from sluice when
# the document did not carry them.
learn() {
  id=$1
  known=$(jq -r --arg i "$id" '.[$i].path // empty' "$paths")
  if [ -n "$known" ] && [ -f "$state/failed/$key-$id" ]; then return; fi
  [ -n "$dir" ] || return 0
  got=$(cd "$dir" && sluice status "$id" --json 2>/dev/null) || return 0
  printf '%s' "$got" | jq -c '[.failed // [] | .[].task]' | put "$state/failed/$key-$id"
  p=$(printf '%s' "$got" | jq -r '.from.path // empty')
  if [ -n "$p" ]; then
    jq -c --arg i "$id" --arg p "$p" --arg b "$(printf '%s' "$got" | jq -r .branch)" \
      '. + {($i): {path: $p, branch: $b}}' "$paths" | put "$paths"
  fi
}

# An event is told once: making its directory is the test, so two
# watchers never both toast it. The directory is named by a SHA-256 of the
# event's branch, kind, submissions and time, and kept only while the event
# is still among the recent.
told=$state/told.d/$key
mkdir -p "$state/told.d"
events=$(printf '%s' "$doc" | jqw -c 'include "words"; events')
names=$(printf '%s\n' "$events" | jq -r 'select(. != null) | .key' | while IFS= read -r k; do digest "$k"; done)
forget_old() {
  for e in "$told"/*; do
    [ -d "$e" ] || continue
    printf '%s\n' "$names" | grep -qxF "${e##*/}" || rmdir "$e" 2>/dev/null
  done
}
if mkdir "$told" 2>/dev/null; then
  # First sight of the repository: what is already in recent[] is old.
  for n in $names; do
    mkdir "$told/$n" 2>/dev/null
  done
  touch "$state/dirty"
  exit 0
fi

panes=
old_ifs=$IFS
IFS='
'
for e in $events; do
  n=$(digest "$(printf '%s' "$e" | jq -r .key)")
  if ! mkdir "$told/$n" 2>/dev/null; then
    [ -d "$told/$n" ] && continue
    # Not told when it cannot be kept as told, or every document would
    # tell it again: a toast missed is better than a stream of them. Said
    # once, until it can be kept again.
    if [ ! -f "$state/told-unwritable" ]; then
      : >"$state/told-unwritable"
      say "cannot record events as told in $told: no toasts until it can be written"
    fi
    continue
  fi
  rm -f "$state/told-unwritable"
  branch=$(printf '%s' "$e" | jq -r .branch)
  kind=$(printf '%s' "$e" | jq -r .event.event)
  case $kind in
  blamed)
    id=$(printf '%s' "$e" | jq -r .event.id)
    learn "$id"
    tests=$(jq -r 'join(", ")' "$state/failed/$key-$id" 2>/dev/null)
    hd notification show "sluice: #$id blamed on $branch" \
      --body "${tests:+$tests failed. }Run jump to go to its pane." >/dev/null 2>&1
    ;;
  conflict)
    id=$(printf '%s' "$e" | jq -r .event.id)
    learn "$id"
    hd notification show "sluice: #$id conflicts on $branch" \
      --body "$(printf '%s' "$e" | jq -r '.event.paths // [] | join(", ")')" >/dev/null 2>&1
    ;;
  landed)
    [ -n "$panes" ] || panes=$(panes_json)
    for id in $(printf '%s' "$e" | jq -r '.event.ids[]'); do
      learn "$id"
      p=$(jq -r --arg i "$id" '.[$i].path // empty' "$paths")
      [ -n "$p" ] || continue
      # Toasted only when its pane is open and not the one in front.
      away=$(printf '%s' "$panes" | jq -r --arg p "$p" '
        [.[] | select(.top == $p)] | if length == 0 then "none" elif any(.front) then "front" else "away" end')
      [ "$away" = away ] || continue
      hd notification show "sluice: #$id landed on $branch" \
        --body "as $(printf '%s' "$e" | jq -r '.event.to[0:7]')" >/dev/null 2>&1
    done
    ;;
  esac
done
IFS=$old_ifs
forget_old
touch "$state/dirty"
