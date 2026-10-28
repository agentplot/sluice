#!/usr/bin/env bash
# SessionStart: in a repository whose policy protects branches, tell the
# session which, and how work lands on them; say so when sluice is missing
# or too old for this plugin. Silent everywhere else.

set -u
command -v jq >/dev/null 2>&1 || exit 0

input=$(cat)
dir=$(printf '%s' "$input" | jq -r '.cwd // empty' 2>/dev/null)
[ -n "$dir" ] || dir=$PWD

# shellcheck disable=SC1091 # beside this script
. "$(dirname "$0")/protected.sh"

patterns=$(sluice_patterns "$(sluice_policy "$dir")")
[ -n "$patterns" ] || exit 0

# shellcheck disable=SC2016 # backticks are the text
list=$(printf '%s' "$patterns" | sed 's/.*/`&`/' | paste -sd, - | sed 's/,/, /g')
# The example names a branch, not a pattern, where the policy has one.
first=$(printf '%s\n' "$patterns" | grep -v '[*?[]' | head -n 1)
[ -n "$first" ] || first=$(printf '%s' "$patterns" | head -n 1)
note="This repository lands work through sluice, a merge queue: $list only move through it. Never merge or push onto them. Before anything else, run \`sluice inbox\` once in your worktree: it prints each outcome of your earlier submissions that you missed, a landing, a failure or a conflict, and nothing when none is pending. Commit your work and submit from your worktree without waiting: \`sluice submit $first --no-wait --json\` returns the submission's id at once. A verdict takes from a minute to most of an hour, and a submission that holds your session keeps every message and the user from you meanwhile, so watch it from your harness's background: start \`sluice wait <id> --timeout 9m --json\` as a background command (in Claude Code, the Bash tool's \`run_in_background\`), again the same way for as long as it exits 5, and read the record's \`state\`. Under worktrunk, submit and wait in one command instead: start \`wt submit $first --timeout 9m --json\` as a background command, again the same way for as long as it exits 5. It submits once, then follows that submission with \`sluice wait\`'s exits, and removes your worktree once it lands, which \`sluice wait\` never does; never remove a landed worktree by hand. Wait in the foreground only where your harness runs nothing in the background, and never search \`sluice status\`'s listing for the outcome. Test while you work with \`sluice exec $first -- <command>\` (your worktree's ports, nothing recorded), and get the gate's verdict before submitting with \`sluice exec $first\` once your work is committed. An agent this session starts to work in a worktree submits its own work through sluice from there, told so in the prompt it is given; never ask the user to run \`sluice submit\`. The using-sluice skill has the details."

if ! command -v sluice >/dev/null 2>&1; then
  note="$note sluice itself is not installed on this host: the setting-up-sluice skill installs it, asking first."
else
  version=$(sluice --version 2>/dev/null | awk '{print $2}')
  major=${version%%.*}
  minor=${version#*.}
  minor=${minor%%.*}
  if [ -n "$version" ] && { [ "${major:-0}" -eq 0 ] && [ "${minor:-0}" -lt 12 ]; } 2>/dev/null; then
    note="$note The installed sluice is $version; this plugin needs 0.12 or later (the setting-up-sluice skill upgrades it)."
  fi
fi

jq -n --arg c "$note" '{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: $c}}'
