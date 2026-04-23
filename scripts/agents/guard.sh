#!/usr/bin/env bash
# scripts/agents/guard.sh
set -euo pipefail

# ---------------------------
# Defaults (override via env)
# ---------------------------
: "${AGENT_EXECUTE:=}"              # empty => plan-only
: "${AGENT_TIMEOUT_SEC:=900}"       # hard stop to avoid runaway jobs
: "${AGENT_JOBS:=1}"                # conservative default for nextest/concurrency
: "${AGENT_LOCKFILE:=.agent.lock}"  # prevents overlapping runs
: "${AGENT_LOG_DIR:=.agent-logs}"   # all agent stdout/stderr
: "${AGENT_CRATE:=perl-parser}"     # focal crate
: "${AGENT_LANE_PATTERN:=^lane/}"   # enforce lane
: "${RUST_BACKTRACE:=1}"

mkdir -p "${AGENT_LOG_DIR}"

# ---------------------------
# Locking (portable)
# ---------------------------
lock_acquire() {
  if ! ln -s "$" "${AGENT_LOCKFILE}" 2>/dev/null; then
    echo "Another agent run is active (lock: ${AGENT_LOCKFILE})." >&2
    exit 2
  fi
}
lock_release() {
  rm -f "${AGENT_LOCKFILE}" || true
}
trap lock_release EXIT INT TERM

# ---------------------------
# Lane enforcement
# ---------------------------
require_lane() {
  local br
  br="$(git branch --show-current 2>/dev/null || true)"
  if [[ -z "${br}" ]]; then
    echo "Not in a git repo or detached HEAD." >&2
    exit 3
  fi
  if ! [[ "${br}" =~ ${AGENT_LANE_PATTERN} ]]; then
    echo "Refusing to run on non-lane branch: '${br}'. Expected pattern: ${AGENT_LANE_PATTERN}" >&2
    exit 4
  fi
}

# ---------------------------
# Execute gate
# ---------------------------
run_or_plan() {
  # Usage: run_or_plan "<label>" <cmd...>
  local label="$1"; shift
  echo "→ PLAN: ${label}"
  printf '   %q' "$@"; echo
  if [[ -n "${AGENT_EXECUTE}" ]]; then
    echo "   EXECUTE: ${label}"
    timeout "${AGENT_TIMEOUT_SEC}" "$@" 2>&1 | tee -a "${AGENT_LOG_DIR}/${label// /_}.log"
  fi
}

# ---------------------------
# Feature detection (crate-local)
# ---------------------------
has_feature() {
  local feat="$1"
  cargo metadata --no-deps --format-version 1 \
    | jq -e --arg C "${AGENT_CRATE}" --arg F "${feat}" \
      '.packages[] | select(.name==$C).features[$F]' >/dev/null
}

# ---------------------------
# Test command builder
# ---------------------------
nextest_cmd() {
  local feats=()
  for F in dynamic-delimiter-recovery recovery-dynamic-delimiters; do
    if has_feature "$F"; then feats+=("--features" "$F"); fi
  done
  echo RUST_BACKTRACE=1 cargo nextest run -p "${AGENT_CRATE}" -j "${AGENT_JOBS}" \
       --status-level=fail --no-capture "${feats[@]}"
}