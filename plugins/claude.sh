#!/usr/bin/env bash
# The pinned Claude Code with a scratch home and no traffic beyond the
# command's, so a plugin's check reads none of the user's configuration or
# credentials and writes nothing of theirs; none of it needs the network.
# The plugins' projects and the marketplace check run `claude` through it.

set -u

# Under sluice's sandbox only SLUICE_TMP is writable beside the checkout.
scratch=$(mktemp -d "${SLUICE_TMP:-${TMPDIR:-/tmp}}/sluice-claude.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/home"
for k in $(env | grep -Eo '^(CLAUDE|ANTHROPIC_)[A-Z_]*=' | tr -d =); do
  unset "$k"
done
export HOME=$scratch/home CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1

claude "$@"
