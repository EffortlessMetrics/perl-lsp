# Implementation Checklist for #2577 PR1: Lexical Reference Extractor + Receipt

## Pre-build verification (VERIFY all paths exist on origin/main)

All paths verified ✓:
- `crates/perl-parser-core/src/pir/model.rs` — LexicalName, PirSourceAnchor, PirOperation::{LexicalRead, LexicalWrite, Modify} ✓
- `crates/perl-parser-core/src/pir/lower.rs` — lower_hir_bodies, lower_body, push_body_node ✓
- `crates/perl-parser-core/src/hir/model.rs` — HirFile with .bodies: Vec<HirBody> ✓
- `crates/perl-parser-core/src/hir/body.rs` — BodyOwnerKind, HirBody with .owner, HirBodyId ✓
- `crates/perl-parser-core/src/lib.rs` — pub mod hir, pub mod pir ✓
- `crates/perl-tdd-support/` — provides must, must_some, perl_test_must re-export ✓

## Implementation sequence (ORDERED, compiles at each step)

### STEP 1: Create extractor module and public types
**File:** `crates/perl-parser-core/src/pir/extractor.rs` (CREATE)

**What:** Define the core data structures and public API. This step is declarative — types only, no impl yet.

**Exact signatures:**

```rust
use crate::hir::{BodyOwnerKind, HirFile};
use crate::pir::model::{LexicalName, PirSourceAnchor};

pub const LEXICAL_EXTRACTOR_RECEIPT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LexicalBindingFact {
    pub name: LexicalName,
    pub role: LexicalRole,
    pub source_anchor: PirSourceAnchor,
    pub body_idx: usize,
    pub body_owner: BodyOwnerKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LexicalRole { Read, Write }

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BodyExtractionResult {
    pub body_idx: usize,
    pub owner: BodyOwnerKind,
    pub facts: Vec<LexicalBindingFact>,
    pub anchored_node_count: usize,
    pub total_node_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LexicalExtractorReceipt {
    pub schema_version: u32,
    pub bodies: Vec<BodyExtractionResult>,
    pub total_read_count: usize,
    pub total_write_count: usize,
    pub skipped_node_count: usize,
    pub provider_behavior_changed: bool,
}

#[must_use]
pub fn extract_lexical_facts(file: &HirFile) -> LexicalExtractorReceipt {
    unimplemented!()
}
```

**Verify after:** `cargo check -p perl-parser-core` compiles, types are public and reachable.

---

### STEP 2: Expose extractor in pir module
**File:** `crates/perl-parser-core/src/pir/mod.rs` (EDIT)

**What:** Add `pub mod extractor;` and re-export the public types.

**Exact changes:**

After the existing `mod lower; mod model;` lines, add:
```rust
mod extractor;
```

After the existing `pub use` statements from `lower` and `model`, add:
```rust
pub use extractor::{
    BodyExtractionResult, LexicalBindingFact, LexicalExtractorReceipt, LexicalRole,
    LEXICAL_EXTRACTOR_RECEIPT_VERSION, extract_lexical_facts,
};
```

**Verify after:** `cargo check -p perl-parser-core` still compiles, `extract_lexical_facts` is reachable as `crate::pir::extract_lexical_facts`.

---

### STEP 3: Implement extract_lexical_facts core logic
**File:** `crates/perl-parser-core/src/pir/extractor.rs` (EDIT)

**What:** Replace the `unimplemented!()` with the main extractor loop.

**Exact algorithm:**

