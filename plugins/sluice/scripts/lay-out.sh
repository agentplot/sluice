#!/usr/bin/env bash
# Lays a plain clone out as the worktrees skill's layout: the git directory
# bare in the folder as <name>.git, the working copy moved into main/ and
# registered as a worktree; then carries Claude Code's history to the keys
# of the new paths (126, S61).
#
#   lay-out.sh plan <folder> [--session-pid <pid>]
#       print the plan, change nothing
#   lay-out.sh go <folder> [--session <id>] [--session-pid <pid>]
#       check again, lay out, carry
#   lay-out.sh kind <path>
#       say how the repository stands, change nothing
#
# kind, asked from any working copy, prints one line: `only <folder>` when
# the git directory is .git inside the repository's only working copy,
# `several <folder>` when inside one of several, `apart` when it stands
# apart from every one (a bare git directory, a .bare directory, a
# --separate-git-dir clone). <folder> holds .git, wherever kind was asked.
# A checkout under the git directory (sluice's own) and a worktree whose
# folder is gone are no working copy. It exits 1 on a path that is no
# working copy of a git repository, and runs no lsof.
#
# Exit 0: done (plan: go would go). 1: refused, nothing moved (plan: go
# would refuse, and why). 2: a step failed after the move began; every
# file is still on disk and nothing was deleted.
#
# The session is the process --session-pid names, and, named none, the
# process that started this script; --session names the session's
# transcript, left for the session to carry itself. It moves nothing while
# a process has its working directory or a file open at or under the
# folder, other than this script, the session, the session's ancestors and
# every process whose parent chain reaches the session (126, record 0057),
# and refuses when it cannot tell. It overwrites and deletes nothing.
# bash 3.2 (macOS's own).

set -u

usage() {
  echo "usage: lay-out.sh plan <folder> [--session-pid <pid>] | lay-out.sh go <folder> [--session <id>] [--session-pid <pid>] | lay-out.sh kind <path>" >&2
  exit 1
}

[ $# -ge 2 ] || usage
form=$1
given=$2
shift 2
session=
session_pid=$PPID
case $form in
plan | go) ;;
kind) [ $# = 0 ] || usage ;;
*) usage ;;
esac
while [ $# -gt 0 ]; do
  case $1 in
  --session)
    [ "$form" = go ] || usage
    [ $# -ge 2 ] || usage
    session=$2
    shift 2
    ;;
  --session-pid)
    [ $# -ge 2 ] || usage
    case $2 in '' | *[!0-9]*) usage ;; esac
    session_pid=$2
    shift 2
    ;;
  *) usage ;;
  esac
done

refuse() {
  echo "refused: $1" >&2
  exit 1
}

if [ "$form" = kind ]; then
  not_one="$given is not a working copy of a git repository"
  [ -d "$given" ] || refuse "$not_one"
  at=$(cd "$given" && pwd -P) || refuse "$not_one"
  cd / || exit 1
  common=$(git -C "$at" rev-parse --path-format=absolute --git-common-dir 2>/dev/null) || refuse "$not_one"
  common=$(cd "$common" && pwd -P) || refuse "$not_one"
  holder=${common%/*}
  if [ "${common##*/}" != .git ] ||
    [ "$(git -C "$holder" rev-parse --is-bare-repository 2>/dev/null)" != false ] ||
    [ "$(git -C "$holder" rev-parse --show-toplevel 2>/dev/null)" != "$holder" ]; then
    echo apart
    exit 0
  fi
  # The working copies: every block of the listing but a bare one, a
  # prunable one, and one at or under the git directory, its path compared
  # once resolved as the git directory's was.
  count=0
  path=
  skip=
  counted() {
    [ -n "$path" ] || return 0
    if [ -z "$skip" ]; then
      if [ -d "$path" ] && p=$(cd "$path" && pwd -P); then
        path=$p
      fi
      case $path in
      "$common" | "$common"/*) ;;
      *) count=$((count + 1)) ;;
      esac
    fi
    path=
    skip=
  }
  while IFS= read -r line; do
    case $line in
    "worktree "*)
      counted
      path=${line#worktree }
      ;;
    bare | prunable | "prunable "*) skip=1 ;;
    esac
  done <<EOF
$(git -C "$holder" worktree list --porcelain)
EOF
  counted
  if [ $count -gt 1 ]; then
    echo "several $holder"
  else
    echo "only $holder"
  fi
  exit 0
fi

[ -d "$given" ] || refuse "$given is not a folder"
# The folder as given, made absolute, and with symbolic links resolved.
folder=$(cd "$given" && pwd -L) || refuse "cannot enter $given"
real=$(cd "$given" && pwd -P) || refuse "cannot enter $given"
name=${folder##*/}
gd=$real/$name.git
# Nothing this script starts has its working directory in the folder.
cd / || exit 1

