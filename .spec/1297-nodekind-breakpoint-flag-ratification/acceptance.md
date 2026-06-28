# Acceptance Criteria: #1297 — Validate NodeKind safe_for_breakpoint/introduces_scope flag values before Phase 7/8 DAP consumption

## §Behavior

| Input / Condition | Expected Result | Notes |
|---|---|---|
| Read `NodeKind::Use` flags | `safe_for_breakpoint=false`, `introduces_scope=false`, `executable=true` | Compile-time pragma; not breakable in runtime debugger. Perl 5.40.1 probe confirms "not breakable". |
| Read `NodeKind::No` flags | `safe_for_breakpoint=false`, `introduces_scope=false`, `executable=true` | Compile-time unimport; not breakable in runtime debugger. Perl 5.40.1 probe confirms "not breakable". |
| Read `NodeKind::Eval` flags | `safe_for_breakpoint=true`, `introduces_scope=true` (variant-level prefilter) | Variant flag is conservative; consumer must check if `block` is `NodeKind::Block` to determine actual scope. `eval BLOCK` has scope; `eval STRING`/`eval EXPR` do not. |
| Read `NodeKind::Package` flags with block | `safe_for_breakpoint=true`, `introduces_scope=true` | `package Foo { ... }` form; consumer should verify `block.is_some()`. |
| Read `NodeKind::Package` flags without block | Variant-level flags are still `true` | `package Foo;` form; consumer must check `block.is_none()` to know scope/breakpoint behavior differs from block form. |
| Read `NodeKind::PhaseBlock` flags | `safe_for_breakpoint=true`, `introduces_scope=true` | Variant flag is prefilter; DAP consumer checks phase name field. BEGIN/CHECK/UNITCHECK (compile-phase, not stoppable in runtime); END (stoppable); INIT (maybe, attach-timing). |
| Read `NodeKind::Class` flags | `safe_for_breakpoint=true`, `introduces_scope=true` | Unchanged from baseline; perl 5.40.1 debugger confirms `class Foo { ... }` header is breakable. |
| Read `NodeKind::Goto` flags | `safe_for_breakpoint=true`, `introduces_scope=false` | Unchanged from baseline; executable statement before control transfer. |
| Read `NodeKind::Typeglob` flags | `safe_for_breakpoint=false`, `introduces_scope=false` | Unchanged from baseline; typeglob references/assignments introduce no lexical scope. |
| Test assertion: SAFE_FOR_BREAKPOINT_TRUE set | 41 variants (Use/No removed) | Test verifies all variants in pinned set have `safe_for_breakpoint=true`. |
| Test assertion: SAFE_FOR_BREAKPOINT_FALSE set | 28 variants (Use/No added) | Test verifies all variants in pinned set have `safe_for_breakpoint=false`. |
| Test assertion: exact partition | All 69 variants in exactly one set | No gaps, no overlaps. Test fails if variant is missing from both sets. |
| Instance-dependent test: Eval.introduces_scope with Block child | True | Variant flag matches actual scope behavior for block case. |
| Instance-dependent test: Package with block | Both flags true | Variant flags match block-form behavior. |
| Instance-dependent test: Package without block | Variant flags still true (prefilter) | Consumer must check `block.is_none()` to distinguish behavior. |
| Instance-dependent test: PhaseBlock with BEGIN phase | Flag true (prefilter); DAP marks not verified | Phase-name check happens in DAP layer, not in variant flag. |
| PARSER_CONTRACTS.md contract documented | Table with static/instance-dependent rows + consumer guidance | Contract section added; all 5 changed/ratified variants (Use, No, Eval, Package, PhaseBlock, Class, Goto, Typeglob) documented with evidence. |

**All tests pass:**
```
cargo test -p perl-ast
cargo test -p perl-ast -- --test-threads=1
```

**No clippy warnings:**
```
cargo clippy -p perl-ast
```

**Formatted:**
```
cargo xtask fmt
```

## §Hazards

**Subsystem:** perl-ast (classification). No DAP/LSP/parser protocol surface touched. Ratification-only: flag values + tests + docs.

