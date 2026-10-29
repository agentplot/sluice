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

# A lease this run holds for a test, as a watcher of this plugin would, so
# --ensure starts none and what the test asks of the watcher stays put.
hold_lease() {
  printf '%s %s\n%s\n%s\n' $$ "$(date +%s)" "$plugin" "$(sed -n 's/^version = "\(.*\)"$/\1/p' "$plugin/herdr-plugin.toml")" >"$S/watch.lease"
}

# Wait up to $1 seconds for the shell condition $2, checking each second.
within() {
  n=$1
  while [ "$n" -gt 0 ]; do
    eval "$2" && return 0
    sleep 1
    n=$((n - 1))
  done
  eval "$2"
}

# Wait up to $2 seconds for process $1 to end, checking each second; say
# how long it took in $took. A process is waited for, never slept on: under
# a loaded gate, ending takes longer than any fixed sleep allows.
ends_within() {
  began=$(date +%s)
  while kill -0 "$1" 2>/dev/null; do
    took=$(($(date +%s) - began))
    [ "$took" -ge "$2" ] && return 1
    sleep 1
  done
  took=$(($(date +%s) - began))
}

# The pids of the sources a watcher started.
sources_of() {
  cat "$S/run/$1"/*.pids 2>/dev/null | cut -f1
}

lease_age_at_least() {
  [ $(($(date +%s) - $(head -1 "$S/watch.lease" | cut -d' ' -f2))) -ge "$1" ]
}

# One of lib/procs.sh's functions.
procs() {
  sh -c '. "$1/lib/procs.sh"; shift; "$@"' _ "$plugin" "$@"
}

# The pid holding a lease: the first word of its first line.
holder_of() {
  head -1 "$1" 2>/dev/null | cut -d' ' -f1
}

# A watcher a test started, through --ensure or itself, is ended with it:
# only those whose lease is in this run's scratch directory.
reap() {
  for l in "$scratch"/t*/state/watch.lease "$scratch"/t*/state/*/*/watch.lease; do
    [ -f "$l" ] || continue
    p=$(holder_of "$l")
    [ -n "$p" ] && [ "$p" != $$ ] && kill "$p" 2>/dev/null
  done
  sleep 1
}
trap 'reap; rm -rf "$scratch"' EXIT
scratch=$(cd "$scratch" && pwd -P)
# git looks for a repository no higher than the scratch directory: the gate
# runs the suite with it inside sluice's own bare repository.
export GIT_CEILING_DIRECTORIES="$scratch"

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 GIT_TERMINAL_PROMPT=0
export GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@localhost
export GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@localhost
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_ASKPASS SSH_ASKPASS
unset HERDR_PLUGIN_CONTEXT_JSON HERDR_SOCKET_PATH HERDR_ENV
# SLUICE_HERDR_SH names the shell every script runs under as `sh`, as
# herdr runs them: dash, as Debian's /bin/sh is, or the system's.
if [ -n "${SLUICE_HERDR_SH:-}" ]; then
  mkdir -p "$scratch/sh"
  ln -s "$(command -v "$SLUICE_HERDR_SH")" "$scratch/sh/sh"
  export PATH="$scratch/sh:$PATH"
fi
echo "scripts run under $(command -v sh) -> $(readlink "$(command -v sh)" || echo itself)"
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
skips=0
name=

# A fresh state, config and stand-in directory for one test.
fresh() {
  reap
  name=$1
  failed_before=$failures
  count=$((count + 1))
  t=$scratch/t$count
  mkdir -p "$t/state" "$t/config" "$t/fake"
  export HERDR_PLUGIN_STATE_DIR="$t/state" HERDR_PLUGIN_CONFIG_DIR="$t/config" FAKE="$t/fake"
  export SLUICE_HERDR_NOW="$NOW"
  # This herdr server's state: the tests run with no socket.
  S=$t/state/servers/$(printf none | cksum | cut -d' ' -f1)
  mkdir -p "$S/docs" "$S/probe"
  : >"$FAKE/herdr-calls"
  : >"$FAKE/sluice-calls"
  panes front-none
}

fail() {
  failures=$((failures + 1))
  printf 'FAIL %s: %s\n' "$name" "$*"
}

# Whether a real ps (or /proc) can be read here, for the few tests that
# need one; where not (sluice's sandbox on macOS), the test is skipped,
# said and counted.
ps_error=$(ps -o pid= -p $$ 2>&1 >/dev/null)
if [ -r /proc/$$/stat ] || ps -o pid= -p $$ >/dev/null 2>&1; then ps_runs=true; else ps_runs=false; fi
needs_ps() {
  [ "$ps_runs" = true ] && return 0
  echo "skip: $name: ps can't run here (${ps_error:-no output})"
  skips=$((skips + 1))
  return 1
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

# The token sluice_branch=$2 on pane $1, as an orchestrator sets it,
# beside another source's token in the one map herdr keeps.
named() {
  jq -c --arg p "$1" --arg b "$2" '.result.panes |= map(if .pane_id == $p then .tokens = {sluice_branch: $b, other_tool: "x"} else . end)' \
    "$FAKE/panes.json" >"$FAKE/panes.next" && mv "$FAKE/panes.next" "$FAKE/panes.json"
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
# release, whose submissions come from other worktrees: #12 under test, 3
# of 8 tasks ended; #13 to #15 queued; two agents' checks held to it.
release_busy='.branches += [{branch: "release", at: $main, version: 1, window: 1, parallel: 1, depth: 4,
    candidates: [{id: "c9", submissions: [12], revision: "x", state: "testing", started: "2026-10-16T19:42:00Z",
      limit: 900, targets: ["#check-fast"], executor: "sandbox", ran: 0, replayed: 0, failed: [],
      tasks: ([range(3) | {target: "t\(.):test", state: "passed"}] + [range(5) | {target: "u\(.):test", state: "waiting"}])}],
    submissions: [12, 13, 14, 15 | {id: ., state: (if . == 12 then "testing" else "queued" end),
      from: {agent: "unit", worktree: "u", path: "/elsewhere/u\(.)"}}],
    recent: []}]
  | .sessions += [{worktree: "/elsewhere/u1", branch: "release", run: {started: "2026-10-16T19:44:00Z"}},
      {worktree: "/elsewhere/u2", branch: "release", run: {started: "2026-10-16T19:44:00Z"}}]'
release_idle='.branches += [{branch: "release", at: $main, version: 1, window: 1, depth: 0, candidates: [], submissions: [], recent: []}]'
status_of() {
  # $1 id, $2 path, $3 state, $4 failed tasks as JSON
  jq -n -c --argjson id "$1" --arg p "$2" --arg s "$3" --argjson f "${4:-[]}" \
    '{schema: "sluice/submission@1", id: $id, branch: "main", from: {agent: "claude", worktree: "w", path: $p}, state: $s, failed: ($f | map({task: ., log: "/l/\(.).log"}))}' >"$FAKE/status-$1.json"
}

## the manifest

fresh 'manifest: a broken one refused'
# A copy of the plugin, broken three ways.
cp -R "$plugin/bin" "$plugin/lib" "$t/"
sed 's|bin/jump.sh|bin/nope.sh|; s|action = "open-link"|action = "gone"|' "$plugin/herdr-plugin.toml" >"$t/herdr-plugin.toml"
echo 'width = 80%' >>"$t/herdr-plugin.toml"
if sh "$here/manifest.sh" "$t/herdr-plugin.toml" >"$t/out" 2>&1; then fail "a broken manifest passed"; fi
grep -q 'runs bin/nope.sh, which is not in the plugin' "$t/out" || fail "a missing script not named: $(cat "$t/out")"
grep -q 'a link handler names no action of this plugin: "gone"' "$t/out" || fail "an unknown action not named"
grep -q 'a value herdr cannot read: width = 80%' "$t/out" || fail "an unreadable line not named"
grep -v '^id = ' "$plugin/herdr-plugin.toml" >"$t/herdr-plugin.toml"
if sh "$here/manifest.sh" "$t/herdr-plugin.toml" >"$t/out" 2>&1; then fail "a manifest with no id passed"; fi
grep -q ': no id$' "$t/out" || fail "the missing id not named: $(cat "$t/out")"
pass

## herdr-watch

fresh 'watch: started twice'
hold_lease
sh "$plugin/bin/watch.sh" --ensure || fail "ensure exited non-zero"
sleep 1
[ "$(holder_of "$S/watch.lease")" = $$ ] || fail "a second watcher took the lease"
[ -s "$FAKE/sluice-calls" ] && fail "a second watcher ran sluice"
[ -f "$S/dirty" ] || fail "a new pane waits a whole tick for its token"
pass

fresh 'watch: watcher gone'
echo "$$ $(($(date +%s) - 90))" >"$S/watch.lease"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 >/dev/null 2>&1
grep -q "overview --json" "$FAKE/sluice-calls" || fail "the new watcher did not look for repositories"
pass

fresh 'watch: a lock left by a watcher that died'
mkdir "$S/lease.lock"
sh -c 'echo $$ >"$1/pid"' _ "$S/lease.lock"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 >/dev/null 2>&1
grep -q "overview --json" "$FAKE/sluice-calls" || fail "a dead watcher's lock stopped the next one"
pass

fresh 'watch: a lock with no owner'
mkdir "$S/lease.lock"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 >/dev/null 2>&1
grep -q "overview --json" "$FAKE/sluice-calls" || fail "a lock with no owner stopped the watcher"
pass

fresh 'watch: a long tick keeps the lease'
echo 6 >"$FAKE/probe-sleep"
SLUICE_HERDR_LEASE=3 SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>"$t/log" &
first=$!
sleep 4
SLUICE_HERDR_LEASE=3 sh "$plugin/bin/watch.sh" --ensure
wait "$first"
sleep 1
if grep -q 'watching' "$S/watch.log" 2>/dev/null; then
  fail "a second watcher started during a long tick"
  # Only the one this test started, never another herdr's.
  stray=$(holder_of "$S/watch.lease")
  [ -n "$stray" ] && [ "$stray" != "$first" ] && kill "$stray" 2>/dev/null
fi
pass

