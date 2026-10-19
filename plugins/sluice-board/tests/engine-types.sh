#!/usr/bin/env bash
# The board's engine-types were written by the pinned Claude Code: tsc
# reads them, so a pin moved alone would type the modules against another
# engine's API.

set -u
board=$(cd "$(dirname "$0")/.." && pwd)

types=$(sed -n '1s|^// Written by Claude Code \(.*\)\.$|\1|p' "$board/engine-types/claude-code/index.d.ts")
engine=$("$board/../claude.sh" --version | sed -n '1s/^\([^ ]*\).*/\1/p')
if [ "$types" != "$engine" ]; then
  echo "plugin-board: engine-types were written by Claude Code ${types:-?}, the pinned claude is $engine: copy them again (README.md)" >&2
  exit 1
fi
