# Parser Contract Index

**Purpose.** One place an agent or PR author can ask:
*"What contract does this change touch? What tests must change? What must not be broadened?"*

Every contract below names: the invariant that must hold, the owner module, the
consumers, the proof tests and oracle, known exceptions, and non-goals / future
migrations.

This document is kept factual and citable. Claims without a primary artifact
(file path, test name, merged PR) are not made.

**Related**: For repo-specific incidents motivating scanner and coverage contracts, see
[docs/learnings/README.md](../learnings/README.md) (especially 2026-06-coverage-gate-measurement.md,
2026-06-ripr-output-schema-break.md). For portable patterns, see [docs/concepts/](../concepts/).
For the parallel DAP wire-protocol contract index (variablesReference wire-band codec), see
[docs/reference/DAP_CONTRACTS.md](DAP_CONTRACTS.md).
For the semantic model substrate contracts (ownership boundary, semantic identity,
FileSemanticBundle, SemanticSnapshot, SemanticResult, dynamic boundaries), see
[docs/reference/SEMANTIC_SNAPSHOT_ARCHITECTURE.md](SEMANTIC_SNAPSHOT_ARCHITECTURE.md)
(implemented by #1600 / #1598 / #1601).

---

## 1. Quote-Like Operators — Canonical Parser

### Contract

All production consumers of `qw`/`q`/`qq` import lists delegate to the two
canonical functions in `perl-parser-core::syntax::quote`:

- **`parse_quote_operator_content(s, operator)`** — strips the operator prefix,
  trims optional whitespace between the operator and its delimiter (Perl allows
  `qw (a b)`), maps the opening delimiter to its closing counterpart
  (`(→)`, `{→}`, `[→]`, `<→>`; all others self-close), rejects an alphanumeric
  or underscore character in delimiter position (so `qwfoo` → `None`), and
  returns the interior slice.

- **`parse_qw_words(s)`** — wraps the above and additionally splits on
  whitespace, returning `Vec<String>`.

Canonical behaviour:
- `qw(foo bar)` → `["foo", "bar"]`
- `qw[foo bar]` → `["foo", "bar"]`
- `qw/foo bar/` → `["foo", "bar"]`
- `qw (foo bar)` (space before delimiter) → `["foo", "bar"]`
- `qwfoo` → `None` (bareword rejection)

### Owner module

`crates/perl-parser-core/src/syntax/quote.rs`

Functions: `parse_quote_operator_content` (line ~969) and `parse_qw_words`
(line ~1014).

### Consumers

| Consumer | Call site | Notes |
|---|---|---|
| `perl-parser-core::hir::model` | `crates/perl-parser-core/src/hir/model.rs:2626` | Internal HIR lowering |
| `perl-module::import::mod` | `crates/perl-module/src/import/mod.rs:661` | Module import extraction |
| `perl-semantic-analyzer::analysis::import_extractor` | `crates/perl-semantic-analyzer/src/analysis/import_extractor.rs:697` | Thin wrapper, delegates immediately |
| `perl-semantic-analyzer::analysis::index` | `crates/perl-semantic-analyzer/src/analysis/index.rs:205` | Dependency index |
| `perl-workspace::semantic::workspace_import_extractor` | `crates/perl-workspace/src/semantic/workspace_import_extractor.rs:640` | Thin wrapper, delegates immediately |

Internal parser usage (not consumer-facing): `declarations.rs` and
`expressions/quotes.rs` inside `perl-parser-core`.

### Proof

**Conformance matrix (PR #1292)** — `fix(semantic-analyzer): parse all qw/q/qq
delimiter forms in import extractor (#1200) (#1292)`. Verified that all five
delimiter forms parse consistently across the consumer crates.

**Centralization commit (PR #1294)** — `refactor(parser): centralize qw/q/qq
delimiter parsing into a shared helper (#1294)`. This is the commit that created
the canonical functions.

**Seam-proof unit tests** (inline in `import_extractor.rs`, starting at line
~1337): `parse_quote_operator_content_compact_paren`,
`parse_quote_operator_content_compact_bracket`,
`parse_quote_operator_content_compact_brace`,
`parse_quote_operator_content_compact_slash`,
`parse_quote_operator_content_space_before_paren`,
`parse_quote_operator_content_space_before_bracket`.

**qw comment tests**: `crates/perl-parser-core/tests/fix_qw_comment_tests.rs`.

### Known exceptions / specializations

`crates/perl-module/src/resolution/use_lib/extract.rs` has an intentionally
**separate** implementation (`extract_qw_paths`). It is specialized for
`use lib` path extraction, which includes FindBin variable interpolation
(`$FindBin::Bin`, `$Bin`, `${RealBin}`, etc.) and path-join semantics that are
not appropriate in a generic qw parser. This copy is intentional and
**out-of-scope** for the canonical migration.

### Non-goals / future migrations

- `qq` and `q` single-word forms (non-list contexts) use
  `parse_quote_operator_content` with the `"qq"` / `"q"` operator argument;
  no separate wrapper function is needed.
- If `use_lib/extract.rs` ever needs multi-delimiter qw handling beyond its
  current hard-coded forms, it should still keep its own implementation rather
  than calling the generic parser, because the FindBin semantics are structural.

---

## 2. Indirect-Object Ambiguity Disambiguation

### Contract

- `print $hash{key}` — subscripted variable — is **NOT** `indirect_call`.
- `print $hash {key}` (space before brace) — **NOT** `indirect_call` (space
  does not create an indirect-object boundary; `{key}` is still `$hash`'s
  subscript).
- `print $array[0]` — array subscript — **NOT** `indirect_call`.
- `say $+{year}` — named-capture hash subscript — **NOT** `indirect_call`.
- `print $fh "text"` — scalar followed by string literal (no comma) — **IS**
  `indirect_call`.
- `print $fh $x` — scalar followed by bare scalar (no comma) — **IS**
  `indirect_call`.
- `print { $fh } $x` — block-filehandle form — **IS** `indirect_call`.
- `printf $fh "%s", $x` — printf with variable filehandle — **IS**
  `indirect_call`.

### Owner module

`crates/perl-parser-core/src/engine/parser/expressions/calls.rs` (call
classifier) backed by `crates/perl-parser-core/src/engine/parser/expressions/hashes.rs`
(subscript lookahead).

### Consumers

Any LSP or DAP feature that walks the AST looking for `NodeKind::IndirectCall`
variants. Current consumers include `crates/perl-lsp-rs/tests/indirect_call_definition_tests.rs`.

### Proof

**8-case fixture pack** in `crates/perl-parser/tests/parser_regressions.rs`,
function `indirect_object_ambiguity_fixture_pack` (merged PR #1296, commit
`b88852816`).

The 8 cases correspond exactly to:

| Case | Source | Expected |
|---|---|---|
| 1 | `print $hash{key};` | NOT `indirect_call` |
| 2 | `print $hash {key};` | NOT `indirect_call` |
| 3 | `print $array[0];` | NOT `indirect_call` |
| 4 | `say $+{year};` | NOT `indirect_call` |
| 5 | `print $fh "text";` | IS `indirect_call` |
| 6 | `print $fh $x;` | IS `indirect_call` |
| 7 | `print { $fh } $x;` | IS `indirect_call` |
| 8 | `printf $fh "%s", $x;` | IS `indirect_call` |

**Oracle.** `perl -MO=Terse` was used to verify ground-truth classification for
all ambiguous forms. Each case carries an inline comment with the oracle command
and output.

**Earlier fixtures** (also in `parser_regressions.rs`): `print_hash_subscript_not_indirect_object`
and `print_arrow_chain_not_indirect_object` predate PR #1296 and are subsumed
by it.

**Original fix.** The disambiguation logic was introduced in
`fix(parser): disambiguate print with subscripted variables from
indirect-object syntax (#974) (#1214)`, commit `b25d91806`.

### Known exceptions / specializations

The `new ClassName arg` constructor pattern is a separate indirect-call variant
covered by `new_constructor_pattern` and `new_qualified_constructor_indirect_call`
tests in the same file. It is governed by the same `IndirectCall` node kind but
a distinct code path.

### Non-goals / future migrations

- `print STDOUT "text"` (bareword filehandle) is a distinct code path. The
  disambiguation here concerns only scalar-variable filehandles.
- A future `say` disambiguation test could be added as a companion; the
  mechanism is the same (`say` goes through the same call classifier).

---

## 3. Embedded-Code Metadata (`s///e`)

### Contract

A `NodeKind::Substitution` node has `has_embedded_code: bool`. It is `true`
when:

1. The replacement string is evaluated as Perl code via the `e` or `ee`
   modifier (`s/pattern/replacement/e`, `s/a/b/ee`), OR
2. The pattern body contains an inline code block (`(?{...})`).

`has_embedded_code: false` otherwise. The S-expression serializer marks `true`
nodes with `(risk:code)`.

### Owner module

**Field definition.** `crates/perl-ast/src/ast.rs`, `NodeKind::Substitution`
variant (lines ~2041–2056):

```
Substitution {
    expr: Box<Node>,
    pattern: String,
    replacement: String,
    modifiers: String,
    has_embedded_code: bool,   // the governed field
    negated: bool,
}
```

**Setting logic.** Two sites in `crates/perl-parser-core/src/engine/parser/`:
- `expressions/primary.rs` — the `=~` binding-operator path
- `expressions/quotes.rs` — the standalone quote-operator form (`s{}{}e`)

### Consumers

- Security linter: `crates/perl-lsp-rs-core/src/providers/diagnostics/lints/security.rs`
- Any future taint-analysis or injection-risk scanner that walks for
  `has_embedded_code: true` nodes.

### Proof

**Merged PR #1238** — `fix(parser): mark s///e substitution as embedded code
(#975) (#1238)`, commit `7753cc032`.

**Regression test file**: `crates/perl-parser-core/tests/fix_subst_e_has_embedded_code_975.rs`.

Test functions:
- `subst_e_modifier_sets_has_embedded_code` — `s///e` sets true
- `subst_ee_modifier_sets_has_embedded_code` — `s///ee` sets true
- `subst_ge_modifier_sets_has_embedded_code` — `s///ge` sets true
- `subst_no_e_modifier_does_not_set_has_embedded_code` — `s///gr` stays false
- `subst_embedded_code_in_pattern_stays_true` — `(?{...})` in pattern sets true
- `subst_quote_operator_form_e_sets_has_embedded_code` — brace form `s{}{}e`
- `subst_e_sexp_contains_risk_code_marker` — S-expression marker check
- `subst_quote_operator_form_no_e_does_not_set_has_embedded_code`
- `subst_quote_operator_form_ee_sets_has_embedded_code`
- `subst_quote_operator_form_both_embedded_code_and_e_modifier`
- `find_first_substitution_returns_none_for_non_subst_ast`

### Known exceptions / specializations

`NodeKind::Regex` and `NodeKind::Match` also have `has_embedded_code` fields
(for `(?{...})` in patterns), but the `s///e` modifier is only meaningful on
`Substitution`. The same field name is reused for consistency.

### Non-goals / future migrations

- Transliteration (`tr///`) has no `has_embedded_code` field — transliteration
  cannot contain code blocks.
- A future taint-flow analysis would traverse the AST and collect all nodes
  where `has_embedded_code` is true; this contract governs that traversal's
  pre-filter.

---

## 4. NodeKind Classification — Static Category and Flags

### Contract

`crates/perl-ast/src/classification.rs` provides a static, variant-level
classification API for all 69 `NodeKind` variants. Two methods are exposed on
`NodeKind`:

- **`category() -> NodeKindCategory`** — classifies each variant into exactly
  one of: `Program`, `Statement`, `Expression`, `Declaration`, `Scope`,
  `Literal`, `Operator`, `CommentDoc`, `Recovery`, `Unknown`.

- **`flags() -> NodeKindFlags`** — returns a struct with seven boolean flags:
  `executable`, `introduces_scope`, `declares_symbol`, `references_symbol`,
  `contains_children`, `recovery_artifact`, `safe_for_breakpoint`.

Both methods use **exhaustive `match self { ... }` expressions with no wildcard
arm.** Adding a new `NodeKind` variant is a compile-time error until both
matches are extended. This is the drift guard.

**Invariant (enforced by `NodeKindFlags::validate()`):**
`recovery_artifact == true` implies `safe_for_breakpoint == false`.

**Invariant (enforced by `contains_children_matches_for_each_child` test):**
`contains_children` is a structural flag that must be `true` for exactly the
variants that have at least one `Node`-typed field. The authoritative source is
`Node::for_each_child`: building every variant with all child slots populated,
`flags().contains_children == (child_count() > 0)`. A consumer that uses the flag
to skip leaf nodes during traversal relies on this — a false negative silently
drops a variant's children. (Corrected for `String`/`Heredoc`/`Readline`/`Glob`/
`Use`/`No`, which carry no `Node` children, and `VariableDeclaration`/`Untie`/
`Error`, which do.)

### Owner module

`crates/perl-ast/src/classification.rs`

Enum definitions: `NodeKindCategory` (line ~53), `NodeKindFlags` (line ~82).

### Consumers

| Gate | Consumer | Status |
|---|---|---|
| Phase 7 (NOW) | Document-symbols provider, semantic-tokens provider | MAY consume `NodeKindCategory` |
| Phase 8 (ready after #1297) | DAP breakpoint validator | MAY consume `safe_for_breakpoint` as prefilter; MUST apply instance-dependent checks (see §Breakpoint contract below) |

### Proof

**Merged PR #1295** — `feat(perl-ast): NodeKindCategory + NodeKindFlags
classification API (#911) (#1295)`, commit `a01460bd3`.

Test file: `crates/perl-ast/tests/classification_tests.rs` (covers
category exhaustiveness, flags invariant, recovery-artifact implies not-breakpoint).

Variant count: **69 total** (confirmed via `NodeKind::ALL_KIND_NAMES` constant
in `crates/perl-ast/src/ast.rs:2283`).

### Critical boundary: Parser Truth vs Consumer Policy

This is the key documentation this contract must communicate.

**`NodeKindCategory` is ready for Phase 7 consumers now.** Document-symbols
and semantic-tokens providers may use it to filter or classify nodes without
waiting for further research.

**`NodeKindFlags.safe_for_breakpoint` is a variant-level pre-filter, not a DAP
guarantee.** The flag answers: *"Can a breakpoint ever be set on this kind of
node?"* A `true` value means the variant is a candidate for breakpoint
placement. It does **not** mean "always stop here." DAP consumers must
additionally inspect instance-level facts (is this cursor inside a heredoc body?
a POD block? after `__DATA__`?) and verify with the runtime/debugger.

### §Breakpoint and Scope Classification Contract (ratified issue #1297, PR #1452)

**Issue #1297 ratification (ChatGPT-Pro + Perl 5.40.1 debugger probe) is now merged.**
The following table documents all ratified flag values and the instance-dependent rows
that DAP consumers must handle with AST-structure or metadata checks.

**Static (variant-level, no instance check needed):**

| Variant | Flag | Ratified value | Evidence |
|---|---|---|---|
| `Use` | `safe_for_breakpoint` | **`false`** | `use Module LIST` is `BEGIN { require; import }` — compile-time; Perl 5.40.1 probe reports "not breakable". |
| `No` | `safe_for_breakpoint` | **`false`** | `no Module LIST` is `BEGIN { unimport }` — compile-time; Perl 5.40.1 probe reports "not breakable". |
| `Class` | `safe_for_breakpoint` | `true` | `class Foo { }` header line is stoppable in runtime debugger; probe confirms. |
| `Goto` | `safe_for_breakpoint` | `true` | Executable statement before control transfer; stoppable. |
| `Typeglob` | `safe_for_breakpoint` | `false` | Typeglob reference/assignment introduces no lexical scope; not a runtime statement. |

**Instance-dependent (variant flag is a conservative prefilter; DAP consumer must verify):**

| Variant | Flag | Variant-level value | Consumer must check |
|---|---|---|---|
| `Eval` | `introduces_scope` | `true` (prefilter) | Whether `block` child is `NodeKind::Block` — `eval STRING`/`eval EXPR` introduce no static scope. |
| `Package` | `introduces_scope` | `true` (prefilter) | Whether `block.is_some()` — `package Foo;` (no block) creates no lexical scope. |
| `Package` | `safe_for_breakpoint` | `true` (prefilter) | Whether `block.is_some()` — statement form differs from block form at runtime. |
| `PhaseBlock` | `safe_for_breakpoint` | `true` (prefilter) | `phase` field: `BEGIN`/`CHECK`/`UNITCHECK` are compile-time (not stoppable); `END` is stoppable; `INIT` may depend on attach timing. |

Phase 8 DAP breakpoint validator **may now consume `safe_for_breakpoint`** as a prefilter,
but MUST apply the instance-dependent checks in the table above before accepting a
breakpoint request. The prefilter eliminates obvious non-candidates (recovery nodes,
literals, compile-time pragmas); the instance checks handle variant-level ambiguity.

**Naming note.** The flag is currently named `safe_for_breakpoint`. A future
rename or split to something like `can_host_executable_code` vs
`dap_verified_breakpoint` would prevent misuse. This is noted here to prevent
premature consumption with the current name implying a stronger guarantee than
it provides. Any rename must update the `SAFE_FOR_BREAKPOINT_*` test constants
in `crates/perl-ast/tests/classification_tests.rs`.

### Known exceptions / specializations

`NodeKindCategory::CommentDoc` and `NodeKindCategory::Unknown` are reserved
for future use. No variants currently map to either category.

`NodeKindCategory::Recovery` covers: `Error`, `MissingExpression`,
`MissingStatement`, `MissingIdentifier`, `MissingBlock`, `UnknownRest`.

### Non-goals / future migrations

- The `flags!()` macro is a crate-internal helper, not a public API. External
  crates access flags via `NodeKind::flags()` only.
- Mutation of the classification table for new variants is mandatory (compile
  error guard). A PR adding a new `NodeKind` variant must add the corresponding
  arm to both `category()` and `flags()`.

### Non-exhaustive consumer audit (REQUIRED for every new variant)

The exhaustive `match self { ... }` arms in `classification.rs` are the compiler-enforced
drift guard — they catch the obvious case. However, two consumer patterns are **invisible
to the exhaustiveness checker** and must be audited manually whenever a new variant is added:

1. **`if let NodeKind::X { .. } = node` in loops without an else branch** — the loop body
   runs for matched variants and silently skips new variants. The compiler sees no problem.

2. **`_ => { /* no children */ }` wildcard arms** in traversal and extraction functions
   (especially `visit_children`, semantic-token dispatch, symbol extractors, declaration
   mappers) — new variants fall into the no-op arm. The match remains exhaustive; the new
   variant is silently dropped.

In PR #1457 (`NodeKind::NestedVariableList`, issue #1362), both patterns caused three
silent consumer drops: the `node_analysis` `if let` loop (no semantic tokens or hover for
inner variables), the `variable_decl_from_node` declaration mapper (no workspace symbols,
breaking go-to-definition and rename), and the `visit_children` wildcard arm (no reference
tracking). Deep-review caught and fixed all three in commit `c5c8f6bf8`.

**The required audit for any new `NodeKind` variant:**

```
grep -r "if let NodeKind::" crates/ -- look for loops with no else
grep -r "_ =>" crates/perl-semantic-analyzer crates/perl-symbol crates/perl-workspace -- look for wildcard arms in traversal/extraction
```

For each hit: add an explicit arm or else branch for the new variant. Write an integration
test asserting that semantic tokens, hover, go-to-definition, and workspace symbols all
return results for a Perl snippet using the new construct.

**See**: [docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md PARSER-5](SUBSYSTEM_HAZARD_DEFAULTS.md)
and [docs/learnings/2026-06-nodekind-variant-silent-consumer-drop.md](../learnings/2026-06-nodekind-variant-silent-consumer-drop.md).

---

## 5. Recovery Nodes — Decision Record

### What exists

`NodeKind` defines six recovery-related variants (in `NodeKind::RECOVERY_KIND_NAMES`,
`crates/perl-ast/src/ast.rs:2355`):

| Variant | In S-expression | Classification |
|---|---|---|
| `Error` | `(error ...)` | Recovery |
| `MissingExpression` | `(missing_expression)` | Recovery |
| `UnknownRest` | `(unknown_rest ...)` | Recovery |
| `MissingStatement` | `(missing_statement)` | Recovery |
| `MissingIdentifier` | `(missing_identifier)` | Recovery |
| `MissingBlock` | `(missing_block)` | Recovery |

### Which are actually emitted by the parser

**Emitted today:**

- `Error` — emitted via `NodeKind::Error { .. }` in the parser error path.
- `MissingExpression` — emitted in
  `crates/perl-parser-core/src/engine/parser/helpers.rs:884`:
  `Node::new(NodeKind::MissingExpression, ...)`.
- `UnknownRest` — emitted (corpus evidence: 1 occurrence in clean corpus per
  issue #915).

**NOT emitted today — reserved for future error-recovery:**

- `MissingStatement` — defined in AST, handled in `hir/lower.rs` and consumer
  match arms, but the parser never constructs one. Confirmed by searching
  `crates/perl-parser-core/src/` — no `Node::new(NodeKind::MissingStatement,
  ...)` site exists outside test files.
- `MissingIdentifier` — same situation: defined, matched in consumers, never
  constructed by the parser.
- `MissingBlock` — same situation.

Construction of all three exists only in unit test files
(`crates/perl-ast/tests/`, `crates/perl-parser/tests/`), where they are
constructed directly to exercise classification, traversal, and S-expression
output.

**Correction to issue #915 description.** Issue #915 states these three are
"emitted by `parse_with_recovery()`", but the implementation of
`parse_with_recovery()` (in `crates/perl-parser-core/src/engine/parser/mod.rs:416`)
simply calls `parse()` and wraps the result — it does not emit any of the three
variants. The issue description reflects the intended future behavior, not the
current code. The code is the authoritative record.

### Decision

These three variants are **not fixture-coverable from the current parser**
because the parser never emits them. They are reserved for a future
error-recovery pass that would emit `MissingStatement` when a statement is
expected but EOF/garbage is found, `MissingIdentifier` on a malformed
declaration, and `MissingBlock` when a block is expected but missing.

The CPAN corpus coverage engine allowlists them (`actionable_never_seen = 0`)
so the corpus gate does not fail. This allowlisting is honest, not a workaround.

**Do not create fake 69/69 coverage** by adding parser calls that emit these
variants — that would mean either: (a) real parse errors now produce these
nodes in production output (a behavior change), or (b) unreachable code is
added (dead code). Both are wrong.

### Open issue

**Issue #915** — `nodekind: 3 recovery variants never seen in corpus (66/69)
— add error-recovery fixtures to reach 69/69 honestly`. This issue is open and
tracks the decision to close it by wiring the parser's recovery paths to emit
these variants when the recovery implementation is ready.

Relevant code:
- Coverage engine: `xtask/src/tasks/corpus_audit/nodekind_analysis.rs`
- Corpus gate test: `crates/perl-parser/tests/corpus_nodekind_coverage_test.rs:36`
- Allowlist logic: `nodekind_analysis.rs:96-108`

### Non-goals / future migrations

- When error-recovery is extended to emit `MissingStatement`, `MissingIdentifier`,
  or `MissingBlock`, issue #915 can be closed and the allowlist entry removed.
  The corpus gate will then enforce real fixture coverage.
- Any such wiring must also add corpus fixtures (malformed Perl inputs that
  trigger the recovery path), not just unit tests that construct the nodes
  directly.

---

## 6. Boundary Detection — Consumer Responsibility

### Contract

Three classes of source region are **non-executable positional boundaries**:
**POD blocks**, **heredoc bodies**, and **data sections** (`__DATA__` / `__END__`).
The parser does not enforce them as *execution* boundaries, and **the degree of AST
support varies**:

- **data sections** are a first-class `NodeKind::DataSection { marker, body }` node,
- **heredoc bodies** are recorded as `Heredoc.body_span` metadata on the
  `NodeKind::Heredoc` node, and
- **POD has no AST node at all** — it is purely consumer-detected.

Any consumer that filters nodes or source positions by executability — a breakpoint
validator, a formatter preserve-gate, a completion-context filter, a semantic-token
painter — MUST apply the relevant positional check for its own domain, layered on
top of the variant-level `NodeKind` classification of §4. Where an authoritative AST
representation exists (the `DataSection` node, the `Heredoc.body_span` metadata),
consumers **SHOULD prefer it** over reimplementing a source scan; a raw positional
scan is the correct path only for POD and for pre-parse callers that have no AST yet.
The classification API is explicit about the positional-layering requirement
(`crates/perl-ast/src/classification.rs:5-7`):

```
//! Consumers that need positional facts (is this
//! inside a heredoc body? inside POD? after `__DATA__`?) must layer those
//! checks on top using their own positional knowledge.
```

**Canonical boundary rules:**

1. **POD blocks.** A POD command paragraph begins with `=<letter>` — an `=`
   immediately followed by an ASCII letter — at **column 0** (the first byte of a
   line, after a newline), and runs until a line that is exactly `=cut`, or to
   **EOF** if no `=cut` is found. Per `perlpodspec` a command paragraph must match
   `\A=[a-zA-Z]`; a leading-whitespace `  =head1` is a *verbatim* paragraph, not a
   POD command. POD content is never executable. (Perl-spec column-0 and EOF-fallback
   facts verified against `perlpodspec`/`perlpod`, issue #1627 research-verifier.)

2. **Heredoc bodies.** The source text between a heredoc's opening `<<DELIM` line
   and its terminator. The parser records this as
   `body_span: Option<SourceLocation>` on `NodeKind::Heredoc` (`SourceLocation` is a
   half-open `[start, end)` byte span), populated post-lexically by
   `drain_pending_heredocs`; an empty body yields `None`. Consumers MAY use this AST
   metadata directly OR scan the source.

3. **Data sections.** From a `__DATA__` or `__END__` marker at **column 0** to EOF;
   everything after the marker is data, not code. The parser models this as a
   first-class `NodeKind::DataSection { marker, body }` node
   (`crates/perl-ast/src/ast.rs:2158`), classified `NodeKindCategory::Declaration`
   with `executable = false` and `safe_for_breakpoint = false`
   (`crates/perl-ast/src/classification.rs:256,836`). AST and semantic consumers
   **should read this node directly** rather than rescanning the source. The lexer's
   `find_data_marker_byte_lexed` is the **pre-parse** path: it recognizes the marker
   as a `TokenType::DataMarker` token at the source level, for callers that must
   split code from data *before* (or without) a full parse.

### Owner module

**Distributed — there is no single owner.** Each subsystem owns boundary detection
for its own domain; this section is the canonical rule set they must agree on. The
parser does **not** enforce these regions as *execution* boundaries at parse time,
but it does provide authoritative AST representation for two of the three (the
`NodeKind::DataSection` node and `Heredoc.body_span`); POD is purely
consumer-detected. Consumers prefer the AST representation where it exists and fall
back to positional source scans otherwise.

### Consumers

| Consumer | Boundary | Call site | Detection method | Column-0 enforced? | EOF fallback? |
|---|---|---|---|---|---|
| DAP breakpoint validator | POD | `crates/perl-dap/src/breakpoint/validator.rs:144` (`find_pod_regions`) + `:175` (`is_pod_directive`) | line scan; opens on any `=<letter>`, closes on a line equal to `=cut` | **yes** (strict) | yes (`:166`) |
| DAP breakpoint validator | heredoc body | `crates/perl-dap/src/breakpoint/validator.rs:263` (`is_inside_heredoc_interior_node`) | reads `Heredoc.body_span` AST metadata | n/a | n/a |
| Native formatter | POD | `crates/perl-lsp-perltidy/src/native.rs:1655` (`literal_preserve_region`) + `:1817` (`is_pod_start`) | line scan; `trim_start()` then a **closed set** of standard directives | **no** (lenient) | implicit (scans every line) |
| Native formatter | data section | `crates/perl-lsp-perltidy/src/native.rs:1661` | exact `__DATA__`/`__END__` match after `trim_start().trim_end()` | **no** (lenient) | n/a |
| Native formatter | heredoc / regex / subst / qw | `crates/perl-lsp-perltidy/src/native.rs:1754` (`token_literal_preserve_region_overlapping`) | token byte-span overlap (`token.start < range_end && token.end > range_start`) | n/a | n/a |
| Parser → AST / semantic consumers | data section | `crates/perl-parser-core/src/engine/parser/declarations.rs:1164` (`parse_data_section`) → `NodeKind::DataSection` | authoritative AST node, classified `Declaration` / non-executable | yes (marker is column-0) | n/a |
| Lexer (pre-parse) | data marker | `crates/perl-lexer/src/tokenizer/util.rs:22` (`find_data_marker_byte_lexed`) + `:14` (`marker_is_unindented_line_start`) | lexes to a `DataMarker` token, then verifies column 0 | **yes** (strict) | n/a |

### Critical boundary: canonical rule vs. consumer risk posture

This is the key thing this contract communicates. **Consumers intentionally diverge
in strictness, and the divergence is governed by each consumer's risk posture — it
is by design, not drift:**

- A **breakpoint validator** must not *wrongly* classify executable code as POD: a
  false positive silently drops a valid breakpoint. It therefore enforces the
  strict, spec-correct **column-0** rule (`is_pod_directive` is fed a line trimmed of
  trailing CR only, so leading whitespace disqualifies the `=` directive).

- A **formatter preserve-gate** is *false-positive-safe*: over-detecting POD only
  makes it bail out and skip reformatting, which never corrupts the document. It
  therefore uses a lenient `trim_start()` check. For the formatter the **dangerous**
  direction is a false *negative* (missing real POD and reflowing it).

So the strictness asymmetry between DAP (strict column-0) and the formatter
(lenient) is the correct engineering choice for each, even though it means the two
detectors do not agree on an indented `  =head1`. The canonical rule (column-0,
`\A=[a-zA-Z]`) is the spec ground-truth; the formatter's leniency is a deliberate
conservative superset.

### Proof

**Issue #1627** — `docs(parser-contracts): add Consumer Boundary Detection contract
for POD/__DATA__/__END__/heredoc`. Perl-spec claims (column-0 POD start, `=cut`/EOF
close, `__DATA__`/`__END__` data semantics) verified by research-verifier against
`perlpodspec`, `perlpod`, and `perldata`.

**DAP POD-region tests** (inline in `validator.rs`): `test_is_pod_directive_basic`
(`:679`), `test_pod_without_cut_extends_to_eof` (`:514`), `test_code_after_pod_is_executable`
(`:503`), `test_multiple_pod_sections` (`:531`), `test_find_pod_regions_unclosed` (`:720`).

**Formatter preserve-gate tests** (inline in `native.rs`):
`literal_preserve_region_detects_perl_constructs_that_must_not_be_reflowed` (`:1912`),
`byte_span_for_line_range_returns_correct_byte_interval` (`:1927`),
`literal_preserve_region_for_range_ignores_pod_outside_range` (`:1973`),
`literal_preserve_region_for_range_detects_pod_inside_range` (`:1984`),
`token_literal_preserve_region_overlapping_detects_substitution_in_range` (`:2058`).
The full set is the 7-test integration file noted in §7 plus these inline unit tests.

**Lexer data-marker tests** (inline in `util.rs`): `test_find_data_marker_lexed`
(`:75`), `test_find_data_marker_handles_crlf_and_leading_whitespace` (`:100`, asserts
indented markers are rejected and CRLF handled), `test_find_data_marker_ignores_markers_inside_heredoc_and_pod`
(`:148`).

### Worked example

```perl
my $x = 1;
=pod
This is documentation
=cut
my $y = 2;
```

Every consumer must agree that lines 2–4 (`=pod` … `=cut`) are a non-executable POD
region: the DAP breakpoint validator must reject breakpoints there
(`is_inside_pod_region`, `validator.rs:187`), the formatter must not reflow them
(`literal_preserve_region` returns `Some("POD")`), and a completion-context filter
must suppress completions inside them. Lines 1 and 5 (`my $x` / `my $y`) are
executable and outside the boundary.

### Known exceptions / specializations

- The formatter's `is_pod_start` (`native.rs:1817`) matches a **closed set** of
  standard directives (`=pod`, `=head1`–`=head4`, `=over`, `=item`, `=back`,
  `=begin`, `=end`, `=for`, `=encoding`, `=cut`), whereas the DAP `is_pod_directive`
  matches **any** `=<ascii-letter>`. A nonstandard directive such as `=custom` opens
  a POD block to DAP but is not recognized by the formatter's line check (it would
  fall through to the token-based path). This is a second, narrower divergence on top
  of the indentation one.

- `find_data_marker_byte_lexed` ignores `__DATA__`/`__END__` substrings embedded in
  heredocs, POD, or string literals, because it lexes rather than substring-matches
  (`test_find_data_marker_ignores_markers_inside_heredoc_and_pod`). A naive
  substring scan would not have this property.

### Non-goals / future migrations

- **The parser does not emit a `PodBlock` node.** POD is not executable and is
  detected by consumers positionally; issue #1627 rejected adding a POD syntax node.
  This is the *opposite* of data sections, which **are** a first-class
  `NodeKind::DataSection` node, and of heredocs, whose bodies are recorded as
  `Heredoc.body_span` metadata — POD is the sole boundary type of the three with no
  AST representation.

- **No centralized boundary-detection helper exists today**, and adding one is out of
  scope for this contract. If a fourth consumer needs POD detection, consider
  extracting the canonical column-0 POD scan into a shared helper — but only if the
  new consumer's risk posture matches the strict (DAP) variant; the formatter's
  lenient variant must stay lenient for the safety reason above.

- **Candidate follow-up:** reconcile the formatter's directive-set narrowness (closed
  list vs. any `=<letter>`) with the canonical rule, or document the per-consumer
  rationale inline at each call site. The completion-context boundary suppression
  tracked by issue #1624 is a new consumer that MUST implement detection per this
  contract.

---

## 7. Formatting Preserve Gates

### Contract

The native Perl formatter (`NativeFormatter` in
`crates/perl-lsp-perltidy/src/native.rs`) applies preserve-sensitive construct
checks differently depending on the operation:

- **`format_document`** — validates preserve-sensitive constructs
  (regex/substitution/transliteration/qw/heredoc/POD) across the **full
  document**. Bails conservatively if any such construct is found.

- **`format_range`** — validates preserve-sensitive constructs **only within
  the requested line range** (not the full document). A token is considered
  overlapping if `token.start < range_byte_end && token.end > range_byte_start`.
  A token that starts before the range but ends inside it is conservatively
  treated as an overlap (bails). A token entirely outside the range is ignored.

- **Post-format parse check.** After formatting, both operations run
  `validate_parse_only` on the output to catch formatting-introduced parse
  errors. This is a parse-error-only check (not a preserve check) applied to
  the formatted result.

### Owner module

`crates/perl-lsp-perltidy/src/native.rs`

Key functions:
- `format_document` (line ~164) — full-document preserve + format
- `format_range` (line ~191) — range-scoped preserve + format
- `literal_preserve_region_for_range` (line ~1692) — range-scoped preserve
  check: returns `Some(reason)` to bail, `None` to proceed
- `byte_span_for_line_range` (line ~1725) — converts a `TextRange` (line
  numbers) to `(byte_start, byte_end)`
- `token_literal_preserve_region_overlapping` (line ~1754) — checks for
  tokens overlapping a byte span
- `token_literal_preserve_region` (line ~1794) — full-document preserve check
  (used by `format_document`)
- `validate_parse_only` (line ~69) — parse-error-only check, no preserve check

### Consumers

- LSP `textDocument/formatting` handler (calls `format_document`)
- LSP `textDocument/rangeFormatting` handler (calls `format_range`)

### Proof

**Merged PR #1314** — `fix(formatting): scope range-format preserve-gate to the
requested range (#1313)`.

Test file: `crates/perl-lsp-perltidy/tests/native_formatter_parse_gate_tests.rs`

7 integration tests verify that `format_range` on a clean range succeeds even
when the rest of the document contains complex constructs that would cause
`format_document` to bail. 6 inline unit tests in `native.rs` cover the
`byte_span_for_line_range` helper and overlap logic directly.

### Known exceptions / specializations

The overlap check uses conservative byte-span intersection, not syntax-tree
intersection. A multi-line heredoc that starts on a line before the requested
range but ends inside it is treated as overlapping and causes a bail-out. This
is intentional: the formatter cannot safely split or partially format constructs
that span line boundaries.

### Non-goals / future migrations

- A future enhancement could use the full parsed AST to determine exact
  construct boundaries rather than token byte spans. Until then, the token-span
  approach is the correct conservative approximation.
- `format_document` behavior is intentionally unchanged by PR #1314 — only
  `format_range` was modified to be range-scoped.

---

## Cross-Reference

| Contract | Owner crate | Key test file | Governing PR |
|---|---|---|---|
| Quote-like canonical | `perl-parser-core` | `crates/perl-parser-core/tests/fix_qw_comment_tests.rs` | #1294, #1292 |
| Indirect-object ambiguity | `perl-parser-core` | `crates/perl-parser/tests/parser_regressions.rs` | #1296, #1214 |
| Embedded code (`s///e`) | `perl-ast`, `perl-parser-core` | `crates/perl-parser-core/tests/fix_subst_e_has_embedded_code_975.rs` | #1238 |
| NodeKind classification | `perl-ast` | `crates/perl-ast/tests/classification_tests.rs` | #1295 |
| NodeKind non-exhaustive consumer audit | `perl-semantic-analyzer`, `perl-symbol`, `perl-workspace` | grep `if let NodeKind::` + `_ =>` wildcard arms | #1457 deep-review |
| Recovery node decision | `perl-ast` | (not fixture-coverable — never emitted) | open #915 |
| Boundary detection (consumer responsibility) | distributed (`perl-dap`, `perl-lsp-perltidy`, `perl-lexer`) | inline tests in `validator.rs`, `native.rs`, `tokenizer/util.rs` | #1627 |
| Formatting preserve gates | `perl-lsp-perltidy` | `crates/perl-lsp-perltidy/tests/native_formatter_parse_gate_tests.rs` | #1314 |

**DAP contracts** are in a separate index: [docs/reference/DAP_CONTRACTS.md](DAP_CONTRACTS.md)
(variablesReference wire-band codec, governing PRs #1430 / #1444, open #1445).