fresh 'watch: a watcher stops only its own sources'
if needs_ps; then
  doc >"$FAKE/overview-work.json"
  doc >"$FAKE/watch-work.jsonl"
  touch "$FAKE/hold"
  SLUICE_HERDR_LOCK_HOLD=3 SLUICE_HERDR_LEASE=2 SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 60 2>/dev/null &
  a=$!
  within 15 'sources_of "$a" | grep -q .' || fail "the first watcher started no source"
  # The first watcher stalls past its lease, its renewer in the middle of
  # renewing it (holding the lock); a second takes over.
  within 15 '[ "$(procs parent_of "$(cut -d" " -f1 "$S/lease.lock/pid" 2>/dev/null)")" = "$a" ]' ||
    fail "the first watcher's renewer was not caught holding the lock"
  kill -STOP "$a" $(pgrep -P "$a") 2>/dev/null
  within 10 'lease_age_at_least 3' || fail "the stalled watcher's lease did not go stale"
  SLUICE_HERDR_LEASE=2 SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 60 2>/dev/null &
  b=$!
  within 30 '[ "$(holder_of "$S/watch.lease")" = "$b" ] && sources_of "$b" | grep -q .' ||
    fail "the second watcher did not take over and start its source"
  # The second watcher's sources, before the first one wakes.
  theirs=$(sources_of "$b")
  kill -CONT "$a" $(pgrep -P "$a") 2>/dev/null
  within 15 '! kill -0 "$a" 2>/dev/null' || fail "the stalled watcher did not give way"
  [ "$(holder_of "$S/watch.lease")" = "$b" ] || fail "the lease is not the second watcher's"
  for p in $theirs; do
    kill -0 "$p" 2>/dev/null || fail "the first watcher's stop ended the second's source $p"
  done
  rm -f "$FAKE/hold"
  kill "$b" 2>/dev/null
  wait 2>/dev/null
  pass
fi

fresh "watch: a killed watcher's sources are ended"
if needs_ps; then
  doc >"$FAKE/overview-work.json"
  doc >"$FAKE/watch-work.jsonl"
  # A watcher killed outright left a source running under its pid.
  sleep 60 &
  orphan=$!
  dead=$(sh -c 'echo $$')
  mkdir -p "$S/run/$dead"
  printf '%s\t%s\n' "$orphan" "$(procs identity "$orphan")" >"$S/run/$dead/x.pids"
  SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
  if ! ends_within "$orphan" 20; then
    fail "the dead watcher's source still runs after ${took}s"
    kill "$orphan"
  fi
  [ -d "$S/run/$dead" ] && fail "the dead watcher's sources are still listed"
  pass
fi

fresh "watch: a source is known without ps's start times"
if needs_ps; then
  doc >"$FAKE/overview-work.json"
  doc >"$FAKE/watch-work.jsonl"
  touch "$FAKE/hold"
  PATH="$here/fake-busybox-ps:$PATH" SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 60 2>/dev/null &
  w=$!
  # Three ticks at least, each starting with a look at herdr's panes, while
  # the source is held open.
  within 30 '[ "$(grep -c "^pane list" "$FAKE/herdr-calls")" -ge 12 ]' || fail "the watcher did not tick"
  [ "$(grep -c '^work: overview --watch --json' "$FAKE/sluice-calls")" = 1 ] ||
    fail "a new source every tick: $(grep -c 'overview --watch' "$FAKE/sluice-calls") started"
  kill "$w" 2>/dev/null
  wait "$w" 2>/dev/null
  rm -f "$FAKE/hold"
  pass
fi

fresh 'watch: a watcher stopped in its source'"'"'s first second ends it'
doc >"$FAKE/overview-work.json"
doc >"$FAKE/watch-work.jsonl"
touch "$FAKE/hold"
# The source's identity is read only after ten seconds here.
SLUICE_HERDR_IDENTITY_DELAY=10 SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 60 2>/dev/null &
w=$!
within 15 'grep -q "^work: overview --watch --json" "$FAKE/sluice-calls"' || fail "no source started"
kill -TERM "$w"
wait "$w" 2>/dev/null
within 5 '! pgrep -f "$here/fake/sluice overview --watch" >/dev/null' || fail "the source outlived its watcher"
rm -f "$FAKE/hold"
pkill -f "$here/fake/sluice overview --watch" 2>/dev/null
pass

fresh 'watch: a pid kept alone and used again is left alone'
# A source that ended in its first second left only its pid, since taken
# by another process.
sleep 60 &
other_process=$!
dead=$(sh -c 'echo $$')
mkdir -p "$S/run/$dead"
printf '%s\tpid:%s\n' "$other_process" "$other_process" >"$S/run/$dead/x.pids"
printf '%s\tpid:%s:%s\n' "$other_process" "$other_process" "$dead" >"$S/run/$dead/y.pids"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
kill -0 "$other_process" 2>/dev/null || fail "a process that only has a recorded pid was killed"
kill "$other_process" 2>/dev/null
pass

fresh 'watch: without ps, one source and it is ended'
doc >"$FAKE/overview-work.json"
doc >"$FAKE/watch-work.jsonl"
touch "$FAKE/hold"
# No ps that runs, and on macOS no /proc: nothing can be told by identity.
PATH="$here/fake-no-ps:$PATH" SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 60 2>"$t/log" &
w=$!
within 30 '[ "$(grep -c "^pane list" "$FAKE/herdr-calls")" -ge 12 ]' || fail "the watcher did not tick"
[ "$(grep -c '^work: overview --watch --json' "$FAKE/sluice-calls")" = 1 ] ||
  fail "a new source every tick: $(grep -c 'overview --watch' "$FAKE/sluice-calls") started"
kill "$w" 2>/dev/null
wait "$w" 2>/dev/null
within 5 '! pgrep -f "$here/fake/sluice overview --watch" >/dev/null' || fail "the source outlived its watcher"
rm -f "$FAKE/hold"
pass

fresh "watch: without ps, an ended watcher's sources are left alone"
sleep 60 &
orphan=$!
dead=$(sh -c 'echo $$')
mkdir -p "$S/run/$dead"
printf '%s\tchild:pid:%s\n' "$orphan" "$orphan" >"$S/run/$dead/x.pids"
printf '%s\tpid:%s:%s\n' "$orphan" "$orphan" "$dead" >"$S/run/$dead/y.pids"
PATH="$here/fake-no-ps:$PATH" SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>"$t/log"
kill -0 "$orphan" 2>/dev/null || fail "a pid that could not be checked was ended"
kill "$orphan" 2>/dev/null
# Said where nothing can be read; with /proc (Linux) the pid-only lines
# are simply no one's.
if [ ! -r /proc/$$/stat ]; then
  [ "$(grep -c 'cannot tell an ended watcher' "$t/log")" = 1 ] || fail "not said once: $(cat "$t/log")"
fi
pass

fresh 'watch: a pid used again by another process is left alone'
# After a reboot the state is still there, and the pid is someone else's.
sleep 60 &
other_process=$!
dead=$(sh -c 'echo $$')
mkdir -p "$S/run/$dead"
printf '%s\t%s\n' "$other_process" "start:1" >"$S/run/$dead/x.pids"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
kill -0 "$other_process" 2>/dev/null || fail "a process that only has a recorded pid was killed"
kill "$other_process" 2>/dev/null
[ -d "$S/run/$dead" ] && fail "the dead watcher's sources are still listed"
pass

fresh 'watch: a reinstalled plugin replaces its watcher'
doc >"$FAKE/overview-work.json"
doc >"$FAKE/watch-work.jsonl"
touch "$FAKE/hold"
SLUICE_HERDR_LEASE=4 SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 30 2>/dev/null &
a=$!
sleep 2
# A reinstall is a new plugin directory, here with a segment that says so.
again=$t/reinstalled
cp -R "$plugin" "$again"
echo '# reinstalled' >>"$again/bin/segment.sh"
HERDR_PLUGIN_ROOT=$again SLUICE_HERDR_LEASE=4 SLUICE_HERDR_TICK=1 sh "$again/bin/watch.sh" --ensure
if ! ends_within "$a" 20; then
  fail "the old watcher kept running after ${took}s"
elif [ "$took" -gt 4 ]; then
  echo "note: the old watcher took ${took}s to end"
fi
within 20 '[ "$(sed -n 2p "$S/watch.lease")" = "$again" ]' || fail "the lease is not the reinstalled watcher's: $(cat "$S/watch.lease")"
within 20 'grep -q "# reinstalled" "$HERDR_PLUGIN_STATE_DIR/segment/bin/segment.sh"' || fail "the segment was not copied again"
rm -f "$FAKE/hold"
kill "$a" 2>/dev/null
pass

