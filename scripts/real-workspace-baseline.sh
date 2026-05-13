#!/usr/bin/env bash
# real-workspace-baseline.sh — Run real-workspace LSP latency baseline and generate report.
#
# Usage:
#   bash scripts/real-workspace-baseline.sh [PROJECT] [SYSTEM]
#
# Arguments:
#   PROJECT  — project key: mojolicious | dancer2 | catalyst  (default: mojolicious)
#   SYSTEM   — system label: linux | macos | windows          (default: auto-detected)
#
# Deliverables:
#   - .ci/metrics/real_project_latency.json      raw p50/p95/p99 data
#   - docs/forensics/<date>-real-workspace-baseline-<project>.md  human report
#
# Related: justfile `real-workspace-baseline`, test_corpus/real_projects/,
#          crates/perl-lsp-rs/tests/real_project_latency.rs

set -euo pipefail

PROJECT="${1:-mojolicious}"
SYSTEM="${2:-}"

# ── System auto-detection ────────────────────────────────────────────────────
if [ -z "$SYSTEM" ]; then
    if command -v uname >/dev/null 2>&1; then
        SYSNAME=$(uname -s | tr '[:upper:]' '[:lower:]')
        case "$SYSNAME" in
            linux*)           SYSTEM="linux"   ;;
            darwin*)          SYSTEM="macos"   ;;
            msys*|mingw*|cygwin*) SYSTEM="windows" ;;
            *)                SYSTEM="$SYSNAME" ;;
        esac
    else
        SYSTEM="windows"
    fi
fi

echo "=== Real-Workspace Baseline: ${PROJECT} (${SYSTEM}) ==="
echo ""

# ── Step 1: Build release binary ─────────────────────────────────────────────
echo "Step 1/3: Building release binary..."
cargo build -p perl-lsp-rs --release --locked

# ── Step 2: Run the latency benchmark ────────────────────────────────────────
echo ""
echo "Step 2/3: Running latency benchmark (project=${PROJECT})..."
RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs --test real_project_latency \
    "${PROJECT}" -- --include-ignored --nocapture --test-threads=1

# ── Step 3: Generate markdown report ─────────────────────────────────────────
echo ""
echo "Step 3/3: Generating markdown report..."

DATE=$(date +%Y-%m-%d)
OUTPUT_DIR="docs/forensics"
OUTPUT_FILE="${OUTPUT_DIR}/${DATE}-real-workspace-baseline-${PROJECT}.md"
JSON_FILE=".ci/metrics/real_project_latency.json"

# Helper: extract a scalar field from the JSON output via python3.
json_field() {
    local metric="$1" field="$2"
    python3 -c "
import json, sys
with open('${JSON_FILE}') as f:
    d = json.load(f)
m = d['projects']['${PROJECT}']['metrics']
print(m['${metric}']['${field}'])
" 2>/dev/null || echo "?"
}

if [ ! -f "$JSON_FILE" ]; then
    echo "Warning: JSON baseline not found at ${JSON_FILE} — metrics may not have written cleanly."
    COLD_P50="?" COLD_P95="?" COLD_P99="?" COLD_N="?"
    COMP_P50="?" COMP_P95="?" COMP_P99="?" COMP_N="?"
    GOTO_P50="?" GOTO_P95="?" GOTO_P99="?" GOTO_N="?"
    REPR_P50="?" REPR_P95="?" REPR_P99="?" REPR_N="?"
    WSYM_P50="?" WSYM_P95="?" WSYM_P99="?" WSYM_N="?"
    FILE_COUNT="?"
