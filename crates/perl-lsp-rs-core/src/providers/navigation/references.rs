//! Find-all-references functionality for symbol usage analysis in Perl scripts
//!
//! This module provides comprehensive reference finding capabilities for Perl script
//! development within the LSP workflow. Enables developers to quickly locate all
//! usage sites of variables, functions, and packages across Perl code.
//!
//! # LSP Workflow Integration
//!
//! - **Parse**: Identifies symbol definitions during Perl script parsing
//! - **Index**: Supports refactoring and symbol standardization
//! - **Navigate**: Analyzes variable flow and dependencies in Perl code
//! - **Complete**: Enables reference highlighting and navigation in editors
//! - **Analyze**: Powers workspace-wide symbol usage tracking
//!
//! # Usage Examples
//!
//! ```rust,ignore
//! use perl_parser_core::{Parser, Node};
//! use perl_lsp_providers::ide::lsp_compat::references::find_references_single_file;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let script = "my $count = 0; $count++; print $count;";
//! let mut parser = Parser::new(script);
//! let ast = parser.parse()?;
//!
//! // Find all references to $count
//! if let Some(refs) = find_references_single_file(&ast, 3) { // Position of first $count
//!     println!("Found {} references to $count", refs.len());
//!     for (start, end) in refs {
//!         println!("Reference at {}-{}: {}", start, end, &script[start..end]);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use perl_parser_core::ast::{Node, NodeKind};
use perl_parser_core::qualified_name::split_qualified_name;

// ─────────────────────────────────────────────────────────────────────────────
// PIR-A Shadow wiring
// ─────────────────────────────────────────────────────────────────────────────
//
// `SHADOW_WIRING_MODE` controls whether the PIR-A promotion path runs beside
// the legacy provider. Shadow mode is behavior-preserving: the legacy result is
// always returned to the caller unchanged. Shadow mode ON for burn-in: it
// accumulates per-request comparison receipts (emitted as structured tracing
// events) without changing any user-visible behavior.
//
// Flip criteria:
//   Shadow → PromoteExact: ops + human sign-off after scorecard shows
//     `extra_in_compiler == 0` across the full set1 fixture set for at least
//     one complete CI green run post PR2 merge (spec #2635 precondition).
//   Shadow → Off: rollback; no criteria needed (safe at any time).
use super::references_pir_shadow::{PromotionMode, ReferenceOptions, references_pir_promote};

/// Identity byte-offset mapper for `references_pir_promote`.
///
/// Required by `references_pir_promote`'s signature (consumed in `PromoteExact`
/// mode) but not invoked in `Shadow` mode — the Shadow arm works at byte-offset
/// granularity via `shadow_references_with_pir` and never calls this mapper.
/// Kept as a free function so the coverage tool can track it independently of
/// the call site that constructs the closure.
fn identity_byte_mapper(start: usize, end: usize) -> lsp_types::Range {
    lsp_types::Range {
        start: lsp_types::Position { line: 0, character: start as u32 },
        end: lsp_types::Position { line: 0, character: end as u32 },
    }
}

/// Shadow-wiring promotion mode for the same-file references path.
///
/// `Shadow` is the burn-in default: PIR-A runs beside the legacy provider and
/// emits a comparison receipt via `tracing::debug!` (target:
/// `"pir_shadow_receipt"`), but the legacy result is always returned unchanged.
/// Flip this to `PromoteExact` after human sign-off (see module comment).
const SHADOW_WIRING_MODE: PromotionMode = PromotionMode::Shadow;

