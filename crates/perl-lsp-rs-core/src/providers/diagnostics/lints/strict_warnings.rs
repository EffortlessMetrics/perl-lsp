//! Strict and warnings pragma lint checks
//!
//! This module provides functionality for checking if 'use strict' and 'use warnings'
//! pragmas are present in Perl code, and detecting misspelled pragma names.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `missing-strict` | Information | `use strict` pragma not found |
//! | `missing-warnings` | Information | `use warnings` pragma not found |
//! | `misspelled-pragma` | Warning | Pragma name appears misspelled |

use perl_diagnostics::codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};
use perl_pragma::PragmaTracker;

use super::super::internal_types::{Diagnostic, RelatedInformation};
use super::super::walker::walk_node;
use perl_diagnostics::codes::DiagnosticSeverity;

/// Known pragma names and their common misspellings.
///
/// Each entry maps a correct pragma to a list of known typos.
const PRAGMA_TYPOS: &[(&str, &[&str])] = &[
    ("strict", &["structs", "strickt", "stricts", "stirct", "stict", "strct", "srict"]),
    ("warnings", &["warning", "warningss", "warnigns", "warrnings", "warnins", "warnnigs"]),
    ("utf8", &["utf-8", "uft8", "utf88"]),
    ("feature", &["feaure", "featrue", "feture"]),
    ("constant", &["constanst", "contstant", "costant", "consant"]),
    ("parent", &["parrent", "parnet"]),
    ("base", &["basse", "bace"]),
    ("lib", &["lbi", "libb"]),
    ("Carp", &["Carb", "Crap"]),
];

const PHASE_PRAGMA_SCOPES: &[&str] = &["BEGIN", "END", "INIT", "CHECK", "UNITCHECK"];

#[derive(Debug, Clone, PartialEq, Eq)]
struct PhaseScopedPragmaUse {
    module: String,
    phase: String,
    use_range: (usize, usize),
    phase_range: (usize, usize),
}

/// Check for common strict/warnings issues
///
/// This function checks if 'use strict' and 'use warnings' pragmas are present
/// in the code and generates informational diagnostics if they are missing.
/// It also detects misspelled pragma names and provides "Did you mean?" suggestions.
pub fn check_strict_warnings(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    // Do not suggest strict/warnings for empty, whitespace-only, comment-only,
    // or shebang-only files — the file has no executable content yet.
    if let NodeKind::Program { statements } = &node.kind
        && statements.is_empty()
    {
        return;
    }

    let pragma_map = PragmaTracker::build(node);
    // Query the top-level pragma state (after all scoped blocks have exited).
    // Using usize::MAX ensures we get the last entry, which reflects the
    // restored top-level state after any eval/sub/block scopes have closed.
    // This avoids the false-negative from .any() which sees eval-interior ranges.
    // signatures_strict is included to honour `use feature 'signatures'` (#4038).
    let top_level_state = PragmaTracker::state_for_offset(&pragma_map, usize::MAX);
    let mut has_strict = top_level_state.strict_vars
        || top_level_state.strict_subs
        || top_level_state.strict_refs
        || top_level_state.signatures_strict;
    let mut has_warnings = top_level_state.warnings;

    // OO frameworks that implicitly provide strict+warnings
    const IMPLICIT_STRICT_MODULES: &[&str] = &[
        "Moo",
        "Moose",
        "MooseX::StrictConstructor",
        "Modern::Perl",
        "Dancer2",
        "Catalyst",
        "Mojolicious",
        "Mojo::Base",
    ];

    for module in collect_file_scope_use_modules(node) {
        if IMPLICIT_STRICT_MODULES.contains(&module.as_str()) {
            has_strict = true;
            has_warnings = true;
        }
    }

    // Detect misspelled pragmas.
    // The strict/warnings arms are intentionally absent: state_for_offset above
    // is the authoritative source of truth. Walking the full AST for strict/warnings
    // would bypass lexical scoping (finding eval-block or sub-scoped pragmas).
    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, .. } = &n.kind {
            if module.starts_with('v') || module.chars().next().is_some_and(|c| c.is_ascii_digit())
            {
                // Version pragmas are already reflected in the shared pragma map.
            } else if module != "strict" && module != "warnings" {
                // Check for misspelled pragmas (strict/warnings are in pragma_map)
                check_misspelled_pragma(module, n, diagnostics);
            }
        }
    });

    emit_phase_scoped_pragma_diagnostics(node, has_strict, has_warnings, diagnostics);

    // Add diagnostics if missing
    if !has_strict {
        diagnostics.push(Diagnostic {
            range: (0, 0),
            severity: DiagnosticSeverity::Information,
            code: Some(DiagnosticCode::MissingStrict.as_str().to_string()),
            message: "Consider adding 'use strict;' for better error checking".to_string(),
            related_information: vec![
                RelatedInformation {
                    location: (0, 0),
                    message: "💡 Add 'use strict;' at the beginning of your script".to_string(),
                },
                RelatedInformation {
                    location: (0, 0),
                    message: "ℹ️ The 'use strict' pragma enforces good coding practices by requiring variable declarations, disabling barewords, and preventing symbolic references.".to_string(),
                }
            ],
            tags: Vec::new(),
            suggestion: Some("Add 'use strict;' at the top of the file".to_string()),
        });
    }

    if !has_warnings {
        diagnostics.push(Diagnostic {
            range: (0, 0),
            severity: DiagnosticSeverity::Information,
            code: Some(DiagnosticCode::MissingWarnings.as_str().to_string()),
            message: "Consider adding 'use warnings;' for better error detection".to_string(),
            related_information: vec![
                RelatedInformation {
                    location: (0, 0),
                    message: "💡 Add 'use warnings;' at the beginning of your script".to_string(),
                },
                RelatedInformation {
                    location: (0, 0),
                    message: "ℹ️ The 'use warnings' pragma enables helpful warning messages about questionable constructs, uninitialized values, and deprecated features.".to_string(),
                }
            ],
            tags: Vec::new(),
            suggestion: Some("Add 'use warnings;' at the top of the file".to_string()),
        });
    }
}