else
    COLD_P50=$(json_field cold_start_to_hover p50_ms)
    COLD_P95=$(json_field cold_start_to_hover p95_ms)
    COLD_P99=$(json_field cold_start_to_hover p99_ms)
    COLD_N=$(json_field   cold_start_to_hover samples)

    COMP_P50=$(json_field first_completion p50_ms)
    COMP_P95=$(json_field first_completion p95_ms)
    COMP_P99=$(json_field first_completion p99_ms)
    COMP_N=$(json_field   first_completion samples)

    GOTO_P50=$(json_field first_goto_definition p50_ms)
    GOTO_P95=$(json_field first_goto_definition p95_ms)
    GOTO_P99=$(json_field first_goto_definition p99_ms)
    GOTO_N=$(json_field   first_goto_definition samples)

    REPR_P50=$(json_field incremental_reparse p50_ms)
    REPR_P95=$(json_field incremental_reparse p95_ms)
    REPR_P99=$(json_field incremental_reparse p99_ms)
    REPR_N=$(json_field   incremental_reparse samples)

    WSYM_P50=$(json_field workspace_symbol_query p50_ms)
    WSYM_P95=$(json_field workspace_symbol_query p95_ms)
    WSYM_P99=$(json_field workspace_symbol_query p99_ms)
    WSYM_N=$(json_field   workspace_symbol_query samples)

    FILE_COUNT=$(python3 -c "
import json
with open('${JSON_FILE}') as f:
    d = json.load(f)
print(d['projects']['${PROJECT}']['file_count'])
" 2>/dev/null || echo "?")
fi

# ── Version info ──────────────────────────────────────────────────────────────
PERLLSP_VER=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
    | python3 -c "
import json, sys
m = json.load(sys.stdin)
pkgs = [p for p in m['packages'] if p['name'] == 'perl-lsp-rs']
print(pkgs[0]['version'] if pkgs else '?')
" 2>/dev/null || echo "?")
RUST_VER=$(rustc --version 2>/dev/null || echo "?")
PERL_VER=$(perl -e 'print $^V' 2>/dev/null || echo "?")
OS_VER=$(uname -srm 2>/dev/null || echo "Windows")
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

# ── Fixture dir mapping ───────────────────────────────────────────────────────
case "$PROJECT" in
    mojolicious) FIXTURE_DIR="mojolicious_skeleton" ;;
    dancer2)     FIXTURE_DIR="dancer2_skeleton"     ;;
    catalyst)    FIXTURE_DIR="catalyst_skeleton"    ;;
    *)           FIXTURE_DIR="${PROJECT}_skeleton"  ;;
esac

# ── Outlier detection (p95 > 500ms) ──────────────────────────────────────────
OUTLIERS=""
for PAIR in \
    "cold_start_to_hover:${COLD_P95}" \
    "first_completion:${COMP_P95}" \
    "first_goto_definition:${GOTO_P95}" \
    "incremental_reparse:${REPR_P95}" \
    "workspace_symbol_query:${WSYM_P95}"
do
    MNAME="${PAIR%%:*}"
    MVAL="${PAIR##*:}"
    if [ "$MVAL" != "?" ] && [ "$MVAL" -gt 500 ] 2>/dev/null; then
        OUTLIERS="${OUTLIERS}- **${MNAME}** p95=${MVAL}ms exceeds 500ms threshold"$'\n'
    fi
done
if [ -z "$OUTLIERS" ]; then
    OUTLIERS="None - all p95 values within 500ms threshold."
fi

# ── Write markdown report ─────────────────────────────────────────────────────
mkdir -p "$OUTPUT_DIR"

python3 - "$OUTPUT_FILE" <<PYEOF
import sys

