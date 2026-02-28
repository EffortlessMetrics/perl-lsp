#!/usr/bin/env bash
set -euo pipefail

# Turnkey release orchestrator for the PR-driven release flow.
#
# 1) Trigger Version Bump & Changelog Generation workflow.
# 2) Wait for (or discover) the generated PR.
# 3) Merge the PR.
# 4) Trigger Release Orchestration.
# 5) Optionally wait for release and publish workflows to complete.

VERSION_BUMP_WORKFLOW="Version Bump & Changelog Generation"
RELEASE_ORCHESTRATION_WORKFLOW="Release Orchestration"
RELEASE_WORKFLOW="Release"
PUBLISH_CRATES_WORKFLOW="Publish to crates.io"
PUBLISH_EXTENSION_WORKFLOW="Publish VSCode Extension"
PUBLISH_DOCKER_WORKFLOW="Publish Docker Images"

DEFAULT_PRERELEASE=false
DEFAULT_SKIP_CRATES=false
DEFAULT_SKIP_EXTENSION=false
DEFAULT_SKIP_DOCKER=false
DEFAULT_AUTO_MERGE=true
DEFAULT_WAIT_PR_MERGE=true
DEFAULT_WAIT_RELEASE=true
DEFAULT_BASE_BRANCH=""
DEFAULT_TIMEOUT_SECONDS=1200

log() {
  printf '[release] %s\n' "$*"
}

warn() {
  printf '[warn] %s\n' "$*" >&2
}

err() {
  printf '[error] %s\n' "$*" >&2
}

die() {
  err "$*"
  exit 1
}

usage() {
  cat <<'USAGE'
Usage:
  scripts/release-turnkey-pr.sh --version <0.x.y> [options]
  scripts/release-turnkey-pr.sh 0.x.y [options]

Options:
  --version <0.x.y>       Release version (for example 0.9.2)
  --prerelease            Force prerelease mode for orchestrator
  --dry-run               Validate commands, do not trigger workflows
  --skip-crates           Skip crates.io publishing
  --skip-extension        Skip VSCode extension publishing
  --skip-docker           Skip Docker image publishing
  --base-branch <branch>  Release base branch (default: repo default)
  --no-auto-merge         Do not merge the version bump PR automatically
  --no-wait-pr-merge      Do not wait for PR merge after requesting
  --no-wait-release       Do not wait for release workflows after orchestration
  --workflow-timeout <s>  Workflow wait timeout (default: 1200)
  --help                  Show this help text

Examples:
  scripts/release-turnkey-pr.sh 0.9.2
  scripts/release-turnkey-pr.sh --version 0.9.2 --no-auto-merge
  scripts/release-turnkey-pr.sh --version 0.9.2 --skip-docker --skip-extension
USAGE
}

need() {
  local binary="$1"
  command -v "$binary" >/dev/null 2>&1 || die "required command not found: $binary"
}

require_clean_worktree() {
  if [[ -n "$(git status --porcelain)" ]]; then
    die "working tree is not clean. Commit or stash changes before running this command."
  fi
}

repo_url() {
  gh repo view --json nameWithOwner -q .nameWithOwner
}

