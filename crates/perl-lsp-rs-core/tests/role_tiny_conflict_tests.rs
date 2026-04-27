//! Integration tests for PL303 — same-file role method conflicts with Role::Tiny.
//!
//! Verifies that the PL303 diagnostic fires for Role::Tiny role conflicts the
//! same way it already does for Moo/Moose role conflicts.

use std::sync::Arc;

use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn pl303_diags(source: &str) -> Vec<Diagnostic> {
    diagnostics_for(source).into_iter().filter(|d| d.code.as_deref() == Some("PL303")).collect()
}

#[test]
fn role_tiny_conflict_emits_pl303() {
    let source = r#"
package MyApp::Consumer;
use strict;
use warnings;
use Role::Tiny::With;
with 'MyApp::RoleA', 'MyApp::RoleB';

package MyApp::RoleA;
use strict;
use warnings;
use Role::Tiny;
sub shared_method { 'A' }

package MyApp::RoleB;
use strict;
use warnings;
use Role::Tiny;
sub shared_method { 'B' }
"#;

    let diags = pl303_diags(source);
    assert_eq!(diags.len(), 1, "Role::Tiny should detect method conflicts: {diags:?}");
    let diag = &diags[0];
    assert!(
        diag.message.contains("shared_method"),
        "conflict message should name the conflicting method: {}",
        diag.message
    );
    assert!(
        diag.message.contains("MyApp::Consumer"),
        "conflict message should name the consuming class: {}",
        diag.message
    );
}

#[test]
fn role_tiny_three_way_conflict_emits_pl303() {
    let source = r#"
package MyApp::Consumer;
use strict;
use warnings;
use Role::Tiny::With;
with 'RoleX', 'RoleY', 'RoleZ';

package RoleX;
use strict;
use warnings;
use Role::Tiny;
sub method { 'X' }

package RoleY;
use strict;
use warnings;
use Role::Tiny;
sub method { 'Y' }

package RoleZ;
use strict;
use warnings;
use Role::Tiny;
sub method { 'Z' }
"#;

    let diags = pl303_diags(source);
    assert_eq!(diags.len(), 1, "three-way Role::Tiny conflict should emit one PL303: {diags:?}");
    assert!(
        diags[0].message.contains("method"),
        "conflict message should name the conflicting method: {}",
        diags[0].message
    );
}

#[test]
fn role_tiny_class_method_suppresses_conflict() {
    let source = r#"
package MyApp::Consumer;
use strict;
use warnings;
use Role::Tiny::With;
with 'MyApp::RoleA', 'MyApp::RoleB';
sub shared_method { 42 }

package MyApp::RoleA;
use strict;
use warnings;
use Role::Tiny;
sub shared_method { 'A' }

package MyApp::RoleB;
use strict;
use warnings;
use Role::Tiny;
sub shared_method { 'B' }
"#;

    let diags = pl303_diags(source);
    assert!(
        diags.is_empty(),
        "class-defined method should suppress Role::Tiny conflict, got: {diags:?}"
    );
}
