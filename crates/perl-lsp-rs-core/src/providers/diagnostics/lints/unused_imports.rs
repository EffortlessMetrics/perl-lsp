//! Unused import detection lint
//!
//! Detects `use Module` statements where the module name does not appear
//! elsewhere in the source text. Pragmas and known implicit-export modules
//! are excluded to avoid false positives.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `unused-import` | Hint | Module appears to be unused |

use perl_diagnostics::codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};

use super::super::internal_types::{Diagnostic, DiagnosticTag, RelatedInformation};
use super::super::walker::walk_node;
use perl_diagnostics::codes::DiagnosticSeverity;

/// Pragmas that should never be flagged as unused (they operate via side effects).
const PRAGMA_SKIP_LIST: &[&str] = &[
    "strict",
    "warnings",
    "utf8",
    "feature",
    "constant",
    "lib",
    "parent",
    "base",
    "Exporter",
    "vars",
    "subs",
    "overload",
    "overloading",
    "integer",
    "bigint",
    "bignum",
    "bigrat",
    "bytes",
    "charnames",
    "encoding",
    "locale",
    "mro",
    "open",
    "ops",
    "re",
    "sigtrap",
    "sort",
    "threads",
    "threads::shared",
    "autodie",
    "autouse",
    "diagnostics",
    "English",
    "experimental",
    "fields",
    "filetest",
    "if",
    "less",
    "POSIX",
    // Namespace management — operate entirely via side effects on the stash.
    "namespace::autoclean",
    "namespace::clean",
    // MRO algorithm compatibility — no exported symbols.
    "MRO::Compat",
    // Syntax restriction pragmas (typically used as `no indirect`, etc.).
    "indirect",
    "multidimensional",
    "bareword::filehandles",
    // Perl built-in sub exports (Perl 5.36+).
    "builtin",
];

/// Modules that implicitly export symbols into the caller's namespace.
const IMPLICIT_EXPORT_SKIP_LIST: &[&str] = &[
    // --- OOP frameworks (export DSL keywords with bare `use`) ---
    "Moose",
    "Moose::Role",
    "Moose::Util::TypeConstraints",
    "MooseX::StrictConstructor",
    "MooseX::Types",
    "MooseX::ClassAttribute",
    "MooseX::Singleton",
    "MooseX::NonMoose",
    "Moo",
    "Moo::Role",
    "Mouse",
    "Mouse::Role",
    // Object::Pad exports `class`, `field`, `method`, `ADJUST`, `BUILD`, etc.
    "Object::Pad",
    // Role systems
    "Role::Tiny",
    "Role::Tiny::With",
    "Class::Method::Modifiers",
    // --- Modern syntax extensions (export keywords) ---
    // Syntax::Keyword::Try exports `try`, `catch`, `finally`.
    "Syntax::Keyword::Try",
    // Feature::Compat::Try is a compat shim that exports the same keywords.
    "Feature::Compat::Try",
    // --- Web / application frameworks ---
    "Modern::Perl",
    "Dancer",
    "Dancer2",
    "Catalyst",
    "Mojolicious",
    "Mojolicious::Lite",
    "Mojo::Base",
    // --- Test frameworks ---
    "Test::More",
    "Test::Most",
    "Test::Simple",
    "Test::Exception",
    "Test::Fatal",
    "Test::Warnings",
    "Test::Deep",
    "Test::Differences",
    "Test::Class",
    "Test::MockModule",
    "Test::MockObject",
    "Test::Spec",
    "Test::Builder",
    "Test2::V0",
    "Test2::Bundle::More",
    // --- Utility modules commonly used for side effects ---
    "Carp",
    "Carp::Always",
    "Scalar::Util",
    "List::Util",
    "List::MoreUtils",
    "Data::Dumper",
    "Try::Tiny",
    "Getopt::Long",
    "File::Basename",
    "FindBin",
    "Fcntl",
    "UNIVERSAL",
    // --- DBIx::Class ORM (exports table(), columns(), relationships, etc.) ---
    "DBIx::Class",
    "DBIx::Class::Core",
    "DBIx::Class::Schema",
];