fresh 'watch: one watcher per herdr server'
HERDR_SOCKET_PATH=/run/one.sock SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ensure
HERDR_SOCKET_PATH=/run/two.sock SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ensure
# The live watchers, one per server's lease.
live_watchers() {
  n=0
  for l in "$HERDR_PLUGIN_STATE_DIR"/servers/*/watch.lease; do
    [ -f "$l" ] && kill -0 "$(holder_of "$l")" 2>/dev/null && n=$((n + 1))
  done
  echo "$n"
}
within 20 '[ "$(live_watchers)" = 2 ]' || fail "$(live_watchers) watchers for two herdr servers"
pass

fresh 'watch: a renewer killed holding the lock'
if needs_ps; then
  # The renewer holds the lock four seconds each time, so it is caught there.
  SLUICE_HERDR_LOCK_HOLD=4 SLUICE_HERDR_LEASE=4 SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 30 2>/dev/null &
  w=$!
  for i in 1 2 3 4 5 6 7 8; do
    [ -f "$S/lease.lock/pid" ] && break
    sleep 1
  done
  owner=$(cut -d' ' -f1 "$S/lease.lock/pid" 2>/dev/null)
  [ -n "$owner" ] || fail "the renewer never took the lock"
  [ "$owner" != "$w" ] || fail "the renewer holds the lock as its watcher"
  [ "$(procs parent_of "$owner")" = "$w" ] || fail "the lock's owner $owner is not the watcher's renewer"
  began=$(date +%s)
  kill -TERM "$w"
  wait "$w" 2>/dev/null
  [ $(($(date +%s) - began)) -lt 5 ] || fail "stop waited on the lock its renewer held"
  [ -f "$S/watch.lease" ] && fail "stop left the lease"
  pass
fi

fresh 'watch: herdr gone'
touch "$FAKE/down"
began=$(date +%s)
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 10 2>/dev/null
[ $(($(date +%s) - began)) -lt 6 ] || fail "the watcher kept going without herdr"
[ -f "$S/watch.lease" ] && fail "the watcher left its lease"
pass

# The state of the herdr server at socket $1.
state_of() {
  echo "$t/state/servers/$(printf '%s' "$1" | cksum | cut -d' ' -f1)"
}

# Whether every process named has ended.
ended() {
  for p in "$@"; do kill -0 "$p" 2>/dev/null && return 1; done
  return 0
}

fresh 'watch: its herdr server stops'
doc >"$FAKE/overview-work.json"
doc >"$FAKE/watch-work.jsonl"
touch "$FAKE/hold"
# The server's socket, here a plain file, which a server removes as it stops.
sock=$t/herdr.sock
: >"$sock"
S=$(state_of "$sock")
HERDR_SOCKET_PATH=$sock SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 60 2>/dev/null &
w=$!
within 15 'sources_of "$w" | grep -q .' || fail "no source started"
theirs=$(sources_of "$w")
rm -f "$sock"
ends_within "$w" 15 || fail "the watcher still runs ${took}s after its server stopped"
within 5 'ended $theirs' || fail "its sources outlived its server"
[ -f "$S/watch.lease" ] && fail "the watcher left its lease"
rm -f "$FAKE/hold"
kill "$w" 2>/dev/null
pass

fresh 'watch: another herdr server in its place'
doc >"$FAKE/overview-work.json"
doc >"$FAKE/watch-work.jsonl"
touch "$FAKE/hold"
sock=$t/herdr.sock
: >"$sock"
S=$(state_of "$sock")
# Its ticks are long, so only the new server's watcher can end it here.
HERDR_SOCKET_PATH=$sock SLUICE_HERDR_TICK=60 sh "$plugin/bin/watch.sh" --ticks 60 2>/dev/null &
a=$!
within 15 'sources_of "$a" | grep -q .' || fail "no source started"
theirs=$(sources_of "$a")
# The server stops and another starts at its path within a tick, with a
# socket of its own, and runs the plugin's startup.
rm -f "$sock"
: >"$sock"
HERDR_SOCKET_PATH=$sock SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ensure
ends_within "$a" 15 || fail "the first server's watcher still runs after ${took}s"
within 5 'ended $theirs' || fail "the first server's sources outlived it"
within 15 'b=$(holder_of "$S/watch.lease") && [ -n "$b" ] && [ "$b" != "$a" ] && kill -0 "$b" 2>/dev/null' ||
  fail "the new server has no watcher: $(cat "$S/watch.lease" 2>/dev/null)"
rm -f "$FAKE/hold"
kill "$a" 2>/dev/null
pass

fresh 'watch: no configuration'
doc >"$FAKE/overview-work.json"
doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 12 '$work' queued)]" >"$FAKE/watch-work.jsonl"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
[ "$(grep -c 'overview --watch --json' "$FAKE/sluice-calls")" = 1 ] || fail "watch ran other than once: $(cat "$FAKE/sluice-calls")"
grep -q '^work: overview --watch --json' "$FAKE/sluice-calls" || fail "watch did not run in the repository with records"
[ -n "$(ls "$S/docs/"*.json 2>/dev/null)" ] || fail "no document kept"
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
set -- "$S"/docs/build--app-*.json
[ -e "$1" ] || fail "its document is not kept under its host and repository"
grep -q 'overview --watch' "$FAKE/sluice-calls" && fail "the default source ran beside the user's"
pass

fresh 'watch: configuration it cannot read'
printf 'ahead_check = false\ncommand = [\n  "a"]\n' >"$HERDR_PLUGIN_CONFIG_DIR/sources.toml"
got=$(sh -c '. "$HERDR_PLUGIN_ROOT/lib/common.sh"; . "$lib/sources.sh"; read_sources' "$plugin/bin/x" 2>"$t/err")
grep -q 'sources.toml:2:' "$t/err" || fail "the error does not name line 2: $(cat "$t/err")"
[ "$(printf '%s' "$got" | jq -r '.sources[0].command | join(" ")')" = "sluice overview --watch --json" ] || fail "not the default: $got"
for bad in 'each_repository = trueish' 'name = "a\q"' 'name = "a" "b"'; do
  printf '[[sources]]\ncommand = ["x"]\n%s\n' "$bad" >"$HERDR_PLUGIN_CONFIG_DIR/sources.toml"
  got=$(sh -c '. "$HERDR_PLUGIN_ROOT/lib/common.sh"; . "$lib/sources.sh"; read_sources' "$plugin/bin/x" 2>"$t/err")
  grep -q 'sources.toml:3:' "$t/err" || fail "$bad: the error does not name line 3: $(cat "$t/err")"
  [ "$(printf '%s' "$got" | jq -r '.sources[0].command | join(" ")' 2>/dev/null)" = "sluice overview --watch --json" ] || fail "$bad: not the default: $got"
done
printf 'ahead_check = false # off\n[[sources]]\nname = "a \\"b\\""\ncommand = ["x", "y"]\neach_repository = true\n' >"$HERDR_PLUGIN_CONFIG_DIR/sources.toml"
got=$(sh -c '. "$HERDR_PLUGIN_ROOT/lib/common.sh"; . "$lib/sources.sh"; read_sources' "$plugin/bin/x")
[ "$got" = '{"ahead_check":false,"sources":[{"name":"a \"b\"","command":["x","y"],"each_repository":true}]}' ] || fail "read as $got"
pass

fresh 'watch: two documents'
feed "$(doc)" "$testing95" "$(doc '.branches[0].recent += [{schema: "sluice/log@1", at: "2026-10-16T19:45:00Z", event: "landed", ids: [95], to: "aaaaaaa"}]')"
f=$(ls "$S"/docs/*.json)
[ "$(jq -r '.branches[0].recent[0].ids[0]' "$f")" = 95 ] || fail "the state file is not the last document"
set -- "$S"/docs/*.tmp.*
[ -e "$1" ] && fail "a temporary file was left"
pass

fresh 'watch: a document missing fields'
feed "$testing95"
tokens
has '--token sluice=#95 testing'
feed "$(doc '.branches[0] |= del(.recent, .submissions, .candidates) | del(.sessions, .previews)')" 2>"$t/err"
for f in "$S"/docs/*.paths; do
  jq -e 'type == "object"' "$f" >/dev/null 2>&1 || fail "$(basename "$f") is no longer an object: $(cat "$f")"
done
tokens 2>>"$t/err"
has '--clear-token sluice'
grep -q 'jq: error' "$t/err" && fail "jq failed: $(cat "$t/err")"
pass

fresh 'watch: a document it cannot read'
feed "$testing95"
f=$(ls "$S"/docs/*.json)
cp "$f" "$t/before"
cp "${f%.json}.paths" "$t/before.paths"
feed "$(doc '.branches = "garbled"')" 2>"$t/err"
cmp -s "$f" "$t/before" || fail "a document it cannot read replaced the last good one"
cmp -s "${f%.json}.paths" "$t/before.paths" || fail "a document it cannot read changed the submissions learned"
grep -q 'cannot read' "$t/err" || fail "nothing logged: $(cat "$t/err")"
pass

fresh 'watch: what a held place failed is kept to its last line'
feed "$(doc '.branches[0].held_checkouts = [{place: "sluice/try/main/48", failed: "preview: stopping\nusage: scripts/preview up\n\n",
      tries: 4, at: "2026-10-16T19:40:00Z", stopped: true}, {place: 7}, "x"]
  | .branches += [{branch: "release", depth: 0}]
  | .sessions = [{worktree: $work, branch: "main", release_failed: {failed: "one\ntwo\n", tries: 1, stopped: true}}]
  | .previews = [{name: "docs", branch: "main", release_failed: {failed: "a\nb", tries: 3, stopped: true}}]')"
f=$(ls "$S"/docs/*.json)
out=$(jq -c '[.branches[] | .held_checkouts | map(.failed)]' "$f")
[ "$out" = '[["usage: scripts/preview up"],[]]' ] || fail "the branches' held checkouts read as $out"
[ "$(jq -r '.sessions[0].release_failed.failed' "$f")" = two ] || fail "a worktree's failure read as $(jq -c .sessions "$f")"
[ "$(jq -r '.previews[0].release_failed.failed' "$f")" = b ] || fail "a preview's failure read as $(jq -c .previews "$f")"
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

# #12 given a subject, #13 none from unit/forecast, under test for 130 s of
# 900 s; changed by the jq in $1.
named_doc() {
  doc ".branches[0].depth = 2 | .branches[0].submissions = [
      {id: 12, state: \"testing\", subject: \"Batch review cards\", from: {agent: \"claude\", worktree: \"unit/review\"}},
      {id: 13, state: \"testing\", from: {agent: \"claude\", worktree: \"unit/forecast\"}}]
    | .branches[0].candidates = [{id: \"c1\", submissions: [12, 13], revision: \"x\", state: \"testing\",
        started: \"2026-10-16T19:43:00Z\", limit: 900, targets: [\"#check-fast\"], executor: \"sandbox\", ran: 0, replayed: 0, failed: []}]
    | ${1:-.}"
}

fresh 'status: a combination names its submissions'
feed "$(named_doc)"
frame_of "$work" --once >"$t/frame"
grep -qxF '    ▶ testing main + #12 Batch review cards + #13 unit/forecast  2m10s / 15m' "$t/frame" ||
  fail "without tasks: $(cat "$t/frame")"
feed "$(named_doc '.branches[0].candidates[0].tasks = [{target: "a:test", state: "passed"}, {target: "b:test", state: "running"}]')"
frame_of "$work" --once >"$t/frame"
grep -qxF '    ▶ testing main + #12 Batch review cards + #13 unit/forecast  1/2 ▓▓▓▓▓░░░░░  2m10s / 15m' "$t/frame" ||
  fail "with tasks: $(cat "$t/frame")"
pass

fresh 'status: a landing of two names both'
feed "$(doc '.branches[0].recent = [{schema: "sluice/log@1", at: "2026-10-16T19:44:00Z", event: "landed", ids: [12, 13], to: "c41e8f2aa0"}]
  | .branches[0].recent_subjects = {"12": "Batch review cards"} | .branches[0].recent_from = {"12": "unit/review", "13": "unit/forecast"}')"
frame_of "$work" --once >"$t/frame"
grep -qxF '    ✓ 19:44 #12 Batch review cards, #13 unit/forecast landed as c41e8f2' "$t/frame" || fail "the landing reads: $(cat "$t/frame")"
pass

fresh 'status: a document that names no subjects names numbers'
feed "$(doc '.branches[0].recent = [{schema: "sluice/log@1", at: "2026-10-16T19:44:00Z", event: "landed", ids: [11], to: "9ecc34a1b2"}]')"
frame_of "$work" --once >"$t/frame"
grep -qxF '    ✓ 19:44 #11 landed as 9ecc34a' "$t/frame" || fail "the landing reads: $(cat "$t/frame")"
pass

fresh 'status: a subject with control characters stays on its line'
# #14 queued with the subject $1, a jq string.
queued14() { doc ".branches[0].depth = 1 | .branches[0].submissions = [{id: 14, state: \"queued\", subject: $1, from: {agent: \"herdr\", worktree: \"pull/41\"}}]"; }
feed "$(queued14 '"Herdr\npopup\u001b[31m"')"
frame_of "$work" --once >"$t/frame"
grep -qxF '    #14 Herdr popup [31m  queued    PR 41 herdr' "$t/frame" || fail "the row reads: $(cat "$t/frame")"
grep -q "$(printf '\033')" "$t/frame" && fail "the subject's escape reached the frame: $(cat -v "$t/frame")"
# Painted, the popup holds the same escapes as with a plain subject.
painted() { printf '%s\n' "$1" | jq -L "$plugin/lib" -r --argjson now "$NOW" 'include "words"; rows($now; true; null)[] | .text' | tr -cd '\033' | wc -c; }
[ "$(painted "$(queued14 '"Herdr\npopup\u001b[31m"')")" = "$(painted "$(queued14 '"Herdr popup"')")" ] ||
  fail "the subject changed the popup's colors"
pass

fresh 'status: tasks redraw with the version unmoved'
feed "$testing95"
(
  sleep 1
  feed "$(printf '%s' "$testing95" | jq -c '.branches[0].candidates[0].tasks = [{target: "a:test", state: "passed",
      started: "2026-10-16T19:42:00Z", ended: "2026-10-16T19:43:05Z", exit: 0}, {target: "b:test", state: "running"}]')"
  printf x
) | SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" >"$t/out"
first=$(awk 'BEGIN{RS="\f"} NR==1' "$t/out")
second=$(awk 'BEGIN{RS="\f"} NR==2' "$t/out")
printf '%s' "$first" | grep -q '░' && fail "a combination whose runner says nothing shows a bar: $first"
printf '%s' "$second" | grep -q '▶ testing main + #95 w  1/2 ▓▓▓▓▓░░░░░  3m10s / 15m' || fail "the redraw does not count the tasks: $second"
printf '%s' "$second" | grep -q '^        ✓ a:test    1m05s$' || fail "the redraw does not list a:test: $second"
printf '%s' "$second" | grep -q '^        … b:test  running$' || fail "the redraw does not list b:test: $second"
pass

fresh 'status: a task list it cannot read is left out'
feed "$(printf '%s' "$testing95" | jq -c '.branches[0].candidates[0].tasks = ["a:test", {state: "passed"}, {target: "c:test", state: "waiting"}]')"
frame_of "$work" --once >"$t/frame"
grep -q '▶ testing main + #95 w  0/1 ░░░░░░░░░░' "$t/frame" || fail "the bad tasks were counted: $(cat "$t/frame")"
grep -q '^        · c:test  waiting$' "$t/frame" || fail "the good task is missing: $(cat "$t/frame")"
pass

fresh 'status: rows are links'
weird="$scratch/a b&c"
mkdir -p "$weird"
blame_logged='.branches[0].candidates = [{id: "c9", submissions: [9], revision: "x", state: "failed", targets: [], executor: null, ran: 1, replayed: 0, failed: [{task: "cli:test", log: "/logs/c9/cli:test.log"}]}]'
both="[$(sub 95 '$work' testing), $(sub 96 "\"$weird\"" queued)]"
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = $both | $blame_logged | $blamed87")"
out=$(SLUICE_HERDR_DIR=$work SLUICE_HERDR_LINKS=1 sh "$plugin/bin/status.sh" --once)
e=$(printf '\033')
printf '%s' "$out" | grep -qF "${e}]8;;sluice-herdr:focus?path=$(printf '%s' "$work" | jq -sRr @uri)${e}\\[1] #95" || fail "no focus link on #95: $(printf '%s' "$out" | cat -v)"
printf '%s' "$out" | grep -qF "sluice-herdr:focus?path=$(printf '%s' "$weird" | jq -sRr @uri)" || fail "the odd path is not encoded"
printf '%s' "$out" | grep -qF "sluice-herdr:log?path=%2Flogs%2Fc9%2Fcli%3Atest.log&task=cli%3Atest" || fail "no log link: $(printf '%s' "$out" | cat -v)"
# The odd path comes back whole: no pane is in it, which the toast names.
sh "$plugin/bin/link.sh" "sluice-herdr:focus?path=$(printf '%s' "$weird" | jq -sRr @uri)"
has "notification show sluice: no pane for $weird"
out=$(SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" --dashboard --once)
printf '%s' "$out" | grep -q '\[1\]' && fail "the dashboard numbers its rows"
pass

fresh 'status: links to paths that are not ASCII'
cafe="$scratch/café"
mkdir -p "$cafe"
logged='.branches[0].candidates = [{id: "c9", submissions: [9], revision: "x", state: "failed", targets: [], executor: null, ran: 1, replayed: 0, failed: [{task: "tést", log: "/logs/é/tést.log"}]}]'
one="[$(sub 97 "\"$cafe\"" queued)]"
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = $one | $logged | $blamed87")"
sh "$plugin/bin/link.sh" "sluice-herdr:focus?path=$(printf '%s' "$cafe" | jq -sRr @uri)" 2>"$t/err"
has "notification show sluice: no pane for $cafe"
HERDR_PLUGIN_CONTEXT_JSON='{"focused_pane_id":"p1"}' sh "$plugin/bin/link.sh" "sluice-herdr:log?path=$(printf '%s' "/logs/é/tést.log" | jq -sRr @uri)" 2>>"$t/err"
has '--env SLUICE_HERDR_LOG=/logs/é/tést.log --env SLUICE_HERDR_TASK=tést'
# Again with gawk as awk in a UTF-8 locale, where a decoder not in the C
# locale makes %C3%A9 two characters.
if command -v gawk >/dev/null 2>&1; then
  mkdir -p "$t/gawk"
  ln -s "$(command -v gawk)" "$t/gawk/awk"
  : >"$FAKE/herdr-calls"
  PATH="$t/gawk:$PATH" LC_ALL=C.UTF-8 sh "$plugin/bin/link.sh" "sluice-herdr:focus?path=$(printf '%s' "$cafe" | jq -sRr @uri)" 2>>"$t/err"
  has "notification show sluice: no pane for $cafe"
  PATH="$t/gawk:$PATH" LC_ALL=C.UTF-8 HERDR_PLUGIN_CONTEXT_JSON='{"focused_pane_id":"p1"}' sh "$plugin/bin/link.sh" "sluice-herdr:log?path=$(printf '%s' "/logs/é/tést.log" | jq -sRr @uri)" 2>>"$t/err"
  has '--env SLUICE_HERDR_LOG=/logs/é/tést.log'
else
  echo "note: $name: no gawk here, so the UTF-8 gawk case did not run"
fi
[ -s "$t/err" ] && fail "logged: $(cat "$t/err")"
pass

fresh 'status: a link focuses, opens, or is left alone'
blame_logged='.branches[0].candidates = [{id: "c9", submissions: [9], revision: "x", state: "failed", targets: [], executor: null, ran: 1, replayed: 0, failed: [{task: "cli:test", log: "/logs/c9/cli:test.log"}]}]'
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 95 '$other' testing)] | $blame_logged")"
sh "$plugin/bin/link.sh" "sluice-herdr:focus?path=$(printf '%s' "$other" | jq -sRr @uri)"
has 'agent focus p2'
HERDR_PLUGIN_CONTEXT_JSON='{"focused_pane_id":"p1"}' sh "$plugin/bin/link.sh" "sluice-herdr:log?path=%2Flogs%2Fc9%2Fcli%3Atest.log&task=cli%3Atest"
has 'plugin pane open --plugin agentplot.sluice --entrypoint log --placement split --direction right --target-pane p1 --env SLUICE_HERDR_LOG=/logs/c9/cli:test.log --env SLUICE_HERDR_TASK=cli:test'
: >"$FAKE/herdr-calls"
sh "$plugin/bin/link.sh" "sluice-herdr:log?path=%2Fetc%2Fpasswd" 2>"$t/err"
sh "$plugin/bin/link.sh" "sluice-herdr:focus?path=%2Felsewhere" 2>>"$t/err"
sh "$plugin/bin/link.sh" "sluice-herdr:run?cmd=rm" 2>>"$t/err"
hasnt 'plugin pane open'
hasnt 'focus'
[ "$(grep -c 'left alone\|not a link' "$t/err")" = 3 ] || fail "not each logged: $(cat "$t/err")"
pass

fresh 'status: the log split'
seq 1 300 | sed 's/^/line /' >"$t/cli.log"
out=$(printf q | SLUICE_HERDR_LOG=$t/cli.log SLUICE_HERDR_TASK=cli:test sh "$plugin/bin/log.sh")
printf '%s\n' "$out" | grep -qx 'log of cli:test' || fail "no heading: $out"
# Not a terminal: its last 200 lines, from line 101.
printf '%s\n' "$out" | grep -qx 'line 101' || fail "the tail does not start at line 101"
printf '%s\n' "$out" | grep -qx 'line 100' && fail "more than the last 200 lines"
printf '%s\n' "$out" | grep -qx 'line 300' || fail "the tail misses the last line"
# q closes it while its input is still open.
mkfifo "$t/keys"
{
  printf 'xq'
  sleep 8
} >"$t/keys" &
writer=$!
began=$(date +%s)
SLUICE_HERDR_LOG=$t/cli.log sh "$plugin/bin/log.sh" <"$t/keys" >/dev/null
[ $(($(date +%s) - began)) -lt 5 ] || fail "q did not close it"
kill "$writer" 2>/dev/null
out=$(printf q | SLUICE_HERDR_LOG=/nowhere/cli.log sh "$plugin/bin/log.sh")
printf '%s\n' "$out" | grep -qx 'cannot read /nowhere/cli.log' || fail "said: $out"
pass

fresh 'status: no control characters from a link or a document'
logged='.branches[0].candidates = [{id: "c9", submissions: [9], revision: "x", state: "failed", targets: [], executor: null, ran: 1, replayed: 0, failed: [{task: "cli:test", log: "/logs/c9/cli:test.log", system: "linux-arm64"}, {task: "evil\u001b]52;c;eA==\u0007", log: "/logs/c9/evil.log"}]}]'
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 95 '$work' testing)] | $logged | $blamed87")"
HERDR_PLUGIN_CONTEXT_JSON='{"focused_pane_id":"p1"}' sh "$plugin/bin/link.sh" "sluice-herdr:log?path=%2Flogs%2Fc9%2Fcli%3Atest.log&task=%1b]52;c;eA==%07"
has '--env SLUICE_HERDR_TASK=cli:test'
e=$(printf '\033')
grep -q "$e" "$FAKE/herdr-calls" && fail "an escape reached herdr: $(cat -v "$FAKE/herdr-calls")"
out=$(SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" --once)
printf '%s' "$out" | grep -q "$e" && fail "an escape reached the popup: $(printf '%s' "$out" | cat -v)"
printf '%s' "$out" | grep -q 'log of evil\]52;c;eA==' || fail "the cleaned task is not shown: $out"
printf '%s' "$out" | grep -q 'log of cli:test (linux-arm64): /logs/c9/cli:test.log' || fail "the system is not named: $out"
pass

fresh 'status: a number does what a click does'
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 95 '$other' testing)]")"
printf '1' | SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" >/dev/null
has 'agent focus p2'
pass

fresh 'status: two hosts'
printf '%s\n' "$board" | sh "$plugin/bin/document.sh" local "$work"
printf '%s\n' "$board" | jq -c '.host = "build" | .repository = "/srv/repo/.git"' | sh "$plugin/bin/document.sh" "build machine" ""
frame_of "$work" --once >"$t/frame"
grep -q "^$common on test\$\|^$common on host-a\$" "$t/frame" || fail "no block for this host"
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
printf '%s' "$first" | grep -q '▶ testing main + #95 w' || fail "the first frame does not show #95 testing"
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
awk 'BEGIN{RS="\f"} NR==2' "$t/out" | grep -q '^/srv/elsewhere.git on host-a' || fail "a does not show every repository"
pass

fresh "status: the focused pane from herdr's context"
feed "$testing95"
ctx=$(jq -c --arg w "$work" '.focused_pane_cwd = $w' "$here/docs/context.json")
HERDR_PLUGIN_CONTEXT_JSON=$ctx sh "$plugin/bin/status.sh" --once | grep -q '▶ testing main + #95 w' || fail "the popup did not read the focused pane's repository"
pass

fresh 'status: a plain directory'
hold_lease
out=$(frame_of "$scratch/home" --once)
printf '%s' "$out" | grep -q '^No sluice records in this repository.$' || fail "said: $out"
echo "none $NOW" >"$S/probe/$(sh -c '. "$HERDR_PLUGIN_ROOT/lib/common.sh"; sum "$1"' "$plugin/bin/x" "$(cd "$scratch/plain/.git" && pwd -P)")"
out=$(frame_of "$scratch/plain" --once)
printf '%s' "$out" | grep -q '^No sluice records in this repository.$' || fail "in a repository without records it said: $out"
out=$(frame_of "$other" --once)
printf '%s' "$out" | grep -q '^Waiting for sluice' || fail "before the watcher read it, it said: $out"
[ -n "$(ls "$S/want")" ] || fail "the watcher was not asked to read it"
pass

# A combination of 30 tasks: 18 passed, a:failed
# failed, b:running running, 10 waiting; and 5 entries under RECENT.
tall=$(printf '%s\n' "$board" | jq -c '.branches[0].candidates[0].tasks = (
    [range(18) | {target: "p\(.):test", state: "passed", started: "2026-10-16T19:43:00Z", ended: "2026-10-16T19:43:\(10 + .)Z"}]
    + [{target: "a:failed", state: "failed", started: "2026-10-16T19:43:00Z", ended: "2026-10-16T19:44:00Z"}, {target: "b:running", state: "running"}]
    + [range(10) | {target: "w\(.):test", state: "waiting"}])
  | .branches[0].recent += [range(3) | {schema: "sluice/log@1", at: "2026-10-16T19:3\(.):00Z", event: "landed", ids: [80 + .], to: "abcdef0"}]')

fresh 'status: a tall frame fits the terminal'
printf '%s\n' "$tall" | sh "$plugin/bin/document.sh" local "$work"
printf q | LINES=20 SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" >"$t/out"
first=$(awk 'BEGIN{RS="\f"} NR==1' "$t/out")
[ "$(printf '%s\n' "$first" | wc -l)" -le 20 ] || fail "the frame is taller than 20 lines: $first"
printf '%s' "$first" | grep -q '^main at 9ecc34a · 3 submissions' || fail "the branch's heading went: $first"
printf '%s' "$first" | grep -q '▶ testing main + #12 Batch review cards + #13 plugin-board  19/30' || fail "the combination went: $first"
printf '%s' "$first" | grep -q '^        ✗ a:failed ' || fail "the failed task went: $first"
printf '%s' "$first" | grep -q '^        … b:running ' || fail "the running task went: $first"
printf '%s' "$first" | grep -q '^        … 18 passed, 10 waiting$' || fail "the tasks are not counted: $first"
printf '%s' "$first" | grep -q 'p0:test' && fail "a passed task is still listed: $first"
printf '%s' "$first" | grep -q '^q close' || fail "the footer went: $first"
# Room enough: every task listed, as before.
printf q | LINES=60 SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" >"$t/out"
awk 'BEGIN{RS="\f"} NR==1' "$t/out" | grep -q '^        · w9:test  *waiting$' || fail "with room the waiting tasks are not listed: $(cat "$t/out")"
# Shorter still: the bottom is cut and said.
printf q | LINES=12 SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" >"$t/out"
first=$(awk 'BEGIN{RS="\f"} NR==1' "$t/out")
[ "$(printf '%s\n' "$first" | wc -l)" -le 12 ] || fail "the frame is taller than 12 lines: $first"
printf '%s' "$first" | grep -q '^        ✗ a:failed ' || fail "at 12 lines the failed task went: $first"
printf '%s' "$first" | grep -q '^… [0-9]* more lines · ↓ scrolls$' || fail "the cut is not said: $first"
pass

fresh 'status: a tall frame scrolls'
printf '%s\n' "$tall" | sh "$plugin/bin/document.sh" local "$work"
# Down twice, Page Down twice, past the end, Up once, then q: six frames.
printf '\033[B\033[B\033[6~\033[6~\033[Aq' | LINES=12 SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" >"$t/out"
nth() { awk -v n="$1" 'BEGIN{RS="\f"} NR==n {sub(/^\n/, ""); print}' "$t/out"; }
second=$(nth 3)
printf '%s' "$second" | grep -q '^… 2 lines above · ↑ scrolls$' || fail "two lines down does not say so: $second"
printf '%s' "$second" | grep -q '^… [0-9]* more lines$' || fail "the lines below are not said: $second"
[ "$(printf '%s\n' "$second" | wc -l)" -le 12 ] || fail "a scrolled frame is taller than 12 lines: $second"
last=$(nth 5)
printf '%s' "$last" | grep -q 'more lines' && fail "the end of the rows still says more: $last"
printf '%s' "$last" | grep -q '✓ 19:32 #82 landed' || fail "the last row is not reached: $last"
[ "$(printf '%s\n' "$last" | wc -l)" -le 12 ] || fail "the last frame is taller than 12 lines: $last"
above=$(printf '%s' "$last" | sed -n 's/^… \([0-9]*\) lines above.*/\1/p')
up=$(nth 6)
printf '%s' "$up" | grep -q "^… $((above - 1)) lines above" || fail "Up from the end does not move one line: $up"
pass

fresh 'status: a branch with nothing queued is left out'
feed "$(printf '%s\n' "$board" | jq -c '.branches += [range(30) | {branch: "feature/b\(.)", at: "1234567", depth: 0, submissions: [], candidates: [], recent: []}]')"
out=$(frame_of "$work" --once)
printf '%s' "$out" | grep -q 'feature/b' && fail "an idle branch is shown: $out"
printf '%s' "$out" | grep -q '^main at 9ecc34a' || fail "the busy branch went: $out"
printf '%s' "$out" | grep -q '^30 other branches with nothing queued$' || fail "the idle branches are not counted: $out"
feed "$(printf '%s\n' "$board" | jq -c '.branches |= map(.depth = 0 | .submissions = [] | .candidates = [] | .recent = [])')"
out=$(frame_of "$work" --once)
printf '%s' "$out" | grep -q '^No branch has a queue.$' || fail "with every branch idle it said: $out"
printf '%s' "$out" | grep -q 'other branch' && fail "with every branch idle it counts them: $out"
pass

fresh 'status: an idle branch is shown a quarter of an hour after its last event'
fresh_doc() { printf '%s\n' "$board" | jq -c --arg at "$1" '.branches += [{branch: "feature/done", at: "1234567", depth: 0, submissions: [], candidates: [],
  recent: [{schema: "sluice/log@1", at: $at, event: "landed", ids: [77], to: "abcdef0"}]}]'; }
