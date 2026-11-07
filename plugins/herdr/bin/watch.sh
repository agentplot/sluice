#!/bin/sh
# shellcheck disable=SC1091,SC2154,SC2016 # jq programs; common.sh, sourced, sets root, lib, state, server and config.
# The plugin's one watcher for all of a user's herdr servers on a host
# (0094). `--ensure` registers the server in its environment and wakes the
# lease's watcher where it lives and runs this plugin folder and version,
# else starts one. Running, it finds the repositories every server's panes
# are in, runs each source there, hands every document to document.sh, and
# keeps each server's sidebar and the forecasts current, until it serves
# no server. Between changes it waits on its tick's one sleep, and a tick
# forks nothing unless the shell's own tests find a change (D4, D5).
# `--ticks N` stops it after N ticks, the first at its start (the tests).
set -u
. "$(dirname "$0")/../lib/common.sh"
. "$lib/panes.sh"
. "$lib/sources.sh"
. "$lib/lease.sh"
. "$lib/procs.sh"

lease=$state/watch.lease
tick=${SLUICE_HERDR_TICK:-15}
nl='
'
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
# the server at that moment cannot remove what is written. `due`, written
# after `socket`, is the watcher's marker that the server asked to be read
# (D4).
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
  : >"$server/due"
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
  # The lease's watcher, woken, reads this server's panes at once (D4).
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
  [ -n "${timer:-}" ] && kill "$timer" 2>/dev/null
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
  rm -rf "$1/socket" "$1/herdr" "$1/link" "$1/tokens.json" "$1/seq" "$1/panes.json" "$1/due"
  rmdir "$1" 2>/dev/null
  eval "missed_${1##*/}=0"
}

# What a call made through `as` to the server in folder $1 said of it, by
# its status $2: 3, no answer (tokens.sh's when herdr does not list the
# server's panes), else an answer. One that has not answered twice in a row
# is dropped, under the lock where it can be had, and 1 returned (D2).
answered() {
  an_k=${1##*/}
  if [ "$2" -ne 3 ]; then
    eval "missed_$an_k=0"
    return 0
  fi
  eval "missed_$an_k=\$((\${missed_$an_k:-0} + 1))"
  eval "an_m=\$missed_$an_k"
  [ "$an_m" -ge 2 ] || return 0
  say "the herdr server at $as_s does not answer; no longer served"
  take_lock && an_t=true || an_t=false
  unregister "$1"
  [ "$an_t" = true ] && drop_lock
  return 1
}

# Each server served, tested at a tick by builtins alone: its socket still
# there and still the file its link is. One whose socket is gone or
# another's is dropped under the lock, if it still is, so a server
# registered again at that moment is kept. A server with no socket's path,
# the tests' stand-in run outside herdr, is asked for its panes, since
# nothing else can tell it gone (D2).
check_servers() {
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
    [ -n "$gone_s" ] && continue
    if as "$c" hd pane list >/dev/null 2>&1; then answered "$c" 0; else answered "$c" 3; fi
  done
}

# The watcher lives while it serves a server: once every server's socket
# is gone or another's, or none answers, it stops, with its sources (D2).
enough() {
  for en_f in "$state"/servers/*/socket; do
    [ -f "$en_f" ] && return 0
  done
  say "no herdr server left to serve; stopping"
  exit 0
}

# Set one server's tokens by tokens.sh with its flags: the pane list it
# makes is the call that tells whether the server answers.
tokens_of() {
  tk_d=$1
  shift
  as "$tk_d" sh "$root/bin/tokens.sh" "$@"
  answered "$tk_d" $?
}

# Every server's tokens.
each_tokens() {
  for et_d in "$state"/servers/*; do
    [ -f "$et_d/socket" ] && tokens_of "$et_d" "$@"
  done
  return 0
}

# Whether the server in folder $1 shows a token of the plugin's: a value in
# the one line of its tokens.json that is not empty, read by the shell.
shows() {
  sh_l=
  IFS= read -r sh_l 2>/dev/null <"$1/tokens.json"
  case $sh_l in *':"'[!\"]*) return 0 ;; esac
  return 1
}

# The minute's pass (D5): each server that shows a token has its panes
# listed once and every token shown set again, so none lapses within its
# two minutes and the clocks in them move; a server showing none has none.
minute() {
  for mp_d in "$state"/servers/*; do
    [ -f "$mp_d/socket" ] && shows "$mp_d" && tokens_of "$mp_d" --refresh
  done
  return 0
}

# Wake the watcher from a source's reader, a subshell of it, where $$ is
# still the watcher's pid: only while the lease names that pid, so a reader
# a killed watcher left signals no process the pid was given since.
rouse() {
  ro_p=
  IFS= read -r ro_p 2>/dev/null <"$lease"
  [ "$ro_p" = $$ ] && kill -USR1 $$ 2>/dev/null
  return 0
}

# Run one source in a directory: its documents go to document.sh one by
# one, and its reader wakes the watcher for each document kept; its pids
# are kept so a stop can end it, and its reader marks it ended at its
# input's end, for the next tick to start it again (D4, D7).
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
    } | {
      while IFS= read -r line; do
        printf '%s\n' "$line" | sh "$root/bin/document.sh" "$name" "$where"
        [ -e "$state/dirty" ] && rouse
      done
      { : >"$run.ended"; } 2>/dev/null
      rouse
    }
  ) &
  remember $! "$run.pids"
}

alive() {
  [ -f "$1.pids" ] || return 1
  [ -n "$(own_listed "$1.pids")" ]
}

# Whether a source is to be started: its reader marked it ended when the
# look began, or none of its processes runs.
dead() {
  case $ended in *" ${1##*/} "*) return 0 ;; esac
  ! alive "$1"
}