/// Check for unused import statements.
///
/// Walks the AST to collect all `use Module` statements, then searches the
/// source text for references to each module name. If no reference is found
/// outside the `use` statement itself, a diagnostic hint is emitted.
pub fn check_unused_imports(node: &Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
    // Collect all use statements from the AST
    let mut use_statements: Vec<(String, usize, usize)> = Vec::new();

    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, .. } = &n.kind {
            use_statements.push((module.clone(), n.location.start, n.location.end));
        }
    });

    for (module, start, end) in &use_statements {
        let module = module.as_str();
        let start = *start;
        let end = *end;

        // Skip pragmas
        if PRAGMA_SKIP_LIST.contains(&module) {
            continue;
        }

        // Skip implicit exporters
        if IMPLICIT_EXPORT_SKIP_LIST.contains(&module) {
            continue;
        }

        // Skip version-like imports (e.g., `use 5.010;`)
        if module.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }

        // Skip if the use statement has explicit import arguments
        if has_use_args(node, module) {
            continue;
        }

        // Check if the module name appears elsewhere in the source
        if !is_module_referenced(source, module, start, end) {
            diagnostics.push(Diagnostic {
                range: (start, end),
                severity: DiagnosticSeverity::Hint,
                code: Some(DiagnosticCode::UnusedImport.as_str().to_string()),
                message: format!("Module '{}' appears to be unused", module),
                related_information: vec![RelatedInformation {
                    location: (start, end),
                    message: format!(
                        "If '{}' is imported for side effects, you can ignore this hint",
                        module
                    ),
                }],
                tags: vec![DiagnosticTag::Unnecessary],
                suggestion: Some(format!("Remove 'use {};' if it is not needed", module)),
            });
        }
    }
}

/// Returns true if the use statement for `target_module` has explicit import args.
fn has_use_args(node: &Node, target_module: &str) -> bool {
    let mut found = false;
    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, args, .. } = &n.kind
            && module == target_module
            && !args.is_empty()
        {
            found = true;
        }
    });
    found
}

