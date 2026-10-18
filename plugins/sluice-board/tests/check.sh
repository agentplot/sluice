#!/usr/bin/env bash
# The board's checks: its engine-types are the pinned Claude Code's, then
# claude plugin validate, tsc and claude plugin test. Claude Code runs with
# a scratch home and no traffic beyond what a command needs, so it reads
# none of the user's configuration or credentials and writes nothing of
# theirs; none of it needs the network.

set -u
here=$(cd "$(dirname "$0")" && pwd)
board=$(cd "$here/.." && pwd)

# Under sluice's sandbox only SLUICE_TMP is writable beside the checkout.
scratch=$(mktemp -d "${SLUICE_TMP:-${TMPDIR:-/tmp}}/sluice-plugin-board.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/home"
for k in $(env | grep -Eo '^(CLAUDE|ANTHROPIC_)[A-Z_]*=' | tr -d =); do
  unset "$k"
done
export HOME=$scratch/home CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1

status=0
fail() {
  echo "plugin-board: $*" >&2
  status=1
}

types=$(sed -n '1s|^// Written by Claude Code \(.*\)\.$|\1|p' "$board/engine-types/claude-code/index.d.ts")
engine=$(claude --version | sed -n '1s/^\([^ ]*\).*/\1/p')
if [ "$types" != "$engine" ]; then
  fail "engine-types were written by Claude Code ${types:-?}, the pinned claude is $engine: copy them again (README.md)"
fi

claude plugin validate --strict "$board" || fail "claude plugin validate --strict failed"
tsc -p "$board" || fail "tsc -p failed"
claude plugin test "$board" || fail "claude plugin test failed"

exit $status
