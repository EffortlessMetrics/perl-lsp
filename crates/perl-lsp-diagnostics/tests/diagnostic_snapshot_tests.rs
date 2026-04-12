//! Diagnostic snapshot regression tests for common Perl patterns.
//!
//! Each test feeds a Perl snippet through the full diagnostic pipeline and
//! asserts the exact set of diagnostic codes produced. If the diagnostic output
//! changes, the test fails and forces explicit review — preventing silent
//! regressions on false positives or missing diagnostics.
//!
//! # Design notes
//!
//! - `diagnostics_for` invokes the full pipeline (parser + scope + all lints).
//! - Tests call `get_diagnostics(..., None)` with no source path, so PL200
//!   (missing-package) fires for script-style snippets. Each test documents
//!   the expected PL200 vs. suppressed state explicitly.
//! - "False positive" here means a diagnostic that fires on correct, idiomatic
//!   Perl where the programmer did nothing wrong. Parse errors on valid Perl,
//!   unused-variable on a variable that IS used, etc. are all false positives.
//! - Tests for "should produce diagnostics" patterns lock the PRESENCE of a
//!   specific code — so both false-negative regressions (stop firing) and
//!   false-positive regressions (fire on the wrong thing) are caught.

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity, DiagnosticsProvider};
use perl_parser::Parser;

// ---------------------------------------------------------------------------
// Pipeline helper
// ---------------------------------------------------------------------------

/// Parse source and run the full diagnostic pipeline (no module resolver,
/// no source path — simulates opening an unnamed buffer).
fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

/// Collect diagnostic codes from a diagnostics slice.
fn codes(diags: &[Diagnostic]) -> Vec<&str> {
    diags.iter().filter_map(|d| d.code.as_deref()).collect()
}

/// Assert there are no parse-error diagnostics (PL001, PL002, PL003).
fn assert_no_parse_errors(diags: &[Diagnostic], label: &str) {
    let parse_errors: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL001") | Some("PL002") | Some("PL003")))
        .collect();
    assert!(parse_errors.is_empty(), "{label}: expected zero parse errors, got {parse_errors:?}");
}

/// Assert there are no scope-error diagnostics (undeclared-variable,
/// variable-redeclaration, duplicate-parameter, unquoted-bareword).
fn assert_no_scope_errors(diags: &[Diagnostic], label: &str) {
    let scope_errors: Vec<_> = diags
        .iter()
        .filter(|d| {
            matches!(
                d.code.as_deref(),
                Some("undeclared-variable")
                    | Some("variable-redeclaration")
                    | Some("duplicate-parameter")
                    | Some("unquoted-bareword")
            )
        })
        .collect();
    assert!(scope_errors.is_empty(), "{label}: expected zero scope errors, got {scope_errors:?}");
}

/// Assert no false-positive Warning or Error diagnostics beyond the known
/// expected set. `allowed_codes` lists codes that are legitimately expected
/// for this snippet (e.g., PL200 for scripts without a package declaration).
fn assert_no_unexpected_warnings(diags: &[Diagnostic], allowed_codes: &[&str], label: &str) {
    let unexpected: Vec<_> = diags
        .iter()
        .filter(|d| {
            let severity_is_warn_or_error =
                matches!(d.severity, DiagnosticSeverity::Warning | DiagnosticSeverity::Error);
            let code = d.code.as_deref().unwrap_or("");
            severity_is_warn_or_error && !allowed_codes.contains(&code)
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "{label}: unexpected Warning/Error diagnostics (all codes {:?}): got {unexpected:?}",
        codes(diags)
    );
}

// ---------------------------------------------------------------------------
// Snippet 1: basic strict/warnings script
// ---------------------------------------------------------------------------

/// A minimal strict/warnings Perl script should produce no parse errors,
/// no scope errors, and no false-positive warnings.
///
/// Expected diagnostics:
/// - PL200 (missing-package): scripts without `package` get this; it is a
///   correct advisory, not a false positive. We assert it fires exactly once.
/// - PL102 (unused-variable): `$x` is passed to `print` so it is USED.
///   Asserting PL102 is absent here locks the "no false unused-variable" invariant.
#[test]
fn snapshot_basic_strict_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = 1;\nprint $x;\n";
    let diags = diagnostics_for(source);

    // No parse errors on valid Perl
    assert_no_parse_errors(&diags, "basic strict/warnings");
    // No scope errors
    assert_no_scope_errors(&diags, "basic strict/warnings");
    // No unused-variable false positive (PL102) — $x is printed
    let unused_var: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL102")).collect();
    assert!(
        unused_var.is_empty(),
        "Used variable $x must not be flagged as unused: {unused_var:?}"
    );
    // No missing-strict/missing-warnings (PL100/PL101) — both pragmas present
    let missing_pragmas: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(
        missing_pragmas.is_empty(),
        "strict+warnings present; must not get PL100/PL101: {missing_pragmas:?}"
    );
    // PL200 is expected for a script buffer with no package declaration
    let pl200: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL200")).collect();
    assert_eq!(pl200.len(), 1, "Expected exactly one PL200 for script without package");
    // Only PL200 may appear as a Warning/Error
    assert_no_unexpected_warnings(&diags, &["PL200"], "basic strict/warnings");

    Ok(())
}

