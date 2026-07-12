#!/usr/bin/env bash
# Regression guard for #3947: .claude/hooks/task-completed.sh used to block
# TaskCompleted on missing receipts/<check>.<commit-hash> files, but nothing
# in the repo ever writes that path (the only live receipt producer is
# `cargo xtask gates --receipt` -> target/receipts/*.json, a different
# location entirely). That dead receipt-gate block has been removed.
#
# This test isolates JUST that behavior: with a .rs change staged and NO
# receipts/ directory present, the hook must never report a missing-receipts
# block. It may still exit non-zero for unrelated reasons (e.g. this
# isolated repo has no docs/project/CURRENT_STATUS.md), so we assert on the
# absence of the receipt-gate's diagnostic text rather than on exit code.

set -eu

REPO_ROOT="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
HOOK="${REPO_ROOT}/.claude/hooks/task-completed.sh"

WORKDIR="$(mktemp -d)"
FAKEBIN="$(mktemp -d)"
cleanup() { rm -rf "$WORKDIR" "$FAKEBIN"; }
trap cleanup EXIT

# Stub `cargo` so the unrelated `cargo xtask fmt --check` gate always
# succeeds -- this test isolates the receipt gate, not the fmt gate, and the
# isolated temp repo below has no real cargo workspace to check.
cat > "${FAKEBIN}/cargo" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
chmod +x "${FAKEBIN}/cargo"

(
  cd "${WORKDIR}"
  git init -q
  git config user.email "test@example.com"
  git config user.name "test"
  echo "placeholder" > README.md
  git add README.md
  git commit -q -m "init"

  # Stage a .rs change with no receipts/ directory anywhere in this repo --
  # the exact precondition the dead gate used to block on.
  echo 'fn main() {}' > fake.rs
  git add fake.rs
)

OUTPUT="$(cd "${WORKDIR}" && PATH="${FAKEBIN}:${PATH}" bash "${HOOK}" </dev/null 2>&1)" || true

FAIL=0

if echo "${OUTPUT}" | grep -qi "missing receipts"; then
  echo "FAIL: hook still reports missing receipts -- dead receipt gate not removed"
  FAIL=1
fi

if echo "${OUTPUT}" | grep -q "receipts/{verify-build,clippy,test}"; then
  echo "FAIL: hook still references the receipts/{verify-build,clippy,test} remediation text"
  FAIL=1
fi

if [[ "${FAIL}" -eq 0 ]]; then
  echo "PASS: task-completed.sh does not gate on receipts/<check>.<hash> (dead path removed)"
  echo "--- hook output for reference ---"
  echo "${OUTPUT}"
  exit 0
else
  echo "--- hook output ---"
  echo "${OUTPUT}"
  exit 1
fi
