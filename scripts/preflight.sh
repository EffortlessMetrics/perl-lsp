#!/usr/bin/env bash
set -euo pipefail

# Show current pressure
pids_used=$(ps -e --no-headers | wc -l | xargs)
pid_max=$(cat /proc/sys/kernel/pid_max)
files_used=$(awk '{print $2}' /proc/sys/fs/file-nr)
files_max=$(cat /proc/sys/fs/file-max)
echo "PIDs: $pids_used / $pid_max | Open files: $files_used / $files_max"

# Conservative defaults (overridable via env in CI)
export UV_THREADPOOL_SIZE="${UV_THREADPOOL_SIZE:-4}"
export PW_WORKERS="${PW_WORKERS:-2}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-2}"
export OMP_NUM_THREADS="${OMP_NUM_THREADS:-1}"
export OPENBLAS_NUM_THREADS="${OPENBLAS_NUM_THREADS:-1}"
export MKL_NUM_THREADS="${MKL_NUM_THREADS:-1}"
export NUMEXPR_NUM_THREADS="${NUMEXPR_NUM_THREADS:-1}"

# If near PID limit, drop to ultra‑safe mode
if [ "$pids_used" -gt $((pid_max * 85 / 100)) ]; then
  export PW_WORKERS=1
  export RUST_TEST_THREADS=1
  export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1 NUMEXPR_NUM_THREADS=1
  echo "System hot → auto‑degraded workers (PW=1, RUST=1, *BLAS=1)"
fi