validate_version() {
  local version="$1"
  if ! [[ "$version" =~ ^0\.[0-9]+\.[0-9]+(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$ ]]; then
    die "invalid 0.x.y release version: $version"
  fi
}

find_matching_run() {
  local workflow_name="$1"
  local head_sha="$2"
  local after_epoch="$3"

  gh run list \
    --workflow "$workflow_name" \
    --json databaseId,headSha,createdAt,status,conclusion \
    --limit 20 |
    jq -r --arg sha "$head_sha" --argjson after "$after_epoch" '
      .[]
      | select(.headSha == $sha)
      | select((.createdAt | fromdateiso8601) >= $after)
      | .databaseId'
}

wait_for_workflow() {
  local workflow_name="$1"
  local head_sha="$2"
  local after_epoch="$3"
  local timeout="$4"
  local deadline=$(( $(date +%s) + timeout ))

  while [[ $(date +%s) -lt $deadline ]]; do
    local run_ids
    if ! run_ids=$(find_matching_run "$workflow_name" "$head_sha" "$after_epoch"); then
      warn "no matching run yet for ${workflow_name}; waiting"
      sleep 5
      continue
    fi

    local run_id
    run_id=$(printf '%s\n' "$run_ids" | head -n 1)
    if [[ -z "$run_id" || "$run_id" == "null" ]]; then
      sleep 5
      continue
    fi

    local status conclusion
    read -r status conclusion < <(
      gh run view "$run_id" --json status,conclusion -q '"\(.status) \(.conclusion // "")"'
    )

    status=${status:-}
    conclusion=${conclusion:-}

    if [[ "$status" == "completed" ]]; then
      if [[ "$conclusion" == "success" ]]; then
        printf '%s' "$run_id"
        return 0
      fi
      gh run view "$run_id" --json url -q '.url' | sed 's/^/workflow failed: /'
      die "workflow '${workflow_name}' failed with conclusion '${conclusion}'"
    fi

    log "${workflow_name} still running (${status:-unknown})"
    sleep 10
  done

  die "timed out waiting for workflow '${workflow_name}'"
}

wait_for_pr() {
  local branch="$1"
  local base="$2"
  local timeout="$3"
  local deadline=$(( $(date +%s) + timeout ))

  while [[ $(date +%s) -lt $deadline ]]; do
    local pr_number
    pr_number=$(gh pr list \
      --head "$branch" \
      --base "$base" \
      --state open \
      --json number \
      --jq '.[0].number // empty'
    )

    if [[ -n "$pr_number" ]]; then
      printf '%s' "$pr_number"
      return 0
    fi

    sleep 6
  done

  return 1
}

wait_for_pr_merge() {
  local pr_number="$1"
  local timeout="$2"
  local deadline=$(( $(date +%s) + timeout ))

  while [[ $(date +%s) -lt $deadline ]]; do
    local merged
    merged=$(gh pr view "$pr_number" --json mergedAt -q '.mergedAt // empty')
    if [[ -n "$merged" ]]; then
      printf '%s' "merged"
      return 0
    fi
    sleep 6
  done

  return 1
}

run_workflow() {
  local workflow_name="$1"
  local ref="$2"
  local head_sha="$3"
  shift 3

  local after
  after=$(date -u +%s)
  LAST_WORKFLOW_DISPATCH_TS="$after"
  local -a args=("$@")

  if (( DRY_RUN )); then
    log "DRY RUN: gh workflow run \"$workflow_name\" --ref $ref ${args[*]}"
    return 0
  fi

  gh workflow run "$workflow_name" --ref "$ref" "${args[@]}" >/dev/null
  wait_for_workflow "$workflow_name" "$head_sha" "$after" "$WORKFLOW_TIMEOUT"
}

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  die "not a git repository"
fi

VERSION=""
PRERELEASE="$DEFAULT_PRERELEASE"
SKIP_CRATES="$DEFAULT_SKIP_CRATES"
SKIP_EXTENSION="$DEFAULT_SKIP_EXTENSION"
SKIP_DOCKER="$DEFAULT_SKIP_DOCKER"
AUTO_MERGE="$DEFAULT_AUTO_MERGE"
WAIT_PR_MERGE="$DEFAULT_WAIT_PR_MERGE"
WAIT_RELEASE="$DEFAULT_WAIT_RELEASE"
WORKFLOW_TIMEOUT="$DEFAULT_TIMEOUT_SECONDS"
DRY_RUN=0

while (($#)); do
  case "$1" in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --prerelease)
      PRERELEASE=true
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --skip-crates)
      SKIP_CRATES=true
      shift
      ;;
    --skip-extension)
      SKIP_EXTENSION=true
      shift
      ;;
    --skip-docker)
      SKIP_DOCKER=true
      shift
      ;;
    --base-branch)
      DEFAULT_BASE_BRANCH="$2"
      shift 2
      ;;
    --no-auto-merge)
      AUTO_MERGE=false
      WAIT_PR_MERGE=false
      shift
      ;;
    --no-wait-pr-merge)
      WAIT_PR_MERGE=false
      shift
      ;;
    --no-wait-release)
      WAIT_RELEASE=false
      shift
      ;;
    --workflow-timeout)
      WORKFLOW_TIMEOUT="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    -* )
      die "unknown flag: $1"
      ;;
    *)
      if [[ -z "$VERSION" ]]; then
        VERSION="$1"
      else
        die "unexpected positional argument: $1"
      fi
      shift
      ;;
  esac
done

if [[ -z "$VERSION" ]]; then
  die "version is required"
fi

validate_version "$VERSION"

need gh
need jq
need git

if ! gh auth status >/dev/null 2>&1; then
  die "gh is not authenticated. Run: gh auth login"
fi

REPO_BRANCH="${DEFAULT_BASE_BRANCH}"
if [[ -z "$REPO_BRANCH" ]]; then
  REPO_BRANCH="$(gh repo view --json defaultBranchRef --jq '.defaultBranchRef.name')"
fi

log "Release target: $VERSION"
log "Orchestration branch: $REPO_BRANCH"
log "Controls: prerelease=$PRERELEASE skip_crates=$SKIP_CRATES skip_extension=$SKIP_EXTENSION skip_docker=$SKIP_DOCKER"

git fetch origin "$REPO_BRANCH" --prune
require_clean_worktree

BASE_SHA="$(git rev-parse "origin/$REPO_BRANCH")"
if [[ -z "$BASE_SHA" ]]; then
  die "could not resolve origin/$REPO_BRANCH"
fi

HEAD_SHA="$BASE_SHA"

BUMP_BRANCH="release/v${VERSION}"