/// Library-only PIR-A shadow-comparison wrapper for reference finding (dark infrastructure).
///
/// This is a **dark** function — it has no production caller in `perl-lsp-rs`. It is
/// library-only infrastructure for the PIR-A shadow-comparison burn-in path. The
/// actually-wired `textDocument/references` same-file entry point is
/// `SemanticAnalyzer::find_all_references` in `crates/perl-lsp-rs/src/runtime/language/references.rs:594`.
///
/// Future promotion to production (replacing the legacy provider) is gated on the
/// scorecard preconditions in the module docstring and a wiring decision; see issue #2635.
///
/// This function:
///
/// 1. Calls `find_references_single_file` (legacy result — scope-blind).
/// 2. If the cursor is on a `Variable` node, builds the PIR-A
///    `LexicalExtractorReceipt` from `source` and calls
///    `references_pir_promote(SHADOW_WIRING_MODE, ...)`, which emits a
///    [`PirShadowCompareReceipt`] as a structured `tracing::debug!` event.
/// 3. Returns the **legacy result unchanged** — no user-visible behavior change.
///
/// The `source` parameter is the full source text of the file, needed to build
/// the PIR-A receipt (the legacy `find_references_single_file` only needs the
/// AST, but the PIR path needs the source for `parse_with_recovery`).
///
/// [`PirShadowCompareReceipt`]: crate::providers::navigation::references_pir_shadow::PirShadowCompareReceipt
pub fn find_references_with_pir_shadow(
    ast: &Node,
    offset: usize,
    source: &str,
) -> Option<Vec<(usize, usize)>> {
    // ── Step 1: compute the legacy result ────────────────────────────────────
    let legacy_result = find_references_single_file(ast, offset)?;

    // ── Step 2: run the PIR-A shadow path for Variable nodes only ────────────
    // Subroutine references are out of scope for the same-file lexical extractor
    // (the PIR-A extractor handles lexical variables, not subs). Skip the shadow
    // path for non-Variable nodes — the legacy result is returned unchanged.
    let needle = find_node_at_offset(ast, offset)?;
    if let NodeKind::Variable { sigil, name } = &needle.kind {
        let target_sigil = sigil.as_str();
        let target_name = name.as_str();

        // Build the PIR-A receipt. `parse_with_recovery` is used (not `parse`)
        // because `lower_ast` and `extract_lexical_facts` require the full
        // recovery output. Body 0 is the program-root body — the same-file path
        // covers exactly this body.
        let pir_receipt = {
            use perl_parser_core::{Parser, hir::lower_ast, pir::extract_lexical_facts};
            let mut parser = Parser::new(source);
            let output = parser.parse_with_recovery();
            let hir = lower_ast(&output.ast);
            extract_lexical_facts(&hir)
        };

        let opts = ReferenceOptions { include_declaration: true };

        // `references_pir_promote` in Shadow mode: evaluates the PIR candidate,
        // builds the `PirShadowCompareReceipt` via `shadow_references_with_pir`,
        // emits it as a structured `tracing::debug!` event, and returns
        // `LegacyFallback` — the legacy result is preserved.
        //
        // `identity_byte_mapper` is required by the signature (consumed only in
        // `PromoteExact` mode); Shadow never calls it. It is a named free
        // function (not an inline closure) so coverage tracks it independently.
        let _outcome = references_pir_promote(
            SHADOW_WIRING_MODE,
            target_sigil,
            target_name,
            &pir_receipt,
            &legacy_result,
            0, // body 0 = program-root body for same-file lexical scope
            &identity_byte_mapper,
            opts,
        );
        // _outcome is LegacyFallback — the legacy_result is embedded in it, but
        // we already have legacy_result directly. No need to unwrap _outcome.
    }

    // ── Step 3: return the legacy result unchanged ────────────────────────────
    Some(legacy_result)
}

