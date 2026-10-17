# shellcheck shell=sh
# A process known by more than its pid, so a pid the system has given to
# another process since (after a reboot, say) is never taken for it: its
# start time from /proc where there is one (Linux, busybox), else from ps
# (macOS); else its command line; else, only where neither can be read,
# its pid alone. Sourced.

identity() {
  if [ -r "/proc/$1/stat" ]; then
    # The fields after the command's closing parenthesis; the start time is
    # the 22nd of the line, the 20th of these.
    t=$(sed 's/.*) //' "/proc/$1/stat" 2>/dev/null | cut -d' ' -f20)
    if [ -n "$t" ]; then
      echo "start:$t"
      return
    fi
  fi
  t=$(ps -o lstart= -p "$1" 2>/dev/null | tr -s ' ')
  if [ -n "$t" ]; then
    echo "start:$t"
    return
  fi
  a=$({ ps -A -o pid,args 2>/dev/null || ps -o pid,args 2>/dev/null; } |
    awk -v p="$1" '$1 == p { $1 = ""; sub(/^ +/, ""); print; exit }')
  if [ -n "$a" ]; then
    echo "args:$a"
    return
  fi
  echo "pid:$1"
}

# Whether pid $1 is alive and still the process recorded as $2. One
# recorded by its pid alone, in its first second (pid:<pid>:<parent>), is
# that process only while its parent is still the one recorded; a pid
# recorded with no parent is no one's.
same_process() {
  [ -n "$1" ] && [ -n "$2" ] && kill -0 "$1" 2>/dev/null || return 1
  case $2 in
  "pid:$1:"*) [ "$(parent_of "$1")" = "${2##*:}" ] ;;
  pid:*) return 1 ;;
  *) [ "$(identity "$1")" = "$2" ] ;;
  esac
}

# The parent of a process.
parent_of() {
  if [ -r "/proc/$1/stat" ]; then
    sed 's/.*) //' "/proc/$1/stat" | cut -d' ' -f2
  else
    ps -o ppid= -p "$1" 2>/dev/null | tr -d ' '
  fi
}
