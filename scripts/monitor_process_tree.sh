#!/bin/bash
# Monitor process tree RSS/VmHWM during LSP server execution
# Usage: ./monitor_process_tree.sh <pid> <output_file> <interval_seconds>

set -e

PID="${1:?usage: monitor_process_tree.sh <pid> <output_file> [interval_seconds]}"
OUTPUT="${2:?usage: monitor_process_tree.sh <pid> <output_file> [interval_seconds]}"
INTERVAL="${3:-5}"

echo "Monitoring PID $PID every ${INTERVAL}s, writing to $OUTPUT" >&2

{
    echo "timestamp,pid,ppid,rss_kb,vsz_kb,etimes,comm,vmhwm_kb,vmsize_kb,threads"
    while kill -0 "$PID" 2>/dev/null; do
        timestamp=$(date '+%s.%N')
        # Get process and immediate children
        ps -o pid:1,ppid:1,rss:1,vsz:1,etimes:1,comm:1 \
            --ppid "$PID" -p "$PID" 2>/dev/null | tail -n +2 | while read -r pid ppid rss vsz etimes comm; do
            vmhwm=0
            vmsize=0
            threads=0
            if [ -r "/proc/$pid/status" ]; then
                vmhwm=$(awk '/^VmHWM:/ {print int($2)}' "/proc/$pid/status")
                vmsize=$(awk '/^VmSize:/ {print int($2)}' "/proc/$pid/status")
                threads=$(awk '/^Threads:/ {print int($2)}' "/proc/$pid/status")
            fi
            echo "$timestamp,$pid,$ppid,$rss,$vsz,$etimes,$comm,$vmhwm,$vmsize,$threads"
        done
        sleep "$INTERVAL"
    done
} >> "$OUTPUT"

echo "Monitoring complete. Results in $OUTPUT" >&2
