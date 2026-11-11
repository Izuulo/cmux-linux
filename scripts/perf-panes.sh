#!/usr/bin/env bash
# M6 perf check: 4x5 panes, 19 flood `seq`, 20th (caller) free for typing.
# Run from a pane of the app under test. From outside: CMUX_SOCKET_PATH=<sock> CALLER=<surface ref> [CMUX=<cli>].
set -euo pipefail
CMUX=${CMUX:-cmux}
CALLER=${CALLER:-${CMUX_SURFACE_ID:?run inside a cmux pane or set CALLER}}
APP_PID=${APP_PID:-$(pgrep -nx cmux-app || pgrep -nx cmux-linux || true)}
[ -n "$APP_PID" ] || { echo "set APP_PID" >&2; exit 1; }
SOAK=${SOAK:-60}

split() { # dir surface -> new surface ref
  local out
  out=$("$CMUX" new-split "$1" --surface "$2" --focus false 2>&1) || {
    echo "split $1 of $2 failed: $out (invalid_state: Not enough space to split -> enlarge the window)" >&2; exit 1; }
  out=$(awk '{print $2}' <<<"$out")
  [ -n "$out" ] || { echo "split $1 of $2: no surface ref in reply" >&2; exit 1; }
  echo "$out"
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
# cpu_window <seconds>: mean CPU % of the app over the window (Linux: /proc ticks; macOS: top delta sample).
cpu_window() {
  if [ -r "/proc/$APP_PID/stat" ]; then
    local a b; a=$(ticks); sleep "$1"; b=$(ticks)
    awk -v d=$((b - a)) -v hz="$(getconf CLK_TCK)" -v s="$1" 'BEGIN{printf "%.1f", d / hz / s * 100}'
  else
    top -l 2 -s "$1" -pid "$APP_PID" | awk '$1 == pid {v = $3} END{print v}' pid="$APP_PID"
  fi
}
ticks() { sed 's/.*) //' "/proc/$APP_PID/stat" | awk '{print $12 + $13}'; } # utime+stime (fields 14,15)
mem() { # rss plus macOS phys_footprint (rss excludes compressed pages there)
  echo "rss_kb=$(ps -o rss= -p "$APP_PID")"
  command -v footprint >/dev/null && footprint "$APP_PID" 2>/dev/null | grep -i 'footprint:' | head -1 || true
}
sleep $((SOAK - 5))
echo "under load: $(mem); cpu% over 5 s = $(cpu_window 5)"
for s in "${others[@]}"; do "$CMUX" send-key --surface "$s" ctrl+c >/dev/null; done
sleep 1
echo "after stop: $(mem); cpu% over 5 s (from +1 s) = $(cpu_window 5)"
