#!/bin/sh
# shellcheck disable=SC2016 # jq programs name their own $variables.
# The herdr plugin's scripts against stand-in `herdr` and `sluice` on PATH:
# recorded overview documents in, the herdr calls made checked. git runs
# isolated from the user's config and credentials on scratch repositories,
# and nothing here reaches a network, a credential or a running herdr.

set -u
here=$(cd "$(dirname "$0")" && pwd -P)
plugin=$(dirname "$here")

# Under sluice's sandbox only SLUICE_TMP is writable beside the checkout.
scratch=$(mktemp -d "${SLUICE_TMP:-${TMPDIR:-/tmp}}/sluice-herdr.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
scratch=$(cd "$scratch" && pwd -P)

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 GIT_TERMINAL_PROMPT=0
export GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@localhost
export GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@localhost
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_ASKPASS SSH_ASKPASS
unset HERDR_PLUGIN_CONTEXT_JSON HERDR_SOCKET_PATH HERDR_ENV
export PATH="$here/fake:$PATH" HERDR_BIN_PATH="$here/fake/herdr" HERDR_PLUGIN_ROOT="$plugin"
NOW=$(jq -n '"2026-10-16T19:45:10Z" | fromdateiso8601')
export SLUICE_HERDR_NOW="$NOW"

# The repositories: `repo` with sluice's records, its worktrees `work`
# (3 commits ahead of main) and `other`; `plain`, a repository without;
# `home`, no repository at all.
repo=$scratch/repo
git init -q -b main "$repo"
git -C "$repo" commit -q --allow-empty -m root
main=$(git -C "$repo" rev-parse HEAD)
git -C "$repo" worktree add -q -b work "$scratch/work" main
for i in 1 2 3; do git -C "$scratch/work" commit -q --allow-empty -m "w$i"; done
git -C "$repo" worktree add -q -b other "$scratch/other" main
git init -q -b main "$scratch/plain"
git -C "$scratch/plain" commit -q --allow-empty -m root
mkdir -p "$scratch/home"
common=$(cd "$repo/.git" && pwd -P)
work=$scratch/work
other=$scratch/other

failures=0
failed_before=0
count=0
name=

# A fresh state, config and stand-in directory for one test.
fresh() {
  name=$1
  failed_before=$failures
  count=$((count + 1))
  t=$scratch/t$count
  mkdir -p "$t/state" "$t/config" "$t/fake"
  export HERDR_PLUGIN_STATE_DIR="$t/state" HERDR_PLUGIN_CONFIG_DIR="$t/config" FAKE="$t/fake"
  export SLUICE_HERDR_NOW="$NOW"
  : >"$FAKE/herdr-calls"
  : >"$FAKE/sluice-calls"
  panes front-none
}

fail() {
  failures=$((failures + 1))
  printf 'FAIL %s: %s\n' "$name" "$*"
}

pass() {
  [ "$failures" = "$failed_before" ] && printf 'ok   %s\n' "$name"
  return 0
}

calls() { cat "$FAKE/herdr-calls"; }
has() { grep -qF -- "$1" "$FAKE/herdr-calls" || fail "no herdr call with: $1"; }
hasnt() { if grep -qF -- "$1" "$FAKE/herdr-calls"; then fail "a herdr call with: $1"; fi; }

# Panes: p1 in work (workspace w1, tab t1), p2 in other (w2, t2), p3 in
# home (w1, t1). $1 names the pane in front, or front-none.
panes() {
  jq -n -c --arg work "$work" --arg other "$other" --arg home "$scratch/home" --arg f "$1" '
    {result: {panes: [
      {pane_id: "p1", tab_id: "t1", workspace_id: "w1", agent: "claude", cwd: $work, foreground_cwd: $work, focused: ($f == "p1")},
      {pane_id: "p2", tab_id: "t2", workspace_id: "w2", agent: "claude", cwd: $other, foreground_cwd: $other, focused: ($f == "p2")},
      {pane_id: "p3", tab_id: "t1", workspace_id: "w1", cwd: $home, foreground_cwd: $home, focused: false}]}}' >"$FAKE/panes.json"
  jq -n -c --arg f "$1" '{result: {tabs: [{tab_id: "t1", focused: ($f == "p1")}, {tab_id: "t2", focused: ($f == "p2")}]}}' >"$FAKE/tabs.json"
  jq -n -c --arg f "$1" '{result: {workspaces: [{workspace_id: "w1", focused: ($f == "p1")}, {workspace_id: "w2", focused: ($f == "p2")}]}}' >"$FAKE/spaces.json"
}

# A document for repo: main with nothing queued, changed by the jq in $1.
doc() {
  jq -n -c --arg repo "$common" --arg main "$main" --arg work "$work" --arg other "$other" "
    {schema: \"sluice/overview@1\", host: \"test\", repository: \$repo, at: \"2026-10-16T19:45:10Z\",
     branches: [{branch: \"main\", at: \$main, version: 1, window: 1, parallel: 2, depth: 0,
                 candidates: [], submissions: [], recent: []}],
     sessions: [], previews: []} | ${1:-.}"
}

# Submission #$1 from worktree $2 in state $3.
sub() { echo "{id: $1, state: \"$3\", from: {agent: \"claude\", worktree: \"w\", path: $2}}"; }

feed() {
  # $@ documents, one per argument, handed to document.sh as the watcher does.
  for d in "$@"; do
    printf '%s\n' "$d" | sh "$plugin/bin/document.sh" local "$work"
  done
}

tokens() { sh "$plugin/bin/tokens.sh" "$@"; }
forecast() { sh "$plugin/bin/forecast.sh"; }

testing95=$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 95 '$work' testing)]
  | .branches[0].candidates = [{id: \"c1\", submissions: [95], revision: \"x\", state: \"testing\",
      started: \"2026-10-16T19:42:00Z\", limit: 900, targets: [\"#check-fast\"], executor: \"sandbox\", ran: 0, replayed: 0, failed: []}]")
blamed87='.branches[0].recent += [{schema: "sluice/log@1", at: "2026-10-16T19:44:00Z", event: "blamed", id: 87, combination: "b03f4a00e8"}]'
landed94='.branches[0].recent += [{schema: "sluice/log@1", at: "2026-10-16T19:33:10Z", event: "landed", ids: [94], to: "46cc4290c0"}]'
conflict53='.branches[0].recent += [{schema: "sluice/log@1", at: "2026-10-16T19:44:30Z", event: "conflict", id: 53, paths: ["a.rs"], with: []}]'
status_of() {
  # $1 id, $2 path, $3 state, $4 failed tasks as JSON
  jq -n -c --argjson id "$1" --arg p "$2" --arg s "$3" --argjson f "${4:-[]}" \
    '{schema: "sluice/submission@1", id: $id, branch: "main", from: {agent: "claude", worktree: "w", path: $p}, state: $s, failed: ($f | map({task: ., log: "/l/\(.).log"}))}' >"$FAKE/status-$1.json"
}

## herdr-watch

fresh 'watch: started twice'
echo "$$ $(date +%s)" >"$HERDR_PLUGIN_STATE_DIR/watch.lease"
sh "$plugin/bin/watch.sh" --ensure || fail "ensure exited non-zero"
sleep 1
[ "$(cut -d' ' -f1 "$HERDR_PLUGIN_STATE_DIR/watch.lease")" = $$ ] || fail "a second watcher took the lease"
[ -s "$FAKE/sluice-calls" ] && fail "a second watcher ran sluice"
pass

fresh 'watch: watcher gone'
echo "$$ $(($(date +%s) - 90))" >"$HERDR_PLUGIN_STATE_DIR/watch.lease"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 >/dev/null 2>&1
grep -q "overview --json" "$FAKE/sluice-calls" || fail "the new watcher did not look for repositories"
pass

fresh 'watch: herdr gone'
touch "$FAKE/down"
began=$(date +%s)
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 10 2>/dev/null
[ $(($(date +%s) - began)) -lt 6 ] || fail "the watcher kept going without herdr"
[ -f "$HERDR_PLUGIN_STATE_DIR/watch.lease" ] && fail "the watcher left its lease"
pass

fresh 'watch: no configuration'
doc >"$FAKE/overview-work.json"
doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 12 '$work' queued)]" >"$FAKE/watch-work.jsonl"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
[ "$(grep -c 'overview --watch --json' "$FAKE/sluice-calls")" = 1 ] || fail "watch ran other than once: $(cat "$FAKE/sluice-calls")"
grep -q '^work: overview --watch --json' "$FAKE/sluice-calls" || fail "watch did not run in the repository with records"
[ -n "$(ls "$HERDR_PLUGIN_STATE_DIR/docs/"*.json 2>/dev/null)" ] || fail "no document kept"
pass

