#!/usr/bin/env bash
# Test suite for scripts/worktree-manager.py `allocate` (issue #3749).
#
# Bug: `allocate()` always branched off `base_ref()` (local/origin master or
# main) regardless of whether `--branch` already exists on `origin`. Slot
# re-allocation for a branch already pushed to `origin` silently diverged the
# worktree from the real branch content instead of checking it out — observed
# 3x in one session, a data-loss footgun. `base_ref()` also never fetched, so
# even the genuinely-new-branch path could cut from a stale local tracking
# ref.
#
# This suite is fully hermetic: it builds a throwaway bare "origin" repo plus
# a clone under a tmpdir, copies the worktree-manager script under test into
# that fixture tree (REPO_ROOT is derived from the script's own file location
# via `Path(__file__).resolve().parents[1]`, not cwd), and asserts against
# real git state. No network access, no interaction with the real repo's
# .ops-perl-lsp state.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
MANAGER_SRC="${REPO_ROOT}/scripts/worktree-manager.py"

PASS_COUNT=0
FAIL_COUNT=0
TMPDIR_BASE=""

pass() { printf 'PASS %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

cleanup() {
  if [[ -n "${TMPDIR_BASE:-}" && -d "${TMPDIR_BASE}" ]]; then
    rm -rf "${TMPDIR_BASE}"
  fi
}
trap cleanup EXIT

if [[ ! -f "$MANAGER_SRC" ]]; then
  echo "ERROR: worktree-manager.py not found at ${MANAGER_SRC}"
  exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "ERROR: python3 not found on PATH"
  exit 1
fi

TMPDIR_BASE="$(mktemp -d)"

git_q() { git -c user.name="Fixture" -c user.email="fixture@example.com" "$@" >/dev/null 2>&1; }

echo "=== worktree-manager allocate self-test (#3749) ==="
echo ""

# ── Fixture setup ───────────────────────────────────────────────────────────
# origin.git: bare "remote" repo.
ORIGIN_BARE="${TMPDIR_BASE}/origin.git"
git -c init.defaultBranch=main init -q --bare "$ORIGIN_BARE"

# agent-one: the clone that will act as REPO_ROOT for the script under test.
AGENT_ONE="${TMPDIR_BASE}/agent-one"
git_q clone -q "$ORIGIN_BARE" "$AGENT_ONE"
(
  cd "$AGENT_ONE"
  git checkout -B main -q
  echo "init" > file.txt
  git -c user.name="Fixture" -c user.email="fixture@example.com" add file.txt
  git -c user.name="Fixture" -c user.email="fixture@example.com" commit -q -m "init"
  git push -q origin main
)
MAIN_A_SHA="$(git -C "$AGENT_ONE" rev-parse main)"

# From a throwaway third clone, push an already-existing remote branch with
# content that diverges from main — this is the branch a slot re-allocation
# will target. Using a separate clone (rather than switching agent-one's own
# branch back and forth) keeps agent-one's working branch untouched.
AGENT_THREE="${TMPDIR_BASE}/agent-three"
git_q clone -q "$ORIGIN_BARE" "$AGENT_THREE"
(
  cd "$AGENT_THREE"
  git checkout -b existing/feature -q
  echo "feature work" >> file.txt
  git -c user.name="Fixture" -c user.email="fixture@example.com" commit -q -am "feature commit"
  git push -q origin existing/feature
)
EXISTING_FEATURE_SHA="$(git -C "$ORIGIN_BARE" rev-parse refs/heads/existing/feature)"

# From a second, independent clone, advance origin/main further. agent-one's
# local `main` (MAIN_A_SHA) does NOT see this commit until it fetches —
# reproducing "stale local main" for the genuinely-new-branch case.
AGENT_TWO="${TMPDIR_BASE}/agent-two"
git_q clone -q "$ORIGIN_BARE" "$AGENT_TWO"
(
  cd "$AGENT_TWO"
  git checkout -B main -q
  echo "advanced by someone else" >> file.txt
  git -c user.name="Fixture" -c user.email="fixture@example.com" commit -q -am "advance main"
  git push -q origin main
)
ORIGIN_MAIN_TIP_SHA="$(git -C "$ORIGIN_BARE" rev-parse refs/heads/main)"

if [[ "$ORIGIN_MAIN_TIP_SHA" == "$MAIN_A_SHA" ]]; then
  echo "ERROR: fixture setup failed to advance origin/main past agent-one's local main"
  exit 1
fi

# A third existing-on-origin branch, used by Case 3 to prove that a fetch
# failure fails closed instead of silently checking out a stale/unverified
# ref. Its commit object is deliberately corrupted on origin below (Case 3
# setup) so `ls-remote` (ref metadata only) still succeeds while the actual
# `fetch` (needs the object) fails -- the exact scenario both review threads
# on #3749 flagged: ls-remote confirming existence does not guarantee the
# follow-up fetch lands the ref.
AGENT_FOUR="${TMPDIR_BASE}/agent-four"
git_q clone -q "$ORIGIN_BARE" "$AGENT_FOUR"
(
  cd "$AGENT_FOUR"
  git checkout -b flaky/branch -q
  echo "flaky branch work" >> file.txt
  git -c user.name="Fixture" -c user.email="fixture@example.com" commit -q -am "flaky branch commit"
  git push -q origin flaky/branch
)
FLAKY_BRANCH_SHA="$(git -C "$ORIGIN_BARE" rev-parse refs/heads/flaky/branch)"

# Place the script under test where REPO_ROOT resolution expects it:
# `<repo>/scripts/worktree-manager.py` inside agent-one's checkout.
mkdir -p "${AGENT_ONE}/scripts"
cp "$MANAGER_SRC" "${AGENT_ONE}/scripts/worktree-manager.py"

STATE_FILE="${TMPDIR_BASE}/state.json"
MANAGED_ROOT="${TMPDIR_BASE}/managed-worktrees"

run_manager() {
  # --state-file/--managed-root must come after the subcommand: the script's
  # argparse defines them on both the top-level parser and each subparser
  # (via a shared `parents=[common]`), and the subparser's own defaults
  # silently overwrite values set at the top level if passed before the
  # subcommand name.
  local subcommand="$1"
  shift
  (
    cd "$AGENT_ONE"
    python3 scripts/worktree-manager.py "$subcommand" --state-file "$STATE_FILE" --managed-root "$MANAGED_ROOT" "$@"
  )
}

# ── Case 3: a fetch failure while resolving an existing-on-origin branch
#    must fail CLOSED — never silently check out a stale/unverified ref.
#    (Review follow-up on #3749: `ls-remote` confirming existence does not
#    guarantee the subsequent fetch actually landed the ref.)
#
# Reproduced by corrupting (deleting) the flaky/branch tip commit's loose
# object on the bare origin repo: `ls-remote` only reads ref metadata (still
# succeeds, still reports the SHA), but `fetch` needs the actual object and
# fails -- an OS-independent stand-in for a dropped connection mid-transfer,
# without relying on filesystem permissions or PATH-shimmed binaries (which
# don't reproduce reliably across platforms for a Python-spawned `git`).
#
# MUST run before Case 1/2: Case 2's genuinely-new-branch path does a plain
# `git fetch origin` (all branches, no refspec) in agent-one's own clone. If
# that ran first, agent-one would already have flaky/branch's object cached
# locally before we corrupt origin's copy, and the later "fetch" for Case 3
# would silently succeed from agent-one's own object store instead of
# hitting (and failing against) origin — masking the exact bug this case
# exists to catch. ───────────────────────────────────────────────────────
FLAKY_OBJ_PATH="${ORIGIN_BARE}/objects/${FLAKY_BRANCH_SHA:0:2}/${FLAKY_BRANCH_SHA:2}"
if [[ ! -f "$FLAKY_OBJ_PATH" ]]; then
  echo "ERROR: expected loose object for flaky/branch tip at ${FLAKY_OBJ_PATH} (got auto-packed?) — Case 3 fixture assumption broken"
  exit 1
fi
# Back up rather than permanently destroy: a later plain `git fetch origin`
# (Case 2's genuinely-new-branch fallback, which fetches ALL branches with no
# refspec) would otherwise keep failing on this same still-missing object for
# the rest of the suite, since git's default-refspec fetch can fail the whole
# operation if any one ref can't be resolved.
cp "$FLAKY_OBJ_PATH" "${FLAKY_OBJ_PATH}.bak"
rm -f "$FLAKY_OBJ_PATH"

CASE3_OUT=""
CASE3_EXIT=0
CASE3_OUT="$(run_manager allocate --slot slot-flaky --branch flaky/branch 2>&1)" || CASE3_EXIT=$?

mv "${FLAKY_OBJ_PATH}.bak" "$FLAKY_OBJ_PATH"

if [[ "$CASE3_EXIT" -eq 0 ]]; then
  fail "allocate fails closed on fetch failure: manager exited 0 (expected non-zero); out=$CASE3_OUT"
elif [[ -e "${MANAGED_ROOT}/slot-flaky" ]]; then
  fail "allocate fails closed on fetch failure: a worktree was created at slot-flaky despite the fetch failure — stale-checkout risk"
elif [[ "$CASE3_OUT" != *"3749"* ]]; then
  fail "allocate fails closed on fetch failure: exited non-zero as expected, but error message doesn't cite the #3749 fail-closed rationale — out=$CASE3_OUT"
else
  pass "allocate fails closed (no worktree created) when the origin fetch fails after ls-remote confirms the branch exists"
fi

# ── Case 1: re-allocating a slot for a branch that exists on origin must
#    check out THAT branch's content, not local/base main. ─────────────────
CASE1_OUT=""
CASE1_EXIT=0
CASE1_OUT="$(run_manager allocate --slot slot-existing --branch existing/feature 2>&1)" || CASE1_EXIT=$?

if [[ "$CASE1_EXIT" -ne 0 ]]; then
  fail "allocate for existing origin branch: manager exited non-zero ($CASE1_EXIT): $CASE1_OUT"
else
  SLOT1_HEAD="$(git -C "${MANAGED_ROOT}/slot-existing" rev-parse HEAD 2>/dev/null || echo "MISSING")"
  if [[ "$SLOT1_HEAD" == "$EXISTING_FEATURE_SHA" ]]; then
    pass "allocate for existing origin branch checks out origin/existing/feature content"
  elif [[ "$SLOT1_HEAD" == "$MAIN_A_SHA" ]]; then
    fail "allocate for existing origin branch: worktree HEAD == local main (${MAIN_A_SHA}), expected origin branch tip (${EXISTING_FEATURE_SHA}) — the #3749 bug"
  else
    fail "allocate for existing origin branch: unexpected worktree HEAD ${SLOT1_HEAD}, expected ${EXISTING_FEATURE_SHA}"
  fi
fi

# ── Case 2: allocating a genuinely NEW branch must cut from a freshly
#    fetched origin/main, not a stale local tracking ref. ─────────────────
CASE2_OUT=""
CASE2_EXIT=0
CASE2_OUT="$(run_manager allocate --slot slot-new --branch brand-new/feature 2>&1)" || CASE2_EXIT=$?

if [[ "$CASE2_EXIT" -ne 0 ]]; then
  fail "allocate for new branch: manager exited non-zero ($CASE2_EXIT): $CASE2_OUT"
else
  SLOT2_HEAD="$(git -C "${MANAGED_ROOT}/slot-new" rev-parse HEAD 2>/dev/null || echo "MISSING")"
  if [[ "$SLOT2_HEAD" == "$ORIGIN_MAIN_TIP_SHA" ]]; then
    pass "allocate for new branch cuts from freshly-fetched origin/main"
  elif [[ "$SLOT2_HEAD" == "$MAIN_A_SHA" ]]; then
    fail "allocate for new branch: worktree HEAD == stale local main (${MAIN_A_SHA}), expected current origin/main tip (${ORIGIN_MAIN_TIP_SHA}) — base_ref() never fetched"
  else
    fail "allocate for new branch: unexpected worktree HEAD ${SLOT2_HEAD}, expected ${ORIGIN_MAIN_TIP_SHA}"
  fi
fi

TOTAL=$((PASS_COUNT + FAIL_COUNT))
echo ""
echo "=== Results: ${PASS_COUNT}/${TOTAL} passed ==="

if [[ "$FAIL_COUNT" -gt 0 ]]; then
  exit 1
fi

exit 0
