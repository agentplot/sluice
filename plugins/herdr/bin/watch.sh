#!/bin/sh
# shellcheck disable=SC1091,SC2154,SC2016 # jq programs; common.sh, sourced, sets root, lib, state and config.
# The plugin's one watcher. `--ensure` starts it unless a watcher holds a
# lease renewed in the last 60 s, and nudges it to look for repositories.
# Running, it finds the repositories the panes are in, runs each source
# there, hands every document to document.sh, and keeps the sidebar and
# the forecasts current. `--ticks N` stops it after N ticks (the tests).
set -u
. "$(dirname "$0")/../lib/common.sh"
. "$lib/panes.sh"
. "$lib/sources.sh"
. "$lib/lease.sh"

lease=$state/watch.lease
tick=${SLUICE_HERDR_TICK:-15}
# A lease is fresh for this long, and renewed four times as often.
term=${SLUICE_HERDR_LEASE:-60}
every=$((term / 4))
[ "$every" -ge 1 ] || every=1

fresh() {
  [ -f "$lease" ] || return 1
  read -r pid at <"$lease" || return 1
  [ $(($(date +%s) - at)) -lt "$term" ] && kill -0 "$pid" 2>/dev/null
}

holder() {
  cut -d' ' -f1 "$lease" 2>/dev/null
}

write_lease() {
  echo "$$ $(date +%s)" >"$lease.tmp.$$" && mv -f "$lease.tmp.$$" "$lease"
}

# The tab-bar segment runs from the state directory, which outlives a
# reinstall; the plugin's own directory does not.
copy_segment() {
  mkdir -p "$state/segment/bin" "$state/segment/lib"
  cp "$root/bin/segment.sh" "$state/segment/bin/segment.sh"
  cp "$lib/common.sh" "$lib/words.jq" "$state/segment/lib/"
}

case ${1:-} in
--ensure)
  touch "$state/nudge"
  fresh && exit 0
  copy_segment
  nohup sh "$root/bin/watch.sh" </dev/null >>"$state/watch.log" 2>&1 &
  exit 0
  ;;
esac

ticks=-1
[ "${1:-}" = --ticks ] && ticks=$2

# Take the lease, unless another watcher holds it.
take_lock || exit 0
if fresh; then
  drop_lock
  exit 0
fi
write_lease
drop_lock
copy_segment
say "watching (pid $$)"

# The lease is renewed on its own clock, however long a tick takes, for as
# long as this watcher lives and holds it.
(
  while sleep "$every" && kill -0 $$ 2>/dev/null; do
    take_lock || continue
    if [ "$(holder)" = $$ ]; then write_lease; else
      drop_lock
      exit 0
    fi
    drop_lock
  done
) &
renewer=$!

# This watcher's sources, apart from any other's.
runs=$state/run/$$
mkdir -p "$runs"

