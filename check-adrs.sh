#!/bin/sh
# Every decision record under docs/adr has a number no other record has.
# The site names a record's page by its frontmatter's number
# (site/scripts/adrs.mjs), so a number two records share leaves the site
# one of them, and a citation of it names two decisions; a frontmatter
# saying another number than the file's name would hide one.
set -eu

status=0
count=0
numbers=""
for file in docs/adr/[0-9][0-9][0-9][0-9]-*.md; do
  count=$((count + 1))
  name=${file##*/}
  number=${name%%-*}
  numbers="$numbers$number $file
"
  front=$(sed -n 's/^number: *0*\([0-9][0-9]*\) *$/\1/p' "$file" | head -n 1)
  if [ -z "$front" ] || [ "$(printf '%04d' "$front")" != "$number" ]; then
    echo "check-adrs: $file says number ${front:-none} in its frontmatter, $number in its name" >&2
    status=1
  fi
done

shared=$(printf '%s' "$numbers" | cut -d ' ' -f 1 | sort | uniq -d)
for number in $shared; do
  files=$(printf '%s' "$numbers" | awk -v n="$number" '$1 == n { printf " %s", $2 }')
  echo "check-adrs: $number is the number of$files" >&2
  status=1
done
if [ -n "$shared" ]; then
  last=$(printf '%s' "$numbers" | cut -d ' ' -f 1 | sort | tail -n 1 | sed 's/^0*//')
  echo "check-adrs: the later of each takes the next free number, from $(printf '%04d' $((last + 1)))" >&2
fi

[ "$status" -ne 0 ] || echo "check-adrs: $count records, each with a number of its own"
exit "$status"
