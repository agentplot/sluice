#!/bin/sh
# The activation's cases (record 0098), which nix/check.sh test runs
# beside the evaluated ones: each runs nix/modules/activate.sh by sh in a
# scratch folder of its own, with a scratch HOME and the module's file in
# a store of its own, so no case reads or writes the host's own
# ~/.config/sluice. It prints each failure as one line,
# nix: activate: <case>: <what>, and nothing where every case holds.
set -eu

activate=$(cd "$(dirname "$0")/../modules" && pwd)/activate.sh
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
cases=0

# A case's folder: its home and the module's file.
fresh() {
  cases=$((cases + 1))
  dir=$scratch/$cases
  home=$dir/home
  file=$dir/store/sluice-config.toml
  folder=$home/.config/sluice
  target=$folder/config.toml
  aside=$target.before-nix
  mkdir -p "$home" "$dir/store"
  echo '# Written by sluice'"'"'s flake module' >"$file"
}

# activate.sh with `$@` in the case's folder: its status, output and
# error, and both on one line, `said`, for a failure's line.
run() {
  status=0
  HOME=$home sh "$activate" "$@" >"$dir/out" 2>"$dir/err" || status=$?
  out=$(cat "$dir/out")
  err=$(cat "$dir/err")
  said=$(cat "$dir/out" "$dir/err" | paste -s -d ';' -)
}

# A failure of the case `$1`, saying `$2`, where the test after them fails.
expect() {
  named=$1
  what=$2
  shift 2
  if ! "$@"; then
    echo "nix: activate: $named: $what"
  fi
}

linked() {
  [ -L "$target" ] && [ "$(readlink "$target")" = "$file" ]
}
holds() {
  [ -f "$1" ] && [ ! -L "$1" ] && [ "$(cat "$1")" = "$2" ]
}
missing() {
  [ ! -e "$1" ] && [ ! -L "$1" ]
}
# `$1` names `$2`, then `$3`.
names() {
  case $1 in *"$2"*"$3"*) ;; *) return 1 ;; esac
}
# The names in the folder, sorted, on one line.
listed() {
  for entry in "$folder"/* "$folder"/.[!.]*; do
    if [ -e "$entry" ] || [ -L "$entry" ]; then
      basename "$entry"
    fi
  done | sort | paste -s -d ' ' -
}

named="Nothing there"
fresh
run link "$file"
expect "$named" "link exits $status: $said" [ "$status" -eq 0 ]
expect "$named" "$target is no link to $file" linked
expect "$named" "link says \"$said\", not that it linked $target to $file" \
  [ "$out" = "sluice: linked $target to $file" ]
expect "$named" "link wrote $(listed) in $folder" [ "$(listed)" = config.toml ]

named="A link there"
fresh
mkdir -p "$folder"
ln -s "$dir/store/another.toml" "$target"
run link "$file"
expect "$named" "link exits $status: $said" [ "$status" -eq 0 ]
expect "$named" "$target is no link to $file" linked
expect "$named" "link says \"$said\", not that it linked $target to $file" \
  [ "$out" = "sluice: linked $target to $file" ]
expect "$named" "link moved the other link aside" missing "$aside"
run link "$file"
expect "$named" "link again exits $status: $said" [ "$status" -eq 0 ]
expect "$named" "link again says \"$said\"" [ -z "$out$err" ]
expect "$named" "$target is no link to $file after link again" linked

named="A file there"
fresh
mkdir -p "$folder"
echo '[runs]' >"$target"
echo 'the key' >"$folder/app.pem"
run link "$file"
expect "$named" "link exits $status: $said" [ "$status" -eq 0 ]
expect "$named" "$target is no link to $file" linked
expect "$named" "$aside does not hold the file that was there" holds "$aside" '[runs]'
expect "$named" "link says \"$said\", not both paths" \
  [ "$out" = "sluice: moved $target to $aside
sluice: linked $target to $file" ]
expect "$named" "app.pem is not as it was" holds "$folder/app.pem" 'the key'
expect "$named" "link left $(listed) in $folder" \
  [ "$(listed)" = "app.pem config.toml config.toml.before-nix" ]

named="A file there and one aside already"
fresh
mkdir -p "$folder"
echo '[runs]' >"$target"
echo '[peers]' >"$aside"
run link "$file"
expect "$named" "link exits $status, not 1" [ "$status" -eq 1 ]
expect "$named" "link changed $target" holds "$target" '[runs]'
expect "$named" "link changed $aside" holds "$aside" '[peers]'
expect "$named" "link's refusal \"$said\" names not both paths" \
  names "$err" "$target" "$aside"
expect "$named" "link says \"$said\" on its output" [ -z "$out" ]

named="Usage"
fresh
run link
expect "$named" "link with no file exits $status, not 2" [ "$status" -eq 2 ]
run keep "$file"
expect "$named" "keep exits $status, not 2" [ "$status" -eq 2 ]
expect "$named" "a refused call made $home/.config" missing "$home/.config"
