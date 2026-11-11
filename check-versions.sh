#!/bin/sh
# Every file of the tree that carries sluice's version gives the
# workspace's, which each crate takes: the files `set-version.sh --files`
# lists, Cargo.lock's packages for sluice's own crates among them. main
# gives 0.0.0 in each, and a release's tag names a release commit in which
# set-version.sh wrote the release's (record 0111; rule core.55). So a
# release's plugins carry the release's version, which Claude Code and
# herdr read, and never drift from its binary's. The marketplace names no
# version for a plugin, which its manifest says once; and every plugin
# manifest under plugins/ is among the writer's files, so a new plugin's
# is refused until the writer writes it.
#
# And set-version.sh, which the release job runs on the commit it
# releases, run on a scratch copy of those files with a version no release
# has, leaves each giving that version and changes no line but the ones
# that give it; refuses a version that is not major.minor.patch, and a
# file with no version, naming it, and writes nothing then.
set -u
root=$(cd "$(dirname "$0")" && pwd)
cd "$root" || exit 1

files=$(sh set-version.sh --files) || {
  echo "versions: set-version.sh --files failed" >&2
  exit 1
}

# The workspace's version in the tree at $1.
workspace() {
  awk '
    /^\[/ { section = $0 }
    section == "[workspace.package]" && /^version = "/ {
      gsub(/^version = "|"$/, "")
      print
      exit
    }
  ' "$1/Cargo.toml"
}

# Each place the tree at $1 gives sluice's version, a line of where and the
# version it gives, "none" where it gives none, for each of the files.
carried() {
  for file in $files; do
    case $file in
    Cargo.toml)
      version=$(workspace "$1")
      printf '%s\t%s\n' "$file" "${version:-none}"
      ;;
    Cargo.lock)
      awk '
          function flush() {
            if (package && !sourced) print "Cargo.lock: " name "\t" (version == "" ? "none" : version)
            package = 0
            sourced = 0
            name = ""
            version = ""
          }
          /^\[\[package\]\]$/ { flush(); package = 1; next }
          package && /^name = "/ { name = $0; gsub(/^name = "|"$/, "", name) }
          package && /^version = "/ { version = $0; gsub(/^version = "|"$/, "", version) }
          package && /^source = / { sourced = 1 }
          END { flush() }
        ' "$1/$file"
      ;;
    */plugin.json)
      jq -r --arg file "$file" '"\($file)\t\(.version // "none")"' "$1/$file"
      ;;
    */herdr-plugin.toml)
      awk -v file="$file" '
          /^\[/ { exit }
          /^version = "/ { version = $0; gsub(/^version = "|"$/, "", version) }
          END { print file "\t" (version == "" ? "none" : version) }
        ' "$1/$file"
      ;;
    *)
      printf '%s\t%s\n' "$file" "unread: check-versions.sh has no reader for it"
      ;;
    esac
  done
}

# Each place in the tree at $1 that does not give the version $2, said with
# $3 ahead of it; fails where there is one.
agree() {
  carried "$1" | awk -F '\t' -v want="$2" -v said="$3" '
    $2 != want {
      print said $1 " gives " $2 ", and the workspace " want > "/dev/stderr"
      failed = 1
    }
    END { exit failed }
  '
}

status=0
version=$(workspace .)
if [ -z "$version" ]; then
  echo "versions: Cargo.toml gives no version under [workspace.package]" >&2
  exit 1
fi
agree . "$version" "versions: " || status=1

# The marketplace names no version for a plugin.
named=$(jq -r '.plugins[] | select(has("version")) | "\(.name)\t\(.version)"' .claude-plugin/marketplace.json) || {
  echo "versions: .claude-plugin/marketplace.json could not be read" >&2
  status=1
}
printf '%s\n' "$named" | while IFS="$(printf '\t')" read -r name given; do
  [ -n "$name" ] || continue
  echo "versions: .claude-plugin/marketplace.json gives $name the version $given; a plugin's version is its manifest's alone" >&2
done
[ -z "$named" ] || status=1

# Every plugin manifest under plugins/ is among the writer's files.
for manifest in plugins/*/.claude-plugin/plugin.json plugins/*/herdr-plugin.toml; do
  [ -f "$manifest" ] || continue
  printf '%s\n' "$files" | grep -Fqx "$manifest" || {
    echo "versions: $manifest is not among the files set-version.sh writes" >&2
    status=1
  }
done

# The release's writer on a scratch copy.
scratch=$(mktemp -d "${SLUICE_TMP:-${TMPDIR:-/tmp}}/sluice-versions.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
copy() {
  rm -rf "$scratch/tree"
  for file in $files; do
    mkdir -p "$scratch/tree/$(dirname "$file")"
    cp "$file" "$scratch/tree/$file"
  done
}
# Whether the copy still holds the tree's files as they are; says which
# differs, after $1.
unchanged() {
  for file in $files; do
    cmp -s "$file" "$scratch/tree/$file" || {
      echo "versions: $1 changed $file" >&2
      return 1
    }
  done
}

copy
release=98.76.54
if ! sh set-version.sh "$release" "$scratch/tree"; then
  echo "versions: set-version.sh $release failed" >&2
  exit 1
fi
agree "$scratch/tree" "$release" "versions: set-version.sh $release left " || status=1
for file in $files; do
  # A line it changed gives the version; it changes as many as it writes.
  changes=$(diff "$file" "$scratch/tree/$file")
  removed=$(printf '%s\n' "$changes" | grep -c '^<')
  added=$(printf '%s\n' "$changes" | grep -c '^>')
  stray=$(printf '%s\n' "$changes" | grep '^>' | grep -vF "\"$release\"")
  if [ "$removed" != "$added" ] || [ -n "$stray" ]; then
    echo "versions: set-version.sh $release changed $file beyond its versions:" >&2
    printf '%s\n' "$changes" >&2
    status=1
  fi
done

# A version that is not major.minor.patch is refused, and nothing written.
copy
if sh set-version.sh 0.88 "$scratch/tree" 2>"$scratch/said"; then
  echo "versions: set-version.sh 0.88 did not refuse the version" >&2
  status=1
elif ! grep -Fq '0.88 is not a version major.minor.patch' "$scratch/said"; then
  echo "versions: set-version.sh 0.88 refused without saying 0.88 is not a version:" >&2
  cat "$scratch/said" >&2
  status=1
fi
unchanged "set-version.sh 0.88" || status=1

# A file with no version is refused, naming it, and nothing written.
copy
herdr=plugins/herdr/herdr-plugin.toml
grep -v '^version = "' "$herdr" >"$scratch/tree/$herdr"
cp "$scratch/tree/$herdr" "$scratch/stripped"
if sh set-version.sh "$release" "$scratch/tree" 2>"$scratch/said"; then
  echo "versions: set-version.sh $release did not refuse $herdr with no version" >&2
  status=1
elif ! grep -Fq "$herdr" "$scratch/said"; then
  echo "versions: set-version.sh $release refused $herdr with no version without naming it:" >&2
  cat "$scratch/said" >&2
  status=1
fi
for file in $files; do
  [ "$file" = "$herdr" ] && want=$scratch/stripped || want=$file
  cmp -s "$want" "$scratch/tree/$file" || {
    echo "versions: set-version.sh $release changed $file though it refused $herdr" >&2
    status=1
  }
done
left=$(find "$scratch/tree" -name '*.set-version')
if [ -n "$left" ]; then
  echo "versions: set-version.sh $release left its scratch files behind: $left" >&2
  status=1
fi

[ "$status" -ne 0 ] || echo "versions: every file gives $version, and set-version.sh writes each"
exit $status
