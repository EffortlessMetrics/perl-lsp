# Acceptance Criteria for #2577 PR1: Lexical Reference Extractor + Receipt

## Extraction correctness (test fixtures)

- [ ] Fixture 1 (scope isolation, shadowed names): outer `$x` facts isolated in body_idx=0 (ProgramRoot), foo's `$x` isolated in body_idx=1 (Subroutine), counts are 1 Write + 3 Reads (outer) and 1 Write + 1 Read (foo), never merged
- [ ] Fixture 2 (state variable): `state $n` extracted as 1 Write + 1 Read in subroutine body, ProgramRoot has 0 facts
- [ ] Fixture 3 (Modify nodes skipped): `$x++` does NOT count as a Read or Write; skipped_node_count >= 1
- [ ] Fixture 4 (empty sub body): empty subroutine yields facts=[], total_node_count=0, no panic
- [ ] Fixture 5 (receipt invariants): provider_behavior_changed == false, schema_version == LEXICAL_EXTRACTOR_RECEIPT_VERSION, total_read_count + total_write_count == sum of all body.facts.len()

## Per-body fact structure

- [ ] Each `LexicalBindingFact` carries: name (sigil + identifier), role (Read or Write), source_anchor with is_anchored() == true, body_idx (0-based), body_owner (BodyOwnerKind discriminator)
- [ ] `BodyExtractionResult` groups facts by body_idx with owner, facts list, anchored_node_count, total_node_count
- [ ] `body_idx` correctly discriminates ProgramRoot (0) from Subroutine bodies (1+)

## Source anchor coverage

- [ ] All emitted `LexicalBindingFact.source_anchor.is_anchored() == true`
- [ ] Each anchor points to the source range of the lexical reference in the original Perl source

## Receipt shape and invariants

- [ ] `LexicalExtractorReceipt` has: schema_version, bodies (Vec<BodyExtractionResult>), total_read_count, total_write_count, skipped_node_count, provider_behavior_changed
- [ ] `schema_version == LEXICAL_EXTRACTOR_RECEIPT_VERSION` (const u32 = 1)
- [ ] `provider_behavior_changed == false` (always, for PR1)
- [ ] `total_read_count + total_write_count == sum(bodies[].facts.len())`

## API contract

- [ ] Public function: `pub fn extract_lexical_facts(file: &HirFile) -> LexicalExtractorReceipt`
- [ ] All types marked `#[non_exhaustive]` for forward compatibility
- [ ] Derives: `Debug, Clone, PartialEq, Eq` (matching PIR-A style)

## Code quality gates

- [ ] No `unwrap()`, `expect()`, `panic!()`, `todo!()`, or `dbg!()` in production code
- [ ] All tests use `Result<()>` return or `perl_tdd_support::must`/`must_some`
- [ ] `cargo test -p perl-parser-core` passes with no regressions
- [ ] `cargo clippy -p perl-parser-core -- -D warnings` passes
- [ ] `cargo xtask fmt` produces no changes

## No-side-effects gate

- [ ] `references_shadow.rs` unchanged (reserved for PR2)
- [ ] `xtask/oracle_runner.rs` unchanged (reserved for PR2/PR3)
- [ ] No provider behavior changes (receipt confirms provider_behavior_changed == false)
- [ ] LSP compliance fully backward compatible — provider output identical before/after

## Module integration

- [ ] New module `crates/perl-parser-core/src/pir/extractor.rs` created
- [ ] `crates/perl-parser-core/src/pir/mod.rs` adds `pub mod extractor;` and re-exports key types
- [ ] New test file `crates/perl-parser-core/tests/pir_lexical_extractor_test.rs` created
- [ ] Test imports compile against public API only (no internal details)

## Verification commands (all must pass)

```bash
cargo test -p perl-parser-core --test pir_lexical_extractor_test -- --nocapture
cargo test -p perl-parser-core
cargo clippy -p perl-parser-core -- -D warnings
cargo xtask fmt
```