/// Returns true if `module` appears in `source` outside the range `[use_start, use_end)`.
///
/// Uses word-boundary matching to avoid substring false positives (e.g.,
/// `FooBar` should not match a reference to `Foo`).
fn is_module_referenced(source: &str, module: &str, use_start: usize, use_end: usize) -> bool {
    let mut search_start = 0;
    while let Some(pos) = source[search_start..].find(module) {
        let abs_pos = search_start + pos;
        let match_end = abs_pos + module.len();

        // Skip the use statement itself
        if abs_pos >= use_start && abs_pos < use_end {
            search_start = match_end;
            continue;
        }

        // Check word boundaries
        let before_ok = abs_pos == 0 || {
            let ch = source.as_bytes()[abs_pos - 1];
            !ch.is_ascii_alphanumeric() && ch != b'_'
        };
        let after_ok = match_end >= source.len() || {
            let ch = source.as_bytes()[match_end];
            !ch.is_ascii_alphanumeric() && ch != b'_'
        };

        if before_ok && after_ok {
            return true;
        }

        search_start = match_end;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_tdd_support::{must, must_some};

    fn unused_import_diags(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let mut diags = Vec::new();
        check_unused_imports(&ast, source, &mut diags);
        diags
    }

    fn has_unused_import(diags: &[Diagnostic]) -> bool {
        diags.iter().any(|d| d.code.as_deref() == Some("PL700"))
    }

    // --- basic detection ---

    #[test]
    fn genuinely_unused_module_is_flagged() {
        let source = "use POSIX::Unused;\nmy $x = 1;\n";
        let diags = unused_import_diags(source);
        assert!(
            has_unused_import(&diags),
            "unused module should produce an unused-import diagnostic: {diags:?}"
        );
    }

    #[test]
    fn used_module_not_flagged() {
        let source = "use Project::Thing;\nmy $thing = Project::Thing->new;\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "module referenced in code should not be flagged: {diags:?}"
        );
    }

    // --- pragma skip list ---

    #[test]
    fn strict_pragma_not_flagged() {
        let source = "use strict;\nmy $x = 1;\n";
        let diags = unused_import_diags(source);
        assert!(!has_unused_import(&diags), "use strict should never be flagged: {diags:?}");
    }

    #[test]
    fn warnings_pragma_not_flagged() {
        let source = "use warnings;\nmy $x = 1;\n";
        let diags = unused_import_diags(source);
        assert!(!has_unused_import(&diags), "use warnings should never be flagged: {diags:?}");
    }

    #[test]
    fn utf8_pragma_not_flagged() {
        let source = "use utf8;\nmy $x = 1;\n";
        let diags = unused_import_diags(source);
        assert!(!has_unused_import(&diags), "use utf8 should never be flagged: {diags:?}");
    }

    #[test]
    fn feature_pragma_not_flagged() {
        let source = "use feature 'say';\nsay 'hello';\n";
        let diags = unused_import_diags(source);
        assert!(!has_unused_import(&diags), "use feature should never be flagged: {diags:?}");
    }

    // --- implicit export skip list ---

    #[test]
    fn moose_not_flagged() {
        let source = "use Moose;\nhas 'name' => (is => 'ro');\n";
        let diags = unused_import_diags(source);
        assert!(!has_unused_import(&diags), "use Moose should never be flagged: {diags:?}");
    }

    #[test]
    fn test_more_not_flagged() {
        let source = "use Test::More;\nok(1, 'passes');\n";
        let diags = unused_import_diags(source);
        assert!(!has_unused_import(&diags), "use Test::More should never be flagged: {diags:?}");
    }

    // --- explicit imports not flagged ---

    #[test]
    fn use_with_explicit_import_list_not_flagged() {
        let source = "use Project::Exports qw(project_value);\nmy $v = project_value();\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "use with explicit import list should not be flagged: {diags:?}"
        );
    }

    // --- version declarations not flagged ---

    #[test]
    fn version_use_not_flagged() {
        let source = "use 5.010;\nmy $x = 1;\n";
        let diags = unused_import_diags(source);
        assert!(!has_unused_import(&diags), "use 5.010 should not be flagged: {diags:?}");
    }

    // --- module used by qualified call ---

    #[test]
    fn module_used_by_qualified_call_not_flagged() {
        let source = "use File::Spec;\nmy $p = File::Spec->catfile('a', 'b');\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "module used via qualified call should not be flagged: {diags:?}"
        );
    }

    // --- diagnostic quality ---

    #[test]
    fn unused_import_diagnostic_names_the_module() {
        let source = "use SomeModule::Unused;\nmy $x = 1;\n";
        let diags = unused_import_diags(source);
        let diag = must_some(diags.iter().find(|d| d.code.as_deref() == Some("PL700")));
        assert!(
            diag.message.contains("SomeModule::Unused"),
            "diagnostic should name the module: {}",
            diag.message
        );
    }

    #[test]
    fn unused_import_has_unnecessary_tag() {
        let source = "use POSIX::Ghost;\nmy $x = 1;\n";
        let diags = unused_import_diags(source);
        let diag = must_some(diags.iter().find(|d| d.code.as_deref() == Some("PL700")));
        assert!(
            diag.tags.contains(&DiagnosticTag::Unnecessary),
            "unused-import should carry the Unnecessary tag"
        );
    }

    // --- namespace management pragmas ---

    #[test]
    fn namespace_autoclean_not_flagged() {
        let source = "use namespace::autoclean;\nhas 'x' => (is => 'ro');\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "namespace::autoclean operates via side effects and should never be flagged: {diags:?}"
        );
    }

    #[test]
    fn namespace_clean_not_flagged() {
        let source = "use namespace::clean;\nmy $x = 1;\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "namespace::clean operates via side effects and should never be flagged: {diags:?}"
        );
    }

    #[test]
    fn mro_compat_not_flagged() {
        let source = "use MRO::Compat;\n__PACKAGE__->mro::set_mro('c3');\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "MRO::Compat is a pragma and should never be flagged: {diags:?}"
        );
    }

    #[test]
    fn indirect_pragma_not_flagged() {
        let source = "no indirect;\nmy $obj = Foo->new;\n";
        let diags = unused_import_diags(source);
        assert!(!has_unused_import(&diags), "indirect pragma should never be flagged: {diags:?}");
    }

    #[test]
    fn multidimensional_pragma_not_flagged() {
        let source = "no multidimensional;\nmy %h = (a => 1);\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "multidimensional pragma should never be flagged: {diags:?}"
        );
    }

    #[test]
    fn bareword_filehandles_pragma_not_flagged() {
        let source = "no bareword::filehandles;\nopen my $fh, '<', $path;\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "bareword::filehandles pragma should never be flagged: {diags:?}"
        );
    }

    #[test]
    fn builtin_pragma_not_flagged() {
        let source = "use builtin 'weaken';\nweaken($ref);\n";
        let diags = unused_import_diags(source);
        assert!(!has_unused_import(&diags), "use builtin should never be flagged: {diags:?}");
    }

    // --- modern OOP systems ---

    #[test]
    fn object_pad_not_flagged() {
        let source = "use Object::Pad;\nclass Point {\n    field $x :param;\n}\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "Object::Pad exports class/field/method keywords and should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn role_tiny_not_flagged() {
        let source = "use Role::Tiny;\nrequires 'do_thing';\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "Role::Tiny exports requires/with and should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn role_tiny_with_not_flagged() {
        let source = "use Role::Tiny::With;\nwith 'Some::Role';\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "Role::Tiny::With exports with and should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn class_method_modifiers_not_flagged() {
        let source = "use Class::Method::Modifiers;\nbefore 'foo' => sub { ... };\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "Class::Method::Modifiers exports before/after/around and should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn moosex_class_attribute_not_flagged() {
        let source = "use MooseX::ClassAttribute;\nclass_has 'instances' => (is => 'ro');\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "MooseX::ClassAttribute exports class_has and should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn moosex_singleton_not_flagged() {
        let source = "use MooseX::Singleton;\nhas 'config' => (is => 'ro');\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "MooseX::Singleton operates through Moose metaclass side effects and should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn moosex_non_moose_not_flagged() {
        let source = "use MooseX::NonMoose;\nextends 'HTTP::Message';\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "MooseX::NonMoose operates through Moose metaclass side effects and should not be flagged: {diags:?}"
        );
    }

    // --- modern syntax extensions ---

    #[test]
    fn syntax_keyword_try_not_flagged() {
        let source = "use Syntax::Keyword::Try;\ntry { foo() } catch ($e) { warn $e }\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "Syntax::Keyword::Try exports try/catch/finally and should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn feature_compat_try_not_flagged() {
        let source = "use Feature::Compat::Try;\ntry { foo() } catch ($e) { warn $e }\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "Feature::Compat::Try is a compat shim and should not be flagged: {diags:?}"
        );
    }

    // --- DBIx::Class ORM ---

    #[test]
    fn dbix_class_not_flagged() {
        let source = "use DBIx::Class;\n__PACKAGE__->table('users');\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "DBIx::Class exports ORM DSL and should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn dbix_class_core_not_flagged() {
        let source = "use DBIx::Class::Core;\n__PACKAGE__->table('items');\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "DBIx::Class::Core exports ORM DSL and should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn dbix_class_schema_not_flagged() {
        let source = "use DBIx::Class::Schema;\n__PACKAGE__->load_namespaces;\n";
        let diags = unused_import_diags(source);
        assert!(
            !has_unused_import(&diags),
            "DBIx::Class::Schema exports schema DSL and should not be flagged: {diags:?}"
        );
    }
}
