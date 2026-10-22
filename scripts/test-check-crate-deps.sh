#!/usr/bin/env bash
# Runs scripts/check-crate-deps.sh over the fixture workspaces: the passing
# one must pass, and every other one must fail naming the crate that breaks
# its rule.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
checker="$here/check-crate-deps.sh"
fixtures="$here/fixtures/check-crate-deps"
failures=0

# The text a failing fixture's output must contain; empty for an unknown
# fixture. (A case statement, because macOS ships bash 3.2 without
# associative arrays.)
expected() {
  case "$1" in
    domain-depends-on-domain) echo "supermath-calc must not depend on supermath-literature" ;;
    lean-depends-on-literature) echo "supermath-lean must not depend on supermath-literature" ;;
    calc-depends-on-files) echo "supermath-calc must not depend on supermath-files" ;;
    domain-depends-on-openhuman) echo "supermath-calc must not depend on openhuman-embed" ;;
    mcp-depends-on-openhuman) echo "supermath-mcp must not depend on openhuman-embed" ;;
    domain-depends-on-agent) echo "supermath-web must not depend on supermath-agent" ;;
    *) echo "" ;;
  esac
}

if output="$("$checker" "$fixtures/passing/Cargo.toml" 2>&1)"; then
  echo "ok   passing"
else
  echo "FAIL passing: expected success, got:" >&2
  echo "$output" >&2
  failures=$((failures + 1))
fi

for dir in "$fixtures"/*/; do
  name="$(basename "$dir")"
  [[ "$name" == passing ]] && continue
  want="$(expected "$name")"
  if [[ -z "$want" ]]; then
    echo "FAIL $name: fixture has no expectation in this script" >&2
    failures=$((failures + 1))
  elif output="$("$checker" "$dir/Cargo.toml" 2>&1)"; then
    echo "FAIL $name: expected the checker to fail" >&2
    failures=$((failures + 1))
  elif [[ "$output" != *"$want"* ]]; then
    echo "FAIL $name: output lacks '$want':" >&2
    echo "$output" >&2
    failures=$((failures + 1))
  else
    echo "ok   $name"
  fi
done

if [[ $failures -ne 0 ]]; then
  echo "$failures check(s) failed" >&2
  exit 1
fi