```rust
pub fn extract_lexical_facts(file: &HirFile) -> LexicalExtractorReceipt {
    let mut bodies = Vec::new();
    let mut total_read_count = 0;
    let mut total_write_count = 0;
    let mut skipped_node_count = 0;

    // Iterate all bodies in the file.
    for (body_idx, body) in file.bodies.iter().enumerate() {
        let owner = body.owner.clone();
        let mut facts = Vec::new();
        let mut anchored_node_count = 0;
        let mut total_node_count = 0;

        // Lower this single body to PIR.
        let pir_nodes = lower_single_body(body, HirBodyId(body_idx as u32), file);

        for pir_node in pir_nodes {
            total_node_count += 1;

            match &pir_node.operation {
                PirOperation::LexicalRead { name } => {
                    if pir_node.source_anchor.is_anchored() {
                        anchored_node_count += 1;
                        facts.push(LexicalBindingFact {
                            name: name.clone(),
                            role: LexicalRole::Read,
                            source_anchor: pir_node.source_anchor.clone(),
                            body_idx,
                            body_owner: owner.clone(),
                        });
                        total_read_count += 1;
                    }
                }
                PirOperation::LexicalWrite { name } => {
                    if pir_node.source_anchor.is_anchored() {
                        anchored_node_count += 1;
                        facts.push(LexicalBindingFact {
                            name: name.clone(),
                            role: LexicalRole::Write,
                            source_anchor: pir_node.source_anchor.clone(),
                            body_idx,
                            body_owner: owner.clone(),
                        });
                        total_write_count += 1;
                    }
                }
                PirOperation::Modify { .. } | PirOperation::StashModify { .. } => {
                    skipped_node_count += 1;
                }
                _ => {
                    // Other operations (StashRead, StashWrite, Call, etc.) are ignored.
                }
            }
        }

        bodies.push(BodyExtractionResult {
            body_idx,
            owner,
            facts,
            anchored_node_count,
            total_node_count,
        });
    }

    LexicalExtractorReceipt {
        schema_version: LEXICAL_EXTRACTOR_RECEIPT_VERSION,
        bodies,
        total_read_count,
        total_write_count,
        skipped_node_count,
        provider_behavior_changed: false,
    }
}
```

**Missing import:** `lower_single_body` must be exposed from `lower.rs`. See STEP 4 below.

**Verify after:** `cargo check -p perl-parser-core` — will fail until STEP 4 completes.

---

### STEP 4: Expose lower_single_body from lower.rs
**File:** `crates/perl-parser-core/src/pir/lower.rs` (EDIT)

**What:** Create a new public function that lowers a single body without merging into a flat graph.

**Exact addition (add after `lower_hir_bodies` functions):**

```rust
/// Lower a single body to PIR nodes, preserving body identity.
///
/// This is the engine for the lexical extractor: it processes one HirBody at a time,
/// yielding all PIR nodes emitted from that body without merging into a flat graph.
/// Body boundaries are preserved, enabling per-body analysis like scope isolation.
///
/// Returns a Vec of PirNode, in lowering order. Each node carries its source anchor.
#[must_use]
pub fn lower_single_body(body: &HirBody, body_id: HirBodyId, file: &HirFile) -> Vec<PirNode> {
    // Create a minimal lowering context for a single body.
    let mut lowerer = PirLowerer::new(file);
    
    // Lower just this body into the lowerer's graph.
    lowerer.lower_body(body, body_id, file);
    
    // Extract the nodes from the graph and return them.
    lowerer.graph.nodes.clone()
}
```

