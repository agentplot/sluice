#!/bin/sh
# shellcheck disable=SC1091,SC2154,SC2016 # jq programs; common.sh, sourced, sets root, lib, base, state and config.
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
. "$lib/procs.sh"

lease=$state/watch.lease
tick=${SLUICE_HERDR_TICK:-15}
# A lease is fresh for this long, and renewed four times as often.
term=${SLUICE_HERDR_LEASE:-60}
every=$((term / 4))
[ "$every" -ge 1 ] || every=1

# The lease: the watcher's pid and when it last renewed, then the plugin
# directory and version it runs, so a reinstall replaces it.
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/herdr-plugin.toml")

fresh() {
  [ -f "$lease" ] || return 1
  read -r pid at <"$lease" || return 1
  [ $(($(date +%s) - at)) -lt "$term" ] && kill -0 "$pid" 2>/dev/null
}

# Fresh, and held by a watcher of this plugin directory and version.
ours() {
  fresh && [ "$(sed -n 2p "$lease")" = "$root" ] && [ "$(sed -n 3p "$lease")" = "$version" ]
}

holder() {
  head -1 "$lease" 2>/dev/null | cut -d' ' -f1
}

write_lease() {
  printf '%s %s\n%s\n%s\n' $$ "$(date +%s)" "$root" "$version" >"$lease.tmp.$$" && mv -f "$lease.tmp.$$" "$lease"
}

# The tab-bar segment runs from the state directory, which outlives a
# reinstall; the plugin's own directory does not.
# Each file replaced in one step, since herdr may be running it.
copy_segment() {
  mkdir -p "$base/segment/bin" "$base/segment/lib"
  for f in bin/segment.sh lib/common.sh lib/words.jq; do
    cp "$root/$f" "$base/segment/$f.tmp.$$" && mv -f "$base/segment/$f.tmp.$$" "$base/segment/$f"
  done
}

case ${1:-} in
--ensure)
  touch "$state/nudge"
  # A pane just made gets its sidebar line within a second, not a tick.
  touch "$state/dirty"
  copy_segment
  ours && exit 0
  nohup sh "$root/bin/watch.sh" </dev/null >>"$state/watch.log" 2>&1 &
  exit 0
  ;;
esac

ticks=-1
[ "${1:-}" = --ticks ] && ticks=$2

# Take the lease, unless another watcher holds it.
take_lock || exit 0
if ours; then
  drop_lock
  exit 0
fi
# Another plugin directory or version's watcher gives way when it sees the
# lease is no longer its own.
write_lease
drop_lock
copy_segment
say "watching (pid $$)"

# The lease is renewed on its own clock, however long a tick takes, for as
# long as this watcher lives and holds it.
(
  # Its own pid, so a renewer killed holding the lock frees it.
  # shellcheck disable=SC2034 # read by take_lock and drop_lock
  lock_owner=$(exec sh -c 'echo $PPID')
  while sleep "$every" && kill -0 $$ 2>/dev/null; do
    take_lock || continue
    # The tests hold it a while, to catch the renewer holding it.
    [ -n "${SLUICE_HERDR_LOCK_HOLD:-}" ] && sleep "$SLUICE_HERDR_LOCK_HOLD"
    if [ "$(holder)" = $$ ]; then write_lease; else
      drop_lock
      exit 0
    fi
    drop_lock
  done
) &
renewer=$!

# A source's process is kept with its identity (lib/procs.sh).
remember() {
  printf '%s\t%s\n' "$1" "$(identity "$1")" >>"$2"
}

# Each process listed in a pids file that is still the one it recorded.
listed() {
  while IFS='	' read -r p id; do
    same_process "$p" "$id" && echo "$p"
  done <"$1"
}

end_listed() {
  for p in $(listed "$1"); do kill "$p" 2>/dev/null; done
}

# This watcher's sources, apart from any other's. A watcher killed outright
# never ran its stop: its sources are ended here, and forgotten.
runs=$state/run/$$
for d in "$state"/run/*; do
  [ -d "$d" ] || continue
  p=${d##*/}
  [ "$p" = $$ ] && continue
  kill -0 "$p" 2>/dev/null && continue
  for f in "$d"/*.pids; do
    [ -e "$f" ] && end_listed "$f"
  done
  rm -rf "$d"
done
mkdir -p "$runs"

stop() {
  # Reaped as well as killed: dash leaves a zombie that kill -0 still finds,
  # and the lock it may hold would wait out its ten seconds.
  kill "$renewer" 2>/dev/null
  wait "$renewer" 2>/dev/null
  for f in "$runs"/*.pids; do
    [ -e "$f" ] && end_listed "$f"
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
      inner=$!
      # Its identity once it has become the source's command: where only
      # the command line tells processes apart, the fork's would not match.
      (
        sleep 1
        remember "$inner" "$run.pids"
      ) &
      wait "$inner"
    } | while IFS= read -r line; do
      printf '%s\n' "$line" | sh "$root/bin/document.sh" "$name" "$where"
    done
  ) &
  remember $! "$run.pids"
}

alive() {
  [ -f "$1.pids" ] || return 1
  [ -n "$(listed "$1.pids")" ]
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
    if ! command -v sluice >/dev/null 2>&1; then
      set_problem "$(sluice_problem)"
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
      if got=$(cd "$t" && sluice overview --json 2>"$state/probe.err"); then
        set_problem ""
        if [ "$(printf '%s' "$got" | jq '((.branches // []) | length) + ((.sessions // []) | length)' 2>/dev/null)" -gt 0 ] 2>/dev/null; then
          echo "records $(now)" >"$probe"
          start "$run" "$name" "$t" "$argv"
          continue
        fi
      elif grep -q "unrecognized subcommand" "$state/probe.err"; then
        set_problem "$(sluice --version 2>/dev/null | head -1) has no overview: the plugin needs sluice 0.12 or later."
      fi
      echo "none $(now)" >"$probe"
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
