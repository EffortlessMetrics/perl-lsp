# PR1 Context: Lexical Reference Extractor + Receipt

## Merged-main reality (corrects stale issue body)

- **#2610 MERGED** (commit 023dc1598): `lower_hir_bodies` in `crates/perl-parser-core/src/pir/lower.rs` now emits `PirOperation::LexicalRead`/`LexicalWrite`/`Modify`/`StashRead`/`StashWrite`/`StashModify` with `PirSourceAnchor` for every operation. Each `PirNode` carries a source anchor.
- **#2537 CLOSED/SUBSUMED**: No separate gating needed. `LexicalRead` is already emitted on origin/main.

## Critical structural insights from plan-review

### Scope isolation requires per-body lowering, not flat-graph walking

The original spec proposed extracting from a flat `PirGraph` returned by `lower_hir_bodies`. **This does NOT preserve scope isolation.** The problem:

- `PirNode.scope` is hardcoded `None` for all body-lowered nodes (per `push_body_node` in `lower.rs`: "body arenas don't carry HirScopeId per-node in this slice")
- `PirNode.package_context` is `None` for all body-lowered nodes
- `LexicalName` carries only `{sigil: String, name: String}` — no scope discriminator
- Two `my $x` variables in different bodies (program root vs subroutine) are **indistinguishable** by name alone in the flat graph

**Solution:** Iterate `HirFile.bodies` directly and use `(body_idx, sigil, name)` as the binding identity. Process bodies one at a time, preserving body boundaries. This is why the extractor must be in `perl-parser-core` with direct access to `HirBody` and `HirFile`.

### Correct placement: perl-parser-core, not perl-lsp-rs-core

The original spec proposed putting the extractor in `perl-lsp-rs-core/src/providers/navigation/references_shadow.rs`. **This is the wrong target.** On origin/main, that file (580 lines) is a high-level LSP module that:

- Computes `SemanticShadowCompareReceipt` using `SemanticQueries` and `OccurrenceFact`
- Depends on `WorkspaceIndex` and other provider traits
- Handles shadow comparison logic for the references provider

Bolting a PIR graph walker onto it would:
- Violate single responsibility (mixing compiler substrate + LSP provider logic)
- Create awkward high-level LSP type deps from a lowering concern
- Make the module impossible to understand

The extractor is **pure PIR-A computation** — it takes a `HirFile` (from `perl-parser-core`) and returns facts about lexical reads/writes grouped by body. It belongs next to the `lower.rs` code. `perl-lsp-rs-core` already depends on `perl-parser-core`, so no new dependency edges are needed.

### Receipt is a new struct, not an extension of PirReceipt

The original spec mentions a receipt with: `document_generation`, `source_hash`, `binding_identity`, `provenance`, `confidence`, `refusal_reason`. None of these exist on `PirReceipt` (which models lowering stats like node_count, edge_count). The extractor defines its own `LexicalExtractorReceipt` struct, distinct from `PirReceipt`.

## Scope IN

- Same-file, same-body resolved lexical bindings (`my`/`state` variables)
- `LexicalRead` and `LexicalWrite` node types only
- Per-body extraction using `HirBody.owner` as body identity discriminator
- New `LexicalExtractorReceipt` struct in `perl-parser-core`
- Source anchors on all emitted facts (verified by `is_anchored() == true`)
- Tests in `crates/perl-parser-core/tests/pir_lexical_extractor_test.rs`

## Scope OUT (explicitly excluded for PR1)

- `StashRead`/`StashWrite` (package globals) — OUT
- `Modify`/`StashModify` — skipped, not counted as Read or Write
- Cross-file references — OUT (future PR)
- `references_shadow.rs` changes — OUT (that is PR2 #2634)
- `xtask/oracle_runner.rs` changes — OUT (that is PR2/PR3)
- Any LSP provider behavior change — OUT
- Guarded promotion to provider — OUT (that is PR3 #2635)

## Follow-up PRs (deferred)

- **PR2 (#2634)**: Shadow compare using PIR-A extractor output
- **PR3 (#2635)**: Guarded promotion to references provider
- Both depend on this PR1 landing first

## Test fixture rationale

Five fixtures exercise:

1. **Multi-body scope isolation** — outer `$x` vs foo's `$x` must not merge
2. **State variables** — treated like `my`, correctly extracted
3. **Modify nodes** — skipped, not counted as facts
4. **Empty bodies** — no panic on zero facts
5. **Receipt invariants** — schema version, counts, behavior flag

Each fixture is a small Perl string that lowers to a minimal `HirFile` with predictable body structure, making assertions on per-body fact counts bulletproof and independent of parser evolution.

## Accepted by

- plan-reviewer (sonnet) — stress-tested scope isolation requirement, verified structural placement, approved spec
- Marked as `builder-ready` and `plan-reviewed` on issue #2577