**Dependencies:** Requires visibility of `PirLowerer` (check if it's `pub` in `lower.rs`; if not, make it `pub` or re-factor). Also requires importing `HirBody`, `HirBodyId`, and `PirNode` at the top of the function. Verify the exact lowerer struct name from origin/main.

**Verify after:** `cargo check -p perl-parser-core` compiles. The extractor (STEP 3) now resolves.

---

### STEP 5: Create test file with five fixtures
**File:** `crates/perl-parser-core/tests/pir_lexical_extractor_test.rs` (CREATE)

**What:** Comprehensive test coverage for all acceptance criteria.

**Exact structure:**

```rust
use perl_parser_core::Parser;
use perl_parser_core::hir::{HirFile, lower_ast};
use perl_parser_core::pir::extract_lexical_facts;
use perl_tdd_support::must_some;

fn extract(source: &str) -> perl_parser_core::pir::LexicalExtractorReceipt {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let hir: HirFile = lower_ast(&output.ast);
    extract_lexical_facts(&hir)
}

#[test]
fn fixture_1_scope_isolation_shadowed_names() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
my $x = 5;
print $x;
$x = 10;
say $x;
sub foo { my $x = 20; return $x; }
say $x;
"#;
    let receipt = extract(source);
    
    assert_eq!(receipt.schema_version, 1);
    assert!(!receipt.provider_behavior_changed);
    assert_eq!(receipt.bodies.len(), 2); // ProgramRoot + foo
    
    // ProgramRoot (body_idx=0): 1 Write + 3 Reads of outer $x
    let prog_body = &receipt.bodies[0];
    assert_eq!(prog_body.body_idx, 0);
    let prog_facts: Vec<_> = prog_body.facts.iter()
        .filter(|f| f.name.name == "x" && f.name.sigil == "$")
        .collect();
    let reads = prog_facts.iter().filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Read)).count();
    let writes = prog_facts.iter().filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Write)).count();
    assert_eq!(reads, 3, "outer $x should have 3 reads");
    assert_eq!(writes, 1, "outer $x should have 1 write");
    
    // All facts must be anchored.
    for fact in &prog_facts {
        assert!(fact.source_anchor.is_anchored(), "fact must be source-anchored");
    }
    
    // foo body (body_idx=1): 1 Write + 1 Read of foo's $x
    let foo_body = &receipt.bodies[1];
    assert_eq!(foo_body.body_idx, 1);
    let foo_facts: Vec<_> = foo_body.facts.iter()
        .filter(|f| f.name.name == "x" && f.name.sigil == "$")
        .collect();
    let foo_reads = foo_facts.iter().filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Read)).count();
    let foo_writes = foo_facts.iter().filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Write)).count();
    assert_eq!(foo_reads, 1, "foo's $x should have 1 read");
    assert_eq!(foo_writes, 1, "foo's $x should have 1 write");
    
    // Outer and foo's facts must be in separate BodyExtractionResult entries.
    assert_ne!(
        receipt.bodies[0].body_idx, receipt.bodies[1].body_idx,
        "scope isolation: outer and foo must be separate bodies"
    );
    
    Ok(())
}

#[test]
fn fixture_2_state_variable() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
sub counter { state $n = 0; return $n; }
"#;
    let receipt = extract(source);
    
    assert_eq!(receipt.schema_version, 1);
    assert!(!receipt.provider_behavior_changed);
    
    // ProgramRoot (body_idx=0): no facts
    let prog_body = &receipt.bodies[0];
    assert!(prog_body.facts.is_empty(), "program root should have no lexical facts");
    
    // counter body (body_idx=1): 1 Write + 1 Read of $n
    let counter_body = &receipt.bodies[1];
    let counter_facts: Vec<_> = counter_body.facts.iter()
        .filter(|f| f.name.name == "n" && f.name.sigil == "$")
        .collect();
    let reads = counter_facts.iter().filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Read)).count();
    let writes = counter_facts.iter().filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Write)).count();
    assert_eq!(reads, 1, "state $n should have 1 read");
    assert_eq!(writes, 1, "state $n should have 1 write");
    
    Ok(())
}

#[test]
fn fixture_3_modify_nodes_skipped() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 0; $x++;";
    let receipt = extract(source);
    
    assert_eq!(receipt.schema_version, 1);
    assert!(!receipt.provider_behavior_changed);
    
    // ProgramRoot: 1 Write (declaration), 0 Reads, 1 Modify (skipped)
    let prog_body = &receipt.bodies[0];
    let x_facts: Vec<_> = prog_body.facts.iter()
        .filter(|f| f.name.name == "x" && f.name.sigil == "$")
        .collect();
    let reads = x_facts.iter().filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Read)).count();
    let writes = x_facts.iter().filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Write)).count();
    assert_eq!(reads, 0, "$x++ should NOT count as a Read");
    assert_eq!(writes, 1, "should have 1 write from declaration");
    
    // Skipped count should include the Modify operation.
    assert!(receipt.skipped_node_count >= 1, "Modify should be in skipped_node_count");
    
    Ok(())
}

#[test]
fn fixture_4_empty_sub_body() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub empty {}";
    let receipt = extract(source);
    
    assert_eq!(receipt.schema_version, 1);
    assert!(!receipt.provider_behavior_changed);
    assert_eq!(receipt.bodies.len(), 2); // ProgramRoot + empty
    
    // empty sub (body_idx=1): facts=[], total_node_count=0, no panic.
    let empty_body = &receipt.bodies[1];
    assert_eq!(empty_body.body_idx, 1);
    assert!(empty_body.facts.is_empty(), "empty body should have no facts");
    assert_eq!(empty_body.total_node_count, 0, "empty body should have 0 total nodes");
    
    Ok(())
}

#[test]
fn fixture_5_receipt_invariants() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
my $x = 1;
print $x;
sub foo { my $y = 2; return $y; }
"#;
    let receipt = extract(source);
    
    // schema_version invariant
    assert_eq!(receipt.schema_version, 1);
    assert_eq!(receipt.schema_version, perl_parser_core::pir::LEXICAL_EXTRACTOR_RECEIPT_VERSION);
    
    // provider_behavior_changed invariant
    assert!(!receipt.provider_behavior_changed, "PR1 must NOT change provider behavior");
    
    // Count invariant: total_read + total_write == sum of all facts
    let sum_facts: usize = receipt.bodies.iter().map(|b| b.facts.len()).sum();
    assert_eq!(
        receipt.total_read_count + receipt.total_write_count,
        sum_facts,
        "count invariant: read + write must equal sum of facts"
    );
    
    Ok(())
}
```

**Verify after:** `cargo test -p perl-parser-core --test pir_lexical_extractor_test -- --nocapture` — all tests pass, fixture assertions succeed.

---

### STEP 6: Run full test suite and verification
**Command after each step:**

After STEP 1: `cargo check -p perl-parser-core`
After STEP 2: `cargo check -p perl-parser-core`
After STEP 3: `cargo check -p perl-parser-core` (fails until STEP 4)
After STEP 4: `cargo check -p perl-parser-core`
After STEP 5: `cargo test -p perl-parser-core --test pir_lexical_extractor_test -- --nocapture`

**Final gate (STEP 6):**

```bash
cargo test -p perl-parser-core
cargo clippy -p perl-parser-core -- -D warnings
```

Both must pass with no errors or warnings.

---

## Key notes for builder

- **No `cargo xtask fmt`** — the fmt xtask has a 30+ minute cold build. Red-TDD will run it once; builder must verify locally with `cargo fmt`.
- **No `unwrap()` / `expect()` / `panic!()` / `todo!()`** in production code. The algorithm is deterministic; all error paths must be handled with `Result` or `Option`.
- **Body identity is `body_idx`**, not body owner kind. Two subroutines with the same name will have different body_idx values and never merge.
- **`lower_single_body` must preserve body boundaries** — do not call `lower_hir_bodies` (which flattens all bodies into one graph). The extractor depends on per-body isolation.
- **All facts must be source-anchored.** The algorithm checks `is_anchored()` before adding to facts. Non-anchored operations (generated, ambient) are skipped silently.
- **Modify/StashModify are explicitly skipped** — they don't count as Read or Write, but they do increment skipped_node_count so the receipt can track what was filtered.

---

## Handoff ready when

- All 6 steps compile and pass verification commands
- All five test fixtures pass with correct assertion counts
- No warnings from clippy
- Red-TDD can write failing tests against the public API (`extract_lexical_facts`, types from `perl_parser_core::pir`)
