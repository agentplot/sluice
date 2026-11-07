#!/bin/sh
# shellcheck disable=SC1091,SC2154,SC2016 # jq programs; common.sh, sourced, sets root, lib, state, server and config.
# The plugin's one watcher for all of a user's herdr servers on a host
# (0094). `--ensure` registers the server in its environment and wakes the
# lease's watcher where it lives and runs this plugin folder and version,
# else starts one. Running, it finds the repositories every server's panes
# are in, runs each source there, hands every document to document.sh, and
# keeps each server's sidebar and the forecasts current, until it serves
# no server. `--ticks N` stops it after N ticks (the tests).
set -u
. "$(dirname "$0")/../lib/common.sh"
. "$lib/panes.sh"
. "$lib/sources.sh"
. "$lib/lease.sh"
. "$lib/procs.sh"

lease=$state/watch.lease
tick=${SLUICE_HERDR_TICK:-15}
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/herdr-plugin.toml")

# The lease, read by the shell alone: the pid of the watcher holding it,
# that process's identity (lib/procs.sh), and the plugin folder and version
# it runs, so a reinstall replaces it (D1). Nothing renews it.
read_lease() {
  lpid=
  lid=
  lroot=
  lversion=
  [ -f "$lease" ] || return 1
  # Each line as written: an identity read from ps may end in a space.
  {
    IFS= read -r lpid
    IFS= read -r lid
    IFS= read -r lroot
    IFS= read -r lversion
  } 2>/dev/null <"$lease"
  [ -n "$lpid" ]
}

# Whether the lease's process lives and is still the one it names. Where
# no process could be told from a later one given its pid when it took the
# lease (no ps, no /proc), its pid alone is read.
held() {
  read_lease || return 1
  case $lpid in *[!0-9]*) return 1 ;; esac
  kill -0 "$lpid" 2>/dev/null || return 1
  [ "$lid" = "pid:$lpid" ] || [ "$(identity "$lpid")" = "$lid" ]
}

# Held by a watcher of this plugin folder and version.
ours() {
  held && [ "$lroot" = "$root" ] && [ "$lversion" = "$version" ]
}

# Register the server in the environment (D2): its socket's path, its
# herdr, and a hard link to its socket where the filesystem allows one,
# which tells a server started later at the same path from it. A server is
# served while its folder holds `socket`. One registered at a socket other
# than its link's is another server, which starts with nothing set, so the
# watcher sets every token in it. Under the lock, so a watcher dropping
# the server at that moment cannot remove what is written.
register() {
  take_lock || return 1
  mkdir -p "$server"
  sock=${HERDR_SOCKET_PATH:-}
  if [ -n "$sock" ] && [ -e "$sock" ] && ! [ "$sock" -ef "$server/link" ]; then
    [ -e "$server/link" ] && rm -f "$server/tokens.json" "$server/panes.json"
    rm -f "$server/link"
    ln "$sock" "$server/link" 2>/dev/null
  fi
  printf '%s\n' "${HERDR_BIN_PATH:-}" >"$server/herdr"
  printf '%s\n' "$sock" >"$server/socket.tmp.$$" && mv -f "$server/socket.tmp.$$" "$server/socket"
  drop_lock
}

# The tab-bar segment runs from the host's state, which outlives a
# reinstall; the plugin's own directory does not. The watcher refreshes the
# copy as it starts, and an ensure leaves it (D12).
# Each file replaced in one step, since herdr may be running it.
copy_segment() {
  mkdir -p "$state/segment/bin" "$state/segment/lib"
  for f in bin/segment.sh lib/common.sh lib/words.jq; do
    cp "$root/$f" "$state/segment/$f.tmp.$$" && mv -f "$state/segment/$f.tmp.$$" "$state/segment/$f"
  done
}

case ${1:-} in
--ensure)
  register
  touch "$state/nudge"
  # The lease's watcher looks at this server's panes within its second.
  if ours && kill -USR1 "$lpid" 2>/dev/null; then exit 0; fi
  nohup sh "$root/bin/watch.sh" </dev/null >>"$state/watch.log" 2>&1 &
  exit 0
  ;;
