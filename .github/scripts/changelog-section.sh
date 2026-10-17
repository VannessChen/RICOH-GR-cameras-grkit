#!/bin/sh
# Prints the body of one version's section of CHANGELOG.md, without its heading.
# Usage: sh .github/scripts/changelog-section.sh <version>
# Exits nonzero when the changelog has no section for that version.
set -eu

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <version>" >&2
  exit 2
fi

changelog="$(cd "$(dirname "$0")/../.." && pwd)/CHANGELOG.md"
[ -f "$changelog" ] || exit 1

# knope writes headings as "## 0.1.1 (2026-10-18)".
awk -v version="$1" '
  /^## / {
    if (found) exit
    if ($2 == version) { found = 1; next }
  }
  found { print }
  END { exit found ? 0 : 1 }
' "$changelog"
