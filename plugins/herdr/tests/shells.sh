#!/bin/sh
# The plugin's tests under the system's sh and, where it is installed,
# under dash, the /bin/sh of Debian and Ubuntu, which herdr runs the
# plugin's commands with there.
set -u
here=$(cd "$(dirname "$0")" && pwd -P)
status=0
echo "== sh"
sh "$here/run.sh" || status=1
if command -v dash >/dev/null 2>&1; then
  echo "== dash"
  SLUICE_HERDR_SH=dash sh "$here/run.sh" || status=1
else
  echo "== dash is not installed here; skipped"
fi
exit "$status"
