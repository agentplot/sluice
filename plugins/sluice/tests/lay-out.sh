#!/usr/bin/env bash
# The plain-clone conversion, scripts/lay-out.sh, on scratch clones: its
# plan changes nothing; it refuses while a process outside its session
# holds the folder, or when it cannot tell, and never on the session or
# what the session started; otherwise it lays the clone out with its changes
# and its worktrees intact, and carries Claude Code's history to the new
# keys, overwriting nothing. Its kind tells a repository's only working
# copy from one of several and from the layout, changing nothing. git runs
# isolated from the user's config and credentials, `lsof` is a stand-in,
# and Claude Code's directory is scratch, so nothing reaches the host's
# processes or the user's history.

set -u
here=$(cd "$(dirname "$0")" && pwd)
lay_out=$here/../scripts/lay-out.sh

# Under sluice's sandbox only SLUICE_TMP is writable beside the checkout.
# Every name here is short: Claude Code derives a key from a path only up
# to 200 characters. A host whose sluice names a checkout's scratch by its
# worktree's path gives a worktree with a long path a SLUICE_TMP too long
# for that, and the suite refuses below; updating the host's sluice gives
# every checkout a short one.
scratch=$(mktemp -d "${SLUICE_TMP:-${TMPDIR:-/tmp}}/l.XXXX")
trap 'rm -rf "$scratch"' EXIT
scratch=$(cd "$scratch" && pwd -P)

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 GIT_TERMINAL_PROMPT=0
export GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@localhost
export GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@localhost
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_ASKPASS SSH_ASKPASS

# A stand-in lsof: it leaves LSOF_RAN behind, and prints what FAKE_LSOF
# holds, in lsof's -F form, or fails when FAKE_LSOF_FAIL is set.
mkdir -p "$scratch/bin"
cat >"$scratch/bin/lsof" <<'EOF'
#!/bin/sh
: >"$LSOF_RAN"
[ -z "${FAKE_LSOF_FAIL:-}" ] || exit 1
[ -z "${FAKE_LSOF:-}" ] || cat "$FAKE_LSOF"
EOF
chmod +x "$scratch/bin/lsof"
export PATH="$scratch/bin:$PATH"
export LSOF_RAN=$scratch/lsof.ran
export CLAUDE_CONFIG_DIR=$scratch/claude
projects=$CLAUDE_CONFIG_DIR/projects

key() { printf '%s' "$1" | sed 's/[^A-Za-z0-9]/-/g'; }