/// Return (start_offset, end_offset) for same-file references
pub fn find_references_single_file(ast: &Node, offset: usize) -> Option<Vec<(usize, usize)>> {
    let needle = find_node_at_offset(ast, offset)?;

    // Determine target "identity"
    let (want_kind, want_pkg, want_name, want_sigil) = match &needle.kind {
        NodeKind::Variable { sigil, name } => {
            let sigil_char = sigil.chars().next();
            ("var", "main".to_string(), name.clone(), sigil_char)
        }
        NodeKind::FunctionCall { name, .. } => {
            let (pkg, bare) = split_qualified_name(name);
            let pkg = pkg.unwrap_or("main").to_string();
            let bare = bare.to_string();
            ("sub", pkg, bare, None)
        }
        NodeKind::Subroutine { name: Some(name), .. } => {
            let (pkg, bare) = split_qualified_name(name);
            let pkg = pkg.unwrap_or("main").to_string();
            let bare = bare.to_string();
            ("sub", pkg, bare, None)
        }
        _ => return None,
    };

    let mut out = Vec::new();

    fn walk(
        node: &Node,
        out: &mut Vec<(usize, usize)>,
        want_kind: &str,
        want_pkg: &str,
        want_name: &str,
        want_sigil: Option<char>,
    ) {
        let location = &node.location;
        match &node.kind {
            NodeKind::Variable { sigil, name } if want_kind == "var" => {
                let sig_char = sigil.chars().next();
                if sig_char == want_sigil && name == want_name {
                    out.push((location.start, location.end));
                }
            }
            NodeKind::FunctionCall { name, .. } if want_kind == "sub" => {
                let (pkg, bare) = split_qualified_name(name);
                let pkg = pkg.unwrap_or("main");
                if bare == want_name && pkg == want_pkg {
                    out.push((location.start, location.end));
                }
            }
            NodeKind::Subroutine { name: Some(name), .. } if want_kind == "sub" => {
                let (pkg, bare) = split_qualified_name(name);
                let pkg = pkg.unwrap_or("main");
                if bare == want_name && pkg == want_pkg {
                    out.push((location.start, location.end));
                }
            }
            _ => {}
        }

        // Walk children
        for ch in get_node_children(node) {
            walk(ch, out, want_kind, want_pkg, want_name, want_sigil);
        }
    }

    walk(ast, &mut out, want_kind, &want_pkg, &want_name, want_sigil);
    Some(out)
}

fn find_node_at_offset(node: &Node, offset: usize) -> Option<&Node> {
    if offset < node.location.start || offset > node.location.end {
        return None;
    }

    // Check children first for more specific match
    let children = get_node_children(node);
    for child in children {
        if let Some(found) = find_node_at_offset(child, offset) {
            return Some(found);
        }
    }

    // If no child contains the offset, return this node
    Some(node)
}

