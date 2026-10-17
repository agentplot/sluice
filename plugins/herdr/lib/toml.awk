# The subset of TOML sources.toml is read as, one JSON line per key:
#   {"i": <0 at the top, n in the nth [[sources]]>, "k": "<key>", "v": <value>}
# Exits 1, naming the line, at anything outside the subset.

function fail(why) {
  printf "sources.toml:%d: %s: %s\n", NR, why, $0 > "/dev/stderr"
  bad = 1
  exit 1
}

# A basic string at the start of s: sets str (as JSON) and rest.
function string(s,   i, c) {
  if (substr(s, 1, 1) != "\"") return 0
  for (i = 2; i <= length(s); i++) {
    c = substr(s, i, 1)
    if (c == "\\") { i++; continue }
    if (c == "\"") { str = substr(s, 1, i); rest = substr(s, i + 1); return 1 }
  }
  return 0
}

function trim(s) { sub(/^[ \t]+/, "", s); sub(/[ \t]+$/, "", s); return s }

function done(s) {
  s = trim(s)
  if (s != "" && substr(s, 1, 1) != "#") fail("text after the value")
}

BEGIN { n = 0 }

{
  line = trim($0)
  if (line == "" || substr(line, 1, 1) == "#") next
  if (line ~ /^\[\[[ \t]*sources[ \t]*\]\]([ \t]*#.*)?$/) { n++; next }
  if (line ~ /^\[/) fail("only [[sources]] tables are read")
  if (!match(line, /^[A-Za-z0-9_-]+[ \t]*=[ \t]*/)) fail("not key = value")
  key = line
  sub(/[ \t]*=.*/, "", key)
  val = substr(line, RLENGTH + 1)
  if (val ~ /^(true|false)/) {
    v = val; sub(/[^a-z].*$/, "", v)
    done(substr(val, length(v) + 1))
  } else if (string(val)) {
    v = str
    done(rest)
  } else if (substr(val, 1, 1) == "[") {
    s = trim(substr(val, 2)); v = "["; first = 1
    while (substr(s, 1, 1) != "]") {
      if (!string(s)) fail("arrays hold strings on one line")
      v = v (first ? "" : ",") str; first = 0
      s = trim(rest)
      if (substr(s, 1, 1) == ",") s = trim(substr(s, 2))
      else if (substr(s, 1, 1) != "]") fail("arrays hold strings on one line")
    }
    v = v "]"
    done(substr(s, 2))
  } else fail("values are strings, arrays of strings, true or false")
  gsub(/"/, "\\\"", key)
  printf "{\"i\":%d,\"k\":\"%s\",\"v\":%s}\n", n, key, v
}

END { if (bad) exit 1 }