feed "$(fresh_doc 2026-10-16T19:40:00Z)"
frame_of "$work" --once | grep -q '^feature/done at 1234567' || fail "a landing five minutes old is not shown"
feed "$(fresh_doc 2026-10-16T19:20:00Z)"
out=$(frame_of "$work" --once)
printf '%s' "$out" | grep -q 'feature/done' && fail "a landing 25 minutes old is still shown: $out"
pass

fresh 'status: a pane outside git shows every repository'
hold_lease
feed "$testing95"
out=$(frame_of "$scratch/home" --once)
printf '%s' "$out" | grep -qF "$scratch/home is in no git repository. Every repository held:" || fail "said: $out"
printf '%s' "$out" | grep -q '▶ testing main + #95 w' || fail "the held repository is not shown: $out"
printf '%s' "$out" | grep -q 'No sluice records' && fail "it still says no records: $out"
pass

fresh 'status: without sluice'
mkdir -p "$t/bin"
for c in git jq awk cksum; do ln -s "$(command -v "$c")" "$t/bin/$c"; done
ln -s "$here/fake/herdr" "$t/bin/herdr"
PATH="$t/bin:/usr/bin:/bin" SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>"$t/log"
grep -q 'sluice is not on PATH' "$t/log" || fail "the log does not say so: $(cat "$t/log")"
out=$(PATH="$t/bin:/usr/bin:/bin" frame_of "$work" --once)
printf '%s' "$out" | grep -q 'sluice is not on PATH' || fail "the popup said: $out"
pass