fn emit_phase_scoped_pragma_diagnostics(
    node: &Node,
    has_strict: bool,
    has_warnings: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for pragma_use in collect_phase_scoped_pragma_uses(node) {
        match pragma_use.module.as_str() {
            "strict" if !has_strict => diagnostics.push(phase_scoped_pragma_diagnostic(
                &pragma_use,
                DiagnosticCode::PhaseScopedStrictPragma,
                "strict",
            )),
            "warnings" if !has_warnings => diagnostics.push(phase_scoped_pragma_diagnostic(
                &pragma_use,
                DiagnosticCode::PhaseScopedWarningsPragma,
                "warnings",
            )),
            _ => {}
        }
    }
}

fn collect_phase_scoped_pragma_uses(node: &Node) -> Vec<PhaseScopedPragmaUse> {
    let mut hits = Vec::new();
    collect_phase_scoped_pragma_uses_inner(node, None, None, &mut hits);
    hits
}

fn collect_phase_scoped_pragma_uses_inner(
    node: &Node,
    current_phase: Option<&str>,
    current_phase_range: Option<(usize, usize)>,
    hits: &mut Vec<PhaseScopedPragmaUse>,
) {
    match &node.kind {
        NodeKind::PhaseBlock { phase, phase_span, block }
            if PHASE_PRAGMA_SCOPES.contains(&phase.as_str()) =>
        {
            let phase_range = phase_span
                .as_ref()
                .map(|span| (span.start, span.end))
                .unwrap_or((node.location.start, node.location.end));
            collect_phase_scoped_pragma_uses_inner(
                block,
                Some(phase.as_str()),
                Some(phase_range),
                hits,
            );
        }
        NodeKind::Use { module, .. } if matches!(module.as_str(), "strict" | "warnings") => {
            if let (Some(phase), Some(phase_range)) = (current_phase, current_phase_range) {
                hits.push(PhaseScopedPragmaUse {
                    module: module.clone(),
                    phase: phase.to_string(),
                    use_range: (node.location.start, node.location.end),
                    phase_range,
                });
            }
        }
        _ => {
            for child in node.children() {
                collect_phase_scoped_pragma_uses_inner(
                    child,
                    current_phase,
                    current_phase_range,
                    hits,
                );
            }
        }
    }
}