log "Dispatching version bump workflow"
bump_inputs=("--field" "version=${VERSION}")
VERSION_BUMP_DISPATCH_TS=""
if (( DRY_RUN )); then
  run_workflow "$VERSION_BUMP_WORKFLOW" "$REPO_BRANCH" "$HEAD_SHA" "${bump_inputs[@]}"
else
  VERSION_BUMP_RUN_ID=$(run_workflow "$VERSION_BUMP_WORKFLOW" "$REPO_BRANCH" "$HEAD_SHA" "${bump_inputs[@]}")
  VERSION_BUMP_DISPATCH_TS="$LAST_WORKFLOW_DISPATCH_TS"
fi

if (( DRY_RUN )); then
  log "DRY RUN complete for version bump dispatch."
  exit 0
fi

log "Version bump run: https://github.com/$(repo_url)/actions/runs/${VERSION_BUMP_RUN_ID}"

log "Waiting for version bump PR branch: $BUMP_BRANCH"
PR_NUMBER=""
if PR_NUMBER=$(wait_for_pr "$BUMP_BRANCH" "$REPO_BRANCH" 600); then
  log "Found PR #${PR_NUMBER}"
else
  die "could not detect version bump PR for branch ${BUMP_BRANCH}"
fi

PR_URL="$(gh pr view "$PR_NUMBER" --json url -q .url)"
log "Version bump PR: ${PR_URL}"

if [[ "$AUTO_MERGE" == "true" ]]; then
  log "Merging PR #${PR_NUMBER} with squash"
  gh pr merge "$PR_NUMBER" --squash --delete-branch
else
  warn "AUTO_MERGE disabled. Merge PR manually before running this script with --base-branch=$REPO_BRANCH"
fi

if [[ "$WAIT_PR_MERGE" == "true" ]]; then
  if ! wait_for_pr_merge "$PR_NUMBER" 600; then
    die "PR #${PR_NUMBER} did not merge within timeout"
  fi
  log "PR #${PR_NUMBER} merged"
fi

git fetch origin "$REPO_BRANCH" --prune
if [[ "$(git rev-parse "origin/$REPO_BRANCH")" == "$HEAD_SHA" ]]; then
  if [[ "$AUTO_MERGE" == "true" ]]; then
    warn "release branch did not move after merge according to origin/$REPO_BRANCH; please verify the PR merge outcome"
  fi
fi

RELEASE_HEAD_SHA="$(git rev-parse "origin/$REPO_BRANCH")"

log "Dispatching release orchestration for ${VERSION}"
RELEASE_ORCH_DISPATCH_TS=""
orchestration_inputs=(
  "--field" "version=${VERSION}"
  "--field" "prerelease=${PRERELEASE}"
  "--field" "skip_crates=${SKIP_CRATES}"
  "--field" "skip_extension=${SKIP_EXTENSION}"
  "--field" "skip_docker=${SKIP_DOCKER}"
)
RELEASE_ORCH_RUN_ID=$(run_workflow "$RELEASE_ORCHESTRATION_WORKFLOW" "$REPO_BRANCH" "$RELEASE_HEAD_SHA" "${orchestration_inputs[@]}")
RELEASE_ORCH_DISPATCH_TS="$LAST_WORKFLOW_DISPATCH_TS"
log "Release orchestration run: https://github.com/$(repo_url)/actions/runs/${RELEASE_ORCH_RUN_ID}"

if [[ "$WAIT_RELEASE" != "true" ]]; then
  log "Release dispatch complete. Remove --no-wait-release to monitor workflows."
  exit 0
fi

wait_for_and_print() {
  local workflow_name="$1"
  local after_epoch="$2"
  local run_id
  run_id=$(wait_for_workflow "$workflow_name" "$RELEASE_HEAD_SHA" "$after_epoch" "$WORKFLOW_TIMEOUT")
  local url
  url=$(gh run view "$run_id" --json url -q .url)
  log "${workflow_name} completed: ${url}"
}

# Wait for directly triggered orchestration target and its main children.
wait_for_and_print "$RELEASE_WORKFLOW" "$RELEASE_ORCH_DISPATCH_TS"

if [[ "$SKIP_CRATES" != "true" ]]; then
  wait_for_and_print "$PUBLISH_CRATES_WORKFLOW" "$RELEASE_ORCH_DISPATCH_TS"
fi

if [[ "$SKIP_EXTENSION" != "true" ]]; then
  wait_for_and_print "$PUBLISH_EXTENSION_WORKFLOW" "$RELEASE_ORCH_DISPATCH_TS"
fi

if [[ "$SKIP_DOCKER" != "true" ]]; then
  wait_for_and_print "$PUBLISH_DOCKER_WORKFLOW" "$RELEASE_ORCH_DISPATCH_TS"
fi

log "Release flow complete for v${VERSION}. Verify package manager update workflows and release artifacts in GitHub Releases."