fresh 'status: a sluice without overview'
touch "$FAKE/old"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>"$t/log"
grep -q 'sluice 0.11.0 has no overview' "$t/log" || fail "the log does not say so: $(cat "$t/log")"
out=$(frame_of "$work" --once)
printf '%s' "$out" | grep -q 'sluice 0.11.0 has no overview' || fail "the popup said: $out"
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
[ "$out" = 'sluice main · 3 submissions · testing main + #12 Batch review cards + #13 plugin-board 3/5 ▓▓▓▓▓▓░░░░ 2m10s/15m · #14 Herdr popup waiting · last #9 unit/sidebar blamed on 4f1c2aa' ] || fail "the segment printed: $out"
feed "$(doc '.host = "host-a"')"
out=$(cd "$work" && sh "$seg")
[ -z "$out" ] || fail "with no queue it printed: $out"
pass

fresh 'status: the segment names what waits and the last event'
feed "$(doc '.branches[0].depth = 2 | .branches[0].submissions = [
    {id: 14, state: "queued", subject: "Herdr popup", from: {agent: "herdr", worktree: "pull/41"}},
    {id: 15, state: "queued", from: {agent: "claude", worktree: "unit/toasts"}}]
  | .branches[0].recent = [{schema: "sluice/log@1", at: "2026-10-16T19:41:00Z", event: "landed", ids: [11], to: "9ecc34a1b2"}]
  | .branches[0].recent_subjects = {"11": "Fix the dates"} | .branches[0].recent_from = {"11": "unit/dates"}')"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
out=$(cd "$work" && sh "$HERDR_PLUGIN_STATE_DIR/segment/bin/segment.sh")
[ "$out" = 'sluice main · 2 submissions · #14 Herdr popup, #15 unit/toasts waiting · last #11 Fix the dates landed as 9ecc34a' ] ||
  fail "the segment printed: $out"
pass