longest=$(key "$scratch/c/main")
if [ ${#longest} -gt 200 ]; then
  echo "FAIL  $scratch is too long to test the carry: a clone's main/ in it makes a key of ${#longest} characters, over Claude Code's 200"
  [ -z "${SLUICE_TMP:-}" ] || echo "      this host's sluice names a checkout's scratch by its worktree's path; updating the host's sluice gives it a short one"
  echo "1 failed"
  exit 1
fi

git init -q -b main "$scratch/origin"
git -C "$scratch/origin" commit -q --allow-empty -m root

# A clone at $1 on branch feat, with a modified and an untracked file,
# and a worktree outside it at $1-side.
clone() {
  git clone -q "$scratch/origin" "$1"
  echo one >"$1/a.txt"
  echo dot >"$1/.hidden"
  git -C "$1" add a.txt .hidden
  git -C "$1" commit -qm files
  git -C "$1" checkout -q -b feat
  echo two >>"$1/a.txt"
  echo new >"$1/untracked.txt"
  git -C "$1" worktree add -q -b side "$1-side"
}

# The stand-in's listing: process $1, its parent $2, command $3,
# working directory $4.
holds() {
  printf 'p%s\nR%s\nc%s\nfcwd\nn%s\n' "$1" "$2" "$3" "$4" >>"$scratch/lsof.out"
}
# Process $1, its parent $2, command $3, working directory / and file $4
# open.
opens() {
  printf 'p%s\nR%s\nc%s\nfcwd\nn/\nf12\nn%s\n' "$1" "$2" "$3" "$4" >>"$scratch/lsof.out"
}
listing() {
  : >"$scratch/lsof.out"
  export FAKE_LSOF=$scratch/lsof.out
}

# What a folder holds and what git says of it, to tell it is as it was.
snapshot() {
  (cd "$1" && find . -path ./.git -prune -o -print | sort)
  [ -d "$1/.git" ] && echo ".git is a directory"
  git -C "$1" status --porcelain
  git -C "$1" branch --show-current
}

failures=0
ok() { echo "ok    $1"; }
fail() {
  echo "FAIL  $1"
  failures=$((failures + 1))
}
# The script, as this shell's own child, so this shell is its parent.
run() {
  "$@" >"$scratch/out" 2>&1
  status=$?
  out=$(cat "$scratch/out")
}
has() { # text what
  case $1 in *"$2"*) return 0 ;; esac
  return 1
}

# The plan of a clone with a process in it changes nothing, and names the
# layout, the process and the history's old and new paths.
p=$scratch/p
clone "$p"
mkdir -p "$projects/$(key "$p")"
echo '{}' >"$projects/$(key "$p")/s1.jsonl"
listing
holds 4194301 1 vite "$p"
before=$(snapshot "$p")
run bash "$lay_out" plan "$p"
if [ $status = 1 ] && has "$out" "main/" && has "$out" "p.git" &&
  has "$out" "4194301  vite  working directory $p" &&
  has "$out" "$projects/$(key "$p") -> $projects/$(key "$p/main")" &&
  [ "$(snapshot "$p")" = "$before" ]; then
  ok "the plan names main/, the git directory, the process and the history, and changes nothing"
else
  fail "plan (exit $status): $out"
fi

# go refuses while that process holds the folder, and moves nothing.
run bash "$lay_out" go "$p"
if [ $status = 1 ] && has "$out" "4194301  vite" && [ "$(snapshot "$p")" = "$before" ] &&
  [ -e "$projects/$(key "$p")/s1.jsonl" ]; then
  ok "go refuses while another process holds the folder, and moves nothing"
else
  fail "go with a holder (exit $status): $out"
fi

# A process with a file open in the folder holds it too.
listing
opens 4194302 1 code "$p/a.txt"
run bash "$lay_out" go "$p"
if [ $status = 1 ] && has "$out" "4194302  code  open file $p/a.txt" &&
  [ "$(snapshot "$p")" = "$before" ]; then
  ok "go refuses while another process has a file in the folder open"
else
  fail "go with an open file (exit $status): $out"
fi

# When it cannot tell what holds the folder, the plan says so, never that
# nothing does, and go refuses.
run env FAKE_LSOF_FAIL=1 bash "$lay_out" plan "$p"
if [ $status = 1 ] && has "$out" "holds the folder: cannot tell (lsof failed)" &&
  ! has "$out" "nothing but this session" && [ "$(snapshot "$p")" = "$before" ]; then
  ok "the plan says it cannot tell what holds the folder when lsof fails"
else
  fail "plan with a failing lsof (exit $status): $out"
fi
run env FAKE_LSOF_FAIL=1 bash "$lay_out" go "$p"
if [ $status = 1 ] && has "$out" "lsof failed" && [ "$(snapshot "$p")" = "$before" ]; then
  ok "go refuses when lsof fails"
else
  fail "go with a failing lsof (exit $status): $out"
fi

# A clone that already has main/ is refused, before anything moves.
listing
mkdir "$p/main"
run bash "$lay_out" go "$p"
rmdir "$p/main"
if [ $status = 1 ] && has "$out" "main already exists" && [ "$(snapshot "$p")" = "$before" ]; then
  ok "go refuses when main/ is already there"
else
  fail "go with main/ there (exit $status): $out"
fi

# The caller names its session. The session, the shell the conversion
# runs in, a helper the session started beside that shell and the
# helper's own child, with a file open, hold nothing: the plan names none
# of them, and go lays the folder out. A process outside the session
# beside them refuses go, which names it and none of the session's own.
h=$scratch/h
clone "$h"
session_pid=4194310
helpers() {
  listing
  holds $session_pid 1 claude "$h"
  holds $$ $session_pid zsh "$h"
  holds 4194311 $session_pid node "$h"
  opens 4194312 4194311 esbuild "$h/a.txt"
}
names_none() { # text
  ! has "$1" 4194310 && ! has "$1" 4194311 && ! has "$1" 4194312 && ! has "$1" "$$  zsh"
}
helpers
before=$(snapshot "$h")
run bash "$lay_out" plan "$h" --session-pid $session_pid
if [ $status = 0 ] && has "$out" "holds the folder: nothing but this session and what it started" &&
  names_none "$out" && [ "$(snapshot "$h")" = "$before" ]; then
  ok "the plan names none of the named session's own: its shell, its helper, the helper's child"
else
  fail "plan with the session's helpers (exit $status): $out"
fi
holds 4194313 1 vim "$h"
run bash "$lay_out" go "$h" --session-pid $session_pid
if [ $status = 1 ] && has "$out" "4194313  vim  working directory $h" && names_none "$out" &&
  [ "$(snapshot "$h")" = "$before" ]; then
  ok "a process outside the session beside its helpers refuses go, which names only that process"
else
  fail "go with a helper and an outsider (exit $status): $out"
fi
helpers
run bash "$lay_out" go "$h" --session-pid $session_pid
if [ $status = 0 ] && [ "$(git -C "$h/h.git" rev-parse --is-bare-repository)" = true ] &&
  [ "$(git -C "$h/main" branch --show-current)" = feat ] && names_none "$out"; then
  ok "go lays the folder out with only the named session and what it started in it"
else
  fail "go with the session's helpers (exit $status): $out"
fi

# With no session named, the process that started the conversion is the
# session: in the folder, it holds nothing.
c=$scratch/c
clone "$c"
listing
holds $$ 4194320 bash "$c"
before=$(snapshot "$c")
run bash "$lay_out" plan "$c"
if [ $status = 0 ] && has "$out" "holds the folder: nothing but this session and what it started" &&
  ! has "$out" "$$  bash" && [ "$(snapshot "$c")" = "$before" ]; then
  ok "with no session named, the process that started the conversion holds nothing"
else
  fail "plan with no session named (exit $status): $out"
fi

# With only the caller's session in the folder, the process that started
# the conversion and the one above it, go lays it out with its
# changes and its outside worktree working, and carries the history: two
# sessions to main/'s key, the caller's left, memory to the git
# directory's key.
old=$projects/$(key "$c")
mkdir -p "$old/memory" "$old/s2" "$old/s1"
for s in s1 s2 s3; do echo "{\"$s\":1}" >"$old/$s.jsonl"; done
echo sub >"$old/s2/agent.jsonl"
echo index >"$old/memory/MEMORY.md"
echo note >"$old/memory/note.md"
listing
holds $$ $PPID bash "$c"
holds $PPID 1 claude "$c"
run bash "$lay_out" go "$c" --session s1
to_main=$projects/$(key "$c/main")
to_git=$projects/$(key "$c/c.git")
list=$(git -C "$c/main" worktree list --porcelain)
if [ $status = 0 ] && [ "$(git -C "$c/c.git" rev-parse --is-bare-repository)" = true ] &&
  has "$list" "worktree $c/c.git" && has "$list" "worktree $c/main" &&
  [ "$(git -C "$c/main" branch --show-current)" = feat ] &&
  [ "$(git -C "$c/main" status --porcelain)" = " M a.txt
?? untracked.txt" ] && [ -e "$c/main/.hidden" ] &&
  ! has "$list" prunable; then
  ok "go lays the clone out: bare git directory, main/ on its branch with its changes, nothing prunable"
else
  fail "go (exit $status): $out
$list"
fi
if [ "$(git -C "$c-side" branch --show-current)" = side ] &&
  git -C "$c-side" status --porcelain >/dev/null 2>&1 && has "$list" "worktree $c-side"; then
  ok "a worktree outside the folder still works"
else
  fail "the outside worktree: $(git -C "$c-side" status 2>&1)"
fi
if [ -e "$to_main/s2.jsonl" ] && [ -e "$to_main/s3.jsonl" ] && [ -e "$to_main/s2/agent.jsonl" ] &&
  [ ! -e "$old/s2.jsonl" ] && [ -e "$old/s1.jsonl" ] && [ -d "$old/s1" ] && [ ! -e "$to_main/s1.jsonl" ] &&
  [ "$(cat "$to_git/memory/MEMORY.md")" = index ] && [ -e "$to_git/memory/note.md" ] &&
  has "$out" "left     $old/s1.jsonl (this session's, which /cd $c/main carries)" &&
  has "$out" "carried  $old/s2.jsonl -> $to_main/s2.jsonl"; then
  ok "two sessions go to main/'s key, the caller's stays for /cd, memory goes to the git directory's key"
else
  fail "the carry: $out
$(cd "$projects" && find . | sort)"
fi

# A memory file already at the git directory's key stays, and so does the
# one that would have replaced it; the report names it.
m=$scratch/m
clone "$m"
old=$projects/$(key "$m")
to_git=$projects/$(key "$m/m.git")
mkdir -p "$old/memory" "$to_git/memory"
echo ours >"$old/memory/MEMORY.md"
echo theirs >"$to_git/memory/MEMORY.md"
echo note >"$old/memory/note.md"
listing
run bash "$lay_out" go "$m"
if [ $status = 0 ] && [ "$(cat "$old/memory/MEMORY.md")" = ours ] &&
  [ "$(cat "$to_git/memory/MEMORY.md")" = theirs ] && [ -e "$to_git/memory/note.md" ] &&
  has "$out" "left     $old/memory/MEMORY.md ($to_git/memory/MEMORY.md is already there)"; then
  ok "a memory file already at the new key is left, with the one it would have replaced, and reported"
else
  fail "a memory file at the new key (exit $status): $out"
fi

# A path whose key is over 200 characters: Claude Code keys it by a hash
# nothing here computes, so nothing is carried, and the report says to
# resume by id.
deep=$scratch/$(printf 'd%.0s' $(seq 1 200))
mkdir -p "$deep"
l=$deep/l
clone "$l"
hashed=$projects/$(key "$l" | cut -c 1-200)-1a2b3c
mkdir -p "$hashed"
echo '{}' >"$hashed/s1.jsonl"
listing
run bash "$lay_out" go "$l"
if [ $status = 0 ] && [ -e "$hashed/s1.jsonl" ] && [ -d "$l/main" ] &&
  [ "$(find "$projects" -mindepth 1 -maxdepth 1 -name "$(key "$l" | cut -c 1-200)*" | wc -l | tr -d ' ')" = 1 ] &&
  has "$out" "claude --resume <session-id>"; then
  ok "a path too long for a derived key carries nothing and says to resume by id"
else
  fail "a long path (exit $status): $out"
fi

# kind, asked from $2, answers $3 and nothing else, runs no lsof and
# leaves $2 as it was.
kind_is() { # what path answer
  local before
  before=$(snapshot "$2")
  rm -f "$LSOF_RAN"
  run bash "$lay_out" kind "$2"
  if [ $status = 0 ] && [ "$out" = "$3" ] && [ ! -e "$LSOF_RAN" ] &&
    [ "$(snapshot "$2")" = "$before" ]; then
    ok "$1"
  else
    fail "$1 (exit $status): $out"
  fi
}

k=$scratch/k
git clone -q "$scratch/origin" "$k"
kind_is "kind names a plain clone its repository's only working copy" "$k" "only $k"

w=$scratch/w
clone "$w"
kind_is "kind, asked from a clone with a worktree beside it, names the clone one of several" \
  "$w" "several $w"
kind_is "kind, asked from the worktree beside the clone, names the clone's folder" \
  "$w-side" "several $w"

# sluice's own checkout under the git directory, and a worktree whose
# folder was deleted, are no working copy.
g=$scratch/g
git clone -q "$scratch/origin" "$g"
git -C "$g" worktree add -q --detach "$g/.git/sluice/.places/x"
git -C "$g" worktree add -q -b gone "$g-gone"
rm -rf "$g-gone"
kind_is "kind counts neither sluice's checkout under .git nor a deleted worktree" "$g" "only $g"

kind_is "kind names a laid-out repository's git directory apart" "$c/main" apart

# A clone reached through a symbolic link, with sluice's checkout added
# through the link too: kind names the folder as it is on disk.
mkdir "$scratch/r"
git clone -q "$scratch/origin" "$scratch/r/k"
ln -s r "$scratch/s"
git -C "$scratch/s/k" worktree add -q --detach "$scratch/s/k/.git/sluice/.places/y"
kind_is "kind, asked through a symbolic link, names the folder as it is on disk" \
  "$scratch/s/k" "only $scratch/r/k"

# A folder that is no working copy is refused, and left as it was; the
# ceiling keeps git from finding a repository above the scratch directory.
n=$scratch/n
mkdir "$n"
echo x >"$n/f.txt"
before=$(cd "$n" && find . | sort)
rm -f "$LSOF_RAN"
run env GIT_CEILING_DIRECTORIES="$scratch" bash "$lay_out" kind "$n"
if [ $status = 1 ] && [ "$out" = "refused: $n is not a working copy of a git repository" ] &&
  [ ! -e "$LSOF_RAN" ] && [ "$(cd "$n" && find . | sort)" = "$before" ]; then
  ok "kind refuses a folder that is no working copy, and changes nothing"
else
  fail "kind on a plain folder (exit $status): $out"
fi

if [ "$failures" -gt 0 ]; then
  echo "$failures failed"
  exit 1
fi
echo "0 failed"
