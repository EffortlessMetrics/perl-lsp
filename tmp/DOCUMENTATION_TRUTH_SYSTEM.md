# Documentation Truth System

**Status**: Implemented ✅
**Version**: 1.0
**Last Updated**: 2025-10-23

## Overview

The Documentation Truth System ensures that all documentation claims are backed by verifiable receipts from automated tooling. This prevents documentation drift by making docs **self-healing** - they update automatically when the codebase changes.

## Core Principle

> **Docs don't lie because they aren't numbers - they're receipts.**

Instead of manually updating test counts, performance metrics, or documentation coverage numbers, we use:


- **Receipts**: Canonical outputs from `cargo test`, `cargo doc`, benchmarks
- **Tokens**: Template variables like `0.8.8`, `0`, `484`
- **Renderer**: Automatic substitution of tokens with receipt values
- **CI Validation**: Automated detection of drift between docs and receipts

## Architecture

### Components

```text
┌─────────────────────────────────────────────────────────────┐
│ 1. Receipt Generation (scripts/generate-receipts.sh)       │
│    - Run tests → artifacts/test-output.txt                  │
│    - Parse results → artifacts/test-summary.json            │
│    - Count rustdoc warnings → artifacts/doc-summary.json    │
│    - Consolidate → artifacts/state.json                     │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. Documentation Rendering (scripts/render-docs.sh)        │
│    - Load state.json                                        │
│    - Replace {{tokens}} in docs/*.md and *.md files         │
│    - Output to tmp/docs/ and tmp/*.md                       │
│    - Validate no unresolved tokens remain                   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. CI Validation (.github/workflows/docs-truth.yml)        │
│    - Generate receipts                                      │
│    - Render documentation                                   │
│    - Diff tmp/ vs committed docs/                           │
│    - Fail if drift detected                                 │
└─────────────────────────────────────────────────────────────┘
```

### File Structure

```text
perl-lsp/
├── scripts/
│   ├── generate-receipts.sh     # Receipt generation (test + doc metrics)
│   ├── quick-receipts.sh        # Fast version (docs only, no tests)
│   └── render-docs.sh           # Token substitution engine
├── artifacts/                   # Generated receipts (gitignored)
│   ├── test-output.txt         # Raw cargo test output
│   ├── test-summary.json       # Parsed test metrics
│   ├── doc-summary.json        # Rustdoc warning counts
│   └── state.json              # Consolidated truth (canonical source)
├── tmp/                        # Rendered docs (gitignored)
│   ├── docs/                   # Rendered versions of docs/
│   ├── README.md              # Rendered README
│   └── CLAUDE.md              # Rendered CLAUDE.md
├── docs/                       # Source documentation (uses tokens)
│   ├── LSP_IMPLEMENTATION_GUIDE.md
│   └── ...
├── CLAUDE.md                   # Source guidance (uses tokens)
└── .github/workflows/
    └── docs-truth.yml          # CI validation workflow
```

## Token Reference

### Available Tokens

#### Version
- `0.8.8` - Current crate version from Cargo.toml

#### Test Metrics

- `0` - Number of passing tests
- `0` - Number of failing tests
- `0` - Number of ignored tests
- `0` - Total active tests (passed + failed)
- `0` - All tests (active + ignored)
- `0.0` - Pass rate of active tests (%)
- `0.0` - Overall pass rate including ignored (%)

#### Documentation Metrics

- `484` - Number of missing doc warnings from `cargo doc`

### Backward Compatibility

Both flat and nested token forms are supported:

```markdown
0        → Works (legacy)
0       → Works (canonical)
484       → Works (legacy)
484  → Works (canonical)
```

## Usage

### 1. Receipt Generation

```bash
# Full receipt generation (runs all tests + rustdoc)
./scripts/generate-receipts.sh

# Quick receipts (version + docs only, no tests)
./scripts/quick-receipts.sh
```

**Outputs:**

- `artifacts/test-summary.json` - Test metrics
- `artifacts/doc-summary.json` - Documentation metrics
- `artifacts/state.json` - Consolidated state (this is the canonical source)

### 2. Documentation Rendering

```bash
# Render all documentation with receipt values
./scripts/render-docs.sh
```

**Outputs:**

- `tmp/docs/` - Rendered documentation directory
- `tmp/CLAUDE.md` - Rendered guidance
- `tmp/README.md` - Rendered readme (if exists)

**Validation:**

- Automatically checks for unresolved tokens (`{{.*}}`)
- Fails if any tokens remain unsubstituted

### 3. Applying Rendered Docs

```bash
# Review changes
diff -ruN docs tmp/docs

# Apply if correct
rsync -av tmp/docs/ docs/
rsync -av tmp/CLAUDE.md CLAUDE.md
```

## CI Integration

The `docs-truth.yml` workflow runs on PRs that touch documentation:

```yaml
on:
  pull_request:
    paths:
      - 'docs/**'
      - 'CLAUDE.md'
      - 'README.md'
      - 'scripts/generate-receipts.sh'
      - 'scripts/render-docs.sh'
```

