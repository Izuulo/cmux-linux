#!/usr/bin/env bash
# M6 perf check: 4x5 panes, 19 flood `seq`, 20th (caller) free for typing.
# Run from a pane of the app under test. From outside: CMUX_SOCKET_PATH=<sock> CALLER=<surface ref> [CMUX=<cli>].
set -euo pipefail
CMUX=${CMUX:-cmux}
CALLER=${CALLER:-${CMUX_SURFACE_ID:?run inside a cmux pane or set CALLER}}
APP_PID=${APP_PID:-$(pgrep -nx cmux-app || pgrep -nx cmux-linux)}
SOAK=${SOAK:-60}

split() { # dir surface -> new surface ref
  local out
  out=$("$CMUX" new-split "$1" --surface "$2" --focus false 2>&1) || {
    echo "split $1 of $2 failed: $out (invalid_state: Not enough space to split -> enlarge the window)" >&2; exit 1; }
  awk '{print $2}' <<<"$out"
}

others=()
rows=("$CALLER"); last=$CALLER
for _ in 1 2 3; do last=$(split down "$last"); rows+=("$last"); others+=("$last"); done
for row in "${rows[@]}"; do # columns after rows, else a row's split nests under one column
  s=$row
  for _ in 1 2 3 4; do s=$(split right "$s"); others+=("$s"); done
done
echo "panes: $((${#others[@]} + 1))"

for s in "${others[@]}"; do
  "$CMUX" send --surface "$s" 'while :; do seq 100000; done' >/dev/null
  "$CMUX" send-key --surface "$s" enter >/dev/null
done
echo "loops started; type in the caller pane now (soak ${SOAK}s)"
sleep "$SOAK"
echo "under load: rss_kb cpu% = $(ps -o rss=,%cpu= -p "$APP_PID")"
for s in "${others[@]}"; do "$CMUX" send-key --surface "$s" ctrl+c >/dev/null; done
sleep 5
echo "after stop: rss_kb cpu% = $(ps -o rss=,%cpu= -p "$APP_PID")"
