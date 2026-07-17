#!/bin/bash
# Run the full viewport matrix with a live log file and a stall watchdog.
# Never run the matrix bare: piping through grep hides all progress until exit,
# and a wedged run then looks identical to a healthy one (341-minute incident,
# 2026-07-16). The harness itself fail-fasts on occupied ports 5173/9227 and
# recycles the browser between phases; this wrapper adds outer visibility.
#
# Usage: bash qa/harnesses/run-matrix-watched.sh [logFile] [stallSeconds]
LOG=${1:-/c/Users/kouty/Documents/KDMX/target/qa/matrix-watched.log}
STALL=${2:-600}
cd "$(dirname "$0")/../../app" || exit 1
rm -f "$LOG"
SYNDOCAL_VIEWPORT_TRACE=1 node scripts/check-viewport-containment.mjs > "$LOG" 2>&1 &
HARNESS_PID=$!
last_size=-1
stall_epoch=$(date +%s)
while kill -0 $HARNESS_PID 2>/dev/null; do
  size=$(stat -c %s "$LOG" 2>/dev/null || echo 0)
  now=$(date +%s)
  if [ "$size" != "$last_size" ]; then
    last_size=$size
    stall_epoch=$now
  elif [ $((now - stall_epoch)) -ge "$STALL" ]; then
    echo "MATRIX STALLED: no log growth for ${STALL}s (pid $HARNESS_PID)"
    echo "--- last 15 log lines at stall:"
    tail -15 "$LOG"
    echo "harness pid $HARNESS_PID kept ALIVE for autopsy (qa/harnesses/cdp-autopsy.mjs 9227)"
    exit 2
  fi
  sleep 20
done
wait $HARNESS_PID
EXIT=$?
echo "MATRIX FINISHED exit=$EXIT"
grep -cE "^pass" "$LOG"
grep -E "^fail|^FAIL" "$LOG" | head -5 || echo "NO FAILURES"
exit $EXIT