esac

# An ensure's signal, trapped before the lease can name this watcher.
woke=0
trap 'woke=1' USR1

ticks=-1
[ "${1:-}" = --ticks ] && ticks=$2

# Whether this system lets a process be told apart from one given its pid
# later: not where ps cannot run and there is no /proc (sluice's sandbox on
# macOS, where ps is refused).
me=$(identity $$)
if [ "$me" = "pid:$$" ]; then knowable=false; else knowable=true; fi
unchecked=false

# The server it was started from is served, as an ensure would have it.
register

# Take the lease, unless a live watcher of this folder and version holds
# it. One of another folder or version gives way once it reads the lease
# naming another pid.
take_lock || exit 0
if ours; then
  drop_lock
  exit 0
fi
printf '%s\n%s\n%s\n%s\n' $$ "$me" "$root" "$version" >"$lease.tmp.$$" && mv -f "$lease.tmp.$$" "$lease"
drop_lock
copy_segment
say "watching (pid $$)"
[ "$knowable" = true ] ||
  say "cannot tell a process from a later one given its pid here (no ps, no /proc); the lease names this watcher by its pid alone"

# A source's subshell, this watcher's own child, kept with its identity
# (lib/procs.sh) for a later watcher to check. While this watcher lives it
# needs none: an unreaped child keeps its pid, so kill -0 cannot mistake
# another process for it.
remember() {
  printf '%s\tchild:%s\n' "$1" "$(identity "$1")" >>"$2"
}

# A pid kept alone, at its fork, given its identity; dropped if it has
# ended already, so the pid is never taken for whoever gets it next. Where
# no identity can be read it stays as it was.
settle() {
  if kill -0 "$1" 2>/dev/null; then id=$(identity "$1"); else id=; fi
  [ "$id" = "pid:$1" ] && return 0
  awk -F '\t' -v p="$1" -v id="$id" '
    $1 == p && index($2, "pid:" p ":") == 1 { if (id != "") print p "\t" id; next }
    { print }' "$2" >"$2.tmp.$$" &&
    mv -f "$2.tmp.$$" "$2"
}

# Each process in a dead watcher's pids file that is still the one it
# recorded. Where none can be checked, none is: an unknown pid is never
# ended, and that is said once.
listed() {
  [ "$knowable" = true ] || return 0
  while IFS='	' read -r p id; do
    same_process "$p" "${id#child:}" && echo "$p"
  done <"$1"
}

end_listed() {
  for p in $(listed "$1"); do kill "$p" 2>/dev/null; done
}

unknowable() {
  [ "$knowable" = false ] && [ "$unchecked" = false ] || return 0
  unchecked=true
  say "cannot tell an ended watcher's sources from other processes here (no ps, no /proc); left alone"
}

# This watcher's own: its children by kill -0 alone; a source under one of
# them by its identity, or, where none can be read, while that child lives.
own_listed() {
  child=false
  while IFS='	' read -r p id; do
    case $id in child:*) kill -0 "$p" 2>/dev/null && child=true ;; esac
  done <"$1"
  while IFS='	' read -r p id; do
    case $id in
    child:*) kill -0 "$p" 2>/dev/null && echo "$p" ;;
    *) if same_process "$p" "$id" || { [ "$knowable" = false ] && [ "$child" = true ] && kill -0 "$p" 2>/dev/null; }; then echo "$p"; fi ;;
    esac
  done <"$1"
}

