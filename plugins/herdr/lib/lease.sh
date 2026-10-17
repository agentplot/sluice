# shellcheck shell=sh disable=SC2154 # common.sh sets state.
# The lock every change to the watcher's lease is made under: a directory
# holding its owner's pid and when it took it. A lock is held for a moment
# only, so one whose owner is gone, that has had no owner for three
# seconds, or that is ten seconds old, is broken. Sourced.

lock=$state/lease.lock

# Take the lock; 1 when it cannot be had.
take_lock() {
  tries=0
  rounds=0
  while ! mkdir "$lock" 2>/dev/null; do
    # Its state directory gone, or the lock fought over: give up.
    [ -d "$state" ] || return 1
    rounds=$((rounds + 1))
    [ "$rounds" -le 10 ] || return 1
    owner=
    since=
    read -r owner since <"$lock/pid" 2>/dev/null
    if [ -n "$owner" ]; then
      if ! kill -0 "$owner" 2>/dev/null || [ $(($(date +%s) - ${since:-0})) -ge 10 ]; then
        rm -rf "$lock"
        continue
      fi
    else
      tries=$((tries + 1))
      if [ "$tries" -ge 3 ]; then
        # One died between taking it and writing its pid.
        rm -rf "$lock"
        tries=0
        continue
      fi
    fi
    sleep 1
  done
  echo "$$ $(date +%s)" >"$lock/pid"
}

drop_lock() {
  [ "$(cut -d' ' -f1 "$lock/pid" 2>/dev/null)" = $$ ] && rm -rf "$lock"
  return 0
}
