# What the plugin says, from sluice/overview@1 documents: the same words as
# plugins/sluice-board/hooks/board.ts, so a blame reads the same in both.

# A repository's name, as name_of in common.sh gives it.
def repo_name: sub("/\\.git$"; "") | split("/") | last | sub("\\.git$"; "");

def pad2: tostring | if length < 2 then "0" + . else . end;

# `2m10s`, `45s`, `1h03m`.
def duration: (if . < 0 then 0 else . end | floor) as $s
  | if $s < 60 then "\($s)s"
    elif $s < 3600 then "\($s / 60 | floor)m\($s % 60 | pad2)s"
    else "\($s / 3600 | floor)h\(($s % 3600) / 60 | floor | pad2)m" end;

# `15m` for a limit: whole minutes where it divides.
def limit: if . % 60 == 0 and . < 3600 then "\(. / 60)m" else duration end;

# Seconds since an RFC 3339 time, any fraction of a second dropped.
def since($now): if . == null then null
  else (sub("\\.[0-9]+"; "") | try fromdateiso8601 catch null) as $t
    | if $t == null then null else $now - $t end end;

def plural($n; $word): "\($n) \($word)\(if $n == 1 then "" else "s" end)";

def combination($branch): [$branch] + map("#\(.)") | join("+");

# --- A combination's tasks --------------------------------------------

def ended: .state == "passed" or .state == "failed" or .state == "replayed";

