# Honest State Assessment

> **Purpose**: A brutally honest, evidence-based assessment of what works, what doesn't,
> and what the metrics actually mean. Written for developers evaluating this project.
>
> **Date**: 2026-02-17
> **Version**: 1.0.0

---

## Executive Summary

This is a **real, functional Perl LSP server** — not a skeleton or toy project. The parser,
LSP server, semantic analyzer, and workspace index are production-quality. The Debug Adapter
Protocol (DAP) has moved from scaffolding to a native preview implementation (with bridge
fallback), but it is not GA-depth yet. The Pest parser is a maintained legacy artifact.

**Bottom line**: A Perl developer pointing VSCode at this server today would get real
completion, hover, go-to-definition, rename, diagnostics, and formatting, plus native DAP
preview workflows (breakpoints/control-flow/attach basics). Deep debugging fidelity still
needs hardening before GA.

---

## What Actually Works

### LSP Server — Production-Ready

**Evidence**: 1045 lib tests pass, proper lifecycle (initialize/initialized/shutdown/exit),
full JSON-RPC dispatch with cancellation support, enterprise-grade error handling.

| Capability | Status | Evidence |
|-----------|--------|---------|
| Completion | Real impl, 150+ builtins, context-aware | `perl-lsp-completion` (877 LOC), type inference engine |
| Hover | Real semantic analysis | Uses `SemanticAnalyzer::analyze_with_source()` |
| Go-to-Definition | Cross-file via workspace index | Dual indexing (bare + qualified names) |
| References | Workspace-wide with regex fallback | Deadline enforcement, graceful degradation |
| Rename | Multi-file with conflict detection | Prepare/rename/workspace rename handlers |
| Formatting | Delegates to perltidy | Only advertised if perltidy found on PATH |
| Diagnostics | Push + pull model | Parse errors + semantic issues |
| Semantic Tokens | Full token legend | Proper delta encoding |
| Code Actions | Pragma injection, quick fixes | Multiple provider pattern |
| Code Lens | Test discovery | Reference counting |
| Call/Type Hierarchy | Implemented | Navigation handlers present |
| Selection Range | Smart expansion | AST-based |
| Folding | Block-based | With fallback |
| Inlay Hints | Parameter names | Type hints |

**53 advertised features, all GA** — verified via `features.toml` and `just ci-gate`.

**What a developer actually gets in VSCode**:
- Fast, context-aware completion with 150+ Perl builtins
- Hover showing types, signatures, and documentation
- Cross-file go-to-definition via workspace index
- Workspace-wide find references
- Multi-file rename with conflict detection
- Real-time diagnostics (parse errors + semantic issues)
- perltidy formatting (if installed)
- Semantic syntax highlighting
- Code actions for common fixes (add `use strict`, etc.)

**What degrades gracefully**:
- During workspace index build: falls back to same-file operations
- Without perltidy: formatting not advertised
- On cancellation: proper cleanup, no resource leaks

### Parser (v3 Native Recursive Descent) — Production-Ready

**Evidence**: 1045 tests pass, 87% mutation score, ~100% Perl 5 syntax coverage,
sub-microsecond incremental parsing (931ns).

**Handles well**:
- All core Perl 5.10-5.38 syntax
- Regex with all delimiters, modifiers, and Unicode properties
- Heredocs including indented `<<~` (Perl 5.26+)
- Complex quoting (`qq{}`, `qw()`, `qr//`)
- Try/catch/finally (Perl 5.34+)
- Subroutine signatures (Perl 5.36+)
- Class/field/method (Corinna, Perl 5.38+)
- Format statements, phase blocks, labeled statements
- Statement modifiers, postfix dereference
- Unicode identifiers
- Legacy syntax (apostrophe package separators, bareword filehandles, indirect objects)

**Resilience**:
- 1MB identifiers parse gracefully
- 3000-deep nesting hits clear recursion limit errors
- 64KB regex budget, 256KB heredoc budget
- Budget-protected lexer prevents infinite loops
- Fuzz targets exist for parser, lexer, heredoc, substitution, and Unicode positions

**Documented limitations** (not bugs):
- Source filters (`use Switch;`) — requires runtime execution
- `eval STRING` content — dynamic code generation
- Dynamic symbol table manipulation — runtime behavior
- BEGIN block side effects — requires execution

