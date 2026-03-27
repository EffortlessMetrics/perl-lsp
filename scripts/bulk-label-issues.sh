#!/usr/bin/env bash
# Backfill triage labels on all open issues.
# Usage: ./scripts/bulk-label-issues.sh [repo]
#   repo defaults to $GITHUB_REPOSITORY (CI) or owner/repo (interactive)

set -euo pipefail

REPO="${1:-${GITHUB_REPOSITORY:-}}"
if [ -z "$REPO" ]; then
  echo "Usage: $0 <owner/repo>"
  exit 1
fi
if [[ ! "$REPO" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
  echo "Invalid repo format: $REPO"
  echo "Expected format: owner/repo"
  exit 1
fi

encode_label() {
  printf '%s' "$1" | jq -sR @uri
}

add_labels() {
  local number="$1"
  shift
  [ "$#" -eq 0 ] && return 0

  local payload
  payload=$(printf '%s\n' "$@" | jq -R . | jq -s '{labels: .}')
  api_call "repos/$REPO/issues/$number/labels" --input "$payload" --silent
}

api_call() {
  local url="$1"; shift
  local attempt=0
  local backoff=5
  while [ $attempt -lt 5 ]; do
    local response
    response=$(gh api "$url" "$@" 2>&1)
    local rc=$?
    if [ $rc -eq 0 ]; then
      echo "$response"
      return 0
    fi
    if echo "$response" | grep -qE 'HTTP 429|HTTP 5[0-9]{2}'; then
      attempt=$((attempt + 1))
      echo "API error (attempt $attempt/5), sleeping ${backoff}s..." >&2
      sleep "$backoff"
      backoff=$((backoff * 2))
    else
      echo "$response" >&2
      return $rc
    fi
  done
  echo "API call failed after 5 retries: $url" >&2
  return 1
}

parse_date() {
  local d="$1"
  local ts
  ts=$(date -d "$d" +%s 2>/dev/null || date -j -f "%Y-%m-%dT%H:%M:%SZ" "$d" +%s 2>/dev/null)
  case "$ts" in
    ''|*[!0-9]*)
      echo ""
      ;;
    *)
      echo "$ts"
      ;;
  esac
}

echo "Backfilling triage labels for $REPO ..."

PAGE=1
TOTAL=0