# This watcher's sources, apart from any other's. A watcher killed outright
# never ran its stop: its sources are ended here, and forgotten.
runs=$state/run/$$
for d in "$state"/run/*; do
  [ -d "$d" ] || continue
  p=${d##*/}
  [ "$p" = $$ ] && continue
  kill -0 "$p" 2>/dev/null && continue
  unknowable
  for f in "$d"/*.pids; do
    [ -e "$f" ] && end_listed "$f"
  done
  rm -rf "$d"
done
mkdir -p "$runs"

# A former version's watchers, one for each server, each with its lease
# and state in its server's folder (D11). Their leases go now, each under
# its own lock, so each gives way within its second; once every holder has
# ended, or after two ticks, the sources a killed one listed are ended by
# their identity and each server's former folders go, its watch.log kept.
olds='docs told told.d checks probe want run failed problem nudge dirty no-sha256 probe.err told-unwritable watch.lease lease.lock'
former=false
formers=
for d in "$state"/servers/*; do
  for f in $olds; do [ -e "$d/$f" ] && former=true; done
  [ -f "$d/watch.lease" ] || continue
  p=
  read -r p _ 2>/dev/null <"$d/watch.lease"
  formers="$formers $p"
  host_lock=$lock
  lock=$d/lease.lock
  take_lock && taken=true || taken=false
  rm -f "$d/watch.lease"
  [ "$taken" = true ] && drop_lock
  lock=$host_lock
done

clear_formers() {
  for d in "$state"/servers/*/run/*; do
    [ -d "$d" ] || continue
    unknowable
    for f in "$d"/*.pids; do
      [ -e "$f" ] && end_listed "$f"
    done
  done
  for d in "$state"/servers/*; do
    for f in $olds; do
      [ -e "$d/$f" ] && rm -rf "${d:?}/$f"
    done
  done
  say "a former version's watchers have given way; each server's former state removed, its watch.log kept"
  former=false
}

# Whether every former holder has ended.
formers_ended() {
  for p in $formers; do
    case $p in '' | *[!0-9]*) continue ;; esac
    kill -0 "$p" 2>/dev/null && return 1
  done
  return 0
}

stop() {
  for f in "$runs"/*.pids; do
    [ -e "$f" ] || continue
    for p in $(own_listed "$f"); do kill "$p" 2>/dev/null; done
  done
  rm -rf "$runs"
  if take_lock; then
    read_lease && [ "$lpid" = $$ ] && rm -f "$lease"
    drop_lock
  fi
}
trap 'stop; exit 0' INT TERM
trap stop EXIT

# Run a command as the server registered in folder $1 runs one: with that
# server's socket and herdr in its environment (D2).
as() {
  as_d=$1
  shift
  as_s=
  as_b=
  read -r as_s 2>/dev/null <"$as_d/socket"
  read -r as_b 2>/dev/null <"$as_d/herdr"
  (
    if [ -n "$as_s" ]; then export HERDR_SOCKET_PATH="$as_s"; else unset HERDR_SOCKET_PATH; fi
    if [ -n "$as_b" ]; then export HERDR_BIN_PATH="$as_b"; else unset HERDR_BIN_PATH; fi
    "$@"
  )
}

# Run a command once as each server served.
each() {
  for each_d in "$state"/servers/*; do
    [ -f "$each_d/socket" ] && as "$each_d" "$@"
  done
  return 0
}

# Whether a server's socket is gone, or is a file other than its link's.
gone() {
  gone_s=
  read -r gone_s 2>/dev/null <"$1/socket"
  [ -n "$gone_s" ] || return 1
  [ ! -e "$gone_s" ] || { [ -e "$1/link" ] && ! [ "$gone_s" -ef "$1/link" ]; }
}

# A server no longer served: its registration goes, with what was set in it
# and its panes; a former version's watch.log stays.
unregister() {
  rm -rf "$1/socket" "$1/herdr" "$1/link" "$1/tokens.json" "$1/seq" "$1/panes.json"
  rmdir "$1" 2>/dev/null
  eval "missed_${1##*/}=0"
}