# Ten cells, one filled for each tenth of the tasks ended, rounded down.
def bar($done; $total): (if $total > 0 then (10 * ([$done, $total] | min) / $total | floor) else 0 end) as $full
  | ("▓" * $full // "") + ("░" * (10 - $full) // "");

# `3/5 ▓▓▓▓▓▓░░░░`: how many of a combination's tasks have ended, or null
# when its runner says nothing.
def progress: if (.tasks // []) | length == 0 then null
  else ([.tasks[] | select(ended)] | length) as $done
    | "\($done)/\(.tasks | length) \(bar($done; .tasks | length))" end;

def at_seconds: if . == null then null else sub("\\.[0-9]+"; "") | try fromdateiso8601 catch null end;

# What follows a task's name: how long it took once ended, else its state.
def task_detail: if .state == "replayed" or .state == "running" or .state == "waiting" then .state
  else ((.ended | at_seconds) as $e | (.started | at_seconds) as $s
    | if $e != null and $s != null then $e - $s | duration else .state end) end;

def pad_end($n): . + (" " * ($n - length) // "");
def pad_start($n): (" " * ($n - length) // "") + .;

# The task list: ended tasks in the order they ended, then running, then
# waiting, each {state, text} as the board's pane lists them.
def task_lines: (.tasks // []) | to_entries
  | map(.value as $t | {t: $t, key: [
      (if $t | ended then 0 elif $t.state == "running" then 1 else 2 end),
      (if $t | ended then ($t.ended | at_seconds) // 0 else 0 end),
      .key]})
  | sort_by(.key) | map(.t)
  | ([.[].target | length] | max // 0) as $name
  | ([.[] | task_detail | length] | max // 0) as $detail
  | map({state, text: "\({passed: "✓", replayed: "✓", failed: "✗", running: "…", waiting: "·"}[.state] // "·") \(.target | pad_end($name))  \(task_detail | pad_start($detail))"});

def testing: .candidates | map(select(.state == "testing" or .state == "starting"));

def waiting: (testing | map(.submissions[]) ) as $under
  | [.submissions[].id | select(. as $i | $under | index($i) | not)];

def short: if type == "string" then .[0:7] else "" end;

# What a recent log line says, in a few words.
def recent_words: if .event == "landed" then "\((.ids // []) | map("#\(.)") | join(" ")) landed as \(.to | short)"
  elif .event == "blamed" then "#\(.id) blamed on \(.combination | short)"
  elif .event == "conflict" then "#\(.id) conflict · \((.paths // []) | join(", "))"
  else (.event | tostring) end;

# The band's line for one branch, or null when it has nothing to show.
def branch_line($now): . as $b | testing as $under
  | if $b.depth == 0 and ($under | length) == 0 then null
    else ["sluice \($b.branch)", plural($b.depth; "submission")]
      + ($under | map((.started | since($now)) as $took
          | "testing \(.submissions | combination($b.branch))"
            + (progress | if . then " " + . else "" end)
            + (if $took == null then "" else " \($took | duration)" + (if .limit then "/\(.limit | limit)" else "" end) end)))
      + ($b | waiting | if length > 0 then [(map("#\(.)") | join(" ")) + " waiting"] else [] end)
      + ($b.recent | if length > 0 then ["last \(.[-1] | recent_words)"] else [] end)
      | join(" · ") end;

# The tab-bar segment: every branch with a queue, on one line.
def segment($now): [.[] | .branches[] | branch_line($now) | select(. != null)] | join("  |  ");

def paint($on; $code): if $on then "\u001b[\($code)m\(.)\u001b[0m" else . end;

def pull_of: (capture("^pull/(?<n>[0-9]+)$") | .n) // null;

# The rows under a combination under test: its tasks, each painted by its state.
def candidate_rows($c; $color): $c | task_lines
  | map(.state as $s | "        \(.text)" | paint($color; {passed: "32", replayed: "32", failed: "31", running: "33", waiting: "2"}[$s] // "2")
    | {text: ., kind: "task"});

# One document's rows, as the board's /sluice pane draws them, under the
# headings TESTING, QUEUED and RECENT: {text, kind, url?}. A row with a url
# is a link: a submission to its worktree's pane, a failed test to its log.
def rows($now; $color): . as $d
  | def row($kind): {text: ., kind: $kind};
  [("\($d.repository) on \($d.host)" | paint($color; "2") | row("heading"))]
  + (if ($d.branches | length) == 0 then ["No branch has a queue." | paint($color; "2") | row("heading")] else [] end)
  + [ $d.branches[] | . as $b | testing as $under
      | ("" | row("blank"))
      , ("\($b.branch) at \(($b.at // "nothing") | .[0:7]) · \(plural($b.depth; "submission")) · \(plural($under | length; "combination")) under test\(if $b.parallel then " (up to \($b.parallel) at once)" else "" end)" | paint($color; "1") | row("heading"))
      , ( ($under + ($b.candidates | map(select(.state == "formed")))) as $shown
          | if ($shown | length) > 0 then
              ("  TESTING" | paint($color; "1") | row("heading"))
              , ($under[] | . as $c | (.started | since($now)) as $took
                  | ("    ▶ testing \(.submissions | combination($b.branch))"
                      + (progress | if . then "  " + . else "" end)
                      + (if $took == null then "" else "  \($took | duration)" + (if .limit then " / \(.limit | limit)" else "" end) end) | paint($color; "33") | row("candidate"))
                  , ("        \(.targets | join(" "))\(if .executor then " · \(.executor)" else "" end)" | paint($color; "2") | row("candidate"))
                  , candidate_rows($c; $color)[])
              , ($b.candidates[] | select(.state == "formed")
                  | "        formed, not running: \(.submissions | combination($b.branch))" | paint($color; "2") | row("candidate"))
            else empty end )
      , ( if ($b.submissions | length) > 0 then
            ("  QUEUED" | paint($color; "1") | row("heading"))
            , ($b.submissions[] | (.from.worktree | pull_of) as $pull
                | ("    #\(.id) \(.state + "         " | .[0:9]) \(if $pull then "PR \($pull)" else .from.worktree end) \(.from.agent)" | row("submission"))
                  + (if .from.path then {url: "sluice-herdr:focus?path=\(.from.path | @uri)"} else {} end))
          else empty end )
      , ( if ($b.recent | length) > 0 then
            ("  RECENT" | paint($color; "1") | row("heading"))
            , ($b.recent | reverse | .[0:5][]
                | (if .event == "landed" then ["✓", "32"] elif .event == "blamed" then ["✗", "31"] else ["⚠", "33"] end) as [$mark, $c]
                | "    \($mark) \(.at[11:16]) \(recent_words)" | paint($color; $c) | row("event"))
            , ([$b.candidates[] | select(any(.failed[]; .log))] | last // empty
                | .failed[] | select(.log)
                | ("      log of \(.task): \(.log)" | row("failure"))
                  + {url: "sluice-herdr:log?path=\(.log | @uri)&task=\(.task | @uri)"})
          else empty end )
    ]
  + [("" | row("blank")), ("\($d.sessions | length) worktrees held · \($d.previews | length) previews" | paint($color; "2") | row("heading"))];

# Rows with a url numbered 1 to 9, in the order drawn.
def numbered: reduce .[] as $r ({n: 0, out: []};
    if $r.url and .n < 9 then .n += 1 | .out += [$r + {n: .n}] else .out += [$r] end) | .out;

# A row as a line: its number in the margin its indent leaves, and, with
# $links, its text an OSC 8 link to its url.
def line($links):
  (if .n then "[\(.n)]" + (.text | .[3:]) else .text end) as $t
  | if $links and .url then "\u001b]8;;\(.url)\u001b\\\($t)\u001b]8;;\u001b\\" else $t end;

# One document drawn as lines, as before rows; the tab-bar and tests use it.
def frame($now; $color): rows($now; $color) | map(.text);

# --- The sidebar -------------------------------------------------------

def minutes: "\(. / 60 | floor)m";

# The `sluice` token for the worktree $w, from its repository's documents,
# the submissions learned for it ($paths: id -> {path, branch}), the
# blamed tests ($failed: id -> [task]) and its last forecast ($check).
def pane_token($w; $paths; $failed; $check; $now):
  . as $docs
  | ($paths | to_entries | map(select(.value.path == $w) | .key | tonumber)) as $mine
  | [ $docs[].branches[] | . as $b | .submissions[] | select(.from.path == $w)
      | . as $s | ($b | testing | map(select(.submissions | index($s.id))) | first) as $c
      | if $c then "#\($s.id) testing" + ($c | progress | if . then " " + . else "" end)
          + (($c.started | since($now)) as $took | if $took == null then "" else " \($took | minutes)" + (if $c.limit then "/\($c.limit | minutes)" else "" end) end)
        else "#\($s.id) \($s.state)" end ] as $live
  | [ $docs[].branches[].recent[]
      | select((.event == "blamed" or .event == "conflict") and (.id as $i | $mine | index($i)))
      | . as $r | select($mine | all(. <= $r.id)) ] | sort_by(.at) | last as $out
  | [ $docs[].branches[].recent[] | select(.event == "landed")
      | select(any(.ids[]; . as $i | $mine | index($i)))
      | {at, id: ([.ids[] | . as $i | select($mine | index($i))] | max)} ] | sort_by(.at) | last as $landed
  | if ($live | length) > 0 then $live[0]
    elif $out and (($landed == null) or ($landed.id < $out.id)) then
      # Only a task with a plain name fits the line: a command runner's
      # task is its whole command, which the popup still shows.
      if $out.event == "blamed" then "#\($out.id) BLAMED" + (($failed[$out.id | tostring] // []) | map(select(test("^\\S{1,24}$")))
          | if length > 0 then " · " + join(", ") else "" end)
      else "#\($out.id) CONFLICT" + (($out.paths // []) | if length > 0 then " · " + .[0] else "" end) end
    elif $check != null and ($check.count // 0) > 0 then
      "\($check.count) ahead" + (if $check.result == "clean" then " · clean"
        elif $check.result == "conflicts" then " · conflicts " + (if ($check.with | length) > 0 then ($check.with | map("#\(.)") | join(" ")) else $check.branch end)
        else "" end)
    elif $landed and (($landed.at | since($now)) // 99999) < 1800 then
      "#\($landed.id) landed \($landed.at | since($now) | minutes)"
    else "" end;

# The `sluice_q` token: every branch's submissions in these documents.
def queue_token: ([.[].branches[].depth] | add // 0) as $n
  | if $n > 0 then plural($n; "submission") else "" end;

# --- Events ------------------------------------------------------------

def event_key($branch): "\($branch)|\(.event)|\(.id // (.ids | map(tostring) | join(",")))|\(.at)";

def events: .branches[] | .branch as $b | .recent[] | {branch: $b, key: event_key($b), event: .};