while true; do
  ISSUES=$(api_call "repos/$REPO/issues?state=open&per_page=100&page=$PAGE" --jq '.[] | select(.pull_request == null) | .number') || break
  [ -z "$ISSUES" ] && break

  for number in $ISSUES; do
    ISSUE=$(api_call "repos/$REPO/issues/$number")
    LABELS=$(echo "$ISSUE" | jq -r '.labels[].name' | tr '\n' ' ')
    CREATED=$(echo "$ISSUE" | jq -r '.created_at')
    UPDATED=$(echo "$ISSUE" | jq -r '.updated_at')
    ASSIGNEES=$(echo "$ISSUE" | jq -r '.assignees | length')
    BODY=$(echo "$ISSUE" | jq -r '.body // ""')
    COMMENTS_COUNT=$(echo "$ISSUE" | jq -r '.comments')

    TO_ADD=()
    TO_REMOVE=()

    # Age
    NOW=$(date +%s)
    UPDATED_TS=$(parse_date "$UPDATED")

    if [ -n "$UPDATED_TS" ]; then
      DAYS=$(( (NOW - UPDATED_TS) / 86400 ))

      [ "$DAYS" -ge 7 ]  && TO_ADD+=("stale-7d")
      [ "$DAYS" -ge 14 ] && TO_ADD+=("stale-14d")
      [ "$DAYS" -ge 30 ] && TO_ADD+=("stale-30d")

      # Remove stale labels that no longer apply
      if [ "$DAYS" -lt 7 ]; then
        TO_REMOVE+=("stale-7d" "stale-14d" "stale-30d")
      elif [ "$DAYS" -lt 14 ]; then
        TO_REMOVE+=("stale-14d" "stale-30d")
      elif [ "$DAYS" -lt 30 ]; then
        TO_REMOVE+=("stale-30d")
      fi
    else
      echo "#$number: WARNING: could not parse updated_at, skipping age labels"
    fi

    # needs-assignee
    CREATED_TS=$(parse_date "$CREATED")

    if [ -n "$CREATED_TS" ]; then
      CREATED_DAYS=$(( (NOW - CREATED_TS) / 86400 ))

      if [ "$ASSIGNEES" -eq 0 ] && [ "$CREATED_DAYS" -ge 3 ]; then
        TO_ADD+=("needs-assignee")
      elif [ "$ASSIGNEES" -gt 0 ]; then
        TO_REMOVE+=("needs-assignee")
      fi
    else
      echo "#$number: WARNING: could not parse created_at, skipping needs-assignee"
    fi

    # needs-spec
    HAS_SPEC=0
    echo "$BODY" | grep -qiE '(## plan|## spec|implementation plan|acceptance criteria|## approach|## steps|## design|## implementation|## how|## proposal|## roadmap|### plan|### spec|### steps|### approach|## task|## todo|## sub-tasks)' && HAS_SPEC=1
    [ "$HAS_SPEC" -eq 0 ] && [ -n "$BODY" ] && [ "$COMMENTS_COUNT" -eq 0 ] && TO_ADD+=("needs-spec")
    [ "$HAS_SPEC" -eq 1 ] && TO_REMOVE+=("needs-spec")

    # Size: require body > 0, cap comment contribution at 20
    BODY_LEN=${#BODY}
    COMMENT_SCORE=$(( COMMENTS_COUNT * 5 ))
    [ "$COMMENT_SCORE" -gt 20 ] && COMMENT_SCORE=20
    if [ "$BODY_LEN" -gt 0 ]; then
      BODY_SCORE=$(( BODY_LEN / 100 ))
    else
      BODY_SCORE=0
    fi
    SCORE=$(( BODY_SCORE + COMMENT_SCORE ))

    [ "$SCORE" -le 8 ]  && TO_ADD+=("size/S")
    [ "$SCORE" -gt 8 ]  && [ "$SCORE" -le 25 ] && TO_ADD+=("size/M")
    [ "$SCORE" -gt 25 ] && TO_ADD+=("size/L")

    # Size removal: clear old size labels when transitioning
    if [ "$SCORE" -le 8 ]; then
      TO_REMOVE+=("size/M" "size/L")
    elif [ "$SCORE" -le 25 ]; then
      TO_REMOVE+=("size/S" "size/L")
    else
      TO_REMOVE+=("size/S" "size/M")
    fi

    # Filter already-present for adds, already-absent for removes
    ADD_ARGS=()
    for lbl in "${TO_ADD[@]}"; do
      echo "$LABELS" | grep -qw "$lbl" || ADD_ARGS+=("$lbl")
    done

    REMOVE_ARGS=()
    for lbl in "${TO_REMOVE[@]}"; do
      echo "$LABELS" | grep -qw "$lbl" && REMOVE_ARGS+=("$lbl")
    done

    if [ ${#ADD_ARGS[@]} -gt 0 ]; then
      echo "#$number: + ${ADD_ARGS[*]}"
      add_labels "$number" "${ADD_ARGS[@]}"
      TOTAL=$((TOTAL + 1))
    fi

    if [ ${#REMOVE_ARGS[@]} -gt 0 ]; then
      echo "#$number: - ${REMOVE_ARGS[*]}"
      for lbl in "${REMOVE_ARGS[@]}"; do
        api_call "repos/$REPO/issues/$number/labels/$(encode_label "$lbl")" \
          -X DELETE --silent
      done
    fi

    if [ ${#ADD_ARGS[@]} -eq 0 ] && [ ${#REMOVE_ARGS[@]} -eq 0 ]; then
      echo "#$number: already labeled"
    fi
  done

  PAGE=$((PAGE + 1))
done

echo "Done. Processed issues with $TOTAL label additions."