# Each server served, tested at a tick: its socket still there and still
# the file its link is, by builtins, and its herdr answering. One whose
# socket is gone or another's is dropped under the lock, if it still is,
# so a server registered again at that moment is kept; one whose calls
# failed twice in a row is dropped. Leaves in served how many remain.
check_servers() {
  served=0
  for c in "$state"/servers/*; do
    [ -f "$c/socket" ] || continue
    if gone "$c" && take_lock; then
      if gone "$c"; then
        say "the herdr server at $gone_s has stopped; no longer served"
        unregister "$c"
        drop_lock
        continue
      fi
      drop_lock
    fi
    k=${c##*/}
    if as "$c" hd pane list >/dev/null 2>&1; then
      eval "missed_$k=0"
    else
      eval "missed_$k=\$((\${missed_$k:-0} + 1))"
      eval "m=\$missed_$k"
      if [ "$m" -ge 2 ]; then
        say "the herdr server at $as_s does not answer; no longer served"
        take_lock && taken=true || taken=false
        unregister "$c"
        [ "$taken" = true ] && drop_lock
        continue
      fi
    fi
    served=$((served + 1))
  done
}

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
      # Its pid at once, so a stop in the next second still ends it; its
      # identity once it has become the source's command, since where only
      # the command line tells processes apart, the fork's would not match.
      group=$(exec sh -c 'echo $PPID')
      printf '%s\tpid:%s:%s\n' "$inner" "$inner" "$group" >>"$run.pids"
      (
        sleep "${SLUICE_HERDR_IDENTITY_DELAY:-1}"
        settle "$inner" "$run.pids"
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
  [ -n "$(own_listed "$1.pids")" ]
}

# Sources and repositories, every tick.
look() {
  cfg=$(read_sources)
  # Repositories every server's panes are in, each with a worktree to run
  # in, and those the popup asked for.
  found=$(each panes_json | jq -s -r 'add // [] | [.[] | select(.repo)] | unique_by(.repo)[] | [.repo, .top] | @tsv')
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

# Look for repositories in every server's panes, forecast what moved, and
# set each server's tokens: with --refresh, every token again, so its time
# to live starts over.
pass() {
  look
  each sh "$root/bin/forecast.sh"
  rm -f "$state/dirty"
  each sh "$root/bin/tokens.sh" "$@"
}

ticked=0
while [ "$ticks" -ne 0 ]; do
  # Its folder's manifest gone or newer than its lease: the plugin was
  # reinstalled there, or a former version put back. The next server's
  # event starts the watcher of the folder installed (D1).
  if [ ! -e "$root/herdr-plugin.toml" ] || [ "$root/herdr-plugin.toml" -nt "$lease" ]; then
    say "the plugin in $root was replaced or removed; stopping"
    exit 0
  fi
  if [ "$former" = true ] && { formers_ended || [ "$ticked" -ge 2 ]; }; then clear_formers; fi
  # The watcher lives while it serves a server: once every server's socket
  # is gone or another's, or its herdr has not answered twice in a row, it
  # stops, with its sources (D2).
  check_servers
  if [ "$served" -eq 0 ]; then
    say "no herdr server left to serve; stopping"
    exit 0
  fi
  pass --refresh
  ticked=$((ticked + 1))
  [ "$ticks" -gt 0 ] && ticks=$((ticks - 1))
  if [ "$ticks" -eq 0 ]; then
    # A bounded run lets its sources finish before it stops; a signal ends
    # the shell's wait, which is taken up again.
    until wait; do :; done
    each sh "$root/bin/tokens.sh"
    break
  fi
  # Between ticks, the sidebar follows each document within a second, and
  # an ensure's signal has every server's panes looked at.
  sec=0
  while [ "$sec" -lt "$tick" ]; do
    sleep 1
    sec=$((sec + 1))
    read_lease
    if [ "$lpid" != $$ ]; then
      say "the lease names another watcher (${lpid:-none}); stopping"
      exit 0
    fi
    if [ "$woke" = 1 ]; then
      woke=0
      pass
    elif [ -f "$state/dirty" ]; then
      rm -f "$state/dirty"
      each sh "$root/bin/tokens.sh"
    fi
  done
done