stop() {
  kill "$renewer" 2>/dev/null
  for f in "$runs"/*.pids; do
    [ -e "$f" ] || continue
    # shellcheck disable=SC2046
    kill $(cat "$f") 2>/dev/null
  done
  rm -rf "$runs"
  if take_lock; then
    [ "$(holder)" = $$ ] && rm -f "$lease"
    drop_lock
  fi
}
trap 'stop; exit 0' INT TERM
trap stop EXIT

# Run one source in a directory: its documents go to document.sh one by
# one; its pids are kept so the next tick can tell it ended.
start() {
  # $1 run file, $2 source name, $3 directory ("" for a source run once), $4 argv as JSON
  run=$1
  name=$2
  where=$3
  argv=$4
  (
    {
      (
        cd "${where:-$config}" || exit 1
        eval "set -- $(printf '%s' "$argv" | jq -r 'map(@sh) | join(" ")')"
        exec "$@"
      ) &
      echo $! >>"$run.pids"
      wait
    } | while IFS= read -r line; do
      printf '%s\n' "$line" | sh "$root/bin/document.sh" "$name" "$where"
    done
  ) &
  echo $! >>"$run.pids"
}

alive() {
  [ -f "$1.pids" ] || return 1
  while read -r p; do
    kill -0 "$p" 2>/dev/null && return 0
  done <"$1.pids"
  return 1
}

# Sources and repositories, every tick.
look() {
  cfg=$(read_sources)
  # Repositories the panes are in, each with a worktree to run in, and
  # those the popup asked for.
  found=$(panes_json | jq -r '[.[] | select(.repo)] | unique_by(.repo)[] | [.repo, .top] | @tsv')
  for w in "$state"/want/*; do
    [ -e "$w" ] || continue
    d=$(cat "$w")
    rm -f "$w"
    r=$(repo_of "$d") && t=$(top_of "$d") && found=$(printf '%s\n%s\t%s' "$found" "$r" "$t")
  done
  n=$(printf '%s' "$cfg" | jq '.sources | length')
  i=0
  while [ "$i" -lt "$n" ]; do
    s=$(printf '%s' "$cfg" | jq -c ".sources[$i]")
    i=$((i + 1))
    name=$(printf '%s' "$s" | jq -r .name)
    argv=$(printf '%s' "$s" | jq -c .command)
    if [ "$(printf '%s' "$s" | jq -r .each_repository)" != true ]; then
      run=$runs/$(sum "$name")
      alive "$run" || {
        rm -f "$run.pids"
        start "$run" "$name" "" "$argv"
      }
      continue
    fi
    old_ifs=$IFS
    IFS='
'
    for line in $found; do
      [ -n "$line" ] || continue
      r=${line%%	*}
      t=${line#*	}
      run=$runs/$(sum "$name $r")
      alive "$run" && continue
      rm -f "$run.pids"
      # Probed once, and again after five minutes or a nudge: a source runs
      # only where sluice has records.
      probe=$state/probe/$(sum "$r")
      if [ -f "$probe" ] && [ "$(cut -d' ' -f1 "$probe")" = none ]; then
        since=$(($(now) - $(cut -d' ' -f2 "$probe")))
        [ "$since" -lt 300 ] && [ ! "$state/nudge" -nt "$probe" ] && continue
      fi
      if got=$(cd "$t" && sluice overview --json 2>/dev/null) &&
        [ "$(printf '%s' "$got" | jq '(.branches | length) + (.sessions | length)' 2>/dev/null)" -gt 0 ] 2>/dev/null; then
        echo "records $(now)" >"$probe"
        start "$run" "$name" "$t" "$argv"
      else
        echo "none $(now)" >"$probe"
      fi
    done
    IFS=$old_ifs
  done
}

i=0
gone=0
while [ "$ticks" -ne 0 ]; do
  # The watcher lives as long as its herdr server: two ticks without an
  # answer and it stops, with its sources.
  if hd pane list >/dev/null 2>&1; then gone=0; else gone=$((gone + 1)); fi
  if [ "$gone" -ge 2 ]; then
    say "herdr does not answer; stopping"
    exit 0
  fi
  look
  sh "$root/bin/forecast.sh"
  sh "$root/bin/tokens.sh" --refresh
  rm -f "$state/dirty"
  [ "$ticks" -gt 0 ] && ticks=$((ticks - 1))
  if [ "$ticks" -eq 0 ]; then
    # A bounded run lets its sources finish before it stops.
    kill "$renewer" 2>/dev/null
    wait
    sh "$root/bin/tokens.sh"
    break
  fi
  # Between ticks, the sidebar follows each document within a second.
  i=0
  while [ "$i" -lt "$tick" ]; do
    sleep 1
    i=$((i + 1))
    [ "$(holder)" = $$ ] || exit 0
    if [ -f "$state/dirty" ]; then
      rm -f "$state/dirty"
      sh "$root/bin/tokens.sh"
    fi
    [ -n "$(ls "$state/want" 2>/dev/null)" ] && break
  done
done
