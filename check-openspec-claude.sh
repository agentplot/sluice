#!/bin/sh
# OpenSpec's Claude Code skills and commands, which the tree tracks under
# .claude/skills/openspec-* and .claude/commands/opsx, are exactly what
# the OpenSpec devenv.nix pins writes (record 0101; core.24): ten
# workflows, propose, explore, new, continue, apply, ff, sync, archive,
# bulk-archive and verify, as skills and as commands. They are written
# afresh into a scratch directory under a configuration of this script's
# own, never the user's, and compared. With --write, the tracked ones are
# replaced by them, which moving the pin asks for.
set -eu

fail() {
  echo "check-openspec-claude: $*" >&2
  exit 1
}

case ${1-} in
"") write=false ;;
--write) write=true ;;
*) fail "usage: sh check-openspec-claude.sh [--write]" ;;
esac

pin=$(sed -n 's/^ *openspecVersion = "\(.*\)";$/\1/p' devenv.nix)
[ -n "$pin" ] || fail "devenv.nix pins no openspecVersion"
OPENSPEC_TELEMETRY=0
OPENSPEC_NO_UPDATE_CHECK=1
export OPENSPEC_TELEMETRY OPENSPEC_NO_UPDATE_CHECK
found=$(openspec --version)
[ "$found" = "$pin" ] ||
  fail "openspec is $found, devenv.nix pins $pin: run this in devenv shell"

scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/config/openspec" "$scratch/tree"
cat >"$scratch/config/openspec/config.json" <<'EOF'
{
  "profile": "custom",
  "delivery": "both",
  "workflows": ["propose", "explore", "new", "continue", "apply", "ff", "sync", "archive", "bulk-archive", "verify"]
}
EOF
XDG_CONFIG_HOME="$scratch/config" openspec init --tools claude --profile custom \
  --no-animation "$scratch/tree" </dev/null >/dev/null ||
  fail "openspec $pin could not write its Claude Code files"
made="$scratch/tree/.claude"

if $write; then
  rm -rf .claude/commands/opsx .claude/skills/openspec-*
  mkdir -p .claude/commands .claude/skills
  cp -R "$made/commands/opsx" .claude/commands/
  cp -R "$made/skills/"openspec-* .claude/skills/
  echo "OpenSpec $pin's Claude Code skills and commands written under .claude"
  exit 0
fi

status=0
diff -r "$made/commands/opsx" .claude/commands/opsx || status=1
# Each skill either side holds, so one missing from the tree and one
# OpenSpec no longer writes both fail.
for name in $({ ls "$made/skills" && ls .claude/skills; } 2>/dev/null | grep '^openspec-' | sort -u); do
  diff -r "$made/skills/$name" ".claude/skills/$name" || status=1
done
[ "$status" -eq 0 ] ||
  fail "OpenSpec's Claude Code files under .claude are not openspec $pin's: sh check-openspec-claude.sh --write"
echo "OpenSpec's Claude Code skills and commands are openspec $pin's"