fresh "status: another repository's document leaves the segment alone"
printf '%s\n' "$board" | jq -c --arg r "$common" '.repository = $r' | sh "$plugin/bin/document.sh" local "$work"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
# Another repository's document that is not JSON, beside the focused one's.
echo '{"schema": "sluice/overview@1", "host": "host-a", "repo' >"$S/docs/host-a--other-1.json"
out=$(cd "$work" && sh "$HERDR_PLUGIN_STATE_DIR/segment/bin/segment.sh")
[ "$out" = 'sluice main · 3 submissions · testing main + #12 Batch review cards + #13 plugin-board 3/5 ▓▓▓▓▓▓░░░░ 2m10s/15m · #14 Herdr popup waiting · last #9 unit/sidebar blamed on 4f1c2aa' ] ||
  fail "beside another repository's broken document the segment printed: $out"
pass

# The board with agents' checks: e2e-run running a command, fix-cards its
# checks with 3 of 8 tasks ended, plugin-board's ended 2 minutes ago with
# 14 passed and reused, stale's 45 minutes ago, and a worktree that only
# has main checked out; two of the host's six runs are agents'.
checks_board=$(printf '%s\n' "$board" | jq -c '.room = {bound: 6, queue: 1, sessions: 2, reserve: 0}
  | .sessions = [
      {worktree: "/work/sluice/e2e-run", branch: "main", ports: {}, last_run: "2026-10-16T19:44:40Z", removed: false,
       run: {started: "2026-10-16T19:44:40Z", command: "./scripts/e2e.sh"}},
      {worktree: "/work/sluice/fix-cards", branch: "main", ports: {web: "4310"}, last_run: "2026-10-16T19:44:00Z", removed: false,
       run: {started: "2026-10-16T19:44:00Z",
             tasks: ([range(3) | {target: "t\(.):test", state: "passed"}] + [range(5) | {target: "w\(.):test", state: "waiting"}])}},
      {worktree: "/work/sluice/plugin-board", branch: "main", ports: {}, last_run: "2026-10-16T19:40:00Z", removed: false,
       last_result: {at: "2026-10-16T19:43:10Z", head: "abc1234", kind: "checks", outcome: "pass", recorded: true, reused: true,
                     tasks: [range(14) | {target: "p\(.):test", state: "passed"}]}},
      {worktree: "/work/sluice/stale", branch: "main", ports: {},
       last_result: {at: "2026-10-16T19:00:00Z", head: "abc1234", kind: "checks", outcome: "pass", recorded: true, reused: true}},
      {worktree: "/work/sluice/held", branch: "main", holds_queued_branch: "main"}]')

fresh "status: agents' checks are load on the host"
printf '%s\n' "$checks_board" | jq -c --arg repo "$common" '.repository = $repo' | sh "$plugin/bin/document.sh" local "$work"
frame_of "$work" --once >"$t/frame"
sed "s|$common|REPO|" "$t/frame" >"$t/got"
diff -u "$here/docs/checks.frame" "$t/got" >"$t/diff" || fail "the frame differs: $(cat "$t/diff")"
grep -Eq 'testing.*e2e-run|#.*fix-cards' "$t/frame" && fail "a check is shown as a submission or as testing: $(cat "$t/frame")"
pass

fresh 'status: a finished check says what it is worth'
finished() {
  # $1 the last_result's fields, as jq
  feed "$(printf '%s\n' "$checks_board" | jq -c --arg repo "$common" ".repository = \$repo | .sessions[2].last_result |= ($1)")"
  frame_of "$work" --once | grep '^    plugin-board → main '
}
out=$(finished '.recorded = false | .reused = false')
[ "$out" = '    plugin-board → main 14 passed · not kept: uncommitted changes' ] || fail "not kept reads: $out"
out=$(finished '.reused = false')
[ "$out" = '    plugin-board → main 14 passed' ] || fail "kept, not reused, reads: $out"
out=$(finished '.outcome = "fail" | .reused = false | .tasks[0:2] |= map(.state = "failed")')
[ "$out" = '    plugin-board → main 2 of 14 failed' ] || fail "a failure reads: $out"
out=$(finished '.kind = "command" | .outcome = "fail" | .recorded = false | del(.reused, .tasks)')
[ "$out" = '    plugin-board → main failed' ] || fail "a command's failure reads: $out"
pass

fresh 'status: the band says when checks hold the queue back'
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
seg=$HERDR_PLUGIN_STATE_DIR/segment/bin/segment.sh
band='sluice main · 3 submissions · testing main + #12 Batch review cards + #13 plugin-board 3/5 ▓▓▓▓▓▓░░░░ 2m10s/15m · #14 Herdr popup waiting · last #9 unit/sidebar blamed on 4f1c2aa'
segment_with() {
  # $1 the document's changes, as jq
  feed "$(printf '%s\n' "$board" | jq -c --arg r "$common" ".repository = \$r | $1")"
  (cd "$work" && sh "$seg")
}
out=$(segment_with '.branches[0].window = 2 | .room = {bound: 2, queue: 1, sessions: 1, reserve: 0}')
[ "$out" = "$band · 1 slot to an agent's checks" ] || fail "held back by one check it printed: $out"
out=$(segment_with '.branches[0].window = 2 | .room = {bound: 3, queue: 1, sessions: 2, reserve: 0}')
[ "$out" = "$band · 2 slots to agents' checks" ] || fail "held back by two checks it printed: $out"
out=$(segment_with '.branches[0].window = 2 | .room = {bound: 3, queue: 1, sessions: 1, reserve: 0}')
[ "$out" = "$band" ] || fail "with a run free it printed: $out"
out=$(segment_with '.room = {bound: 2, queue: 1, sessions: 1, reserve: 0}')
[ "$out" = "$band" ] || fail "with the window full it printed: $out"
# #14's combination passed: it waits on no run.
out=$(segment_with '.branches[0].window = 2 | .room = {bound: 2, queue: 1, sessions: 1, reserve: 0}
  | .branches[0].submissions |= map(if .id == 14 then .state = "passed" else . end)')
[ "$out" = "$band" ] || fail "with nothing queued it printed: $out"
pass

fresh "status: branches keep the overview's order"
feed "$(printf '%s\n' "$board" | jq -c '.branches = [(.branches[0] | .branch = "release"),
  {branch: "feature/a", at: "1234567", depth: 0, submissions: [], candidates: [], recent: []}, .branches[0]]')"
out=$(frame_of "$work" --once)
[ "$(printf '%s\n' "$out" | sed -n -E 's/^(release|main) at .*/\1/p' | tr '\n' ' ')" = 'release main ' ] ||
  fail "the branches are not in the document's order: $out"
printf '%s' "$out" | grep -q '^1 other branch with nothing queued$' || fail "the idle branch is not counted: $out"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
out=$(cd "$work" && sh "$HERDR_PLUGIN_STATE_DIR/segment/bin/segment.sh")
case $out in "sluice release · "*"  |  sluice main · "*) ;; *) fail "the segment is not in the document's order: $out" ;; esac
printf '\033[B\033[Bq' | LINES=12 SLUICE_HERDR_DIR=$work sh "$plugin/bin/status.sh" >"$t/out"
third=$(awk 'BEGIN{RS="\f"} NR==3 {sub(/^\n/, ""); print}' "$t/out")
printf '%s' "$third" | grep -q '^… 2 lines above · ↑ scrolls$' || fail "it does not scroll: $third"
[ "$(printf '%s\n' "$third" | wc -l)" -le 12 ] || fail "a scrolled frame is taller than 12 lines: $third"
pass

focused_p1=$(jq -c --arg w "$work" '.focused_pane_cwd = $w | .focused_pane_id = "p1"' "$here/docs/context.json")

fresh "status: the focused pane's named branch comes first"
feed "$(printf '%s\n' "$board" | jq -c '.branches = [(.branches[0] | .branch = "release"),
  {branch: "feature/a", at: "1234567", depth: 0, submissions: [], candidates: [], recent: []}, .branches[0]]')"
named p1 feature/a
out=$(HERDR_PLUGIN_CONTEXT_JSON=$focused_p1 sh "$plugin/bin/status.sh" --once)
[ "$(printf '%s\n' "$out" | sed -n -E 's/^(release|main|feature\/a) at .*/\1/p' | tr '\n' ' ')" = 'feature/a release main ' ] ||
  fail "the named branch is not first: $out"
printf '%s' "$out" | grep -q 'with nothing queued$' && fail "the named branch is counted with nothing queued: $out"
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
out=$(cd "$work" && sh "$HERDR_PLUGIN_STATE_DIR/segment/bin/segment.sh")
case $out in "sluice release · "*"  |  sluice main · "*) ;; *) fail "the segment moved: $out" ;; esac
pass

fresh 'status: a named branch with no records'
feed "$testing95"
named p1 hotfix
out=$(HERDR_PLUGIN_CONTEXT_JSON=$focused_p1 sh "$plugin/bin/status.sh" --once)
first=$(printf '%s\n' "$out" | grep -E -m1 '^(hotfix|main)( at | · )')
[ "$first" = 'hotfix · no records' ] || fail "the first branch line is '$first': $out"
pass

# main's checkout sluice/try/main/48, held after four failed releases;
# a worktree of main held after its one; the rows they draw.
held48='.branches[0].held_checkouts = [{place: "sluice/try/main/48", failed: "the release command ended exit status: 2\nusage: scripts/preview up",
    tries: 4, at: "2026-10-16T19:40:00Z", stopped: true}]'
held_unit_a='.sessions += [{worktree: "/work/sluice/unit-a", branch: "main", ports: {},
    release_failed: {failed: "a\nrm: cannot remove .stack", tries: 1, at: "2026-10-16T19:41:00Z", stopped: true}}]'
rows48='  RELEASE FAILED
    ✗ sluice/try/main/48 held · failed 4 times · usage: scripts/preview up
        sluice release sluice/try/main/48 · --accept-failure frees it without'
# The rows under main's heading, $1 of them, in the popup of the documents fed.
under_main() {
  frame_of "$work" --once | sed -n '/^main at /,$p' | sed -n "2,$(($1 + 1))p"
}

