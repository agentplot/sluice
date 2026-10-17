# shellcheck shell=sh disable=SC2154,SC2016 # jq programs; common.sh sets lib, state and config.
# sources.toml, read as the subset of TOML the README states: `[[sources]]`
# tables; `key = "string"` with \" and \\; one-line arrays of strings;
# true and false; # comments. Prints {"ahead_check", "sources": [...]};
# a line outside the subset is logged with its number and the default is
# printed instead. Sourced, never run.

default_sources='{"ahead_check":true,"sources":[{"name":"local","command":["sluice","overview","--watch","--json"],"each_repository":true}]}'

read_sources() {
  file=$config/sources.toml
  if [ ! -f "$file" ]; then
    echo "$default_sources"
    return
  fi
  if ! got=$(awk -f "$lib/toml.awk" "$file"); then
    echo "$default_sources"
    return
  fi
  printf '%s\n' "$got" | jq -s -c --argjson d "$default_sources" '
    (map(select(.i == 0)) | map({(.k): .v}) | add // {}) as $top
    | [ group_by(.i)[] | select(.[0].i > 0) | map({(.k): .v}) | add ] as $s
    | { ahead_check: (if $top | has("ahead_check") then $top.ahead_check else true end),
        sources: (if ($s | length) == 0 then $d.sources
                  else $s | map({name: (.name // "source"), command, each_repository: (.each_repository // false)})
                    | map(select((.command | type) == "array" and (.command | length) > 0)) end) }'
}