fn phase_scoped_pragma_diagnostic(
    pragma_use: &PhaseScopedPragmaUse,
    code: DiagnosticCode,
    pragma_name: &str,
) -> Diagnostic {
    Diagnostic {
        range: pragma_use.use_range,
        severity: DiagnosticSeverity::Warning,
        code: Some(code.as_str().to_string()),
        message: format!(
            "`use {pragma_name}` inside a {} block does not enable {pragma_name} for the rest of the file",
            pragma_use.phase
        ),
        related_information: vec![
            RelatedInformation {
                location: pragma_use.phase_range,
                message: format!(
                    "Perl phase blocks are lexically scoped: `use {pragma_name}` only applies inside `{}` {{ ... }}.",
                    pragma_use.phase
                ),
            },
            RelatedInformation {
                location: (0, 0),
                message: format!("Move `use {pragma_name};` to file scope for file-wide effect."),
            },
        ],
        tags: Vec::new(),
        suggestion: Some(format!("Move `use {pragma_name};` to the top of the file")),
    }
}

fn collect_file_scope_use_modules(node: &Node) -> Vec<String> {
    let mut modules = Vec::new();
    if let NodeKind::Program { statements } = &node.kind {
        for statement in statements {
            if let NodeKind::Use { module, .. } = &statement.kind {
                modules.push(module.clone());
            }
        }
    }
    modules
}

