# Test Tier Taxonomy

Closes #2096

## Tier Definitions

### smoke (<1s per test, <30s total per crate)
**Criteria:** No I/O, no subprocess, no network, no file system access. Deterministic and idempotent.
**Scope:** Parser unit tests, lexer unit tests, token stream tests, AST construction, keyword tables, error type construction.
**Crates:** `perl-parser`, `perl-lexer`, `perl-keywords`, `perl-token`, `perl-error`
**CI trigger:** Every PR (via `pr-smoke` job, ~1-2 min total)
**Examples:**
- Lexing a known string produces expected token sequence
- Parsing `"my $x = 1"` produces correct AST node
- Error type Display impl matches expected format

### deep (1-10s per test, <5 min total per crate)
**Criteria:** May touch file system, may spawn perl subprocess, may use test fixtures. Tests edge cases, error paths, and cross-crate interactions.
**Scope:** Integration tests with fixtures, golden file tests, snapshot tests, semantic analysis, code actions, completions.
**CI trigger:** Merge gate + nightly (~3-5 min total)
**Examples:**
- LSP code action tests (`lsp_code_actions_test`)
- Semantic definition tests (`lsp_unhappy_paths`)
- DAP integration tests (`perl-dap` test suite)
- BDD workflow tests (`lsp_bdd_workflows`)

### slow (>10s per test or >5 min total)
**Criteria:** Performance-sensitive, corpus sweeps, CPAN corpus checks, full workspace compilation, stress tests.
**Scope:** Benchmarks, corpus-audit, cpan-corpus-check, mutation testing, fuzz runs.
**CI trigger:** Nightly only (~15-30 min total)
**Examples:**
- `just benchmarks`
- `just cpan-corpus-check`
- `just mutation-subset`
- `just fuzz-bounded`

### e2e-process (>30s total)
**Criteria:** Launches LSP/DAP server process, speaks JSON-RPC over stdio. Requires process lifecycle management.
**Scope:** `lsp_smoke_e2e`, `dap_smoke_e2e`, `lsp_bdd_workflows`
**CI trigger:** Merge gate (subset) + nightly (full)

## Current Test Inventory

| Crate | Test Files | Primary Tier |
|-------|-----------|-------------|
| perl-parser | 144 | smoke |
| perl-lexer | 8 | smoke |
| perl-lsp | 232 | deep / e2e-process |
| perl-dap | 35 | deep |
| Other workspace crates | ~517 | smoke / deep |

## CI Integration

```
PR push          → pr-smoke job:   fmt + clippy-core + test-core (~1-2 min)
Merge-ready PR   → merge-gate job: full test suite (~5-10 min)
Push to main     → merge-gate job: full test suite (~5-10 min)
Nightly (3am UTC) → nightly job:   all tiers + mutation + fuzz (~15-30 min)
```

## Nextest Partitions

Tier filtering uses `cargo nextest` partition files defined in `.config/nextest-partitions.toml`. This avoids proc-macro overhead while providing CI-selectable test subsets.

## Tier Assignment Guidelines

1. **Default to smoke.** If a test has no I/O and runs in <1s, it's smoke.
2. **Promote to deep** if it touches the file system, spawns a process, or uses fixtures.
3. **Promote to slow** if it takes >10s or sweeps a large corpus.
4. **When in doubt, classify as deep.** It's better to over-classify than to miss a slow test in the smoke tier.

## Deferred: `#[test_tier]` Attribute Macro

The issue requested a `#[test_tier(smoke)]` proc-macro attribute. This was evaluated and deferred because:

- `cargo nextest` filter expressions provide equivalent CI selection without compile-time deps
- The justfile already defines tier boundaries at the recipe level
- A proc-macro crate adds maintenance burden for marginal benefit

If compile-time tier enforcement is needed later, the implementation path is:
1. Create `crates/perl-test-tier/` proc-macro crate
2. `#[test_tier(smoke)]` → expands to `#[test]` + cfg marker
3. Wire to nextest custom groups or env-var filtering