fn get_node_children(node: &Node) -> Vec<&Node> {
    node.children()
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_tdd_support::{must, must_some};

    fn parse(source: &str) -> Node {
        let mut parser = Parser::new(source);
        must(parser.parse())
    }

    #[test]
    fn qualified_sub_declaration_found_in_references() {
        // `sub Foo::bar` stores name as "Foo::bar"; the walk must split it to
        // match against the bare "bar" and package "Foo" extracted at lookup
        // time.  Before the fix, name == want_name compared "Foo::bar" to "bar"
        // and the subroutine declaration was silently dropped from results.
        let source = "sub Foo::bar { 1 } Foo::bar();";
        let ast = parse(source);
        // Cursor at position 4 sits on the 'F' in `sub Foo::bar { ... }`,
        // which resolves to the Subroutine node whose name is "Foo::bar".
        let refs = find_references_single_file(&ast, 4);
        assert!(refs.is_some(), "should return Some for a known sub name");
        let refs = refs.unwrap();
        // Must include both the declaration and the call site
        assert!(refs.len() >= 2, "expected at least 2 references, got {}", refs.len());
    }

    #[test]
    fn bare_sub_declaration_still_found() {
        let source = "sub greet { } greet();";
        let ast = parse(source);
        let refs = find_references_single_file(&ast, 4);
        assert!(refs.is_some());
        let refs = refs.unwrap();
        assert!(refs.len() >= 2, "expected declaration + call, got {}", refs.len());
    }

    #[test]
    fn sub_in_different_package_not_confused() {
        // A subroutine named `bar` in package `Other` must NOT appear when
        // searching for `Foo::bar`.
        let source = "sub Foo::bar { 1 } sub Other::bar { 2 } Foo::bar();";
        let ast = parse(source);
        let refs = find_references_single_file(&ast, 4);
        assert!(refs.is_some());
        let refs = refs.unwrap();
        // Should find `sub Foo::bar` and the call `Foo::bar()`, but NOT `sub Other::bar`
        for &(start, end) in &refs {
            let slice = &source[start..end];
            assert!(
                !slice.contains("Other"),
                "Other::bar must not appear in results for Foo::bar, but got: {slice:?}"
            );
        }
    }

    // ── PIR-A Shadow wiring tests ─────────────────────────────────────────────

    fn parse_with_recovery(source: &str) -> Node {
        let mut parser = Parser::new(source);
        parser.parse_with_recovery().ast
    }

    /// Behavior-preserving: `find_references_with_pir_shadow` returns the exact
    /// same ranges as `find_references_single_file` for a variable reference.
    ///
    /// This is the primary guarantee: Shadow mode NEVER changes user-visible output.
    #[test]
    fn shadow_wiring_returns_same_ranges_as_legacy_for_variable() {
        let source = "my $x = 1;\nprint $x;\n$x = 10;\n";
        let ast = parse_with_recovery(source);

        // Cursor on first `$x` (byte 3).
        let legacy = find_references_single_file(&ast, 3);
        let shadow = find_references_with_pir_shadow(&ast, 3, source);

        assert_eq!(
            legacy, shadow,
            "shadow wiring must return identical ranges to legacy for $x; \
             legacy={legacy:?}, shadow={shadow:?}"
        );
        // Sanity: we got at least 2 sites (decl + 2 uses).
        let ranges = must_some(shadow);
        assert!(ranges.len() >= 2, "expected >=2 $x sites, got {ranges:?}");
    }

    /// Behavior-preserving: `find_references_with_pir_shadow` returns the exact
    /// same ranges as `find_references_single_file` for a subroutine reference.
    ///
    /// Subroutine nodes are out of scope for the PIR lexical extractor. The shadow
    /// path must skip the PIR evaluation for subs and still return the legacy result.
    #[test]
    fn shadow_wiring_returns_same_ranges_as_legacy_for_subroutine() {
        let source = "sub greet { } greet();";
        let ast = parse_with_recovery(source);

        let legacy = find_references_single_file(&ast, 4);
        let shadow = find_references_with_pir_shadow(&ast, 4, source);

        assert_eq!(
            legacy, shadow,
            "shadow wiring must return identical ranges to legacy for sub greet; \
             legacy={legacy:?}, shadow={shadow:?}"
        );
    }

    /// Behavior-preserving: `find_references_with_pir_shadow` returns `None` in the
    /// same cases as `find_references_single_file` (cursor not on a known symbol).
    #[test]
    fn shadow_wiring_returns_none_when_legacy_returns_none() {
        let source = "my $x = 1;\n";
        let ast = parse_with_recovery(source);

        // Cursor on whitespace (byte 10 = newline) — not on any symbol.
        let legacy = find_references_single_file(&ast, 10);
        let shadow = find_references_with_pir_shadow(&ast, 10, source);

        assert_eq!(legacy, shadow, "shadow must agree with legacy on None result");
    }

    /// Shadow path ran: verify that the shadow wiring actually runs the PIR path
    /// by checking its output for a multi-scope fixture where compiler and legacy
    /// disagree (scope-narrowing case: inner `$x` is excluded by compiler but
    /// included by legacy scope-blind walk).
    ///
    /// This test does NOT assert the PIR receipt content (it's emitted as a
    /// tracing event, not returned). It asserts:
    ///   - `find_references_with_pir_shadow` returns the legacy result (behavior
    ///     preserved — includes all 4 `$x` occurrences, both scopes).
    ///   - The call completes without error (PIR path ran successfully).
    #[test]
    fn shadow_wiring_completes_on_multi_scope_source() {
        // F1 from references_promotion_test: outer + inner `$x`.
        // Legacy returns 4 (all $x); PIR-A returns 2 (outer only).
        // Shadow wiring must return 4 (legacy) — the scope-narrowing evidence
        // is captured in the tracing receipt, not surfaced to the caller.
        const F1_SOURCE: &str = "my $x = 1;\n{\n    my $x = 2;\n    print $x;\n}\nprint $x;\n";
        let ast = parse_with_recovery(F1_SOURCE);

        let legacy = find_references_single_file(&ast, 3);
        let shadow = find_references_with_pir_shadow(&ast, 3, F1_SOURCE);

        assert_eq!(
            legacy, shadow,
            "shadow wiring must return the legacy (4-site, scope-blind) result; \
             legacy={legacy:?}, shadow={shadow:?}"
        );

        let ranges = must_some(shadow);
        assert_eq!(
            ranges.len(),
            4,
            "legacy scope-blind result includes all 4 $x sites; got {ranges:?}"
        );
    }

    /// Shadow wiring handles non-ASCII (multi-byte) source without panic.
    ///
    /// Regression guard: the PIR path builds a receipt from source bytes; verify
    /// that a source with a multi-byte character does not panic or corrupt ranges.
    #[test]
    fn shadow_wiring_handles_non_ascii_source() {
        // é is 2 UTF-8 bytes; $x is after it on line 0.
        let source = "my $x = \"caf\u{e9}\";\nprint $x;\n";
        let ast = parse_with_recovery(source);

        let legacy = find_references_single_file(&ast, 3);
        let shadow = find_references_with_pir_shadow(&ast, 3, source);

        assert_eq!(
            legacy, shadow,
            "shadow must agree with legacy on non-ASCII source; \
             legacy={legacy:?}, shadow={shadow:?}"
        );
        // Must find at least 2 sites (decl + read).
        let ranges = must_some(shadow);
        assert!(ranges.len() >= 2, "expected >=2 $x sites, got {ranges:?}");
    }

    /// `identity_byte_mapper` constructs a valid `lsp_types::Range` from byte offsets.
    ///
    /// Coverage target: `identity_byte_mapper` is the named free function used as the
    /// uri_mapper argument in `find_references_with_pir_shadow`. Shadow mode never
    /// calls it (Shadow uses byte-offset comparison directly). This test exercises the
    /// function body so coverage tools can track it independently of its call site.
    #[test]
    fn identity_byte_mapper_produces_correct_range() {
        let range = super::identity_byte_mapper(10, 20);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 10);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 20);

        // Zero-length range (declaration-only case).
        let zero = super::identity_byte_mapper(5, 5);
        assert_eq!(zero.start.character, 5);
        assert_eq!(zero.end.character, 5);
    }

    /// Shadow emit path: `find_references_with_pir_shadow` on the multi-scope
    /// fixture drives the full Shadow receipt-emit branch including all receipt
    /// field accesses inside `tracing::debug!`.
    ///
    /// The multi-scope source has two `$x` bindings at different scopes, so the
    /// PIR compiler set (lexical-scope-aware) differs from the legacy set (scope-
    /// blind). This ensures the receipt fields (`missing_from_compiler`,
    /// `extra_in_compiler`, `range_disagreements`) are populated and their
    /// `.len()` calls inside the `tracing::debug!` emission actually execute.
    #[test]
    fn shadow_emit_path_runs_with_populated_receipt_fields() {
        // Outer `$x` at byte 3, inner `$x` at byte 23.
        // Legacy (scope-blind): 4 sites. PIR-A (scope-aware): 2 outer sites.
        // The disagreement means `missing_from_compiler` and `range_disagreements`
        // will be non-empty → all receipt `.len()` calls are exercised.
        const F1_SOURCE: &str = "my $x = 1;\n{\n    my $x = 2;\n    print $x;\n}\nprint $x;\n";
        let ast = parse_with_recovery(F1_SOURCE);

        // Cursor on the outer `$x` at byte 3.
        let shadow = find_references_with_pir_shadow(&ast, 3, F1_SOURCE);
        let ranges = must_some(shadow);

        // Behavior-preserving: legacy returns all 4 sites (both scopes).
        assert_eq!(
            ranges.len(),
            4,
            "shadow must return the scope-blind legacy result (4 sites); got {ranges:?}"
        );
    }
}