# A plain clone: its git directory is .git inside its one working copy,
# and the folder is the top of it.
[ "$(git -C "$real" rev-parse --is-bare-repository 2>/dev/null)" = false ] ||
  refuse "$folder is not a working copy of a git repository"
top=$(git -C "$real" rev-parse --show-toplevel 2>/dev/null)
[ "$top" = "$real" ] || refuse "$folder is not the top of its working copy (that is $top)"
common=$(git -C "$real" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)
if [ "$common" != "$real/.git" ] || [ ! -d "$real/.git" ]; then
  refuse "$folder is not a plain clone: its git directory is $common, not .git inside it"
fi

reasons=
stop() { reasons="$reasons$1
"; }

# Under the folder, in either of its forms.
under() { # path
  case $1 in
  "$real" | "$real"/* | "$folder" | "$folder"/*) return 0 ;;
  esac
  return 1
}

# Where a path under the old .git directory is once it is <name>.git.
in_git_dir() { # path
  case $1 in
  "$real"/.git/*) printf '%s\n' "$gd/${1#"$real"/.git/}" ;;
  "$folder"/.git/*) printf '%s\n' "$gd/${1#"$folder"/.git/}" ;;
  *) return 1 ;;
  esac
}

[ ! -e "$real/$name.git" ] || stop "$name.git already exists in the folder"
[ ! -e "$real/main" ] || stop "main already exists in the folder"
branch=$(git -C "$real" symbolic-ref -q --short HEAD)
[ -n "$branch" ] || stop "HEAD is detached: check out a branch first"
git -C "$real" rev-parse -q --verify HEAD >/dev/null ||
  stop "branch ${branch:-HEAD} has no commit yet"
for f in MERGE_HEAD CHERRY_PICK_HEAD REVERT_HEAD BISECT_LOG rebase-merge rebase-apply; do
  if [ -e "$real/.git/$f" ]; then
    stop "a merge, rebase, cherry-pick, revert or bisect is in progress: finish or abort it first"
    break
  fi
done
changes=$(git -C "$real" status --porcelain 2>/dev/null | wc -l | tr -d ' ')

# The other worktrees: outside the folder, repaired in place; under .git
# (sluice's own checkouts), moved with it and repaired; anywhere else in
# the folder, they would be carried into main/, so they stop the move.
worktrees=
inner=
first=1
while IFS= read -r line; do
  case $line in
  "worktree "*)
    path=${line#worktree }
    if [ $first = 1 ]; then
      first=0
      continue
    fi
    if moved=$(in_git_dir "$path"); then
      worktrees="$worktrees    $path  moves with the git directory to $moved, repaired
"
      inner="$inner$moved
"
    elif under "$path"; then
      worktrees="$worktrees    $path  inside the folder: stops the move
"
      stop "worktree $path is inside the folder and would be carried into main/"
    else
      worktrees="$worktrees    $path  repaired in place
"
    fi
    ;;
  esac
done <<EOF
$(git -C "$real" worktree list --porcelain)
EOF

# What holds the folder: every process's working directory and open files,
# listed once, kept where the path is the folder or under it, leaving out
# this process, the session, its ancestors and every process whose parent
# chain reaches it. Both are found by walking the parent ids the same
# listing gives: `ps` is setuid on macOS, and a sandbox may refuse to run
# it.
held=
blind=
if ! command -v lsof >/dev/null 2>&1; then
  blind="lsof is not on PATH"
  stop "cannot tell what holds the folder: $blind"
elif ! listing=$(lsof -n -P -w +c 0 -F pRcfn 2>/dev/null); then
  blind="lsof failed"
  stop "cannot tell what holds the folder: $blind"
else
  held=$(printf '%s\n' "$listing" | awk -v self=$$ -v session="$session_pid" -v a="$folder" -v b="$real" '
    /^p/ { pid = substr($0, 2); next }
    /^R/ { parent[pid] = substr($0, 2); next }
    /^c/ { cmd[pid] = substr($0, 2); next }
    /^f/ { fd = substr($0, 2); next }
    /^n/ {
      if (pid in how) next
      n = substr($0, 2)
      if (n == a || index(n, a "/") == 1 || n == b || index(n, b "/") == 1) {
        how[pid] = (fd == "cwd" ? "working directory " : "open file ") n
        order[++count] = pid
      }
    }
    END {
      skip[self] = 1
      p = session
      for (i = 0; i < 64 && p != "" && p != "0" && !(p in skip); i++) {
        skip[p] = 1
        p = parent[p]
      }
      for (i = 1; i <= count; i++) {
        q = order[i]
        if (q in skip) continue
        mine = 0
        p = parent[q]
        for (j = 0; j < 64 && p != "" && p != "0"; j++) {
          if (p == session) {
            mine = 1
            break
          }
          p = parent[p]
        }
        if (!mine) printf "    %s  %s  %s\n", q, cmd[q], how[q]
      }
    }')
  if [ -n "$held" ]; then
    stop "$(printf '%s\n' "$held" | grep -c .) process(es) hold the folder: stop each, or close the folder in it"
  fi
fi

# Claude Code's history: a path's key is the path with every character but
# a letter or a digit made a dash; a key over 200 characters ends in a hash
# of the path, which nothing here computes.
projects=${CLAUDE_CONFIG_DIR:-$HOME/.claude}/projects
key() { printf '%s' "$1" | sed 's/[^A-Za-z0-9]/-/g'; }

forms=$real
[ "$folder" = "$real" ] || forms="$real
$folder"
long=
carries=
while IFS= read -r p; do
  old=$(key "$p")
  to_main=$(key "$p/main")
  to_git=$(key "$p/$name.git")
  if [ ${#old} -gt 200 ] || [ ${#to_main} -gt 200 ] || [ ${#to_git} -gt 200 ]; then
    long=1
    continue
  fi
  [ -d "$projects/$old" ] || continue
  sessions=$(find "$projects/$old" -mindepth 1 -maxdepth 1 -name '*.jsonl' | wc -l | tr -d ' ')
  carries="$carries    $sessions session(s)  $projects/$old -> $projects/$to_main
"
  if [ -d "$projects/$old/memory" ]; then
    memories=$(find "$projects/$old/memory" -mindepth 1 -maxdepth 1 | wc -l | tr -d ' ')
    carries="$carries    memory, $memories file(s)  $projects/$old/memory -> $projects/$to_git/memory
"
  fi
done <<EOF
$forms
EOF
resume="resume those sessions by id from anywhere: claude --resume <session-id>"

print_plan() {
  echo "Plan for $folder (nothing has moved yet):"
  echo "  git directory  .git -> $name.git (bare)"
  echo "  main/          the working copy, on branch ${branch:-(none)}, with its $changes changed path(s) as they are (staged changes come out unstaged)"
  if [ -n "$worktrees" ]; then
    echo "  other worktrees"
    printf '%s' "$worktrees"
  fi
  if [ -n "$held" ]; then
    echo "  holds the folder, to stop, or to close the folder in, and start again from $folder/main"
    printf '%s\n' "$held"
  elif [ -n "$blind" ]; then
    echo "  holds the folder: cannot tell ($blind)"
  else
    echo "  holds the folder: nothing but this session and what it started"
  fi
  echo "  Claude Code history, carried, overwriting nothing"
  if [ -n "$long" ]; then
    echo "    none: the folder's path makes a key over 200 characters, which Claude Code shortens with a hash; $resume"
  elif [ -n "$carries" ]; then
    printf '%s' "$carries"
  else
    echo "    none under $projects/$(key "$real")"
  fi
  echo "  left behind: prompt recall, and the folder's trust and permission answers, which Claude asks again once for main/;"
  echo "    sessions started in a subfolder keep their own keys: $resume"
}

if [ "$form" = plan ]; then
  print_plan
  if [ -n "$reasons" ]; then
    echo "go refuses now:"
    printf '%s' "$reasons" | sed 's/^/  /'
    exit 1
  fi
  exit 0
fi

if [ -n "$reasons" ]; then
  [ -z "$held" ] || {
    echo "holds the folder:" >&2
    printf '%s\n' "$held" >&2
  }
  printf '%s' "$reasons" | sed 's/^/refused: /' >&2
  echo "nothing moved" >&2
  exit 1
fi

# The move, the worktrees skill's steps (references/convert-from-plain-clone.md).
fail() {
  echo "failed: $1" >&2
  echo "Every file is still on disk and nothing was deleted: show this to the user, and see the worktrees skill's repair reference." >&2
  exit 2
}
step() { # what command...
  local what=$1
  shift
  "$@" || fail "$what"
}
# shellcheck disable=SC2329 # run through step
write_git_file() { printf 'gitdir: ./%s\n' "$name.git" >"$real/.git"; }
# shellcheck disable=SC2329 # run through step
move_files() {
  local e
  for e in "$real"/* "$real"/.[!.]* "$real"/..?*; do
    [ -e "$e" ] || [ -L "$e" ] || continue
    case ${e##*/} in .git | "$name.git" | main) continue ;; esac
    mv "$e" "$real/main/" || return 1
  done
}

