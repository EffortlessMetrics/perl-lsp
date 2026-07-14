# Acceptance Criteria: #3766 — Migrate hover to generation-owned analyzer/type_environment

## §Behavior

| Input / Condition | Expected Result | Notes |
|---|---|---|
| Hover on symbol in generation N | Hover returns facts from generation N's semantic analysis | Basic happy path |
| Edit document (bumps generation to N+1) | Generation counter increments, snapshot becomes stale | Prerequisite for round-trip test |
| Hover on SAME symbol in generation N+1 after edit | Hover returns facts from NEW generation N+1 (not cached N) | Freshness proof: generation-gated, not hash-gated |
| Edit that changes symbol's type (e.g., `my $x = 42;` → `my $x = "hi";`) | Hover reflects the NEW type, not the old type | Type freshness — most critical case |
| Hover during pending parse (generation N+1 in flight, unpublished) | Returns EITHER last-published (N) OR degraded/pending, NEVER stale-wrong | Honesty under concurrency (pending-parse-gap) |
| Hover on symbol with POD documentation | Hover includes the doc text (proves real source is used, not empty-source fallback) | Fidelity preservation |
| Multiple hovers in same generation N on different offsets | Analyzer/type-engine constructed exactly 1x, all hovers reuse the same instance | Construction efficiency |
| Hover on malformed code (parse failed, `degradation_tier == Minimal`) | Returns textual fallback or "no hover", does not crash | Degradation-tier safety |
| Hover on empty/whitespace-only line | Returns null/no hover, does not crash | Edge case (empty line) |
| Request version mismatch (hover requests old version after newer edit) | Hover rejects with `CONTENT_MODIFIED` error (existing check at handle_hover line 52) | Stale-request rejection (pre-existing, should still work) |

**Acceptance gate**:
- `cargo test -p perl-lsp-rs --lib` — ALL tests pass
- `cargo test -p perl-lsp-rs --test hover_*` — new integration tests pass
- `cargo clippy -p perl-lsp-rs --locked -- -D warnings -A missing_docs` — NO clippy warnings
- `cargo xtask fmt && cargo fmt` — formatted code

## §Hazards

Seeded from `docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md` (LSP subsystem). Six hazard classes per `docs/agents/SPEC_UPDATE_CHECKLIST.md §8`.

| Class | Invariant | Surface | Required Adversarial Test | Mitigation |
|---|---|---|---|---|
| **Generation-correctness** | Hover never returns facts from a prior generation after generation bumps | `hover.rs:extract_symbol_hover` — analyzer and type-engine reads | Generation round-trip test: edit symbol's type, verify hover reflects new type, not cached old | ParsedSnapshot generation-ownership is built-in: `semantic_analyzer()` / `type_environment()` are scoped to snapshot's own generation; snapshot supersedes, old snapshot's cells become unreachable |
| **Pending-parse staleness** | Hover returns honest answer from current-published generation OR degraded/pending, NEVER stale-wrong from a yet-unpublished future generation | `hover.rs:handle_hover` — calls `doc.current_parsed()` which gates on generation match | Pending-parse test: arm parse delay, verify hover returns from last-published snapshot (generation N), not pre-filled from in-flight N+1 | `current_parsed()` contract (line 455 in document.rs): returns `None` if generation mismatched; hover must handle None gracefully (falls through to textual fallback at line 124) |
| **Source-fidelity loss** | Hover must use real source text, not empty-source overload, to preserve hover-doc extraction and precise text-range lookup | `hover.rs:extract_symbol_hover` — use `snapshot.semantic_analyzer()` which carries real source via ParsedSnapshot::source field | Fidelity test: hover on symbol with POD doc, assert doc is present; hover on symbol in narrow range, assert range is precise | `snapshot.semantic_analyzer()` calls `SemanticAnalyzer::analyze_with_source(ast, &self.source)` (real source); the empty-source overload `analyze(ast)` does NOT include source and loses doc extraction — MUST NOT regress to it |
| **Double-initialization / race** | Analyzer and type-engine cells must be constructed exactly once even under concurrent hover requests on same generation | `hover.rs:extract_symbol_hover` — 2-3 concurrent hovers at same offset, same generation N | Construction-count test: 3 hovers on generation N, verify build-count == 1 (not 3); then new generation N+1, verify it has its own 1x count | OnceLock on ParsedSnapshot guarantees exactly-once construction via `get_or_init()` atomicity; concurrent requests to `.semantic_analyzer()` block on the same lock until the first completes, then all return the same Arc |
| **Off-lock analysis correctness** | Hover analysis (analyzer walks AST, type-engine infers types) must complete entirely after documents-map lock is released, never re-entering the lock | `hover.rs:handle_hover` — lock released at line 68, analysis begins at line 77 | Concurrency test: stress-test with concurrent hovers + didChange events, verify no deadlock and results are consistent | The off-lock design is already in place (line 54-68 comment explains it); snapshot is Arc-owned and escapes the lock; analyzer and type-engine only read the AST (no lock re-entry); safe by design |
| **Cache-coherence / stale metrics** | Memory accounting fields for old caches must be removed; missing a field produces a compile error (no silent dead code) | `runtime/mod.rs:MemoryStateSnapshot` or similar metrics struct | Compile-only test: verify all cache-related metrics fields are deleted (compiler error if builder forgets) | Delete ALL cache fields and all code that populates them; compiler will error if any stray reference remains |
| **Lock/contention regression** | Hover must NOT hold documents-map lock during analysis (already satisfied by off-lock design); change must not re-introduce lock-holding | `hover.rs:handle_hover` — lock boundaries at line 62-68 | Performance regression test (optional): measure lock-hold time before/after, verify no increase | The snapshot is already owned (Arc) before lock release; analyzer/type-engine are methods on snapshot (no lock access); no regression risk |