### Semantic Analyzer — Real Analysis (7,400 LOC)

**Evidence**: Scope analysis with variable lifecycle tracking, 9 issue types detected,
type inference engine (1,106 LOC), dead code detection, symbol table construction.

| Analysis | What It Does |
|----------|-------------|
| Scope Analysis | Lexical scope tree, variable lifecycle, depth metadata |
| Variable Tracking | Unused, shadowed, undeclared, uninitialized, redeclared |
| Type Inference | Best-effort for dynamic Perl, array/hash tracking, builtins |
| Dead Code | Unreachable code, unused imports |
| Symbol Extraction | Table construction, declaration-to-reference linking |

### Workspace Index — Enterprise-Grade (6,825 LOC)

**Evidence**: State machine lifecycle (Idle->Building->Ready->Degraded), dual indexing for
98% reference coverage, O(1) symbol lookups, <50us query latency, SLO enforcement.

| Characteristic | Value |
|---------------|-------|
| Symbol lookup | O(1) average via hash table |
| Cross-file queries | <50us typical |
| Incremental updates | <=1ms |
| Scaling design | 50K+ files |
| State machine | Idle->Initializing->Building->Updating->Ready->Degraded->Error |

### VSCode Extension — Published and Working

**Evidence**: Available as "Perl Language Server" by `effortlesssteven` on VSCode Marketplace,
v0.9.0, auto-downloads perl-lsp binary for 6 platforms.

### Test Corpus — Comprehensive

**Evidence**: 732KB in `test_corpus/` (78 `.pl` files), 611+ tree-sitter corpus sections,
real-world examples (async Mojo patterns, enterprise CPAN, web frameworks, database patterns),
advanced regex with recursive patterns and Unicode properties, legacy syntax.

Tests verify AST correctness (specific `NodeKind` matching), not just "no crash".

---

## What Does NOT Work

### GAP 1: DAP Is Preview, Not GA

Native DAP now exists and runs, but behavior depth is still uneven across modes.

**What exists**:
- Native breakpoint setting with AST validation (`perl-dap-breakpoint` microcrate)
- Step/continue/pause control-flow handlers with monotonic DAP sequencing
- Safe-evaluation guardrails and command-injection protections
- PID and TCP attach modes, plus stdio and socket transports
- BridgeAdapter fallback to Perl::LanguageServer for compatibility
- Feature-gated Phase 2/3 test suites with real assertions (no universal "not yet implemented" stubs)

**What is still incomplete**:
- Deep variable inspection/evaluation fidelity is limited in PID signal-control attach mode
- Shim distribution strategy (`Devel::TSPerlDAP` vs bundled equivalent) is not finalized
- Native cross-editor smoke receipts are less mature than core LSP coverage receipts

**Impact**: Users can debug in native preview mode today, but should treat advanced
inspect/evaluate workflows as in-progress rather than GA-stable.

**Status in `features.toml`**: DAP features are `maturity = "preview"` and intentionally not GA.

### GAP 2: Moo/Moose Semantic Blindness

The parser tokenizes Moo/Moose code correctly, but the semantic analyzer does not understand
framework-specific idioms:

- `has` declarations are not recognized as field definitions
- Hover on Moo attributes returns nothing useful
- Completion inside Moose `has` blocks doesn't suggest type constraints
- Role composition (`with 'Role'`) is parsed but roles are not tracked
- `Class::Accessor` auto-generated accessors are invisible to the IDE

**Why this matters**: Most production Perl uses Moo or Moose. The parser handles the syntax,
but IDE features like "show me what fields this class has" or "complete attribute options"
don't work. This is the #1 real-world gap for daily use.

### GAP 3: No End-to-End Smoke Test

All testing is unit/integration against internal APIs. There is no automated test that:
1. Starts the LSP server via stdio
2. Sends an `initialize` request
3. Opens a document
4. Requests completion/hover/definition
5. Verifies the response
6. Shuts down cleanly

The closest is `just ci-gate` which runs lib tests. A real E2E test would catch
integration issues that unit tests miss.

### GAP 4: Pest Parser Is Orphaned

