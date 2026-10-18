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
sleep 2
n=0
for l in "$HERDR_PLUGIN_STATE_DIR"/servers/*/watch.lease; do
  [ -f "$l" ] && kill -0 "$(holder_of "$l")" 2>/dev/null && n=$((n + 1))
done
[ "$n" = 2 ] || fail "$n watchers for two herdr servers"
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
awk 'BEGIN{RS="\f"} NR==2' "$t/out" | grep -q '^/srv/elsewhere.git on host-a' || fail "a does not show every repository"
pass

fresh "status: the focused pane from herdr's context"
feed "$testing95"
ctx=$(jq -c --arg w "$work" '.focused_pane_cwd = $w' "$here/docs/context.json")
HERDR_PLUGIN_CONTEXT_JSON=$ctx sh "$plugin/bin/status.sh" --once | grep -q '▶ testing main+#95' || fail "the popup did not read the focused pane's repository"
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
[ "$out" = 'sluice main · 3 submissions · testing main+#12+#13 2m10s/15m · #14 waiting · last #9 blamed on 4f1c2aa' ] || fail "the segment printed: $out"
feed "$(doc '.host = "host-a"')"
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
  echo "$failures failed of $((count - skips)), $skips skipped"
  exit 1
fi
echo "all $((count - skips)) passed, $skips skipped"
