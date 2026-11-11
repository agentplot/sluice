#!/bin/sh
# Every command that installs sluice or one of its plugins names a release,
# and none names main (records 0111 and 0112, rule core.56). Main's tree
# names 0.0.0, so a plugin installed from main says 0.0.0, its herdr
# watcher, whose lease is keyed by the plugin's version (record 0094), is
# never replaced at a new release, and it may call commands a released
# sluice lacks.
#
# `sh check-installs.sh [<root>]` reads the tree at <root>, by default the
# folder this script is in: README.md, docs/examples/README.md,
# plugins/*/README.md, every *.md under plugins/*/skills/, and every *.md
# and *.mdx under site/src/content/docs/. Each file is read with its
# whitespace collapsed to single spaces, so a command a page wraps across
# lines is one command. Each install command is found by its fixed start,
# with the text up to the next space, backtick, quote or `)`, and held to
# its form:
#   marketplace add agentplot/sluice           followed at once by #release
#   plugin install agentplot/sluice/plugins/herdr
#                                              followed by ` --ref v<version>`
#   github:agentplot/sluice                    followed by /v<version>; a
#                                              module's key, #darwinModules.
#                                              or #nixosModules., is passed
#   --git https://github.com/agentplot/sluice  followed by ` --tag v<version>`
#   github.com/agentplot/sluice/releases/...   with /download/, under
#                                              releases/latest/download/ or
#                                              releases/download/v<version>/;
#                                              the releases page is passed
# where <version> is written `<version>`, as digits, or as a shell
# variable. Each failure is one line,
# installs: <file>: <command as found>: <what it must be>.
set -u
root=${1:-$(cd "$(dirname "$0")" && pwd)}
cd "$root" || exit 1
status=0

# A character of a command: anything but a space, a backtick, a quote or
# a closing parenthesis.
char="[^ \`\"')]"

fail() {
  echo "installs: $1: $2: $3" >&2
  status=1
}

# The files that give install commands, by their path from the root.
files() {
  for file in README.md docs/examples/README.md plugins/*/README.md; do
    [ -f "$file" ] && printf '%s\n' "$file"
  done
  for skills in plugins/*/skills; do
    [ -d "$skills" ] && find "$skills" -type f -name '*.md'
  done | sort
  if [ -d site/src/content/docs ]; then
    find site/src/content/docs -type f \( -name '*.md' -o -name '*.mdx' \) | sort
  fi
}

# Each match of the pattern $2 in the collapsed text $1, one a line.
found() {
  printf '%s\n' "$1" | grep -oE -- "$2" || true
}

# Whether $1 names a release's version: `<`, a digit or `$` at its start.
versioned() {
  case $1 in
  "<"* | [0-9]* | '$'*) return 0 ;;
  *) return 1 ;;
  esac
}

while IFS= read -r file; do
  text=$(tr -s '[:space:]' ' ' <"$file")

  while IFS= read -r command; do
    [ -n "$command" ] || continue
    case $command in
    "marketplace add agentplot/sluice#release"*) ;;
    *) fail "$file" "$command" "the marketplace is added at agentplot/sluice#release" ;;
    esac
  done <<EOF
$(found "$text" "marketplace add agentplot/sluice$char*")
EOF

  while IFS= read -r command; do
    [ -n "$command" ] || continue
    case $command in
    "plugin install agentplot/sluice/plugins/herdr --ref v"*)
      versioned "${command#plugin install agentplot/sluice/plugins/herdr --ref v}" ||
        fail "$file" "$command" "the herdr plugin is installed with --ref v<version>"
      ;;
    *) fail "$file" "$command" "the herdr plugin is installed with --ref v<version>" ;;
    esac
  done <<EOF
$(found "$text" "plugin install agentplot/sluice/plugins/herdr$char*( $char+){0,2}")
EOF

  while IFS= read -r command; do
    [ -n "$command" ] || continue
    case $command in
    "github:agentplot/sluice#darwinModules."* | "github:agentplot/sluice#nixosModules."*) ;;
    "github:agentplot/sluice/v"*)
      versioned "${command#github:agentplot/sluice/v}" ||
        fail "$file" "$command" "nix takes github:agentplot/sluice/v<version>"
      ;;
    *) fail "$file" "$command" "nix takes github:agentplot/sluice/v<version>" ;;
    esac
  done <<EOF
$(found "$text" "github:agentplot/sluice$char*")
EOF

  while IFS= read -r command; do
    [ -n "$command" ] || continue
    rest=${command#--git https://github.com/agentplot/sluice}
    rest=${rest#.git}
    case $rest in
    " --tag v"*)
      versioned "${rest# --tag v}" ||
        fail "$file" "$command" "cargo takes --tag v<version> after the repository"
      ;;
    *) fail "$file" "$command" "cargo takes --tag v<version> after the repository" ;;
    esac
  done <<EOF
$(found "$text" "--git https://github\.com/agentplot/sluice$char*( $char+){0,2}")
EOF

  while IFS= read -r url; do
    [ -n "$url" ] || continue
    case $url in
    */download/*) ;;
    *) continue ;;
    esac
    case $url in
    "github.com/agentplot/sluice/releases/latest/download/"*) ;;
    "github.com/agentplot/sluice/releases/download/v"*)
      versioned "${url#github.com/agentplot/sluice/releases/download/v}" ||
        fail "$file" "$url" "a release's archive is under releases/latest/download/ or releases/download/v<version>/"
      ;;
    *) fail "$file" "$url" "a release's archive is under releases/latest/download/ or releases/download/v<version>/" ;;
    esac
  done <<EOF
$(found "$text" "github\.com/agentplot/sluice/releases/$char*")
EOF
done <<FILES
$(files)
FILES

[ "$status" -ne 0 ] || echo "installs: every install command names a release"
exit "$status"
