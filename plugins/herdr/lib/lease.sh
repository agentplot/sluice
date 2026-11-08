# shellcheck shell=sh disable=SC2154 # common.sh sets state.
# The lock that taking and releasing the watcher's lease, registering and
# dropping a server, and keeping what was set in one, are done under: a
# directory holding its owner's
# pid and when it took it. A lock is held for a moment only, so one whose
# owner is gone, that has had no owner for three seconds, or that is ten
# seconds old, is broken. Sourced; `lock` may be pointed at another lock
# of the same kind, a former version's, for a moment.

lock=$state/lease.lock

# Break the lock if it is still the one judged stale (its pid file read as
# $1): moved aside in one step, so of two breakers only one succeeds, and
# put back if what was moved is a lock taken since.
break_lock() {
  aside=$lock.broken.$$
  mv "$lock" "$aside" 2>/dev/null || return 0
  was=
  read -r was _ 2>/dev/null <"$aside/pid"
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
  tries=0
  rounds=0
  while ! mkdir "$lock" 2>/dev/null; do
    # Its state directory gone, or the lock fought over: give up.
    [ -d "$state" ] || return 1
    rounds=$((rounds + 1))
    [ "$rounds" -le 15 ] || return 1
    owner=
    since=
    read -r owner since 2>/dev/null <"$lock/pid"
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
  echo "$$ $(date +%s)" >"$lock/pid"
}

drop_lock() {
  owner=
  read -r owner _ 2>/dev/null <"$lock/pid"
  [ "$owner" = $$ ] && rm -rf "$lock"
  return 0
}