**Workflow Steps:**

1. Generate receipts
2. Render documentation
3. Compare `tmp/` with committed `docs/`
4. Fail if drift detected
5. Upload receipts as artifacts

## Best Practices

### For Documentation Writers

1. **Use tokens for all dynamic values**

   ```markdown
   ❌ We have 828 passing tests
   ✅ We have 0 passing tests

   ❌ Version 0.8.8 includes...
   ✅ Version 0.8.8 includes...

   ❌ 484 missing docs warnings
   ✅ 484 missing docs warnings
   ```

2. **Regenerate after major changes**

   ```bash
   ./scripts/generate-receipts.sh
   ./scripts/render-docs.sh
   # Review tmp/ and apply if correct
   ```

3. **Check for unresolved tokens**

   ```bash
   rg '{{[^}]+}}' docs/
   ```

### For CI/CD

1. **Use thread-constrained test runs**

   ```bash
   RUST_TEST_THREADS=2 cargo test --workspace --exclude xtask -- --test-threads=2
   ```

2. **Handle test timeouts gracefully**
   - Receipt parser tolerates missing test output
   - Falls back to zeros with warning

3. **Validate token resolution**
   - Renderer checks for unresolved tokens
   - CI fails if tokens remain after rendering

## Implementation Details

### Receipt Parsing (Tolerant)

```bash
# Handles missing test output gracefully
RESULTS="$(grep -E '^[[:space:]]*test result:' artifacts/test-output.txt || true)"
if [ -z "$RESULTS" ]; then
  echo "Warning: no test summaries found; treating as zeroes" >&2
  TOTAL_PASSED=0
  TOTAL_FAILED=0
  TOTAL_IGNORED=0
else
  TOTAL_PASSED=$(echo "$RESULTS" | awk '{sum += $4} END {print sum+0}')
  TOTAL_FAILED=$(echo "$RESULTS" | awk '{sum += $6} END {print sum+0}')
  TOTAL_IGNORED=$(echo "$RESULTS" | awk '{sum += $8} END {print sum+0}')
fi
```

### State Consolidation

```bash
# Merge all receipts into single state.json
jq -n \
  --arg version "${VERSION}" \
  --slurpfile tests "artifacts/test-summary.json" \
  --slurpfile docs "artifacts/doc-summary.json" \
  '{
    version: $version,
    tests: $tests[0],
    docs: $docs[0],
    generated_at: (now | strftime("%Y-%m-%dT%H:%M:%SZ"))
  }' > artifacts/state.json
```

### Token Substitution

```bash
# Render with both flat and nested token support
sed \
  -e "s/0.8.8/${VERSION}/g" \
  -e "s/{{tests\.passed}}/${TEST_PASSED}/g" \
  -e "s/0/${TEST_PASSED}/g" \
  -e "s/{{docs\.missing_docs}}/${MISSING_DOCS}/g" \
  -e "s/484/${MISSING_DOCS}/g" \
  "${source}" > "${target}"
```

### Validation

```bash
# Check for unresolved tokens
if rg -n '{{[^}]+}}' tmp/ | grep .; then
  echo "ERROR: Unresolved tokens remain in rendered docs"
  exit 1
fi
```

## Troubleshooting

### Issue: "State file not found"

```bash
# Solution: Generate receipts first
./scripts/generate-receipts.sh
```

### Issue: "Unresolved tokens in rendered docs"

```bash
# Solution: Check token names match state.json structure
jq . artifacts/state.json

# Add missing tokens to render-docs.sh sed commands
```

### Issue: "Test parsing found 0 tests"

```bash
# Cause: Test output format changed or tests didn't run
# Solution: Check artifacts/test-output.txt

# Verify test result lines match pattern:
grep "test result:" artifacts/test-output.txt
```

### Issue: "CI drift check failing"

```bash
# Solution: Regenerate and commit rendered docs
./scripts/generate-receipts.sh
./scripts/render-docs.sh
rsync -av tmp/docs/ docs/
git add docs/ CLAUDE.md
git commit -m "docs: update receipts to match current state"
```

## Future Enhancements

### Planned

- [ ] Benchmark receipt integration
- [ ] Coverage percentage tracking
- [ ] Mutation score receipts
- [ ] Security audit receipts
- [ ] Performance regression receipts

### Under Consideration

- [ ] Automatic PR comments with receipt diffs
- [ ] Receipt versioning and history
- [ ] Multi-crate receipt aggregation
- [ ] Visual receipt dashboards

## References

- [Receipt Generation Script](scripts/generate-receipts.sh)
- [Quick Receipts Script](scripts/quick-receipts.sh)
- [Renderer Script](scripts/render-docs.sh)
- [CI Workflow](.github/workflows/docs-truth.yml)

## Governance

This system is validated by the `docs-truth` CI workflow and enforces:

- **Zero tolerance for hardcoded metrics** in documentation
- **Automated drift detection** on all doc-touching PRs
- **Receipt-backed claims only** in CLAUDE.md and docs/

**Accountability**: All claims in documentation must be traceable to a receipt in `artifacts/state.json`.