| Class | Invariant | Surface | Required adversarial test |
|---|---|---|---|
| **Exhaustiveness (drift-guard)** | All 69 NodeKind variants have exactly one match arm in `flags()` with no wildcard. Adding a variant is a compile error. | `crates/perl-ast/src/classification.rs::NodeKind::flags()` (match exhaustiveness) | Test: compile fails if any variant is missing from match arm. (Rust type system enforces this.) |
| **Classification invariant** | `recovery_artifact=true` implies `safe_for_breakpoint=false`. Recovery nodes must never be breakpoint candidates. | `crates/perl-ast/src/classification.rs::NodeKindFlags::validate()` + `crates/perl-ast/tests/classification_tests.rs::recovery_artifact_implies_not_safe_for_breakpoint()` | Test: call `flags.validate()` on all variants; assert all recovery nodes (Error, MissingExpression, etc.) have `safe_for_breakpoint=false`. |
| **Pinned-set partition** | SAFE_FOR_BREAKPOINT_TRUE and SAFE_FOR_BREAKPOINT_FALSE partition all 69 variants exactly (no gaps, no overlaps). | `crates/perl-ast/tests/classification_tests.rs::SAFE_FOR_BREAKPOINT_TRUE` + `SAFE_FOR_BREAKPOINT_FALSE` (union = all variants) | Test: `safe_for_breakpoint_exact_true_set()` and `safe_for_breakpoint_exact_false_set()` iterate all_variants(); assert each is in exactly one set. |
| **Instance-dependent semantics** | Eval/Package/PhaseBlock have conservative variant-level flags; consumer docs explain when actual behavior differs (checked on AST structure, phase name). | `crates/perl-ast/src/classification.rs` (doc comments) + `docs/reference/PARSER_CONTRACTS.md` (contract section) | Test: `instance_dependent_flags_documented()` constructs Eval/Package/PhaseBlock variants, asserts variant flags, and documents where runtime/consumer verification differs. |
| **Evidence trace** | All flag values flip (Use/No) or decisions (keep Eval/Class/Goto/Typeglob/PhaseBlock) cite ChatGPT-Pro + perl-debugger probe results in comments. | `crates/perl-ast/src/classification.rs` (inline comments on each variant) + `docs/reference/PARSER_CONTRACTS.md` (evidence column in contract table) | Test: code review verifies each flag has a comment linking to evidence (perl 5.40.1 probe, perldoc, DAP spec). No blind flag values. |
| **No silent consumer breakage** | grep confirms no DAP/LSP code reads safe_for_breakpoint/introduces_scope yet. Flipping Use/No is safe. Phase 8 (separate PR) adds the consumer. | Search: `grep -r "safe_for_breakpoint\|introduces_scope" crates/perl-dap* crates/perl-lsp*` (should return no results in production code) | Test: builder verifies grep result before merging. Document Phase 8 dependency: this PR gates Phase 8 (DAP breakpoint validator consumer). |

N/A rows: none. All six hazard classes apply to a classification/contract change.

## §Contracts

