#!/bin/sh
# The modules' activation, run as the user who runs sluice (record 0098,
# core.51):
#
#   activate.sh link <file>
#
# links ~/.config/sluice/config.toml to the module's file, so every
# sluice command that user runs reads it. A link there is replaced. A file
# or folder there that is no link is moved once, to
# config.toml.before-nix, and both paths are named. Where
# config.toml.before-nix is there already, it changes neither, names both
# and fails. It makes ~/.config/sluice where it is missing and links the
# file alone, never the folder, which holds the host's keys. It says a
# line only where it moved or linked something. agentplot/boomdrive's
# nix/modules/activate.sh at 3f428cf, its link alone, for sluice's path:
# sluice has no service to restart.
set -eu

usage() {
  echo "usage: activate.sh link <file>" >&2
  exit 2
}

link() {
  file=$1
  folder=$HOME/.config/sluice
  target=$folder/config.toml
  aside=$target.before-nix

  if [ -L "$target" ] && [ "$(readlink "$target")" = "$file" ]; then
    return
  fi
  if [ -e "$target" ] && [ ! -L "$target" ]; then
    if [ -e "$aside" ] || [ -L "$aside" ]; then
      echo "sluice: $target is no link and $aside is there already: left both, linked nothing; move one aside to link $file" >&2
      exit 1
    fi
    mv "$target" "$aside"
    echo "sluice: moved $target to $aside"
  fi
  mkdir -p "$folder"
  ln -sfn "$file" "$target"
  echo "sluice: linked $target to $file"
}

case ${1:-} in
link)
  [ $# -eq 2 ] || usage
  link "$2"
  ;;
*) usage ;;
esac