echo "Laying out $folder:"
step "renaming .git to $name.git" mv "$real/.git" "$gd"
step "making $name.git bare" git --git-dir="$gd" config core.bare true
step "writing the .git file" write_git_file
step "making main/" mkdir "$real/main"
step "moving the working copy's files into main/" move_files
step "registering main/ as a worktree on $branch" git --git-dir="$gd" worktree add -q --no-checkout "$real/main-tmp" "$branch"
step "moving main/'s .git file" mv "$real/main-tmp/.git" "$real/main/.git"
step "removing the empty main-tmp/" rmdir "$real/main-tmp"
step "repairing main/" git -C "$gd" worktree repair "$real/main"
step "matching main/'s index to $branch" git -C "$real/main" reset -q --mixed
if git --git-dir="$gd" config remote.origin.url >/dev/null; then
  step "mapping origin's branches" git --git-dir="$gd" config remote.origin.fetch '+refs/heads/*:refs/remotes/origin/*'
fi
step "recording worktrees by relative paths" git --git-dir="$gd" config worktree.useRelativePaths true
# shellcheck disable=SC2329 # run through step
repair_all() {
  local paths=() p
  while IFS= read -r p; do
    [ -n "$p" ] && paths+=("$p")
  done <<EOF
$inner
EOF
  git -C "$gd" worktree repair ${paths[@]+"${paths[@]}"}
}
step "repairing every worktree" repair_all
echo "  $name.git (bare), main/ on $branch with its changes"

