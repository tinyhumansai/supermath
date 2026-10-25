#!/usr/bin/env bash
# Enforces the crate dependency rules of docs/specs/toolkit/crates.md.
#
#   scripts/check-crate-deps.sh [path/to/Cargo.toml]
#
# With no argument it checks the repository's own workspace. The optional
# manifest path lets scripts/test-check-crate-deps.sh run it on fixtures.
#
# Rules, applied to every dependency kind (normal, dev, build):
#   1. A domain crate (supermath-files, -lean, -python, -literature, -web,
#      -ledger, -calc) may depend on no other supermath-* crate than
#      supermath-tool. supermath-lean and supermath-python may also depend
#      on supermath-files.
#   2. No crate depends on an openhuman* crate: the agents run in the
#      rsi-ralph binary (docs/adr/0010-run-on-rsi-ralph.md).
set -euo pipefail

manifest="${1:-}"
args=(--format-version 1 --no-deps)
if [[ -n "$manifest" ]]; then
  args+=(--manifest-path "$manifest")
fi

metadata="$(cargo metadata "${args[@]}")"

violations="$(jq -r '
  def domains: ["supermath-files", "supermath-lean", "supermath-python",
                "supermath-literature", "supermath-web", "supermath-ledger",
                "supermath-calc"];
  def allowed($crate):
    ["supermath-tool"]
    + (if $crate == "supermath-lean" or $crate == "supermath-python"
       then ["supermath-files"] else [] end);
  .packages[]
  | .name as $crate
  | .dependencies[]
  | .name as $dep
  | (
      if ((domains | index($crate)) != null)
         and ($dep | startswith("supermath-"))
         and ((allowed($crate) | index($dep)) == null)
      then "\($crate) must not depend on \($dep) (domain crates may use: \(allowed($crate) | join(", ")))"
      else empty end
    ),
    (
      if ($dep | startswith("openhuman"))
      then "\($crate) must not depend on \($dep) (no crate may reach openhuman; the agents run in rsi-ralph)"
      else empty end
    )
' <<<"$metadata" | sort -u)"

if [[ -n "$violations" ]]; then
  echo "crate dependency rules violated:" >&2
  while IFS= read -r line; do
    echo "  - $line" >&2
  done <<<"$violations"
  exit 1
fi

echo "crate dependency rules hold"
