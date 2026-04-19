//! Integration tests for same-file Moo/Moose role-conflict diagnostics.

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn pl303_diags(source: &str) -> Vec<Diagnostic> {
    diagnostics_for(source)
        .into_iter()
        .filter(|diag| diag.code.as_deref() == Some("PL303"))
        .collect()
}

#[test]
fn same_file_role_conflict_emits_pl303_at_with_reference() {
    let source = r#"
package MyApp::Consumer;
use strict;
use warnings;
use Moo;
with 'MyApp::Role::Printable', 'MyApp::Role::Auditable';

package MyApp::Role::Printable;
use strict;
use warnings;
use Moo::Role;
sub shared { 1 }
sub printable_only { 1 }

package MyApp::Role::Auditable;
use strict;
use warnings;
use Moo::Role;
sub shared { 2 }
sub auditable_only { 2 }
"#;

    let diags = pl303_diags(source);
    assert_eq!(
        diags.len(),
        1,
        "expected exactly one PL303 diagnostic: {diags:?}"
    );
    let diag = &diags[0];
    let anchor = source
        .find("with 'MyApp::Role::Printable', 'MyApp::Role::Auditable'")
        .unwrap();
    assert_eq!(
        diag.range.0, anchor,
        "PL303 should anchor at the `with` reference"
    );
    assert!(diag.message.contains("shared"));
    assert!(diag.message.contains("MyApp::Consumer"));
}

#[test]
fn class_defining_the_method_suppresses_pl303() {
    let source = r#"
package MyApp::Consumer;
use strict;
use warnings;
use Moo;
with 'MyApp::Role::Printable', 'MyApp::Role::Auditable';
sub shared { 42 }

package MyApp::Role::Printable;
use strict;
use warnings;
use Moo::Role;
sub shared { 1 }

package MyApp::Role::Auditable;
use strict;
use warnings;
use Moo::Role;
sub shared { 2 }
"#;

    assert!(
        pl303_diags(source).is_empty(),
        "class-defined method should suppress PL303"
    );
}

#[test]
fn distinct_role_methods_do_not_conflict() {
    let source = r#"
package MyApp::Consumer;
use strict;
use warnings;
use Moo;
with 'MyApp::Role::Printable', 'MyApp::Role::Auditable';

package MyApp::Role::Printable;
use strict;
use warnings;
use Moo::Role;
sub printable_only { 1 }

package MyApp::Role::Auditable;
use strict;
use warnings;
use Moo::Role;
sub auditable_only { 2 }
"#;

    assert!(
        pl303_diags(source).is_empty(),
        "distinct role methods should not produce PL303"
    );
}

#[test]
fn requires_does_not_count_as_a_provided_method() {
    let source = r#"
package MyApp::Consumer;
use strict;
use warnings;
use Moo;
with 'MyApp::Role::Printable', 'MyApp::Role::Auditable';

package MyApp::Role::Printable;
use strict;
use warnings;
use Moo::Role;
sub shared { 1 }

package MyApp::Role::Auditable;
use strict;
use warnings;
use Moo::Role;
requires 'shared';
"#;

    assert!(
        pl303_diags(source).is_empty(),
        "`requires` should not create a role-method conflict"
    );
}

#[test]
fn multiple_with_calls_accumulate_and_conflict() {
    let source = r#"
package MyApp::Consumer;
use strict;
use warnings;
use Moo;
with 'MyApp::RoleA';
with 'MyApp::RoleB';

package MyApp::RoleA;
use strict;
use warnings;
use Moo::Role;
sub clash { 1 }

package MyApp::RoleB;
use strict;
use warnings;
use Moo::Role;
sub clash { 2 }
"#;
    let diags = pl303_diags(source);
    assert!(
        !diags.is_empty(),
        "multiple separate `with` calls should still detect method conflicts: {diags:?}"
    );
}

#[test]
fn three_conflicting_roles_emit_single_pl303() {
    let source = r#"
package MyApp::Consumer;
use strict;
use warnings;
use Moo;
with 'MyApp::RoleA', 'MyApp::RoleB', 'MyApp::RoleC';

package MyApp::RoleA;
use strict;
use warnings;
use Moo::Role;
sub clash { 1 }

package MyApp::RoleB;
use strict;
use warnings;
use Moo::Role;
sub clash { 2 }

package MyApp::RoleC;
use strict;
use warnings;
use Moo::Role;
sub clash { 3 }
"#;
    let diags = pl303_diags(source);
    assert_eq!(
        diags.len(),
        1,
        "three roles providing the same method is still one conflict: {diags:?}"
    );
    assert!(
        diags[0].message.contains("clash"),
        "message should mention the method name"
    );
}

#[test]
fn role_consuming_roles_does_not_trigger_pl303() {
    // A role with its own `with` should not trigger PL303 since it is itself a role
    let source = r#"
package MyApp::RoleComposite;
use strict;
use warnings;
use Moo::Role;
with 'MyApp::RoleA', 'MyApp::RoleB';

package MyApp::RoleA;
use strict;
use warnings;
use Moo::Role;
sub clash { 1 }

package MyApp::RoleB;
use strict;
use warnings;
use Moo::Role;
sub clash { 2 }
"#;
    assert!(
        pl303_diags(source).is_empty(),
        "a role composing conflicting roles should not itself trigger PL303: {:?}",
        pl303_diags(source)
    );
}