# The carry. A name already at the new key stays where it is, and so does
# the one that would have replaced it.
carry_failed=
carry() { # path dir
  local base=${1##*/}
  if [ -e "$2/$base" ] || [ -L "$2/$base" ]; then
    echo "  left     $1 ($2/$base is already there)"
    return 1
  elif mkdir -p "$2" && mv "$1" "$2/"; then
    echo "  carried  $1 -> $2/$base"
  else
    echo "  left     $1 (could not move it)"
    carry_failed=1
    return 1
  fi
}

echo "Claude Code history:"
if [ -n "$long" ]; then
  echo "  nothing carried: the folder's path makes a key over 200 characters, which Claude Code shortens with a hash; $resume"
fi
carried_any=
while IFS= read -r p; do
  old=$(key "$p")
  to_main=$(key "$p/main")
  to_git=$(key "$p/$name.git")
  if [ ${#old} -gt 200 ] || [ ${#to_main} -gt 200 ] || [ ${#to_git} -gt 200 ]; then
    continue
  fi
  [ -d "$projects/$old" ] || continue
  carried_any=1
  for t in "$projects/$old"/*.jsonl; do
    [ -e "$t" ] || continue
    id=${t##*/}
    id=${id%.jsonl}
    if [ -n "$session" ] && [ "$id" = "$session" ]; then
      echo "  left     $t (this session's, which /cd $folder/main carries)"
      continue
    fi
    if carry "$t" "$projects/$to_main"; then
      [ ! -d "$projects/$old/$id" ] || carry "$projects/$old/$id" "$projects/$to_main"
    elif [ -d "$projects/$old/$id" ]; then
      echo "  left     $projects/$old/$id (with its transcript)"
    fi
  done
  if [ -d "$projects/$old/memory" ]; then
    for m in "$projects/$old/memory"/* "$projects/$old/memory"/.[!.]*; do
      [ -e "$m" ] || [ -L "$m" ] || continue
      carry "$m" "$projects/$to_git/memory"
    done
  fi
  for e in "$projects/$old"/* "$projects/$old"/.[!.]*; do
    [ -e "$e" ] || [ -L "$e" ] || continue
    base=${e##*/}
    case $base in *.jsonl | memory) continue ;; esac
    [ ! -e "$projects/$old/$base.jsonl" ] || continue
    echo "  left     $e (not a session's transcript or memory)"
  done
  for d in "$projects/$old"-*; do
    [ -d "$d" ] || continue
    case ${d##*/} in "$to_main" | "$to_git") continue ;; esac
    echo "  left     $d (a subfolder's sessions, or another folder named alike; $resume)"
  done
done <<EOF
$forms
EOF
[ -n "$carried_any" ] || [ -n "$long" ] || echo "  none under $projects/$(key "$real")"

echo "Start what held the folder again from $folder/main. In Claude Code, /cd $folder/main moves this session there."
[ -z "$carry_failed" ] || exit 2
exit 0
