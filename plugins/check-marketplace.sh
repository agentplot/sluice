#!/usr/bin/env bash
# The marketplace, validated by the pinned Claude Code, and every plugin it
# lists found with a manifest. Each plugin's own manifest is validated by
# its project's `validate` task.

set -u
root=$(cd "$(dirname "$0")/.." && pwd)

status=0
cd "$root" || exit 1
"$root/plugins/claude.sh" plugin validate --strict . || {
  echo "marketplace: the marketplace failed claude plugin validate --strict" >&2
  status=1
}
while IFS=$'\t' read -r name source; do
  if [ ! -f "$source/.claude-plugin/plugin.json" ]; then
    echo "marketplace: $name: $source has no .claude-plugin/plugin.json" >&2
    status=1
  fi
done < <(jq -r '.plugins[] | [.name, .source] | @tsv' .claude-plugin/marketplace.json)

exit $status
