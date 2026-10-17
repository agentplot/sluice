# shellcheck shell=sh disable=SC2154 # common.sh sets state.
# The lock every change to the watcher's lease is made under: a directory
# holding its owner's pid, broken when that owner is gone. Sourced.

lock=$state/lease.lock

# Take the lock; 1 when a live owner keeps it for three seconds.
take_lock() {
  tries=0
  while ! mkdir "$lock" 2>/dev/null; do
    owner=$(cat "$lock/pid" 2>/dev/null)
    if [ -n "$owner" ] && ! kill -0 "$owner" 2>/dev/null; then
      # Its owner died holding it.
      rm -rf "$lock"
      continue
    fi
    tries=$((tries + 1))
    if [ "$tries" -ge 3 ]; then
      # Still no owner after three seconds: one died between taking it and
      # writing its pid.
      if [ -z "$owner" ]; then
        rm -rf "$lock"
        tries=0
        continue
      fi
      return 1
    fi
    sleep 1
  done
  echo $$ >"$lock/pid"
}

drop_lock() {
  [ "$(cat "$lock/pid" 2>/dev/null)" = $$ ] && rm -rf "$lock"
  return 0
}