# Keep the git folder of the worktree at $1, for the ticks to test its head
# against its stamp, made here where it has none (D10). Read by the shell
# from the worktree's `.git`, a folder or a file naming one.
keep_head() {
  kh_g=$1/.git
  if [ -f "$kh_g" ]; then
    kh_l=
    IFS= read -r kh_l 2>/dev/null <"$kh_g"
    kh_g=${kh_l#gitdir: }
    case $kh_g in /*) ;; *) kh_g=$1/$kh_g ;; esac
  fi
  [ -d "$kh_g" ] || return 0
  kh_k=$(sum "$1")
  [ -e "$state/checks/$kh_k.stamp" ] || : >"$state/checks/$kh_k.stamp"
  heads="$heads$kh_g	$kh_k$nl"
}

# Whether a kept worktree's head moved: its HEAD or logs/HEAD no older than
# its stamp, a time equal to the stamp's counting as moved, by the shell's
# `-nt` alone; their keys in moved (D10).
heads_moved() {
  moved=
  [ -n "$heads" ] || return 1
  hm_ifs=$IFS
  IFS=$nl
  set -f
  for hm in $heads; do
    hm_g=${hm%	*}
    hm_s=$state/checks/${hm##*	}.stamp
    [ -e "$hm_g/HEAD" ] || continue
    if ! [ "$hm_s" -nt "$hm_g/HEAD" ] ||
      { [ -e "$hm_g/logs/HEAD" ] && ! [ "$hm_s" -nt "$hm_g/logs/HEAD" ]; }; then
      moved="$moved ${hm##*	}"
    fi
  done
  set +f
  IFS=$hm_ifs
  [ -n "$moved" ]
}

# Sources and repositories, and the worktrees whose heads the ticks test:
# at a server's ensure and its next tick, the popup's asking, and a tick
# after a source ended.
look() {
  cfg=$(read_sources)
  # The sources whose readers marked them ended, those marks taken up.
  ended=' '
  set --
  for f in "$runs"/*.ended; do
    [ -e "$f" ] || continue
    set -- "$@" "$f"
    f=${f##*/}
    ended="$ended${f%.ended} "
  done
  [ $# -gt 0 ] && rm -f "$@"
  # Repositories every server's panes are in, each with a worktree to run
  # in, and those the popup asked for; each worktree's git folder kept.
  listed=$(each panes_json | jq -s -r 'add // [] | [.[] | select(.repo)]
    | (unique_by(.repo)[] | ["r", .repo, .top]), (unique_by(.top)[] | ["h", .top]) | @tsv')
  found=
  heads=
  old_ifs=$IFS
  IFS=$nl
  set -f
  for line in $listed; do
    case $line in
    "r	"*) found="$found${line#r	}$nl" ;;
    "h	"*) keep_head "${line#h	}" ;;
    esac
  done
  set +f
  IFS=$old_ifs
  for w in "$state"/want/*; do
    [ -e "$w" ] || continue
    d=$(cat "$w")
    rm -f "$w"
    r=$(repo_of "$d") && t=$(top_of "$d") && found="$found$r	$t$nl"
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
      dead "$run" && {
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
    IFS=$nl
    for line in $found; do
      [ -n "$line" ] || continue
      r=${line%%	*}
      t=${line#*	}
      run=$runs/$(sum "$name $r")
      dead "$run" || continue
      rm -f "$run.pids"
      # Probed once, and again at a look five minutes on or after a nudge:
      # a source runs only where sluice has records.
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

# Forecast what moved and set each server's tokens: with --refresh, every
# token shown again, so its time to live starts over. The forecast is
# followed by the tokens here, so it marks nothing dirty itself.
changed() {
  rm -f "$state/dirty"
  each sh "$root/bin/forecast.sh"
  each_tokens "$@"
}

# Look for repositories in every server's panes, then forecast and set the
# tokens.
pass() {
  look
  changed "$@"
}

# Whether the lease names this watcher still: one that reads another pid
# there stops with its sources (D1).
mine() {
  read_lease
  [ "$lpid" = $$ ] && return 0
  say "the lease names another watcher (${lpid:-none}); stopping"
  exit 0
}

# Whether an ensure or the popup asked for a look since the last, their
# markers taken up: an ensure's `due` in its server's folder, the popup's
# `want/<sum>`, which the look reads (D4).
asked() {
  ak=false
  for ak_f in "$state"/servers/*/due "$state"/want/*; do
    [ -e "$ak_f" ] && ak=true && break
  done
  [ "$ak" = true ] || return 1
  rm -f "$state"/servers/*/due
}

