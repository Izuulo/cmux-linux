#!/usr/bin/env bash
# STATUS.md is the permanent handoff and must stay at most 100 lines.
set -euo pipefail
FILE="${1:-$(dirname "$0")/../STATUS.md}"
n=$(wc -l < "$FILE")
if (( n > 100 )); then
  echo "$FILE has $n lines (max 100). Archive detail to docs/status-archive.md." >&2
  exit 1
fi
echo "$FILE: $n lines"
