#!/bin/sh
# herdr-plugin.toml as herdr will read it: every line parses, the top-level
# keys herdr requires are there, every command of [[startup]], [[events]],
# [[actions]] and [[panes]] that runs a script names one that exists, and
# every [[link_handlers]] entry names an action declared here. $1 is the
# manifest (default: the plugin's own); the plugin's directory is its.
set -u
manifest=${1:-$(cd "$(dirname "$0")/.." && pwd -P)/herdr-plugin.toml}
dir=$(cd "$(dirname "$manifest")" && pwd -P)
awk -v dir="$dir" '
function fail(why) { printf "%s:%d: %s\n", FILENAME, NR, why; bad = 1 }
function trim(s) { sub(/^[ \t]+/, "", s); sub(/[ \t]+$/, "", s); return s }
# The strings of a one-line array, into arr; their count.
function strings(v, arr,   n, s) {
  n = 0
  v = trim(v)
  if (v !~ /^\[.*\]$/) return -1
  v = substr(v, 2, length(v) - 2)
  while ((v = trim(v)) != "") {
    if (!match(v, /^"([^"\\]|\\.)*"/)) return -1
    arr[++n] = substr(v, 2, RLENGTH - 2)
    v = trim(substr(v, RLENGTH + 1))
    if (substr(v, 1, 1) == ",") v = substr(v, 2)
    else if (v != "") return -1
  }
  return n
}
function exists(path,   cmd, r) {
  cmd = "test -f \"" dir "/" path "\""
  r = system(cmd)
  return r == 0
}
function close_table() {
  if (table == "link_handlers") { handlers[++nh] = action; handler_line[nh] = start }
  table = ""
}
BEGIN { table = ""; nh = 0 }
{
  line = $0
  sub(/[ \t]+#.*$/, "", line)
  line = trim(line)
  if (line == "" || substr(line, 1, 1) == "#") next
  if (line ~ /^\[\[[a-z_]+\]\]$/) {
    close_table()
    table = substr(line, 3, length(line) - 4)
    start = NR
    action = ""
    if (table !~ /^(startup|events|actions|panes|link_handlers)$/) fail("a table herdr does not read: " line)
    next
  }
  if (!match(line, /^[a-z_]+[ \t]*=[ \t]*/)) { fail("not key = value: " $0); next }
  key = line; sub(/[ \t]*=.*/, "", key)
  val = trim(substr(line, RLENGTH + 1))
  if (val !~ /^("([^"\\]|\\.)*"|[0-9]+|true|false|\[.*\])$/) { fail("a value herdr cannot read: " $0); next }
  if (table == "") top[key] = 1
  if (key == "id" && table == "actions") actions[substr(val, 2, length(val) - 2)] = 1
  if (key == "action" && table == "link_handlers") action = substr(val, 2, length(val) - 2)
  if (key == "command" && table != "") {
    delete c
    n = strings(val, c)
    if (n < 1) { fail("a command that is not an array of strings: " $0); next }
    script = (c[1] == "sh" && n >= 2) ? c[2] : (c[1] ~ /^bin\// ? c[1] : "")
    if (script != "" && !exists(script)) fail("runs " script ", which is not in the plugin")
  }
}
END {
  close_table()
  split("id name version min_herdr_version", need, " ")
  for (i in need) if (!(need[i] in top)) { printf "%s: no %s\n", FILENAME, need[i]; bad = 1 }
  for (i = 1; i <= nh; i++)
    if (!(handlers[i] in actions)) { printf "%s:%d: a link handler names no action of this plugin: \"%s\"\n", FILENAME, handler_line[i], handlers[i]; bad = 1 }
  exit bad
}' "$manifest"
