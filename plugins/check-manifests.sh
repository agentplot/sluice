#!/usr/bin/env bash
# The marketplace and every plugin it lists, validated by the pinned Claude
# Code with a scratch home and no traffic beyond the command's, as the
# board's checks run it (sluice-board/tests/check.sh).

set -u
root=$(cd "$(dirname "$0")/.." && pwd)

# Under sluice's sandbox only SLUICE_TMP is writable beside the checkout.
scratch=$(mktemp -d "${SLUICE_TMP:-${TMPDIR:-/tmp}}/sluice-plugin-manifests.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/home"
for k in $(env | grep -Eo '^(CLAUDE|ANTHROPIC_)[A-Z_]*=' | tr -d =); do
  unset "$k"
done
export HOME=$scratch/home CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1

status=0
cd "$root" || exit 1
claude plugin validate --strict . || {
  echo "plugin-manifests: the marketplace failed claude plugin validate --strict" >&2
  status=1
}
while IFS=$'\t' read -r name source; do
  if [ ! -f "$source/.claude-plugin/plugin.json" ]; then
    echo "plugin-manifests: $name: $source has no .claude-plugin/plugin.json" >&2
    status=1
  elif ! claude plugin validate --strict "$source"; then
    echo "plugin-manifests: $name failed claude plugin validate --strict" >&2
    status=1
  fi
done < <(jq -r '.plugins[] | [.name, .source] | @tsv' .claude-plugin/marketplace.json)

exit $status