fresh 'status: a held checkout'
feed "$(doc "$held48 | .branches[0].depth = 1 | .branches[0].submissions = [$(sub 95 '$work' queued)]")"
out=$(under_main 4)
[ "$out" = "$rows48
  QUEUED" ] || fail "under main the popup shows: $out"
pass

fresh 'status: a held worktree'
feed "$(doc "$held_unit_a")"
out=$(under_main 3)
[ "$out" = '  RELEASE FAILED
    ✗ worktree unit-a held · failed once · rm: cannot remove .stack
        sluice release --worktree /work/sluice/unit-a · --accept-failure frees it without' ] ||
  fail "under main the popup shows: $out"
pass

fresh "status: another host's held checkout"
feed "$(doc "$held48 | .host = \"vega\" | .branches[0].held_checkouts += [.branches[0].held_checkouts[0]
  | .place = \"sluice/try/main/50\" | .host = \"vega\"] | .branches[0].held_checkouts[0].host = \"deneb\"")"
out=$(under_main 5)
[ "$out" = '  RELEASE FAILED
    ✗ sluice/try/main/48 held on deneb · failed 4 times · usage: scripts/preview up
        on deneb: sluice release sluice/try/main/48 · --accept-failure frees it without
    ✗ sluice/try/main/50 held · failed 4 times · usage: scripts/preview up
        sluice release sluice/try/main/50 · --accept-failure frees it without' ] ||
  fail "under main the popup shows: $out"
pass

fresh 'status: an idle branch with a held place'
feed "$(doc "$held48 | $release_idle")"
out=$(frame_of "$work" --once)
[ "$(under_main 3)" = "$rows48" ] || fail "main is not shown with its held checkout: $out"
printf '%s' "$out" | grep -q '^1 other branch with nothing queued$' || fail "release is not counted as idle: $out"
printf '%s' "$out" | grep -q 'No branch has a queue' && fail "it says no branch has a queue: $out"
pass

fresh 'status: a checkout still being tried'
feed "$(doc "$held48 | .branches[0].held_checkouts[0] |= (.stopped = false | .tries = 1)
  | .branches[0].depth = 1 | .branches[0].submissions = [$(sub 95 '$work' queued)]")"
out=$(frame_of "$work" --once)
printf '%s' "$out" | grep -q 'RELEASE FAILED\|sluice/try/main/48' && fail "a checkout still tried is shown: $out"
pass

fresh 'status: a document from before held places'
feed "$(doc '.branches[0].depth = 1 | .branches[0].submissions = [{id: 95, state: "queued", from: {agent: "claude", worktree: "w"}}]')"
frame_of "$work" --once >"$t/old"
feed "$(doc '.branches[0].depth = 1 | .branches[0].submissions = [{id: 95, state: "queued", from: {agent: "claude", worktree: "w"}}]
  | .branches[0].held_checkouts = [] | .sessions = [{worktree: "/w/x", branch: "main", ports: {}}]')"
frame_of "$work" --once >"$t/new"
grep -q 'RELEASE FAILED' "$t/old" && fail "an older document shows RELEASE FAILED: $(cat "$t/old")"
sed '/worktree held/d; /worktrees held/d' "$t/old" >"$t/old.rows"
sed '/worktree held/d; /worktrees held/d' "$t/new" >"$t/new.rows"
cmp -s "$t/old.rows" "$t/new.rows" || fail "an older document draws otherwise: $(diff "$t/old.rows" "$t/new.rows")"
pass

fresh 'status: the segment counts held places'
SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
band='sluice main · 3 submissions · testing main + #12 Batch review cards + #13 plugin-board 3/5 ▓▓▓▓▓▓░░░░ 2m10s/15m · #14 Herdr popup waiting · last #9 unit/sidebar blamed on 4f1c2aa'
feed "$(printf '%s\n' "$board" | jq -c --arg r "$common" ".repository = \$r | $held48")"
out=$(cd "$work" && sh "$HERDR_PLUGIN_STATE_DIR/segment/bin/segment.sh")
[ "$out" = "$band · 1 held: release failed" ] || fail "beside a queue the segment printed: $out"
# The same host's document, in place of the board's.
feed "$(doc "$held48 | $held_unit_a | .host = \"host-a\"")"
out=$(cd "$work" && sh "$HERDR_PLUGIN_STATE_DIR/segment/bin/segment.sh")
[ "$out" = 'sluice main · 0 submissions · 2 held: release failed' ] || fail "with nothing queued the segment printed: $out"
pass

fresh 'status: setup shows, never edits'
before=$(find "$HERDR_PLUGIN_CONFIG_DIR" "$scratch/home" -type f | sort)
# A state directory outside the home directory is named whole.
out=$(HOME=$scratch/home sh "$plugin/bin/setup.sh" </dev/null)
for want in 'key = "prefix+s"' 'command = "agentplot.sluice.status"' '"$sluice"' '"$sluice_q"' \
  "sh '$HERDR_PLUGIN_STATE_DIR/segment/bin/segment.sh'" 'interval_seconds = 5, timeout_seconds = 30 }'; do
  printf '%s' "$out" | grep -qF -- "$want" || fail "setup does not show $want"
done
[ "$(find "$HERDR_PLUGIN_CONFIG_DIR" "$scratch/home" -type f | sort)" = "$before" ] || fail "setup wrote a file"
[ -s "$FAKE/herdr-calls" ] && fail "setup called herdr: $(calls)"
pass

fresh 'status: one tab-bar line for every host'
lines=
for h in mac box; do
  home=$t/$h
  st=$home/.local/state/herdr/plugins/agentplot.sluice
  mkdir -p "$st"
  printf '%s\n' "$board" | jq -c --arg r "$common" --arg h "$h" '.repository = $r | .branches[0].branch = "main-\($h)"' |
    HERDR_PLUGIN_STATE_DIR=$st sh "$plugin/bin/document.sh" local "$work"
  HOME=$home HERDR_PLUGIN_STATE_DIR=$st SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
  line=$(HOME=$home HERDR_PLUGIN_STATE_DIR=$st sh "$plugin/bin/setup.sh" </dev/null | grep 'type = "command"')
  lines="$lines$line
"
  cmd=$(printf '%s' "$line" | sed -n "s/.*command = '\(.*\)', interval_seconds.*/\1/p")
  [ "$cmd" = 'sh "$HOME/.local/state/herdr/plugins/agentplot.sluice/segment/bin/segment.sh"' ] || fail "on $h setup printed: $line"
  # As herdr runs it: through a login shell, in the focused pane's directory.
  out=$(cd "$work" && env -u HERDR_PLUGIN_ROOT -u HERDR_PLUGIN_STATE_DIR HOME="$home" /bin/sh -lc "$cmd")
  case $out in "sluice main-$h · 3 submissions · "*) ;; *) fail "on $h the line printed: $out" ;; esac
done
[ "$(printf '%s' "$lines" | sort -u | wc -l)" -eq 1 ] || fail "the hosts' lines differ: $lines"
pass

fresh "status: the segment is the server's"
home=$t/server
st=$home/.local/state/herdr/plugins/agentplot.sluice
sock=$t/herdr.sock
mkdir -p "$st"
printf '%s\n' "$board" | jq -c --arg r "$common" '.repository = $r' |
  HERDR_SOCKET_PATH=$sock HERDR_PLUGIN_STATE_DIR=$st sh "$plugin/bin/document.sh" local "$work"
HOME=$home HERDR_SOCKET_PATH=$sock HERDR_PLUGIN_STATE_DIR=$st SLUICE_HERDR_TICK=1 sh "$plugin/bin/watch.sh" --ticks 1 2>/dev/null
cmd=$(HOME=$home HERDR_PLUGIN_STATE_DIR=$st sh "$plugin/bin/setup.sh" </dev/null | sed -n "s/.*command = '\(.*\)', interval_seconds.*/\1/p")
# As herdr runs it on the server, whatever host the client is on: the
# server's home, socket and PATH, nothing else, so the clock is the host's
# own and the time under test is however long ago the board's run started.
out=$(cd "$work" && env -i HOME="$home" HERDR_SOCKET_PATH="$sock" PATH="$PATH" /bin/sh -lc "$cmd")
case $out in 'sluice main · 3 submissions · testing main + #12 Batch review cards + #13 plugin-board 3/5 ▓▓▓▓▓▓░░░░ '*'/15m · #14 Herdr popup waiting · last #9 unit/sidebar blamed on 4f1c2aa') ;;
*) fail "with only the server's environment it printed: $out" ;;
esac
pass

fresh 'status: fields it does not know change nothing'
status_of 87 "$work" withdrawn '["tests:t-cards"]'
extra='.later = {"anything": 1} | .branches[0].candidates |= map(.tasks = [{"name": "x", "state": "running"}])'
# The same documents read twice: as sluice prints them today, and with
# fields a later sluice may add.
for run in plain extra; do
  fresh_state=$t/$run
  mkdir -p "$fresh_state"
  : >"$FAKE/herdr-calls"
  if [ "$run" = extra ]; then e=$extra; else e=.; fi
  one=$(doc "$e | .branches[0].depth = 1 | .branches[0].submissions = [$(sub 95 '$work' testing)] | .branches[0].candidates = [{id: \"c1\", submissions: [95], revision: \"x\", state: \"testing\", started: \"2026-10-16T19:42:00Z\", limit: 900, targets: [\"#check-fast\"], executor: \"sandbox\", ran: 0, replayed: 0, failed: []}] | $e")
  two=$(doc "$e | $blamed87")
  {
    HERDR_PLUGIN_STATE_DIR=$fresh_state feed "$(doc "$e")" "$one"
    HERDR_PLUGIN_STATE_DIR=$fresh_state frame_of "$work" --once >"$t/$run.frame"
    HERDR_PLUGIN_STATE_DIR=$fresh_state tokens
    HERDR_PLUGIN_STATE_DIR=$fresh_state feed "$two"
    HERDR_PLUGIN_STATE_DIR=$fresh_state tokens
  } 2>>"$t/$run.err"
  sed 's/--seq [0-9]*//' "$FAKE/herdr-calls" >"$t/$run.calls"
done
grep -q '"later"' "$t"/extra/servers/*/docs/*.json || fail "the extra fields were not kept"
cmp -s "$t/plain.frame" "$t/extra.frame" || fail "the popup differs: $(diff "$t/plain.frame" "$t/extra.frame")"
cmp -s "$t/plain.calls" "$t/extra.calls" || fail "the herdr calls differ: $(diff "$t/plain.calls" "$t/extra.calls")"
grep -q 'notification show sluice: #87 blamed' "$t/extra.calls" || fail "no toast with the extra fields"
grep -q 'token sluice=#95 testing' "$t/extra.calls" || fail "no token with the extra fields"
[ -s "$t/extra.err" ] && fail "something was logged: $(cat "$t/extra.err")"
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

fresh 'toasts: two watchers tell an event once'
feed "$(doc)"
for i in 1 2 3 4 5; do
  status_of "$((80 + i))" "$work" withdrawn '["t"]'
  d=$(doc ".branches[0].recent += [{schema: \"sluice/log@1\", at: \"2026-10-16T19:44:0${i}Z\", event: \"blamed\", id: $((80 + i)), combination: \"c\"}]")
  printf '%s\n' "$d" | sh "$plugin/bin/document.sh" local "$work" &
  printf '%s\n' "$d" | sh "$plugin/bin/document.sh" local "$work" &
  wait
done
[ "$(grep -c 'notification show' "$FAKE/herdr-calls")" = 5 ] || fail "not one toast per blame: $(grep 'notification show' "$FAKE/herdr-calls")"
pass

fresh 'toasts: what is told is kept only while it is recent'
status_of 53 "$work" withdrawn
feed "$(doc "$blamed87 | $landed94")" "$(doc "$landed94 | $conflict53")"
d=$(ls -d "$S"/told.d/*)
got=$(for e in "$d"/*; do echo "${e##*/}"; done | sort)
want=$(for k in 'main|conflict|53|2026-10-16T19:44:30Z' 'main|landed|94|2026-10-16T19:33:10Z'; do
  sh -c '. "$HERDR_PLUGIN_ROOT/lib/common.sh"; digest "$1"' "$plugin/bin/x" "$k"
done | sort)
[ "$got" = "$want" ] || fail "told holds: $got, not $want"
pass

