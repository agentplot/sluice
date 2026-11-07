# shellcheck shell=sh
# The stand-ins in tests/fake-logged, which a test puts first on PATH to
# count what the plugin runs: each call of the tool they are named for is
# logged in $FAKE/<tool>-calls, one line with the directory it ran in, as
# the stand-in sluice logs its own, and handed to the real tool, the next
# of that name on PATH. Sourced by them, with builtins alone.

tool=${0##*/}
nl='
'
# The call on one line: a jq program's lines joined by spaces.
line=
for a in "$@"; do
  while :; do
    case $a in
    *"$nl"*)
      line="$line${a%%"$nl"*} "
      a=${a#*"$nl"}
      ;;
    *) break ;;
    esac
  done
  line="$line$a "
done
printf '%s: %s\n' "${PWD##*/}" "${line% }" >>"$FAKE/$tool-calls"

old_ifs=$IFS
IFS=:
for d in $PATH; do
  if [ -x "$d/$tool" ] && [ ! "$d/$tool" -ef "$0" ]; then
    IFS=$old_ifs
    exec "$d/$tool" "$@"
  fi
done
echo "$tool: no real $tool on PATH" >&2
exit 127
