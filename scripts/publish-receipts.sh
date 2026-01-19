#!/usr/bin/env bash
# Publish a receipt bundle under review/receipts/YYYY-MM-DD/
#
# This makes Phase 0 executable: a single folder containing
#  - the gate output
#  - the receipt artifacts (artifacts/state.json etc.)
#  - scope notes (README)
set -euo pipefail

DATE="${1:-$(date -u +%Y-%m-%d)}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/review/receipts/$DATE"
ART="$ROOT/artifacts"

mkdir -p "$DEST"
mkdir -p "$DEST/artifacts"

echo "Publishing receipts to: $DEST"
echo ""

echo "==> Running ci-gate (log only; authoritative merge gate)"
(cd "$ROOT" && just ci-gate) 2>&1 | tee "$DEST/ci-gate.log"

echo ""
echo "==> Running generate-receipts.sh (writes to artifacts/)"
(cd "$ROOT" && ./scripts/generate-receipts.sh) 2>&1 | tee "$DEST/generate-receipts.log"

echo ""
echo "==> Copying artifacts/"
for f in test-output.txt test-summary.json rustdoc.log doc-summary.json state.json; do
  if [[ -f "$ART/$f" ]]; then
    cp -f "$ART/$f" "$DEST/artifacts/$f"
  fi
done

sha="$(cd "$ROOT" && git rev-parse HEAD 2>/dev/null || echo UNVERIFIED)"
rustc_ver="$(rustc --version 2>/dev/null || echo UNVERIFIED)"
cargo_ver="$(cargo --version 2>/dev/null || echo UNVERIFIED)"
uname_s="$(uname -a 2>/dev/null || echo UNVERIFIED)"

cat > "$DEST/README.md" <<EOF
# Receipt Bundle: $DATE

## Provenance
- Commit: \`$sha\`
- rustc: \`$rustc_ver\`
- cargo: \`$cargo_ver\`
- Host: \`$uname_s\`

## What ran
- \`just ci-gate\` (see \`ci-gate.log\`)
- \`./scripts/generate-receipts.sh\` (see \`generate-receipts.log\`)

## Artifacts
- \`artifacts/state.json\` (canonical consolidated receipt)
- \`artifacts/test-summary.json\`
- \`artifacts/doc-summary.json\`
- \`artifacts/rustdoc.log\`
- \`artifacts/test-output.txt\`

## Scope notes
- \`ci-gate\` is the merge contract.
- \`generate-receipts.sh\` is a broader receipt generator (workspace tests + rustdoc count).
  Treat it as evidence, not as the merge gate.
EOF

echo ""
echo "Receipt bundle ready: $DEST"