The Pest parser (`crates/perl-parser-pest/`) compiles, has a real grammar (200+ rules),
and a three-stage pipeline (Pest->AST->S-expression). It is:
- Intentionally excluded from CI (`v2_parity` gate only)
- Not used for anything in production
- 10-100x slower than v3
- Maintained for parity checks and benchmarking

**Recommendation**: Keep as-is. It works, costs nothing when excluded from CI, and provides
a useful benchmark reference. Document its role explicitly rather than letting it appear
abandoned.

---

## What the Metrics Mean vs. What They Suggest

### "100% LSP Coverage" (53/53)

**What it means**: Every LSP method that the server advertises has a real handler that
processes requests and returns responses. The handlers are tested.

**What it does NOT mean**: Every IDE scenario works perfectly. Coverage measures protocol
compliance, not semantic depth. For example, "completion works" means the server returns
completion items — it does not mean completions are perfect for every Perl idiom.

### "100% Protocol Compliance" (89/89)

**What it means**: All 89 LSP protocol methods (including lifecycle, sync, and plumbing)
have implementations.

**What it does NOT mean**: Full compliance with every edge case in the LSP specification.
Some methods may have simplified implementations.

### "87% Mutation Score"

**What it means**: When cargo-mutants introduces bugs into the parser, 87% of those bugs
are caught by the test suite. This is excellent for a parser.

**What it does NOT mean**: 87% of real-world bugs would be caught. Mutation testing measures
test sensitivity, not completeness of the specification.

### "954 Tests, 0 Ignored"

**What it means**: Every test in the suite passes. No tests are skipped.

**What it does NOT mean**: No bugs exist. The tests cover the implemented behavior;
they cannot cover behavior that hasn't been fully implemented (Moo semantics, GA-depth DAP, etc.).

---

## Comparison to Other Perl LSPs

| Feature | perl-lsp (this project) | Perl::LanguageServer | PLS |
|---------|------------------------|---------------------|-----|
| Parser | Native recursive descent, ~100% Perl 5 | Perl-based (PPI) | tree-sitter |
| Completion | 150+ builtins, type inference, workspace | Basic | Basic |
| Go-to-definition | Cross-file, dual indexing | Single-file mainly | Cross-file |
| Diagnostics | Parser + semantic analyzer | Perl::Critic integration | Perl::Critic |
| Formatting | perltidy delegation | perltidy delegation | perltidy delegation |
| Debugging | Native preview + BridgeAdapter fallback | Working (native Perl) | None |
| Language | Rust | Perl | Perl |
| Speed | Sub-microsecond incremental | Slower (Perl runtime) | Moderate |
| Installation | Single binary, no Perl required | Requires Perl + CPAN modules | Requires Perl |

**Honest take**: For editing/navigation, this project is the most capable Perl LSP.
For debugging, this project now has native preview support plus bridge fallback, but
Perl::LanguageServer still has more mature debugger depth today. For pure simplicity, PLS
is lighter weight.

---

## Roadmap to Close the Gaps

See [ROADMAP.md](ROADMAP.md) for the full gap-closing plan. Summary:

**v1.0 Readiness**:
1. Keep DAP preview maturity/docs/tests aligned
2. Add E2E LSP smoke test
3. Document Moo/Moose limitations honestly
4. Stability statement and packaging stance

**v1.1 (Semantic Depth)**:
1. Moo/Moose `has` attribute recognition
2. Class::Accessor support
3. Type constraint awareness

**v1.2 (DAP Preview -> GA)**:
1. Deep variable inspection/evaluate fidelity in native sessions
2. Cross-editor native debug smoke receipts
3. Finalize shim/package strategy

---

## Verification Commands

```bash
# Verify everything claimed above
cargo test --workspace --lib               # All tests pass
cargo test -p perl-dap --features dap-phase2,dap-phase3  # Native DAP preview suites
just ci-gate                               # Full gate clean
grep 'maturity = "ga"' features.toml       # No DAP features marked GA
grep 'maturity = "preview"' features.toml  # DAP features correctly marked
```

---

*Last updated: 2026-02-17*
*Evidence sources: `features.toml`, `just ci-gate`, investigation of all crate source code*