/// Check if a module name is a misspelling of a known pragma.
///
/// Produces a `misspelled-pragma` warning with a "Did you mean?" suggestion
/// when the module name matches a known typo.
fn check_misspelled_pragma(module: &str, node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    for &(correct, typos) in PRAGMA_TYPOS {
        if typos.contains(&module) {
            diagnostics.push(Diagnostic {
                range: (node.location.start, node.location.end),
                severity: DiagnosticSeverity::Warning,
                code: Some(DiagnosticCode::MisspelledPragma.as_str().to_string()),
                message: format!(
                    "Did you mean 'use {};'? '{}' is not a known pragma",
                    correct, module
                ),
                related_information: vec![RelatedInformation {
                    location: (node.location.start, node.location.end),
                    message: format!("Replace '{}' with '{}'", module, correct),
                }],
                tags: Vec::new(),
                suggestion: Some(format!("Replace 'use {};' with 'use {};'", module, correct)),
            });
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_tdd_support::must;

    fn strict_warnings_diags(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_strict_warnings(&ast, &mut diags);
        diags
    }

    #[test]
    fn empty_file_no_strict_warnings_diagnostic() {
        assert!(
            strict_warnings_diags("").is_empty(),
            "empty file should not get strict/warnings diagnostics"
        );
    }

    #[test]
    fn whitespace_only_no_strict_warnings_diagnostic() {
        assert!(
            strict_warnings_diags("   \n\t\n").is_empty(),
            "whitespace-only file should not get strict/warnings diagnostics"
        );
    }

    #[test]
    fn comment_only_no_strict_warnings_diagnostic() {
        assert!(
            strict_warnings_diags("# just a comment\n").is_empty(),
            "comment-only file should not get strict/warnings diagnostics"
        );
    }

    #[test]
    fn shebang_only_no_strict_warnings_diagnostic() {
        assert!(
            strict_warnings_diags("#!/usr/bin/perl\n").is_empty(),
            "shebang-only file should not get strict/warnings diagnostics"
        );
    }

    #[test]
    fn non_empty_file_without_strict_still_gets_diagnostic() {
        let diags = strict_warnings_diags("my $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL100")),
            "non-empty file without strict should still get missing-strict diagnostic"
        );
    }

    #[test]
    fn file_with_strict_and_warnings_no_diagnostic() {
        let diags = strict_warnings_diags("use strict;\nuse warnings;\nmy $x = 1;\n");
        let has_strict_warn =
            diags.iter().any(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")));
        assert!(
            !has_strict_warn,
            "file with both pragmas should get no strict/warnings diagnostic"
        );
    }

    #[test]
    fn version_pragma_suppresses_strict_warnings_diagnostic() {
        let diags = strict_warnings_diags("use v5.40;\nmy $x = 1;\n");
        let has_strict_warn =
            diags.iter().any(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")));
        assert!(!has_strict_warn, "use v5.40 should suppress strict/warnings diagnostics");
    }

    #[test]
    fn numeric_version_pragma_suppresses_strict_warnings_diagnostic() {
        let diags = strict_warnings_diags("use 5.040;\nmy $x = 1;\n");
        let has_strict_warn =
            diags.iter().any(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")));
        assert!(!has_strict_warn, "use 5.040 should suppress strict/warnings diagnostics");
    }

    #[test]
    fn developer_version_pragma_suppresses_strict_warnings_diagnostic() {
        let diags = strict_warnings_diags("use 5.040_001;\nmy $x = 1;\n");
        let has_strict_warn =
            diags.iter().any(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")));
        assert!(!has_strict_warn, "use 5.040_001 should suppress strict/warnings diagnostics");
    }

    #[test]
    fn v5_36_suppresses_both_strict_and_warnings_diagnostics() {
        // use v5.36 enables both strict and warnings via the feature bundle.
        // Neither PL100 (missing-strict) nor PL101 (missing-warnings) should fire.
        let diags = strict_warnings_diags("use v5.36;\nsub foo ($x) { my $y = $x; }\n");
        let has_strict_warn =
            diags.iter().any(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")));
        assert!(
            !has_strict_warn,
            "use v5.36 should suppress both missing-strict and missing-warnings diagnostics"
        );
    }

    #[test]
    fn v5_36_numeric_form_suppresses_both_strict_and_warnings() {
        // use 5.036 is the numeric form of use v5.36.
        let diags = strict_warnings_diags("use 5.036;\nmy $x = 1;\n");
        let has_strict_warn =
            diags.iter().any(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")));
        assert!(
            !has_strict_warn,
            "use 5.036 should suppress both missing-strict and missing-warnings diagnostics"
        );
    }

    #[test]
    fn v5_12_suppresses_strict_but_not_missing_warnings() {
        let diags = strict_warnings_diags("use v5.12;\nmy $x = 1;\n");
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL100")),
            "use v5.12 should imply strict"
        );
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL101")),
            "use v5.12 should not imply warnings"
        );
    }

    #[test]
    fn crlf_only_no_strict_warnings_diagnostic() {
        // Windows CRLF line endings in an otherwise-empty file — both \r and \n
        // are whitespace-skipped by the lexer, so statements remains empty.
        assert!(
            strict_warnings_diags("\r\n\r\n").is_empty(),
            "CRLF-only file should not get strict/warnings diagnostics"
        );
    }

    #[test]
    fn shebang_plus_comment_no_strict_warnings_diagnostic() {
        // Combined: shebang line followed by a comment — both are skipped as trivia.
        assert!(
            strict_warnings_diags("#!/usr/bin/perl\n# a comment\n").is_empty(),
            "shebang + comment file should not get strict/warnings diagnostics"
        );
    }

    #[test]
    fn misspelled_pragma_in_non_empty_file_still_detected() {
        // The guard must not suppress misspelled-pragma detection in real files.
        // MisspelledPragma = PL111; the guard only fires for empty statements vec.
        let diags = strict_warnings_diags("use structs;\nmy $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL111")),
            "misspelled pragma should still be detected in non-empty files"
        );
    }

    #[test]
    fn pod_only_no_strict_warnings_diagnostic() {
        // POD blocks are consumed as trivia by the lexer, so a POD-only file
        // produces Program { statements: [] }.  The empty-file guard fires and
        // suppresses PL100/PL101 — the same as comment-only files.
        // EDGE_CASES.md documents this behaviour.
        let pod_only = "=head1 NAME\n\nMy::Module - description\n\n=cut\n";
        assert!(
            strict_warnings_diags(pod_only).is_empty(),
            "POD-only file should not get strict/warnings diagnostics — POD is trivia"
        );
    }

    #[test]
    fn eval_block_strict_does_not_suppress_missing_strict_diagnostic() {
        // use strict inside eval { } is lexically scoped to that block only.
        // The file still lacks top-level strict -- PL100 must fire.
        let diags = strict_warnings_diags("eval { use strict; };\nmy $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL100")),
            "eval-scoped strict must not suppress missing-strict (PL100)"
        );
    }

    #[test]
    fn eval_block_warnings_does_not_suppress_missing_warnings_diagnostic() {
        // use warnings inside eval { } is lexically scoped to that block only.
        // The file still lacks top-level warnings -- PL101 must fire.
        let diags = strict_warnings_diags("eval { use warnings; };\nmy $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL101")),
            "eval-scoped warnings must not suppress missing-warnings (PL101)"
        );
    }

    #[test]
    fn eval_string_containing_pragmas_is_handled_conservatively() {
        // String eval is runtime-generated code. We cannot trust pragma text
        // inside the string to affect file-level strict/warnings analysis.
        let diags = strict_warnings_diags("eval \"use strict; use warnings;\";\nmy $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL100")),
            "eval STRING content must not suppress missing-strict (PL100)"
        );
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL101")),
            "eval STRING content must not suppress missing-warnings (PL101)"
        );
    }

    #[test]
    fn top_level_strict_after_eval_block_suppresses_diagnostic() {
        // Top-level use strict/warnings after an eval block are still honored.
        // Neither PL100 nor PL101 should fire.
        let diags =
            strict_warnings_diags("eval { my $y = 1; };\nuse strict;\nuse warnings;\nmy $x = 1;\n");
        assert!(
            diags.iter().all(|d| !matches!(d.code.as_deref(), Some("PL100") | Some("PL101"))),
            "top-level strict after eval must suppress PL100/PL101"
        );
    }

    #[test]
    fn sub_scoped_strict_does_not_suppress_missing_strict_diagnostic() {
        // NodeKind::Subroutine also uses build_scoped_body — verify the fix covers both.
        // use strict inside a sub body should not suppress the file-level PL100.
        let diags = strict_warnings_diags("sub foo { use strict; }\nmy $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL100")),
            "sub-scoped strict must not suppress missing-strict (PL100)"
        );
    }

    #[test]
    fn conditional_use_if_pragmas_suppress_file_level_missing_diagnostics() {
        // `use if COND, 'strict'/'warnings'` is represented as module "if" in the AST,
        // and should still update tracked top-level pragma state.
        let diags =
            strict_warnings_diags("use if 1, 'strict';\nuse if 1, 'warnings';\nmy $x = 1;\n");
        let codes: Vec<&str> = diags.iter().filter_map(|d| d.code.as_deref()).collect();
        assert!(
            diags.iter().all(|d| !matches!(d.code.as_deref(), Some("PL100") | Some("PL101"))),
            "conditional use-if pragmas should suppress PL100/PL101 (got: {codes:?})"
        );
    }

    #[test]
    fn begin_scoped_strict_emits_phase_scoped_strict_diagnostic() {
        let diags = strict_warnings_diags("BEGIN { use strict; }\nmy $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL502")),
            "BEGIN-scoped strict should emit PL502"
        );
    }

    #[test]
    fn begin_scoped_strict_with_top_level_strict_does_not_emit_phase_diagnostic() {
        let diags = strict_warnings_diags("BEGIN { use strict; }\nuse strict;\nmy $x = 1;\n");
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL502")),
            "top-level strict should suppress PL502"
        );
    }

    #[test]
    fn end_scoped_warnings_emits_phase_scoped_warnings_diagnostic() {
        let diags = strict_warnings_diags("use strict;\nEND { use warnings; }\nmy $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL503")),
            "END-scoped warnings should emit PL503"
        );
    }

    #[test]
    fn phase_scoped_non_strict_pragma_does_not_emit_phase_diagnostic() {
        let diags = strict_warnings_diags("BEGIN { use utf8; }\nmy $x = 1;\n");
        assert!(
            diags.iter().all(|d| !matches!(d.code.as_deref(), Some("PL502") | Some("PL503"))),
            "non-strict pragmas inside phase blocks should not emit PL502/PL503"
        );
    }

    // ── Edge cases added by deep-reviewer ──────────────────────────────────

    #[test]
    fn nested_eval_strict_does_not_suppress_missing_strict_diagnostic() {
        // eval inside eval: use strict inside the inner eval must not bubble up.
        let diags = strict_warnings_diags("eval { eval { use strict; }; };\nmy $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL100")),
            "nested-eval-scoped strict must not suppress missing-strict (PL100)"
        );
    }

    #[test]
    fn eval_then_top_level_strict_then_eval_no_strict_restores_correctly() {
        // eval { no strict; } after top-level use strict.
        // usize::MAX must land on the restore entry (strict=true), not the inner no-strict.
        let diags =
            strict_warnings_diags("use strict;\nuse warnings;\neval { no strict; };\nmy $x = 1;\n");
        assert!(
            diags.iter().all(|d| !matches!(d.code.as_deref(), Some("PL100") | Some("PL101"))),
            "top-level strict before eval must not be revoked by no-strict inside eval"
        );
    }

    #[test]
    fn package_block_no_strict_restores_top_level_state() {
        let diags = strict_warnings_diags(
            "use strict;\nuse warnings;\npackage Foo {\n  no strict;\n  no warnings;\n  my $tmp = 1;\n}\nmy $x = 1;\n",
        );
        assert!(
            diags.iter().all(|d| !matches!(d.code.as_deref(), Some("PL100") | Some("PL101"))),
            "no strict/no warnings inside package block must not revoke top-level strict/warnings"
        );
    }

    #[test]
    fn begin_block_no_strict_restores_top_level_state() {
        let diags = strict_warnings_diags(
            "use strict;\nuse warnings;\nBEGIN { no strict; no warnings; my $tmp = 1; }\nmy $x = 1;\n",
        );
        assert!(
            diags.iter().all(|d| !matches!(d.code.as_deref(), Some("PL100") | Some("PL101"))),
            "no strict/no warnings inside BEGIN block must not revoke top-level strict/warnings"
        );
    }

    #[test]
    fn sub_inside_eval_scoped_strict_does_not_suppress() {
        // sub inside eval: use strict inside sub inside eval.
        // Three scoping levels — none should leak to top level.
        let diags = strict_warnings_diags("eval { sub inner { use strict; } };\nmy $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL100")),
            "sub-inside-eval-scoped strict must not suppress missing-strict (PL100)"
        );
    }

    #[test]
    fn implicit_strict_module_inside_eval_does_not_suppress_missing_strict() {
        let diags = strict_warnings_diags("eval { use Moose; };\nmy $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL100")),
            "eval-scoped Moose should not suppress missing-strict (PL100)"
        );
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL101")),
            "eval-scoped Moose should not suppress missing-warnings (PL101)"
        );
    }

    #[test]
    fn top_level_implicit_strict_module_suppresses_both_diagnostics() {
        let diags = strict_warnings_diags("use Moo;\nmy $x = 1;\n");
        assert!(
            diags.iter().all(|d| !matches!(d.code.as_deref(), Some("PL100") | Some("PL101"))),
            "top-level Moo should suppress both missing strict/warnings diagnostics"
        );
    }

    #[test]
    fn implicit_strict_module_inside_sub_body_does_not_suppress_missing_strict() {
        // `use Moose` inside a sub body is not at file scope.
        // collect_file_scope_use_modules only checks Program.statements, so
        // a sub-scoped `use Moose` must not suppress PL100/PL101.
        let diags = strict_warnings_diags("sub configure { use Moose; }\nmy $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL100")),
            "sub-scoped Moose should not suppress missing-strict (PL100)"
        );
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL101")),
            "sub-scoped Moose should not suppress missing-warnings (PL101)"
        );
    }

    #[test]
    fn all_implicit_strict_modules_suppress_at_top_level() {
        // Spot-check two more members of IMPLICIT_STRICT_MODULES to ensure
        // collect_file_scope_use_modules covers the full list, not just Moo.
        for module in &["Moose", "Modern::Perl"] {
            let source = format!("use {};\nmy $x = 1;\n", module);
            let diags = strict_warnings_diags(&source);
            assert!(
                diags.iter().all(|d| !matches!(d.code.as_deref(), Some("PL100") | Some("PL101"))),
                "top-level `use {module}` should suppress both missing strict/warnings diagnostics"
            );
        }
    }
}
