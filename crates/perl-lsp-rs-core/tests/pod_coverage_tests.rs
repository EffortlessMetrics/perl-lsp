//! Integration tests for POD coverage diagnostic (PL304)
//!
//! These tests parse real Perl source through the full parser and diagnostics
//! pipeline, verifying that the PL304 diagnostic fires when exported
//! subroutines lack POD documentation.

use std::sync::Arc;

use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn pod_diags(source: &str) -> Vec<Diagnostic> {
    diagnostics_for(source).into_iter().filter(|d| d.code.as_deref() == Some("PL304")).collect()
}

// ---------------------------------------------------------------------------
// Should fire
// ---------------------------------------------------------------------------

#[test]
fn given_exporter_with_undocumented_exports_then_pl304_fires() {
    let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT = qw(foo bar);
sub foo { 1 }
sub bar { 2 }
"#;
    let diags = pod_diags(source);
    assert_eq!(diags.len(), 2, "both foo and bar lack POD: {diags:?}");
    assert!(diags.iter().any(|d| d.message.contains("foo")));
    assert!(diags.iter().any(|d| d.message.contains("bar")));
}

#[test]
fn given_export_ok_with_undocumented_exports_then_pl304_fires() {
    let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT_OK = qw(alpha beta);
sub alpha { 1 }
sub beta { 2 }
"#;
    let diags = pod_diags(source);
    assert_eq!(diags.len(), 2, "both alpha and beta lack POD: {diags:?}");
}

#[test]
fn given_partial_pod_then_only_undocumented_reported() {
    let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT = qw(foo bar baz);

=head2 foo

Does foo things.

=cut

sub foo { 1 }
sub bar { 2 }
sub baz { 3 }
"#;
    let diags = pod_diags(source);
    assert_eq!(diags.len(), 2, "bar and baz lack POD: {diags:?}");
    assert!(!diags.iter().any(|d| d.message.contains("foo")), "foo is documented");
    assert!(diags.iter().any(|d| d.message.contains("bar")));
    assert!(diags.iter().any(|d| d.message.contains("baz")));
}

// ---------------------------------------------------------------------------
// Should NOT fire
// ---------------------------------------------------------------------------

#[test]
fn given_no_exporter_then_no_diagnostic() {
    let source = r#"package Internal;
sub helper { 1 }
sub private { 2 }
"#;
    let diags = pod_diags(source);
    assert!(diags.is_empty(), "no exporter module = no PL304");
}

#[test]
fn given_all_exports_documented_with_head2_then_no_diagnostic() {
    let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT = qw(process run);

=head2 process

Process data.

=head2 run

Run the thing.

=cut

sub process { 1 }
sub run { 2 }
"#;
    let diags = pod_diags(source);
    assert!(diags.is_empty(), "all exports are documented: {diags:?}");
}

#[test]
fn given_item_pod_then_export_recognized_as_documented() {
    let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT = qw(calculate);

=over 4

=item calculate()

Calculates something.

=back

=cut

sub calculate { 42 }
"#;
    let diags = pod_diags(source);
    assert!(diags.is_empty(), "=item documentation should count: {diags:?}");
}

#[test]
fn given_bold_markup_in_pod_then_name_extracted() {
    let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT = qw(deploy);

=head2 B<deploy>

Deploy to production.

=cut

sub deploy { 1 }
"#;
    let diags = pod_diags(source);
    assert!(diags.is_empty(), "B<deploy> markup should match: {diags:?}");
}

#[test]
fn given_code_markup_in_pod_then_name_extracted() {
    let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT = qw(init);

=head2 C<init()>

Initialize the system.

=cut

sub init { 1 }
"#;
    let diags = pod_diags(source);
    assert!(diags.is_empty(), "C<init()> markup should match: {diags:?}");
}

#[test]
fn given_only_variable_exports_then_no_diagnostic() {
    let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT_OK = qw($VERSION @DATA %CONFIG);
"#;
    let diags = pod_diags(source);
    assert!(diags.is_empty(), "variable exports should be ignored: {diags:?}");
}

#[test]
fn given_parent_based_exporter_then_lint_applies() {
    let source = r#"package MyModule;
use parent 'Exporter';
our @EXPORT_OK = qw(helper);
sub helper { 1 }
"#;
    let diags = pod_diags(source);
    assert_eq!(diags.len(), 1, "parent-based exporter should trigger lint: {diags:?}");
    assert!(diags[0].message.contains("helper"));
}

#[test]
fn given_base_based_exporter_then_lint_applies() {
    let source = r#"package MyModule;
use base 'Exporter';
our @EXPORT = qw(action);
sub action { 1 }
"#;
    let diags = pod_diags(source);
    assert_eq!(diags.len(), 1, "base-based exporter should trigger lint: {diags:?}");
    assert!(diags[0].message.contains("action"));
}

#[test]
fn given_empty_export_list_then_no_diagnostic() {
    let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT = ();
sub internal { 1 }
"#;
    let diags = pod_diags(source);
    assert!(diags.is_empty(), "empty export list = no lint: {diags:?}");
}

#[test]
fn given_diagnostic_has_correct_code_and_severity() {
    let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT = qw(check);
sub check { 1 }
"#;
    let diags = pod_diags(source);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code.as_deref(), Some("PL304"));
    assert_eq!(
        diags[0].severity,
        perl_diagnostics::codes::DiagnosticSeverity::Hint,
        "PL304 should be Hint severity"
    );
    assert!(diags[0].suggestion.is_some(), "should have a suggestion");
}