// ---------------------------------------------------------------------------
// Snippet 2: OOP pattern (package + constructor + accessor)
// ---------------------------------------------------------------------------

/// A minimal OOP module should produce no false-positive diagnostics.
/// It declares `package Foo;` so PL200 is suppressed.
///
/// Hash keys are quoted (`'name'`) to avoid PL109 (unquoted bareword) — hash
/// subscripts in `->{'key'}` form are unambiguously string keys and must not
/// produce spurious bareword warnings.
#[test]
fn snapshot_oop_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "package Foo;\n",
        "use strict;\n",
        "use warnings;\n",
        "sub new { my ($class, %args) = @_; bless \\%args, $class }\n",
        "sub name { return $_[0]->{'name'} }\n",
        "1;\n",
    );
    let diags = diagnostics_for(source);

    assert_no_parse_errors(&diags, "OOP pattern");
    assert_no_scope_errors(&diags, "OOP pattern");

    // No missing-strict/missing-warnings
    let missing_pragmas: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(
        missing_pragmas.is_empty(),
        "OOP module must not get missing-pragma: {missing_pragmas:?}"
    );

    // No missing-package (has `package Foo;`)
    let pl200: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL200")).collect();
    assert!(pl200.is_empty(), "OOP module with package declaration must not get PL200: {pl200:?}");

    // No unused-variable diagnostics for the constructor/accessor args
    let unused_var: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL102")).collect();
    assert!(
        unused_var.is_empty(),
        "Constructor/accessor args must not be flagged unused: {unused_var:?}"
    );

    // No unexpected warnings
    assert_no_unexpected_warnings(&diags, &[], "OOP pattern");

    Ok(())
}

// ---------------------------------------------------------------------------
// Snippet 3: File I/O pattern
// ---------------------------------------------------------------------------

/// A standard 3-argument `open` with a `while (<$fh>)` loop.
/// The 3-arg open must NOT trigger PL401 (security-two-arg-open).
/// The filehandle variable must not be flagged as unused.
#[test]
fn snapshot_file_io() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "use strict;\n",
        "use warnings;\n",
        "open my $fh, '<', 'file.txt' or die \"Cannot open: $!\";\n",
        "while (<$fh>) { chomp; print \"$_\\n\"; }\n",
        "close $fh;\n",
    );
    let diags = diagnostics_for(source);

    assert_no_parse_errors(&diags, "file I/O");
    assert_no_scope_errors(&diags, "file I/O");

    // 3-arg open must NOT produce PL401 (security-two-arg-open)
    let two_arg_open: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("PL401")).collect();
    assert!(
        two_arg_open.is_empty(),
        "3-arg open must not trigger PL401 (security-two-arg-open): {two_arg_open:?}"
    );

    // No missing-strict/missing-warnings
    let missing_pragmas: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(missing_pragmas.is_empty(), "file I/O with strict+warnings must not get PL100/PL101");

    // No unexpected warnings beyond PL200 (script without package)
    assert_no_unexpected_warnings(&diags, &["PL200"], "file I/O");

    Ok(())
}

#[test]
fn snapshot_open_my_readline_no_pl102() -> Result<(), Box<dyn std::error::Error>> {
    // Regression for #3446: `open my $fh, ...` must declare `$fh` for later `<$fh>` and `close`.
    let source = concat!(
        "use strict;\n",
        "use warnings;\n",
        "open my $fh, '<', 'file.txt' or die $!;\n",
        "print <$fh>;\n",
        "close $fh;\n",
    );
    let diags = diagnostics_for(source);

    assert_no_parse_errors(&diags, "open-my/readline regression");

    let pl102: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL102")).collect();
    assert!(pl102.is_empty(), "open my/readline must not trigger PL102: {pl102:?}");

    Ok(())
}

