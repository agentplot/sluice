#!/bin/sh
# shellcheck disable=SC1091,SC2154,SC2016 # jq programs; common.sh, sourced, sets root, lib, state and config.
# The tab-bar segment: the queue of the focused pane's repository on one
# line, worded as the Claude Code board's band; nothing while there is no
# queue. Run by herdr's `tab_bar_right` from the copy the watcher keeps in
# its state directory, with the focused pane's directory as cwd. It reads
# only that repository's documents: another repository's neither adds to
# its work nor blanks it.
set -u
here=$(cd "$(dirname "$0")/.." && pwd -P)
HERDR_PLUGIN_ROOT=$here
# The copy lives in <state>/segment; the plugin's own run gives the state.
[ -n "${HERDR_PLUGIN_STATE_DIR:-}" ] || HERDR_PLUGIN_STATE_DIR=$(dirname "$here")
export HERDR_PLUGIN_ROOT HERDR_PLUGIN_STATE_DIR
. "$here/lib/common.sh"

repo=$(repo_of "${SLUICE_HERDR_DIR:-$PWD}") || exit 0
# Its documents by the key document.sh keeps them under, any host's; the
# repository is checked again, since two paths may share a checksum.
set -- "$state"/docs/*--"$(name_of "$repo")-$(sum "$repo")".json
[ -e "$1" ] || exit 0
jqw -s -r --arg r "$repo" --argjson now "$(now)" 'include "words";
  map(select(.repository == $r)) | segment($now) | select(. != "")' "$@"
