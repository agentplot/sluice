# shellcheck shell=sh disable=SC2154 # common.sh sets state.
# The lock every change to the watcher's lease is made under: a directory
# holding its owner's pid and when it took it. A lock is held for a moment
# only, so one whose owner is gone, that has had no owner for three
# seconds, or that is ten seconds old, is broken. A subshell that takes it
# sets lock_owner to its own pid first, so its death frees the lock. Sourced.

lock=$state/lease.lock

# Break the lock if it is still the one judged stale (its pid file read as
# $1): moved aside in one step, so of two breakers only one succeeds, and
# put back if what was moved is a lock taken since.
break_lock() {
  aside=$lock.broken.$$
  mv "$lock" "$aside" 2>/dev/null || return 0
  was=
  read -r was _ <"$aside/pid" 2>/dev/null
  if [ "$was" = "$1" ]; then
    rm -rf "$aside"
  elif [ ! -e "$lock" ]; then
    mv "$aside" "$lock" 2>/dev/null || rm -rf "$aside"
  else
    rm -rf "$aside"
  fi
}

# Take the lock; 1 when it cannot be had.
take_lock() {
  me=${lock_owner:-$$}
  tries=0
  rounds=0
  while ! mkdir "$lock" 2>/dev/null; do
    # Its state directory gone, or the lock fought over: give up.
    [ -d "$state" ] || return 1
    rounds=$((rounds + 1))
    [ "$rounds" -le 15 ] || return 1
    owner=
    since=
    read -r owner since <"$lock/pid" 2>/dev/null
    if [ -n "$owner" ]; then
      if ! kill -0 "$owner" 2>/dev/null || [ $(($(date +%s) - ${since:-0})) -ge 10 ]; then
        break_lock "$owner"
        continue
      fi
    else
      tries=$((tries + 1))
      if [ "$tries" -ge 3 ]; then
        # One died between taking it and writing its pid.
        break_lock ""
        tries=0
        continue
      fi
    fi
    sleep 1
  done
  echo "$me $(date +%s)" >"$lock/pid"
}

holds_lock() {
  [ "$(cut -d' ' -f1 "$lock/pid" 2>/dev/null)" = "${lock_owner:-$$}" ]
}

drop_lock() {
  [ "$(cut -d' ' -f1 "$lock/pid" 2>/dev/null)" = "${lock_owner:-$$}" ] && rm -rf "$lock"
  return 0
}