// ---------------------------------------------------------------------------
// Snippet 4: Hash/array operations
// ---------------------------------------------------------------------------

/// Hash and array operations including `map`, `sort`, `%lookup`.
/// `$a` and `$b` are sort comparison vars — must not be flagged as undeclared.
#[test]
fn snapshot_hash_array_operations() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "use strict;\n",
        "use warnings;\n",
        "my @items = (1, 2, 3);\n",
        "my %lookup = map { $_ => 1 } @items;\n",
        "my @sorted = sort { $a <=> $b } @items;\n",
        "print scalar @sorted, \"\\n\";\n",
        "print scalar keys %lookup, \"\\n\";\n",
    );
    let diags = diagnostics_for(source);

    assert_no_parse_errors(&diags, "hash/array operations");
    assert_no_scope_errors(&diags, "hash/array operations");

    // $a and $b in sort block must not be flagged as undeclared or unused
    let undeclared: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("undeclared-variable")).collect();
    assert!(
        undeclared.is_empty(),
        "$a/$b must not be flagged undeclared in sort block: {undeclared:?}"
    );

    // @sorted and %lookup are used in print — must not be flagged unused
    let unused_var: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL102")).collect();
    assert!(
        unused_var.is_empty(),
        "Used collections must not be flagged as unused: {unused_var:?}"
    );

    // No unexpected warnings beyond PL200
    assert_no_unexpected_warnings(&diags, &["PL200"], "hash/array operations");

    Ok(())
}

// ---------------------------------------------------------------------------
// Snippet 5: Regex pattern
// ---------------------------------------------------------------------------

/// Standard regex with capture groups. Capture vars `$1`, `$2` are used in
/// print — must not produce undeclared-variable diagnostics.
#[test]
fn snapshot_regex_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "use strict;\n",
        "use warnings;\n",
        "my $str = \"hello world\";\n",
        "if ($str =~ /(\\w+)\\s+(\\w+)/) { print \"$1 $2\\n\"; }\n",
    );
    let diags = diagnostics_for(source);

    assert_no_parse_errors(&diags, "regex pattern");
    assert_no_scope_errors(&diags, "regex pattern");

    // $1 and $2 are regex capture variables — must not be undeclared
    let undeclared: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("undeclared-variable")).collect();
    assert!(undeclared.is_empty(), "Regex capture vars must not be undeclared: {undeclared:?}");

    // $str is used in the regex — must not be flagged unused
    let unused_var: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL102")).collect();
    assert!(unused_var.is_empty(), "Used $str must not be flagged unused: {unused_var:?}");

    // No unexpected warnings beyond PL200
    assert_no_unexpected_warnings(&diags, &["PL200"], "regex pattern");

    Ok(())
}

// ---------------------------------------------------------------------------
// Snippet 6: Error handling (eval/die/warn)
// ---------------------------------------------------------------------------

/// Block-eval error handling. Must not trigger PL600 (security-string-eval)
/// (only string eval is flagged; block eval is safe).
/// `$@` is a Perl special variable — must not be undeclared.
#[test]
fn snapshot_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "use strict;\n",
        "use warnings;\n",
        "eval { die \"oops\" };\n",
        "if ($@) { warn \"caught: $@\" }\n",
    );
    let diags = diagnostics_for(source);

    assert_no_parse_errors(&diags, "error handling");
    assert_no_scope_errors(&diags, "error handling");

    // Block eval must NOT trigger PL600 (security-string-eval)
    let string_eval: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL600")).collect();
    assert!(
        string_eval.is_empty(),
        "Block eval must not trigger PL600 (security-string-eval): {string_eval:?}"
    );

    // $@ must not be flagged as undeclared — it is a Perl special variable
    let undeclared: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("undeclared-variable")).collect();
    assert!(undeclared.is_empty(), "$@ must not be flagged as undeclared variable: {undeclared:?}");

    // No unexpected warnings beyond PL200
    assert_no_unexpected_warnings(&diags, &["PL200"], "error handling");

    Ok(())
}

// ---------------------------------------------------------------------------
// Snippet 7: Module usage with Carp
// ---------------------------------------------------------------------------