## §Contracts

| Contract | Source Document + Section | How This Change Satisfies / Extends |
|---|---|---|
| ParsedSnapshot generation-ownership invariant | `docs/reference/SPEC_TEMPLATE.md` + Phase 5 (#3765 PR description) | Hover now consumes `snapshot.semantic_analyzer()` and `snapshot.type_environment()` — both are generation-owned (same generation as the snapshot itself). Old cache keys `(uri, content_hash)` were not generation-gated; new snapshot methods are inherently generation-scoped. Hover's freshness is now guaranteed by ParsedSnapshot's generation contract. |
| SemanticAnalyzer::analyze_with_source source preservation | `perl-semantic-analyzer` crate docs + #3765 notes | Hover must call `snapshot.semantic_analyzer()` (which internally calls `analyze_with_source(ast, &snapshot.source)` — real source). DO NOT regress to bare `SemanticAnalyzer::analyze(ast)` (empty source, loses hover-docs). This preserves the fidelity contract established by #3765. |
| Off-lock analysis after generation-ownership | `docs/reference/ORCHESTRATION_DOCTRINE.md` + #3396 on off-lock providers | Hover releases documents-map lock before analysis begins (line 68 releases guard). ParsedSnapshot is Arc-owned, escapes the lock, and is analyzed entirely off-lock. This satisfies the off-lock provider contract. Analyzer and type-engine do not access the lock. |
| Lazy construction efficiency | `#3765 PR description` + Phase 5 requirements | ParsedSnapshot's `semantic_analyzer()` and `type_environment()` use OnceLock to guarantee exactly-once construction per snapshot. Hover benefits: repeated hovers on same generation share the same Arc<SemanticAnalyzer> and Arc<TypeInferenceEngine>, no rebuild per request. |
| LSP server state coherence | `docs/reference/ORCHESTRATION_DOCTRINE.md` | Removing LspServer-level `semantic_analyzer_cache` and `type_inference_engine_cache` fields improves state locality: analysis facts now live alongside the parsed state (on ParsedSnapshot) instead of in separate caches. This reduces the surface for dual-write bugs (old pattern: ast + cache could disagree; new pattern: snapshot's cells and ast are always consistent). |

## §API-Shape

| Item | Kind | Signature / Range | Dup-risk (grep result) | Caller count |
|---|---|---|---|---|
| N/A | — | No new public API surface | — | — |

**Explanation**: This is a pure refactoring. No new public functions, structs, enums, or ID-spaces. `ParsedSnapshot.semantic_analyzer()` and `ParsedSnapshot.type_environment()` already exist (added in #3765); this change only makes hover consume them. The old `LspServer.get_or_build_analyzer()` and `LspServer.get_or_build_type_engine()` methods are internal (not in a public trait or export), so their removal is not a breaking change.

## §Test-Grid

| Scenario | Kind | Test name | Invariant discharged |
|---|---|---|---|
| Symbol hover in generation N | positive | `test_hover_happy_path` | Basic behavior: hover returns well-formed response with kind, name, doc |
| Generation round-trip: edit changes symbol's type | positive | `test_hover_generation_roundtrip_type_change` | **Freshness**: hover reflects new type after edit (not cached old) — this is the critical proof that generation-gating works |
| Generation round-trip: edit then hover at different offset | positive | `test_hover_generation_roundtrip_different_offset` | Generation-owned state applies to the whole snapshot, not per-offset |
| Hover on symbol with POD doc | positive | `test_hover_fidelity_pod_documentation` | **Fidelity**: doc present (proves real source used, not empty-source) |
| Hover on symbol with narrow range | positive | `test_hover_fidelity_precise_range` | **Fidelity**: text range is tight (real source enables precise lookup) |
| Multiple hovers in same generation | positive | `test_hover_construction_count_single_gen` | **Construction-count**: analyzer/type-engine built exactly 1x (not per-hover) — uses `semantic_analyzer_build_count()` and `type_environment_build_count()` test-only methods |
| Hovers in successive generations | positive | `test_hover_construction_count_multi_gen` | Each generation has its own 1x construction; old snapshot's cells don't carry over |
| Pending parse: generation N+1 in flight (unpublished) | negative | `test_hover_pending_parse_gap` | **Pending-parse-gap honesty**: returns from last-published (N) OR degraded, NEVER stale-wrong (N+1) |
| Malformed code (parse failed) | negative | `test_hover_degradation_tier_minimal` | Hover gracefully degrades to textual fallback when AST is None |
| Empty line / whitespace | negative | `test_hover_empty_input` | No panic, returns null hover |
| Concurrent hovers same generation | adversarial | `test_hover_concurrent_same_gen` | No deadlock, all return consistent results, construction count still 1x |
| Concurrent hovers + didChange | adversarial | `test_hover_concurrent_with_didchange` | Hovers on generation N while N+1 is in flight; no stale results, eventual consistency |
| Stale request (version mismatch) | adversarial | `test_hover_stale_request_rejection` | Existing check at line 52 still works; stale requests rejected with CONTENT_MODIFIED |

## §Blast-Radius

| Consumer | Crate | Dependency type | Impact | Required update |
|---|---|---|---|---|
| `handle_hover` provider | perl-lsp-rs | direct change | Internal implementation only; external behavior identical | None — the provider's signature and response contract remain the same |
| Hover integration tests | perl-lsp-rs tests | direct | 4 new test files added (no existing tests broken) | Test files created in checklist Step 10-13 |
| Hover documentation / doc comments | perl-lsp-rs | documentation | Line 197 in hover.rs: update doc comment to reference snapshot methods instead of cache methods | Update comment in Step 1 (or Step 2 as follow-up) |

**Must-not-touch boundary:**
- References provider (`references.rs` — separate follow-up #3766-slice-2)
- Rename provider (`rename.rs` — separate follow-up #3766-slice-3)
- Completion provider (already migrated in #3765; do not change)
- Navigation provider (`navigation.rs` — separate follow-up if needed)
- Any workspace indexing or module resolution code
- Parser, lexer, semantic analyzer, type inference crates (consumers only, no changes)

## §Coverage-Map

Not applicable — this is a refactoring with no coverage changes. No new code paths introduced; logic paths remain identical to before. The test-grid (§Test-Grid) proves that freshness and construction efficiency work; these tests should cover any new or changed seams.

---

## Proof obligations (from issue #3766)

✓ **Generation round-trip**: Hover on symbol → edit doc (bumps generation) → hover again → answer reflects NEW generation's analysis, never prior's. 
  - Test: `test_hover_generation_roundtrip_type_change` (§Test-Grid)

✓ **Pending-parse-gap honesty**: Hover during pending parse (generation N+1 in flight, unpublished) returns honest answer from last-published or degraded/pending — NEVER stale-wrong from a yet-unpublished generation.
  - Test: `test_hover_pending_parse_gap` (§Test-Grid)

✓ **Fidelity preservation**: Hover reads WITH source via snapshot's `source: Arc<str>` (do NOT regress to empty-source `analyze(ast)` overload that loses fidelity).
  - Tests: `test_hover_fidelity_pod_documentation`, `test_hover_fidelity_precise_range` (§Test-Grid)

✓ **Construction-count**: Reuse #3765 pattern — many hovers on one generation → one analyzer/type-engine construction; superseded generation never queried → zero construction.
  - Test: `test_hover_construction_count_single_gen`, `test_hover_construction_count_multi_gen` (§Test-Grid)

✓ **Cache retirement**: Grep for ALL consumers of `semantic_analyzer_cache` / `type_inference_engine_cache` BEFORE deleting. If hover is the LAST consumer → retire caches + invalidation/memory-accounting in this slice.
  - Verification: Step 3-9 in checklist; builder must confirm grep result == only hover before deleting
