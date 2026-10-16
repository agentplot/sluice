# shellcheck shell=bash
# Sourced by the hooks: what sluice protects in the repository holding a
# directory. bash 3.2 (macOS's own), no eval of anything the session wrote.

# The policy committed at HEAD of the repository holding $1, or nothing.
sluice_policy() {
  git -C "$1" show HEAD:.config/sluice.toml 2>/dev/null
}

# The branch patterns the policy names, one per line: `main`, `bolt/*`.
# A `[branches."<pattern>"]` table protects its pattern (rule core.4);
# its `.pull_requests` sub-table names the same pattern again.
sluice_patterns() {
  printf '%s\n' "$1" |
    sed -n 's/^[[:space:]]*\[branches\.\(.*\)\][[:space:]]*\(#.*\)\{0,1\}$/\1/p' |
    sed 's/\.pull_requests$//; s/^"\(.*\)"$/\1/' |
    sort -u
}

# The patterns whose branch takes pull requests.
sluice_pull_patterns() {
  printf '%s\n' "$1" |
    sed -n 's/^[[:space:]]*\[branches\.\(.*\)\.pull_requests\][[:space:]]*\(#.*\)\{0,1\}$/\1/p' |
    sed 's/^"\(.*\)"$/\1/' |
    sort -u
}

# Whether branch $1 matches one of the patterns in $2 (newline-separated).
sluice_matches() {
  local branch=$1 pattern
  while IFS= read -r pattern; do
    [ -n "$pattern" ] || continue
    # shellcheck disable=SC2053 # the pattern is a glob on purpose
    if [[ $branch == $pattern ]]; then
      return 0
    fi
  done <<EOF
$2
EOF
  return 1
}