/// Standard `use Carp qw(croak)` pattern with a validation subroutine.
/// `Carp` is in the implicit-export skip list — must not get PL700 (unused-import).
#[test]
fn snapshot_module_usage_carp() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "use strict;\n",
        "use warnings;\n",
        "use Carp qw(croak);\n",
        "sub validate { croak \"invalid\" unless $_[0] }\n",
    );
    let diags = diagnostics_for(source);

    assert_no_parse_errors(&diags, "module usage (Carp)");
    assert_no_scope_errors(&diags, "module usage (Carp)");

    // Carp must not produce PL700 (unused-import) — it is in the implicit-export skip list
    let unused_import: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("PL700")).collect();
    assert!(
        unused_import.is_empty(),
        "Carp must not be flagged as unused import (PL700): {unused_import:?}"
    );

    // No missing-strict/missing-warnings
    let missing_pragmas: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(missing_pragmas.is_empty(), "Carp snippet must not get PL100/PL101");

    // No unexpected warnings beyond PL200
    assert_no_unexpected_warnings(&diags, &["PL200"], "module usage (Carp)");

    Ok(())
}

// ---------------------------------------------------------------------------
// Snippet 8: Modern Perl (use v5.36 with signatures)
// ---------------------------------------------------------------------------

/// `use v5.36` implicitly enables strict, warnings, and signatures.
/// Must not get PL100/PL101 (missing-strict/warnings) since v5.36 enables them.
/// Must not get PL900 (version-compat warning) since `say` and signatures are
/// enabled by v5.36.
#[test]
fn snapshot_modern_perl_v536() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "use v5.36;\n",
        "sub greet ($name) { say \"Hello, $name!\" }\n",
        "greet(\"World\");\n",
    );
    let diags = diagnostics_for(source);

    assert_no_parse_errors(&diags, "modern Perl v5.36");
    assert_no_scope_errors(&diags, "modern Perl v5.36");

    // use v5.36 must suppress missing-strict/missing-warnings (PL100/PL101)
    let missing_pragmas: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(
        missing_pragmas.is_empty(),
        "use v5.36 must suppress PL100/PL101, got: {missing_pragmas:?}"
    );

    // No version-compat warning — say and signatures are enabled at v5.36
    let version_compat: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("PL900")).collect();
    assert!(
        version_compat.is_empty(),
        "say/signatures at v5.36 must not trigger PL900: {version_compat:?}"
    );

    // PL200 is expected (no package declaration; v5.36 doesn't suppress it)
    let pl200: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL200")).collect();
    assert_eq!(
        pl200.len(),
        1,
        "Expected exactly one PL200 for v5.36 script without package declaration"
    );

    // No unexpected warnings beyond PL200
    assert_no_unexpected_warnings(&diags, &["PL200"], "modern Perl v5.36");

    Ok(())
}

// ---------------------------------------------------------------------------
// Positive cases: patterns that SHOULD produce diagnostics
// ---------------------------------------------------------------------------

/// A bare `my $x = 1; print $x` without `use strict` should trigger
/// PL100 (missing-strict) and PL200 (missing-package).
///
/// This test locks the PRESENCE of PL100 — if the lint stops firing on
/// code without strict, this test catches the regression.
#[test]
fn snapshot_missing_strict_fires() -> Result<(), Box<dyn std::error::Error>> {
    // Intentionally no strict/warnings
    let source = "my $x = 1;\nprint $x;\n";
    let diags = diagnostics_for(source);

    let missing_strict: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("PL100")).collect();
    assert!(
        !missing_strict.is_empty(),
        "Code without 'use strict' must emit PL100 (missing-strict), got codes: {:?}",
        codes(&diags)
    );
    assert_eq!(
        missing_strict[0].severity,
        DiagnosticSeverity::Information,
        "PL100 must be Information severity"
    );

    Ok(())
}

/// An unused variable with strict+warnings should trigger PL102 (unused-variable).
///
/// This test locks the PRESENCE of PL102 — if unused-variable detection is
/// accidentally disabled, this test catches the regression.
#[test]
fn snapshot_unused_variable_fires() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $unused = 1;\n";
    let diags = diagnostics_for(source);

    let unused_var: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL102")).collect();
    assert!(
        !unused_var.is_empty(),
        "Declared-but-never-used variable must emit PL102 (unused-variable), got codes: {:?}",
        codes(&diags)
    );
    assert_eq!(
        unused_var[0].severity,
        DiagnosticSeverity::Warning,
        "PL102 must be Warning severity"
    );

    Ok(())
}