path = sys.argv[1]
content = """\
# Real-Workspace Baseline: ${PROJECT} (${SYSTEM})

**Date**: ${DATE}
**Commit**: ${COMMIT}
**System**: ${SYSTEM}
**Project**: ${PROJECT}

## Substrate Versions

| Component | Version |
|-----------|---------|
| perl-lsp  | ${PERLLSP_VER} |
| Rust      | ${RUST_VER} |
| Perl      | ${PERL_VER} |
| OS        | ${OS_VER} |

## Metrics

### Cold-Start to First Hover (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| ${COLD_P50} | ${COLD_P95} | ${COLD_P99} | ${COLD_N} |

### First Completion (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| ${COMP_P50} | ${COMP_P95} | ${COMP_P99} | ${COMP_N} |

### Goto-Definition (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| ${GOTO_P50} | ${GOTO_P95} | ${GOTO_P99} | ${GOTO_N} |

### Incremental Reparse (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| ${REPR_P50} | ${REPR_P95} | ${REPR_P99} | ${REPR_N} |

### Workspace Symbol Query (ms)

| p50 | p95 | p99 | Samples |
|-----|-----|-----|---------|
| ${WSYM_P50} | ${WSYM_P95} | ${WSYM_P99} | ${WSYM_N} |

## Project Stats

- **Perl files**: ${FILE_COUNT} (.pm / .pl / .t)
- **Fixture source**: test_corpus/real_projects/${FIXTURE_DIR}/

## Provider Coverage

| Surface | Status | Receipt |
|---------|--------|---------|
| Cold start / first hover | covered | \`cold_start_to_hover\` |
| Completion latency | covered | \`first_completion\` |
| Goto definition latency | covered | \`first_goto_definition\` |
| Incremental reparse | covered | \`incremental_reparse\` |
| Workspace symbols | covered | \`workspace_symbol_query\` |
| Workspace indexing | indirect | initialization, document open, and provider requests exercise the fixture workspace; dedicated index-shape receipts remain in provider/status docs |
| Module resolution | indirect | fixture package layout is exercised through hover, completion, goto, and workspace-symbol requests; dedicated module-resolution receipts remain separate |
| Diagnostics | deferred | latency harness does not wait for publishDiagnostics; use diagnostics/provider receipts for diagnostic correctness claims |
| Memory profile | deferred | this harness records wall-clock latency, not heap or RSS |

## Provider Confidence Links

- [Provider cutover](../project/status/provider_cutover.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)
- [Semantic scorecard](../project/status/semantic_scorecard.md)
- [Semantic shadow compare](../project/status/semantic_shadow_compare.md)

## Claim Boundary

This receipt supports a measured editor-latency claim for the selected fixture
and host system only. It does not claim full CPAN compatibility, broader
framework coverage, memory/resource ceilings, diagnostic correctness, or live
provider cutover by itself.

## Outliers

${OUTLIERS}

Outliers are recorded threshold misses for the named metric. They do not block
the receipt, but they do block promotion of a no-outlier latency claim for that
metric until a follow-up run or fix clears the threshold.

## Reproducibility Notes

\`\`\`bash
# Reproduce this measurement
just real-workspace-baseline ${PROJECT} ${SYSTEM}

# Windows fallback when just cannot locate its shell
"C:/Program Files/Git/bin/bash.exe" scripts/real-workspace-baseline.sh ${PROJECT} ${SYSTEM}
\`\`\`

- Binary built with: \`cargo build -p perl-lsp-rs --release\`
- Test invoked via: \`cargo test -p perl-lsp-rs --test real_project_latency ${PROJECT} -- --include-ignored --nocapture\`
- Samples per metric: 10 (p50/p95/p99)
- Fixture path: \`test_corpus/real_projects/${FIXTURE_DIR}/\`

## Notes

Current baseline run for ${PROJECT} on ${SYSTEM}. Establishes a Real Perl Editor Trust measurement anchor for the selected fixture and host.
"""
with open(path, 'w') as f:
    f.write(content)
PYEOF

# ── Summary output ────────────────────────────────────────────────────────────
echo ""
echo "=== Results ==="
echo "  cold_start_to_hover  : p50=${COLD_P50}ms  p95=${COLD_P95}ms  p99=${COLD_P99}ms"
echo "  first_completion     : p50=${COMP_P50}ms  p95=${COMP_P95}ms  p99=${COMP_P99}ms"
echo "  first_goto_definition: p50=${GOTO_P50}ms  p95=${GOTO_P95}ms  p99=${GOTO_P99}ms"
echo "  incremental_reparse  : p50=${REPR_P50}ms  p95=${REPR_P95}ms  p99=${REPR_P99}ms"
echo "  workspace_symbol     : p50=${WSYM_P50}ms  p95=${WSYM_P95}ms  p99=${WSYM_P99}ms"
echo ""
echo "Report written to: ${OUTPUT_FILE}"
echo "Raw JSON at:       ${JSON_FILE}"