| Contract | Source document + section | How this change satisfies or extends it |
|---|---|---|
| **NodeKind classification keystone** | `crates/perl-ast/src/classification.rs` §Design contract | This PR ratifies the keystone baseline (PR #930). Flipping Use/No reflects evidence from ChatGPT-Pro + perl-debugger probe. Validates that the baseline is correct before Phase 7/8 consumers depend on it. |
| **safe_for_breakpoint prefilter semantics** | `crates/perl-ast/src/classification.rs` §`safe_for_breakpoint` semantics (lines 21–27) | Extends module-level doc to document instance-dependent rows (Eval/Package/PhaseBlock). Clarifies that the flag is a variant-level prefilter, not a DAP guarantee. Consumers must check AST structure (block field, phase name). |
| **recovery_artifact → !safe_for_breakpoint invariant** | `crates/perl-ast/src/classification.rs` §Invariant (lines 29–31) | Unchanged. Use/No are not recovery nodes (recovery_artifact=false). Flipping their safe_for_breakpoint does not violate the invariant. Test asserts the invariant holds for all variants. |
| **Exhaustiveness (drift-guard)** | `crates/perl-ast/src/classification.rs` §No wildcard arms (lines 15–19) | Unchanged. Drift-guard remains: all 69 variants matched explicitly, compile error if missed. Flipping Use/No changes the match arm body only, not the structure. |
| **Breakpoint/scope prefilter contract (Phase 8 consumer policy)** | `docs/reference/PARSER_CONTRACTS.md` — new §Breakpoint and Scope Classification Contract | New section added by this PR. Documents static (variant-level) flags (Use, No, Goto, Class, Typeglob) and instance-dependent rows (Eval, Package, PhaseBlock) with consumer implementation guidance. Defines the contract for Phase 8 DAP breakpoint validator. |

## §API-Shape

| Item | Kind | Signature / Range | Dup-risk (grep result) | Caller count |
|---|---|---|---|---|
| `NodeKind::Use` | enum variant | `Use { module: String, args: Vec<Node>, has_filter_risk: bool }` (unchanged) | `grep -r "NodeKind::Use"` returns 23 results (parser construction, tests, traversal). No dup-risk; variant is unique by name. | 23 (internal parser + tests; no external consumers) |
| `NodeKind::No` | enum variant | `No { module: String, args: Vec<Node> }` (unchanged) | `grep -r "NodeKind::No"` returns 14 results. No dup-risk. | 14 (internal parser + tests) |
| `NodeKind::Eval` | enum variant | `Eval { block: Box<Node> }` (unchanged) | `grep -r "NodeKind::Eval"` returns 18 results. No dup-risk. | 18 (parser + tests) |
| `NodeKind::Package` | enum variant | `Package { name: String, name_span: SourceLocation, block: Option<Box<Node>> }` (unchanged) | `grep -r "NodeKind::Package"` returns 16 results. No dup-risk. | 16 (parser + tests) |
| `NodeKind::PhaseBlock` | enum variant | `PhaseBlock { phase: String, phase_span: Option<SourceLocation>, block: Box<Node> }` (unchanged) | `grep -r "NodeKind::PhaseBlock"` returns 11 results. No dup-risk. | 11 (parser + tests) |
| `NodeKindFlags::safe_for_breakpoint` | struct field | `pub safe_for_breakpoint: bool` (unchanged) | `grep -r "safe_for_breakpoint"` returns 25 results (classification.rs + classification_tests.rs only; no external consumers yet). | 0 (external); 25 (internal tests) |
| `NodeKindFlags::introduces_scope` | struct field | `pub introduces_scope: bool` (unchanged) | `grep -r "introduces_scope"` returns 15 results (classification.rs + classification_tests.rs only). | 0 (external); 15 (internal tests) |

**N/A rows:** No new public API surface. All changes are flag value updates (data) and doc extensions. No new functions, enums, or trait methods. NodeKind variants themselves are unchanged (constructors, fields). No Rust API breakage.

## §Test-Grid

| Scenario | Kind | Test name | Invariant discharged |
|---|---|---|---|
| Use variant flag correct | positive | `safe_for_breakpoint_exact_false_set()` (Use in FALSE set) | Use.safe_for_breakpoint=false after flip. |
| No variant flag correct | positive | `safe_for_breakpoint_exact_false_set()` (No in FALSE set) | No.safe_for_breakpoint=false after flip. |
| Eval variant flag (prefilter) | positive | `safe_for_breakpoint_exact_true_set()` (Eval in TRUE set) | Eval.safe_for_breakpoint=true (prefilter; instance check in consumer). |
| Class variant flag | positive | `safe_for_breakpoint_exact_true_set()` (Class in TRUE set) | Class.safe_for_breakpoint=true (unchanged from baseline). |
| Goto variant flag | positive | `safe_for_breakpoint_exact_true_set()` (Goto in TRUE set) | Goto.safe_for_breakpoint=true (unchanged). |
| Typeglob variant flag | positive | `safe_for_breakpoint_exact_false_set()` (Typeglob in FALSE set) | Typeglob.safe_for_breakpoint=false (unchanged). |
| PhaseBlock introduces_scope | positive | `safe_for_breakpoint_exact_true_set()` (PhaseBlock in TRUE set) | PhaseBlock.introduces_scope=true (unchanged); DAP layer checks phase name. |
| Package introduces_scope flag | positive | `safe_for_breakpoint_exact_true_set()` (Package in TRUE set) | Package.introduces_scope=true (prefilter); consumer checks block.is_some(). |
| Pinned set partition exact | negative (implicit) | `safe_for_breakpoint_covers_all_69_variants()` | All 69 variants in exactly one pinned set. No gaps or overlaps. |
| Recovery invariant | negative | `recovery_artifact_implies_not_safe_for_breakpoint()` | All recovery nodes (Error, MissingExpression, etc.) have safe_for_breakpoint=false. |
| Flags validate() invariant | negative | `validate_flags_on_all_variants()` | No variant has recovery_artifact=true AND safe_for_breakpoint=true simultaneously. |
| Instance-dependent: Eval with Block child | adversarial | `instance_dependent_flags_documented()` | Eval.introduces_scope variant flag is true; comment documents consumer must check block type. |
| Instance-dependent: Package with block | adversarial | `instance_dependent_flags_documented()` | Package.introduces_scope/safe_for_breakpoint variant flags are true; comment documents consumer must check block.is_some(). |
| Instance-dependent: Package without block | adversarial | `instance_dependent_flags_documented()` | Package.introduces_scope/safe_for_breakpoint variant flags remain true (prefilter); comment documents consumer must check block.is_none() to distinguish statement form. |
| Instance-dependent: PhaseBlock phase name | adversarial | `instance_dependent_flags_documented()` | PhaseBlock.safe_for_breakpoint variant flag is true (has block); comment documents DAP consumer must check phase field (BEGIN/CHECK/UNITCHECK not stoppable in runtime session). |
| Contract documentation | positive | Code review of PARSER_CONTRACTS.md § Breakpoint section | Table with static/instance-dependent rows + consumer guidance. All 5 variant-dependent rows (Use, No, Eval, Package, PhaseBlock, Class, Goto, Typeglob, TypeGlob) have evidence column linking to ChatGPT-Pro + probe results or perldoc. |

## §Blast-Radius

| Consumer | Crate | Dependency type | Impact | Required update |
|---|---|---|---|---|
| Classification tests | `perl-ast` (internal) | test dependency | Test pinned sets must be updated (Use/No moved from TRUE to FALSE). | Step 4–5 in checklist (modify SAFE_FOR_BREAKPOINT arrays). |
| Parser (internal) | `crates/perl-parser/` | indirect (flags read by consumers) | None — parser construction code is unchanged. NodeKind variants match exactly. | None. |
| LSP symbol/workspace indexing | `crates/perl-workspace/`, `crates/perl-lsp-*/` | not yet; prefilter for Phase 8 | None — LSP code does not yet read safe_for_breakpoint/introduces_scope. | Phase 8 (separate PR): DAP breakpoint validator will read flags + instance-check AST. |
| DAP breakpoint validator | `crates/perl-dap/` (Phase 8) | prefilter input | None in this PR — flags are inputs, not behavior change. Phase 8 consumer will read flags + check instances. | Phase 8 (separate PR): implement setBreakpoints handler using safe_for_breakpoint as prefilter + instance checks per Eval/Package/PhaseBlock. |
| Semantic analyzer | `crates/perl-semantic-analyzer/` | not yet | None — semantic analysis does not depend on breakpoint flags. | None. |

**Must-not-touch boundary:**
- `crates/perl-ast/src/ast.rs` — NodeKind enum unchanged (no field additions, no variant changes)
- `crates/perl-parser/` — parser construction unchanged (Use/No/Eval/Package/PhaseBlock nodes constructed identically)
- `crates/perl-dap/` — DAP code untouched in this PR (Phase 8 follows)
- `crates/perl-lsp*/` — LSP providers untouched
- Feature governance (`features.toml`) — unchanged
- Protocol schema (`DAP_PROTOCOL_SCHEMA.md`, LSP spec docs) — unchanged

**Builder flag:** Verify `cargo grep` or `git diff` shows changes only to classification.rs (flag values + doc comments), classification_tests.rs (pinned sets + instance-dependent test), and PARSER_CONTRACTS.md. No other files touched.