/// String eval (`eval("...")`) should trigger PL600 (SecurityStringEval).
///
/// This test locks detection of string eval as a security anti-pattern.
/// The stable diagnostic code is PL600.
#[test]
fn snapshot_string_eval_fires() -> Result<(), Box<dyn std::error::Error>> {
    // eval("...") with a string literal — the security checker must flag this
    let source = concat!("use strict;\n", "use warnings;\n", "eval(\"system('rm -rf /');\");\n",);
    let diags = diagnostics_for(source);

    let string_eval: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL600")).collect();
    assert!(
        !string_eval.is_empty(),
        "String eval must emit PL600 (SecurityStringEval), got codes: {:?}",
        codes(&diags)
    );
    assert_eq!(
        string_eval[0].severity,
        DiagnosticSeverity::Warning,
        "PL600 must be Warning severity"
    );

    Ok(())
}

/// A two-argument `open` should trigger PL401 (TwoArgOpen).
///
/// The security checker requires `NodeKind::FunctionCall { name: "open", args }` with
/// exactly 2 args. This test exercises the checker directly via `check_security` since
/// the full pipeline may parse `open(FH, ...)` with a different AST shape depending on
/// how the parser handles bare filehandles. The direct API test verifies the invariant
/// that check_security correctly classifies 2-arg open.
///
/// # Full-pipeline gap
///
/// A separate integration test should cover the full-pipeline path once the parser
/// reliably produces `FunctionCall { args: [fh, mode_file] }` for 2-arg open syntax.
#[test]
fn snapshot_two_arg_open_fires() -> Result<(), Box<dyn std::error::Error>> {
    use perl_parser_core::{Node, NodeKind, SourceLocation};

    // Build a minimal 2-arg open AST: open(FH, ">file.txt")
    let fh_node = Node::new(
        NodeKind::Identifier { name: "FH".to_string() },
        SourceLocation { start: 5, end: 7 },
    );
    let file_node = Node::new(
        NodeKind::String { value: ">file.txt".to_string(), interpolated: false },
        SourceLocation { start: 9, end: 20 },
    );
    let open_call = Node::new(
        NodeKind::FunctionCall { name: "open".to_string(), args: vec![fh_node, file_node] },
        SourceLocation { start: 0, end: 21 },
    );
    let stmt = Node::new(
        NodeKind::ExpressionStatement { expression: Box::new(open_call) },
        SourceLocation { start: 0, end: 22 },
    );
    let root = Node::new(
        NodeKind::Program { statements: vec![stmt] },
        SourceLocation { start: 0, end: 100 },
    );

    let mut diags = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diags);

    let two_arg: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL401")).collect();
    assert!(
        !two_arg.is_empty(),
        "2-arg open must emit PL401 (TwoArgOpen) from check_security, got: {:?}",
        diags
    );
    assert_eq!(two_arg[0].severity, DiagnosticSeverity::Warning, "PL401 must be Warning severity");
    assert!(two_arg[0].suggestion.is_some(), "PL401 must carry a suggestion for the developer");

    Ok(())
}

// ---------------------------------------------------------------------------
// Snapshot invariant: diagnostic codes are stable PL/PC prefixed values
// ---------------------------------------------------------------------------

/// Every diagnostic produced by the pipeline must carry a code in the
/// stable `PL`/`PC` namespace or a known legacy code pattern.
///
/// This test prevents introduction of raw string codes that bypass the
/// `DiagnosticCode` enum (which would break clients relying on stable codes).
#[test]
fn snapshot_all_codes_are_stable() -> Result<(), Box<dyn std::error::Error>> {
    // Run the pipeline on a few distinct snippets and check all codes
    let snippets = [
        "use strict;\nuse warnings;\nmy $x = 42;\nprint $x;\n",
        "my $x = 1;\n",
        "use strict;\nuse warnings;\nmy $unused = 1;\n",
    ];

    for snippet in &snippets {
        let diags = diagnostics_for(snippet);
        for d in &diags {
            if let Some(code) = &d.code {
                assert!(
                    code.starts_with("PL")
                        || code.starts_with("PC")
                        || code.starts_with("security-")
                        || code.starts_with("unused-")
                        || code.starts_with("parse-error-"),
                    "Diagnostic code '{code}' in snippet {:?} must be a stable prefixed code",
                    snippet
                );
            }
        }
    }

    Ok(())
}