fresh 'toasts: an event on a long branch, or of many submissions'
long=$(printf 'b%.0s' $(seq 1 200))
on_long=".branches[0].branch = \"$long\""
many='.branches[0].recent += [{schema: "sluice/log@1", at: "2026-10-16T19:44:40Z", event: "landed", ids: [range(100; 161)], to: "aaaaaaa"}]'
first=$(doc "$on_long")
second=$(doc "$on_long | $blamed87 | $many")
feed "$first" "$second" 2>"$t/err"
has "notification show sluice: #87 blamed on $long"
n=0
for f in "$S"/told.d/*/*; do
  [ "${#f}" -lt 400 ] || fail "a told name is ${#f} long"
  n=$((n + 1))
done
[ "$n" = 2 ] || fail "not both events are recorded as told: $(cat "$t/err")"
pass

fresh 'toasts: none when what was told cannot be kept'
status_of 87 "$work" withdrawn '["t"]'
status_of 53 "$work" withdrawn
feed "$(doc)"
# Unwritable even to root, as the containers run it: a file where the
# repository's directory of told events was.
for d in "$S"/told.d/*; do
  rmdir "$d" && : >"$d"
done
for i in 1 2 3; do feed "$(doc "$blamed87 | $conflict53 | .branches[0].version = $i")"; done 2>"$t/err"
hasnt 'notification show'
[ "$(grep -c 'cannot record' "$t/err")" = 1 ] || fail "not logged once: $(cat "$t/err")"
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

fresh 'sidebar: testing, with tasks'
feed "$(printf '%s' "$testing95" | jq -c '.branches[0].candidates[0].tasks = [{target: "a:test", state: "passed"}, {target: "b:test", state: "running"}]')"
tokens
has '--token sluice=#95 testing 1/2 ▓▓▓▓▓░░░░░ 3m/15m'
pass

checking='{worktree: $work, branch: "main", ports: {}, run: {started: "2026-10-16T19:43:10Z",
  tasks: [{target: "a:test", state: "passed"}, {target: "b:test", state: "running"}, {target: "c:test", state: "waiting"}]}}'

fresh 'sidebar: checking'
feed "$(doc ".sessions = [$checking]")"
tokens
has '--token sluice=checking 1/3'
hasnt 'testing'
pass

fresh 'sidebar: checking a command'
feed "$(doc ".sessions = [$checking | .run = {started: .run.started, command: \"make e2e\"}]")"
tokens
has '--token sluice=checking 2m'
pass

fresh 'sidebar: a submission comes before a check'
feed "$(printf '%s' "$testing95" | jq -c --arg work "$work" ".sessions = [$checking]")"
tokens
has '--token sluice=#95 testing 3m/15m'
pass

fresh 'sidebar: a blame comes before a check'
status_of 87 "$work" withdrawn '["tests:t-cards"]'
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 87 '$work' testing)]")" "$(doc "$blamed87 | .sessions = [$checking]")"
tokens
has '--token sluice=#87 BLAMED · tests:t-cards'
hasnt 'checking'
pass

fresh 'sidebar: blamed'
status_of 87 "$work" withdrawn '["tests:t-cards"]'
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 87 '$work' testing)]")" "$(doc "$blamed87")"
tokens
has '--token sluice=#87 BLAMED · tests:t-cards'
pass

fresh "sidebar: blamed, a command runner's task"
status_of 87 "$work" withdrawn '["sleep 15; test ! -f bad"]'
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 87 '$work' testing)]")" "$(doc "$blamed87")"
tokens
has '--token sluice=#87 BLAMED'
hasnt 'BLAMED ·'
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

fresh 'sidebar: ahead again after a landing'
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 94 '$work' testing)]")" "$(doc "$landed94")"
forecast
tokens
[ "$(grep -c 'work: submit main --check --json' "$FAKE/sluice-calls")" = 1 ] || fail "no forecast for a worktree whose last submission landed"
has '--token sluice=3 ahead · clean'
pass

fresh 'sidebar: conflicts'
echo '{"conflict":{"paths":["docs/architecture/builds.md","b.rs"],"with":[{"id":95,"from":{"agent":"x","worktree":"y"}}]}}' >"$FAKE/check.json"
feed "$(doc)"
forecast
tokens
has '--token sluice=3 ahead · conflicts #95 in builds.md +1'
# A forecast kept before it named its files says what it did then.
for c in "$S"/checks/*; do
  jq -c 'del(.paths)' "$c" >"$c.new" && mv "$c.new" "$c"
done
: >"$FAKE/herdr-calls"
tokens
has '--token sluice=3 ahead · conflicts #95'
hasnt '3 ahead · conflicts #95 in'
pass

fresh 'sidebar: conflicts with what landed'
echo '{"conflict":{"paths":["docs/spec/identity.md"],"with":[]}}' >"$FAKE/check.json"
feed "$(doc)"
forecast
tokens
has '--token sluice=3 ahead · conflicts main in identity.md'
hasnt 'identity.md +'
pass

fresh 'sidebar: nothing changed'
feed "$(doc)"
forecast
feed "$(doc '.at = "2026-10-16T19:46:00Z"')"
forecast
[ "$(grep -c 'submit main --check' "$FAKE/sluice-calls")" = 1 ] || fail "forecast again: $(cat "$FAKE/sluice-calls")"
# A combination starting moves the branch's version, not what the forecast
# rests on.
feed "$(doc '.branches[0].version = 2 | .branches[0].candidates = [{id: "c1", submissions: [], revision: "x", state: "testing"}]')"
forecast
[ "$(grep -c 'submit main --check' "$FAKE/sluice-calls")" = 1 ] || fail "forecast on a new version alone"
# A submission waiting on the branch does.
feed "$(doc ".branches[0].version = 3 | .branches[0].depth = 1 | .branches[0].submissions = [$(sub 96 '$other' queued)]")"
forecast
[ "$(grep -c 'submit main --check' "$FAKE/sluice-calls")" = 2 ] || fail "no forecast on a new submission waiting"
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

fresh 'sidebar: a named branch'
named p1 release
feed "$(doc "$release_busy")"
tokens
has '--token sluice=release · 3 queued · #12 testing 3/8 · 2 checking'
pass

fresh 'sidebar: a submission comes before a named branch'
named p1 release
feed "$(printf '%s' "$testing95" | jq -c --arg main "$main" "$release_busy")"
tokens
has '--token sluice=#95 testing 3m/15m'
hasnt 'sluice=release'
pass

fresh 'sidebar: a blame comes before a named branch'
status_of 87 "$work" withdrawn '["tests:t-cards"]'
named p1 release
feed "$(doc ".branches[0].depth = 1 | .branches[0].submissions = [$(sub 87 '$work' testing)]")" "$(doc "$blamed87 | $release_busy")"
tokens
has '--token sluice=#87 BLAMED · tests:t-cards'
hasnt 'sluice=release'
pass

fresh 'sidebar: a named branch with nothing queued'
named p1 release
feed "$(doc "$release_idle")"
tokens
has '--token sluice=release · nothing queued'
pass

fresh 'sidebar: a named branch with no records'
named p1 release
feed "$(doc)"
tokens
has '--token sluice=release · no records'
pass

fresh 'sidebar: a named branch counts its checks'
named p1 release
feed "$(doc "$release_idle | .sessions = [$checking,
  {worktree: \"/elsewhere/u1\", branch: \"release\", run: {started: \"2026-10-16T19:44:00Z\"}},
  {worktree: \"/elsewhere/u2\", branch: \"release\"}]")"
tokens
has '--token sluice=release · 1 checking'
hasnt 'checking 1/3'
pass

fresh 'sidebar: a named branch with more than one under test'
named p1 release
feed "$(doc "$release_busy | .sessions = []
  | .branches[1].candidates[0] |= (del(.tasks) | .started = \"2026-10-16T19:41:10Z\")
  | .branches[1].candidates += [.branches[1].candidates[0] | .id = \"c10\" | .submissions = [14]]
  | .branches[1].submissions |= map(select(.id == 12 or .id == 14) | .state = \"testing\")")"
tokens
has '--token sluice=release · #12 testing 4m +1 more'
pass

fresh 'sidebar: no forecast for a named pane'
named p1 release
feed "$(doc "$release_idle | .sessions = [{worktree: \$work, branch: \"main\"}]")"
forecast
grep -q 'submit' "$FAKE/sluice-calls" && fail "a forecast ran for a named pane: $(cat "$FAKE/sluice-calls")"
# Another pane in the same worktree, with no branch named, has one.
jq -c --arg work "$work" '.result.panes += [{pane_id: "p4", tab_id: "t1", workspace_id: "w1", cwd: $work, foreground_cwd: $work, focused: false}]' \
  "$FAKE/panes.json" >"$FAKE/panes.next" && mv "$FAKE/panes.next" "$FAKE/panes.json"
forecast
tokens
[ "$(grep -c 'work: submit main --check --json' "$FAKE/sluice-calls")" = 1 ] || fail "no forecast for the pane with no branch named: $(cat "$FAKE/sluice-calls")"
has 'pane report-metadata p4 --source agentplot.sluice'
has '--token sluice=3 ahead · clean'
has '--token sluice=release · nothing queued'
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

fresh 'sidebar: queued and held'
feed "$(doc ".branches[0].depth = 3 | .branches += [.branches[0] | .branch = \"release\" | .depth = 1] | $held48")"
tokens
has '--token sluice_q=4 submissions · 1 held: release failed'
pass

fresh 'sidebar: held alone'
feed "$(doc "$held_unit_a")"
tokens
has '--token sluice_q=1 held: release failed'
pass

fresh 'sidebar: a checkout two hosts list is counted once'
for h in deneb vega; do
  feed "$(doc "$held48 | .host = \"$h\" | .branches[0].held_checkouts[0].host = \"deneb\"")"
done
[ "$(find "$S/docs" -name '*.json' | wc -l)" -eq 2 ] || fail "not two documents: $(ls "$S/docs")"
tokens
has '--token sluice_q=1 held: release failed'
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
  echo "$failures failed of $((count - skips)), $skips skipped"
  exit 1
fi
echo "all $((count - skips)) passed, $skips skipped"