fresh "watch: a source of the user's"
cat >"$HERDR_PLUGIN_CONFIG_DIR/sources.toml" <<'EOF'
# a stream of the user's own
[[sources]]
name = "build machine"
command = ["my-overview-stream", "app"]
EOF
jq -n -c '{schema: "sluice/overview@1", host: "build", repository: "/srv/app.git", at: "2026-10-16T19:45:10Z", branches: [], sessions: [], previews: []}' >"$FAKE/stream.jsonl"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
[ "$(cat "$FAKE/stream-calls" 2>/dev/null)" = app ] || fail "the source did not run once with its argv"
set -- "$HERDR_PLUGIN_STATE_DIR"/docs/build--app-*.json
[ -e "$1" ] || fail "its document is not kept under its host and repository"
grep -q 'overview --watch' "$FAKE/sluice-calls" && fail "the default source ran beside the user's"
pass

fresh 'watch: configuration it cannot read'
printf 'ahead_check = false\ncommand = [\n  "a"]\n' >"$HERDR_PLUGIN_CONFIG_DIR/sources.toml"
got=$(sh -c '. "$HERDR_PLUGIN_ROOT/lib/common.sh"; . "$lib/sources.sh"; read_sources' "$plugin/bin/x" 2>"$t/err")
grep -q 'sources.toml:2:' "$t/err" || fail "the error does not name line 2: $(cat "$t/err")"
[ "$(printf '%s' "$got" | jq -r '.sources[0].command | join(" ")')" = "sluice overview --watch --json" ] || fail "not the default: $got"
printf 'ahead_check = false # off\n[[sources]]\nname = "a \\"b\\""\ncommand = ["x", "y"]\neach_repository = true\n' >"$HERDR_PLUGIN_CONFIG_DIR/sources.toml"
got=$(sh -c '. "$HERDR_PLUGIN_ROOT/lib/common.sh"; . "$lib/sources.sh"; read_sources' "$plugin/bin/x")
[ "$got" = '{"ahead_check":false,"sources":[{"name":"a \"b\"","command":["x","y"],"each_repository":true}]}' ] || fail "read as $got"
pass

