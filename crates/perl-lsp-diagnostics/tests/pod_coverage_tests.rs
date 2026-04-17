//! Tests for PL304 — Exported subroutine POD coverage lint
//!
//! This lint checks that all exported subroutines have corresponding POD
//! documentation (via `=head2 subroutine_name`). The check only applies to
//! files that use Exporter.
//!
//! # Codes tested
//!
//! | Code  | Name                              | Status      |
//! |-------|-----------------------------------|-------------|
//! | PL304 | ExportedSubroutineWithoutPodDocs  | Not yet implemented |
//!
//! Tests FAIL before `pod_coverage.rs` is created and wired.
//! Tests PASS after the implementation is complete.
//!
//! See: crates/perl-lsp-diagnostics/src/lints/pod_coverage.rs

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn codes_for(source: &str) -> Vec<String> {
    diagnostics_for(source).into_iter().filter_map(|d| d.code).collect()
}

fn has_code(source: &str, code: &str) -> bool {
    codes_for(source).iter().any(|c| c == code)
}

fn count_code(source: &str, code: &str) -> usize {
    codes_for(source).iter().filter(|c| c.as_str() == code).count()
}

// =========================================================================
// PL304 — ExportedSubroutineWithoutPodDocs
// =========================================================================

/// Test 1: PL304 fires when an exported subroutine has no POD documentation.
#[test]
fn pl304_fires_when_exported_sub_has_no_pod() {
    let source = r#"
package My::Module;
use strict;
use warnings;
use Exporter 'import';
our @EXPORT = qw(foo);

sub foo { }

1;
"#;
    assert!(
        has_code(source, "PL304"),
        "Expected PL304 (ExportedSubroutineWithoutPodDocs) when exported sub 'foo' \
         has no POD documentation. Got codes: {:?}",
        codes_for(source)
    );
}

/// Test 2: PL304 does NOT fire when the exported subroutine has POD documentation
/// via `=head2 subroutine_name`.
#[test]
fn pl304_suppressed_when_exported_sub_has_pod() {
    let source = r#"
package My::Module;
use strict;
use warnings;
use Exporter 'import';
our @EXPORT = qw(foo);

sub foo { }

=head2 foo

Returns a foo.

=cut

1;
"#;
    assert!(
        !has_code(source, "PL304"),
        "PL304 should NOT fire when exported sub 'foo' has =head2 foo documentation. \
         Got codes: {:?}",
        codes_for(source)
    );
}

/// Test 3: PL304 does NOT fire for non-exported (private) subroutines without POD.
/// Private subs are not part of the public API and don't need documentation.
#[test]
fn pl304_not_fired_for_non_exported_subs() {
    let source = r#"
package My::Module;
use strict;
use warnings;
use Exporter 'import';
our @EXPORT = qw(public_fn);

sub public_fn { }
sub _private_helper { }  # not exported — should not trigger PL304

=head2 public_fn

Returns a thing.

=cut

1;
"#;
    assert!(
        !has_code(source, "PL304"),
        "PL304 must NOT fire for private subroutines (not in @EXPORT). \
         Got codes: {:?}",
        codes_for(source)
    );
}

/// Test 4: PL304 handles both @EXPORT and @EXPORT_OK lists.
#[test]
fn pl304_checks_both_export_and_export_ok() {
    let source = r#"
package My::Module;
use strict;
use warnings;
use Exporter 'import';
our @EXPORT = qw(foo);
our @EXPORT_OK = qw(bar);

sub foo { }
sub bar { }

1;
"#;
    // Both foo and bar are exported but neither has POD — should get 2 PL304 diagnostics
    let count = count_code(source, "PL304");
    assert_eq!(
        count,
        2,
        "Expected 2 PL304 diagnostics (one for foo in @EXPORT, one for bar in @EXPORT_OK). \
         Got {} PL304 diagnostics. All codes: {:?}",
        count,
        codes_for(source)
    );
}

/// Test 5: PL304 does NOT fire when the file does not use Exporter.
/// Only Exporter-based modules have a public API to document.
#[test]
fn pl304_not_fired_when_no_exporter_used() {
    let source = r#"
package My::Module;
use strict;

sub internal_helper { }

1;
"#;
    assert!(
        !has_code(source, "PL304"),
        "PL304 must NOT fire for files that don't use Exporter. \
         Got codes: {:?}",
        codes_for(source)
    );
}

/// Test 6: PL304 does NOT fire when ALL exported subroutines have POD.
#[test]
fn pl304_suppressed_when_all_exported_subs_have_pod() {
    let source = r#"
package My::Module;
use strict;
use warnings;
use Exporter 'import';
our @EXPORT = qw(foo bar);

sub foo { }
sub bar { }

=head2 foo

Returns a foo.

=head2 bar

Returns a bar.

=cut

1;
"#;
    assert!(
        !has_code(source, "PL304"),
        "PL304 must NOT fire when all exported subs have POD documentation. \
         Got codes: {:?}",
        codes_for(source)
    );
}

/// Test 7: PL304 fires only for undocumented exported subs when some are documented.
#[test]
fn pl304_fires_only_for_undocumented_exported_subs() {
    let source = r#"
package My::Module;
use strict;
use warnings;
use Exporter 'import';
our @EXPORT = qw(foo bar);

sub foo { }
sub bar { }

=head2 foo

Returns a foo.

=cut

1;
"#;
    // bar is exported and has no POD, so should get PL304
    // foo is documented so should NOT get PL304
    let count = count_code(source, "PL304");
    assert_eq!(
        count,
        1,
        "Expected exactly 1 PL304 diagnostic (for undocumented 'bar'). \
         Got {} PL304 diagnostics. All codes: {:?}",
        count,
        codes_for(source)
    );
}

/// Test 8: PL304 is suppressed when file has only POD and no exported subs.
#[test]
fn pl304_not_fired_when_no_exported_subs() {
    let source = r#"
package My::Module;
use strict;
use warnings;
use Exporter 'import';
our @EXPORT_OK = qw();  # empty export list

1;
"#;
    assert!(
        !has_code(source, "PL304"),
        "PL304 must NOT fire when there are no exported subroutines. \
         Got codes: {:?}",
        codes_for(source)
    );
}

/// Test 9: PL304 fires for multiple undocumented exported subs — one diagnostic per sub.
#[test]
fn pl304_one_diagnostic_per_undocumented_exported_sub() {
    let source = r#"
package My::Module;
use strict;
use warnings;
use Exporter 'import';
our @EXPORT = qw(alpha beta gamma);

sub alpha { }
sub beta { }
sub gamma { }

=head2 alpha

Returns alpha.

=cut

1;
"#;
    // beta and gamma are undocumented — should get 2 PL304 diagnostics
    let count = count_code(source, "PL304");
    assert_eq!(
        count,
        2,
        "Expected 2 PL304 diagnostics (for undocumented 'beta' and 'gamma'). \
         Got {} PL304 diagnostics. All codes: {:?}",
        count,
        codes_for(source)
    );
}

/// Test 10: PL304 is skipped for scripts (files with #! shebang).
#[test]
fn pl304_not_fired_for_scripts() {
    let source = r#"#!/usr/bin/env perl
use strict;
use warnings;
use Exporter 'import';
our @EXPORT = qw(tool_fn);

sub tool_fn { }

1;
"#;
    // Scripts are not modules — no expectation of POD coverage
    assert!(
        !has_code(source, "PL304"),
        "PL304 must NOT fire for scripts (files starting with #!). \
         Got codes: {:?}",
        codes_for(source)
    );
}
