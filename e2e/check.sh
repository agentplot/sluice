#!/usr/bin/env bash
# The check-e2e tier: e2e.sh's S1 to S4 with the sluice and wt-submit built
# from this tree, once for each place method. The scratch repositories are
# removed when every run passes and kept, with their path printed, when one
# fails.
set -euo pipefail

cd "$(dirname "$0")/.."
cargo build -p sluice --bins
PATH="$PWD/target/debug:$PATH"
export PATH
scratch=$(mktemp -d "${TMPDIR:-/tmp}/sluice-e2e.XXXXXX")
for method in git-worktree worktrunk; do
  if ! E2E_CHECKOUT=$method bash e2e/e2e.sh "$scratch/$method" sluice; then
    echo "check-e2e: $method failed; its repository is in $scratch/$method" >&2
    exit 1
  fi
done
rm -rf "$scratch"