fresh 'watch: two documents'
feed "$(doc)" "$testing95" "$(doc '.branches[0].recent += [{schema: "sluice/log@1", at: "2026-10-16T19:45:00Z", event: "landed", ids: [95], to: "aaaaaaa"}]')"
f=$(ls "$HERDR_PLUGIN_STATE_DIR"/docs/*.json)
[ "$(jq -r '.branches[0].recent[0].ids[0]' "$f")" = 95 ] || fail "the state file is not the last document"
set -- "$HERDR_PLUGIN_STATE_DIR"/docs/*.tmp.*
[ -e "$1" ] && fail "a temporary file was left"
pass

fresh 'watch: source exits'
doc >"$FAKE/overview-work.json"
doc >"$FAKE/watch-work.jsonl"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 3 2>/dev/null
[ "$(grep -c 'overview --watch --json' "$FAKE/sluice-calls")" -ge 2 ] || fail "the source was not run again: $(cat "$FAKE/sluice-calls")"
pass

fresh 'watch: the watcher only reads'
status_of 9 "$work" withdrawn '["cli:test"]'
feed "$(doc)" "$(doc '.branches[0].recent += [{schema: "sluice/log@1", at: "2026-10-16T19:44:00Z", event: "blamed", id: 9, combination: "c"}]')"
forecast
tokens
if grep -v -e 'overview --' -e 'status 9 --json' -e 'submit main --check --json' "$FAKE/sluice-calls" | grep -q .; then
  fail "other sluice commands ran: $(cat "$FAKE/sluice-calls")"
fi
pass

## herdr-status

frame_of() {
  # $1 directory of the focused pane, then flags
  d=$1
  shift
  SLUICE_HERDR_DIR=$d sh "$plugin/bin/status.sh" "$@" </dev/null
}

fresh 'status: testing'
board=$(jq -c --arg repo "$common" '.repository = $repo' "$here/docs/board.json")
printf '%s\n' "$board" | sh "$plugin/bin/document.sh" local "$work"
frame_of "$work" --once >"$t/frame"
sed "s|$common|REPO|" "$t/frame" >"$t/got"
diff -u "$here/docs/board.frame" "$t/got" >"$t/diff" || fail "the frame differs: $(cat "$t/diff")"
pass

fresh 'status: two hosts'
printf '%s\n' "$board" | sh "$plugin/bin/document.sh" local "$work"
printf '%s\n' "$board" | jq -c '.host = "build" | .repository = "/srv/repo/.git"' | sh "$plugin/bin/document.sh" "build machine" ""
frame_of "$work" --once >"$t/frame"
grep -q "^$common on test\$\|^$common on mac-studio\$" "$t/frame" || fail "no block for this host"
grep -q '^/srv/repo/.git on build$' "$t/frame" || fail "no block for the other host"
pass

fresh 'status: redraws on change'
feed "$testing95"
(
  sleep 1
  feed "$(doc '.branches[0].recent += [{schema: "sluice/log@1", at: "2026-10-16T19:45:00Z", event: "landed", ids: [95], to: "abcdef0"}]')"
  printf x
) | SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" >"$t/out"
first=$(awk 'BEGIN{RS="\f"} NR==1' "$t/out")
second=$(awk 'BEGIN{RS="\f"} NR==2' "$t/out")
printf '%s' "$first" | grep -q '▶ testing main+#95' || fail "the first frame does not show #95 testing"
printf '%s' "$second" | grep -q '✓ 19:45 #95 landed as abcdef0' || fail "the redraw does not show the landing: $second"
printf '%s' "$second" | grep -q '▶ testing' && fail "the redraw still shows #95 under test"
pass

fresh 'status: close'
feed "$testing95"
printf '\033' | SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" >/dev/null || fail "Escape did not close it cleanly"
printf 'q' | SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" >/dev/null || fail "q did not close it cleanly"
pass

fresh 'status: all repositories'
feed "$testing95"
printf '%s\n' "$board" | jq -c '.repository = "/srv/elsewhere.git"' | sh "$plugin/bin/document.sh" "build" ""
printf 'a' | SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" >"$t/out"
awk 'BEGIN{RS="\f"} NR==1' "$t/out" | grep -q elsewhere && fail "the first frame shows another repository"
awk 'BEGIN{RS="\f"} NR==2' "$t/out" | grep -q '^/srv/elsewhere.git on mac-studio' || fail "a does not show every repository"
pass

fresh "status: the focused pane from herdr's context"
feed "$testing95"
ctx=$(jq -c --arg w "$work" '.focused_pane_cwd = $w' "$here/docs/context.json")
HERDR_PLUGIN_CONTEXT_JSON=$ctx sh "$plugin/bin/status.sh" --once | grep -q '▶ testing main+#95' || fail "the popup did not read the focused pane's repository"
pass

fresh 'status: a plain directory'
out=$(frame_of "$scratch/home" --once)
printf '%s' "$out" | grep -q '^No sluice records in this repository.$' || fail "said: $out"
echo "none $NOW" >"$HERDR_PLUGIN_STATE_DIR/probe/$(sh -c '. "$HERDR_PLUGIN_ROOT/lib/common.sh"; sum "$1"' "$plugin/bin/x" "$(cd "$scratch/plain/.git" && pwd -P)")"
out=$(frame_of "$scratch/plain" --once)
printf '%s' "$out" | grep -q '^No sluice records in this repository.$' || fail "in a repository without records it said: $out"
out=$(frame_of "$other" --once)
printf '%s' "$out" | grep -q '^Waiting for sluice' || fail "before the watcher read it, it said: $out"
[ -n "$(ls "$HERDR_PLUGIN_STATE_DIR/want")" ] || fail "the watcher was not asked to read it"
pass

fresh 'status: the dashboard'
feed "$testing95"
printf '%s\n' "$board" | jq -c '.repository = "/srv/elsewhere.git"' | sh "$plugin/bin/document.sh" "build" ""
out=$(frame_of "$scratch/home" --dashboard --once)
printf '%s' "$out" | grep -q '^/srv/elsewhere.git' || fail "the dashboard misses a repository"
printf '%s' "$out" | grep -q "^$common" || fail "the dashboard misses this repository"
pass

fresh 'status: segment'
printf '%s\n' "$board" | jq -c --arg r "$common" '.repository = $r' | sh "$plugin/bin/document.sh" local "$work"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
seg=$HERDR_PLUGIN_STATE_DIR/segment/bin/segment.sh
# The copy stands alone: nothing in it names the plugin's own directory.
grep -rqF "$plugin" "$HERDR_PLUGIN_STATE_DIR/segment" && fail "the copy names the plugin's directory"
out=$(cd "$work" && env -u HERDR_PLUGIN_ROOT -u HERDR_PLUGIN_STATE_DIR sh "$seg")
[ "$out" = 'sluice main · 3 submissions · testing main+#12+#13 2m10s/15m · #14 waiting · last #9 blamed on 4f1c2aa' ] || fail "the segment printed: $out"
feed "$(doc '.host = "mac-studio"')"
out=$(cd "$work" && sh "$seg")
[ -z "$out" ] || fail "with no queue it printed: $out"
pass

fresh 'status: setup shows, never edits'
before=$(find "$HERDR_PLUGIN_CONFIG_DIR" "$scratch/home" -type f | sort)
out=$(sh "$plugin/bin/setup.sh" </dev/null)
for want in 'key = "prefix+s"' 'command = "agentplot.sluice.status"' '"$sluice"' '"$sluice_q"' \
  "sh '$HERDR_PLUGIN_STATE_DIR/segment/bin/segment.sh'"; do
  printf '%s' "$out" | grep -qF -- "$want" || fail "setup does not show $want"
done
[ "$(find "$HERDR_PLUGIN_CONFIG_DIR" "$scratch/home" -type f | sort)" = "$before" ] || fail "setup wrote a file"
[ -s "$FAKE/herdr-calls" ] && fail "setup called herdr: $(calls)"
pass

## herdr-toasts

fresh 'toasts: blamed'
status_of 87 "$work" withdrawn '["tests:t-cards"]'
feed "$(doc)" "$(doc "$blamed87")"
has 'notification show sluice: #87 blamed on main --body tests:t-cards failed.'
[ "$(grep -c 'notification show' "$FAKE/herdr-calls")" = 1 ] || fail "not one toast: $(calls)"
pass

fresh 'toasts: watcher restarted'
status_of 87 "$work" withdrawn '["tests:t-cards"]'
feed "$(doc)" "$(doc "$blamed87")" "$(doc "$blamed87")"
[ "$(grep -c 'notification show' "$FAKE/herdr-calls")" = 1 ] || fail "told twice: $(calls)"
pass

fresh 'toasts: first sight of a repository'
feed "$(doc "$blamed87 | $conflict53")"
hasnt 'notification show'
pass

fresh 'toasts: a conflict'
status_of 53 "$other" withdrawn
feed "$(doc)" "$(doc "$conflict53")"
has 'notification show sluice: #53 conflicts on main --body a.rs'
pass

fresh 'toasts: landed, looking elsewhere'
panes p2
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 94 '$work' testing)]")" "$(doc "$landed94")"
has 'notification show sluice: #94 landed on main --body as 46cc429'
pass

fresh 'toasts: landed, looking at it'
panes p1
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 94 '$work' testing)]")" "$(doc "$landed94")"
hasnt 'notification show'
pass

fresh 'toasts: landed, no pane'
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 94 '"/elsewhere"' testing)]")" "$(doc "$landed94")"
hasnt 'notification show'
pass

fresh 'jump: jump'
status_of 87 "$other" withdrawn '["tests:t-cards"]'
feed "$(doc)" "$(doc "$blamed87")"
: >"$FAKE/herdr-calls"
sh "$plugin/bin/jump.sh"
has 'agent focus p2'
touch "$FAKE/no-agent"
sh "$plugin/bin/jump.sh"
has 'workspace focus w2'
has 'tab focus t2'
pass

fresh 'jump: no pane'
status_of 87 /elsewhere withdrawn
feed "$(doc)" "$(doc "$blamed87")"
: >"$FAKE/herdr-calls"
sh "$plugin/bin/jump.sh"
has 'notification show sluice: no pane for #87'
hasnt 'focus'
pass

## herdr-sidebar

fresh 'sidebar: testing'
feed "$testing95"
tokens
has 'pane report-metadata p1 --source agentplot.sluice --seq'
has '--ttl-ms 120000 --token sluice=#95 testing 3m/15m'
pass

fresh 'sidebar: blamed'
status_of 87 "$work" withdrawn '["tests:t-cards"]'
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 87 '$work' testing)]")" "$(doc "$blamed87")"
tokens
has '--token sluice=#87 BLAMED · tests:t-cards'
pass

fresh 'sidebar: a conflict'
status_of 53 "$work" withdrawn
feed "$(doc)" "$(doc "$conflict53")"
tokens
has '--token sluice=#53 CONFLICT · a.rs'
pass

fresh 'sidebar: landed, then gone'
git -C "$scratch/other" checkout -q --detach "$main"
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 94 '$other' testing)]")" "$(doc "$landed94")"
tokens
has 'pane report-metadata p2 --source agentplot.sluice'
has '--token sluice=#94 landed 12m'
: >"$FAKE/herdr-calls"
SLUICE_HERDR_NOW=$((NOW + 1200)) tokens
has 'pane report-metadata p2 --source agentplot.sluice'
has '--clear-token sluice'
git -C "$scratch/other" checkout -q other
pass

fresh 'sidebar: nothing to say'
feed "$(doc)"
tokens
hasnt 'pane report-metadata p2'
hasnt 'pane report-metadata p3'
pass

fresh 'sidebar: clean'
feed "$(doc)"
forecast
tokens
has '--token sluice=3 ahead · clean'
[ "$(grep -c 'work: submit main --check --json' "$FAKE/sluice-calls")" = 1 ] || fail "not one forecast: $(cat "$FAKE/sluice-calls")"
pass

fresh 'sidebar: conflicts'
echo '{"conflict":{"paths":["a.rs"],"with":[{"id":95,"from":{"agent":"x","worktree":"y"}}]}}' >"$FAKE/check.json"
feed "$(doc)"
forecast
tokens
has '--token sluice=3 ahead · conflicts #95'
pass

fresh 'sidebar: nothing changed'
feed "$(doc)"
forecast
feed "$(doc '.at = "2026-10-16T19:46:00Z"')"
forecast
[ "$(grep -c 'submit main --check' "$FAKE/sluice-calls")" = 1 ] || fail "forecast again: $(cat "$FAKE/sluice-calls")"
feed "$(doc '.branches[0].version = 2')"
forecast
[ "$(grep -c 'submit main --check' "$FAKE/sluice-calls")" = 2 ] || fail "no forecast on a new version"
pass

fresh 'sidebar: forecasts off'
echo 'ahead_check = false' >"$HERDR_PLUGIN_CONFIG_DIR/sources.toml"
feed "$(doc)"
forecast
tokens
grep -q 'submit' "$FAKE/sluice-calls" && fail "a forecast ran"
has '--token sluice=3 ahead'
hasnt '3 ahead ·'
pass

fresh 'sidebar: queued'
release='.branches[0].depth = 3 | .branches += [.branches[0] | .branch = "release" | .depth = 1]'
feed "$(doc "$release")"
tokens
has 'workspace report-metadata w1 --source agentplot.sluice'
has '--token sluice_q=4 submissions'
: >"$FAKE/herdr-calls"
feed "$(doc)"
tokens
has 'workspace report-metadata w1 --source agentplot.sluice'
has '--clear-token sluice_q'
pass

fresh 'sidebar: tokens fade without the watcher'
feed "$testing95"
tokens
: >"$FAKE/herdr-calls"
tokens
hasnt 'report-metadata'
tokens --refresh
has '--ttl-ms 120000 --token sluice=#95 testing'
pass

echo
if [ "$failures" -gt 0 ]; then
  echo "$failures failed of $count"
  exit 1
fi
echo "all $count passed"
