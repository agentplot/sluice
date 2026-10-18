#!/bin/sh
# A failed test's log, in the split a link opened: less where there is one,
# else its last 200 lines until q.
set -u
log=${SLUICE_HERDR_LOG:-}
[ -n "${SLUICE_HERDR_TASK:-}" ] && printf 'log of %s\n\n' "$SLUICE_HERDR_TASK"
if [ ! -r "$log" ]; then
  echo "cannot read $log"
elif command -v less >/dev/null 2>&1 && [ -t 1 ]; then
  exec less +G "$log"
else
  tail -n 200 "$log"
fi
[ -t 0 ] || exit 0
echo
echo "q close"
while :; do
  k=$(dd bs=1 count=1 2>/dev/null)
  case $k in q | "") exit 0 ;; esac
done