# A wake (D4): an ensure's server, or the popup's repository, is read now,
# an ensure's again at the next tick; a document kept has the tokens set
# and what moved forecast. A source's end waits for the tick.
wake() {
  mine
  if asked; then
    again=true
    pass
  elif [ -e "$state/dirty" ]; then
    changed
  fi
}

# A tick (D5). The shell's own tests read its folder's manifest, the
# lease, each server's socket, the sources' ends, each worktree's head and
# the markers; it forks for what they found changed and, every fourth
# tick, for the minute's pass.
tock() {
  # Its folder's manifest gone or newer than its lease: the plugin was
  # reinstalled there, or a former version put back. The next server's
  # event starts the watcher of the folder installed (D1).
  if [ ! -e "$root/herdr-plugin.toml" ] || [ "$root/herdr-plugin.toml" -nt "$lease" ]; then
    say "the plugin in $root was replaced or removed; stopping"
    exit 0
  fi
  mine
  if [ "$former" = true ] && { formers_ended || [ "$ticked" -ge 2 ]; }; then clear_formers; fi
  check_servers
  enough
  tk_read=$again
  again=false
  if asked; then
    tk_read=true
    again=true
  fi
  # A moved head's stamp is renewed before its forecast reads it, so a
  # commit after the forecast's read is no older than the stamp.
  if heads_moved; then
    for k in $moved; do : >"$state/checks/$k.stamp"; done
  fi
  tk_all=
  [ "$ticked" -gt 0 ] && [ $((ticked % 4)) -eq 0 ] && tk_all=--refresh
  if [ "$tk_read" = true ]; then
    pass $tk_all
    return
  fi
  tk_ended=false
  for f in "$runs"/*.ended; do
    [ -e "$f" ] && tk_ended=true && break
  done
  [ "$tk_ended" = true ] && look
  if [ -n "$moved" ] || [ -e "$state/dirty" ]; then
    changed $tk_all
  elif [ -n "$tk_all" ]; then
    minute
  fi
}

ticked=0
# The first tick, at its start, reads every server registered, and the
# next reads them again.
again=true
timer=
heads=
while :; do
  tock
  enough
  ticked=$((ticked + 1))
  [ "$ticks" -gt 0 ] && ticks=$((ticks - 1))
  if [ "$ticks" -eq 0 ]; then
    # A bounded run lets its sources finish before it stops; a signal ends
    # the shell's wait, which is taken up again.
    until wait; do :; done
    each_tokens
    break
  fi
  # The tick's one sleep, waited on until it ends: a signal ends the wait
  # at once, the markers say what changed, and the wait is taken up again
  # on the same sleep (D4).
  sleep "$tick" &
  timer=$!
  while :; do
    if [ "$woke" = 1 ]; then
      woke=0
      wake
      enough
      continue
    fi
    wait "$timer"
    [ "$woke" = 1 ] || break
  done
  timer=
done
