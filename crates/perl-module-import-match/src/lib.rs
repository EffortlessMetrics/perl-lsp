//! Single-line module-import match predicates.
//!
//! This crate has one responsibility: determine whether a source line should
//! be considered a target for module-import rename rewrites.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_module_boundary::contains_standalone_module_token;
use perl_module_import::{ModuleImportKind, parse_module_import_head};

/// Return `true` when `line` references `module_name` in an import statement.
///
/// Matching behavior:
/// - `use Module::Name;` and `require Module::Name;` require exact head-token match.
/// - `use parent ...` and `use base ...` use boundary-aware token matching.
/// - Non-import lines return `false`.
#[must_use]
pub fn line_references_module_import(line: &str, module_name: &str) -> bool {
    if line.is_empty() || module_name.is_empty() {
        return false;
    }

    line.split(';').any(|statement| {
        let statement = statement.trim();
        if statement.is_empty() {
            return false;
        }

        let Some(parsed) = parse_module_import_head(statement) else {
            return false;
        };

        match parsed.kind {
            ModuleImportKind::Use | ModuleImportKind::Require => parsed.token == module_name,
            ModuleImportKind::UseParent | ModuleImportKind::UseBase => {
                contains_standalone_module_token(statement, module_name)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::line_references_module_import;

    #[test]
    fn matches_direct_use_and_require_statements() {
        assert!(line_references_module_import("use Foo::Bar;", "Foo::Bar"));
        assert!(line_references_module_import("require Foo::Bar;", "Foo::Bar"));
    }

    #[test]
    fn matches_parent_and_base_lines_with_boundary_checks() {
        assert!(line_references_module_import("use parent qw(Foo::Bar Other::Base);", "Foo::Bar"));
        assert!(line_references_module_import("use base 'Foo::Bar';", "Foo::Bar"));
    }

    #[test]
    fn rejects_partial_matches_for_direct_imports() {
        assert!(!line_references_module_import("use Foo::Barista;", "Foo::Bar"));
        assert!(!line_references_module_import("require Foo::Barista;", "Foo::Bar"));
    }

    #[test]
    fn rejects_non_import_contexts() {
        assert!(!line_references_module_import("my $x = Foo::Bar->new();", "Foo::Bar"));
        assert!(!line_references_module_import("package Foo::Bar;", "Foo::Bar"));
    }

    #[test]
    fn matches_later_import_statement_on_same_line() {
        assert!(line_references_module_import("use Foo::Bar; use Foo::Baz;", "Foo::Baz"));
        assert!(line_references_module_import("my $x = 1; require Foo::Bar;", "Foo::Bar"));
    }

    #[test]
    fn rejects_empty_inputs() {
        assert!(!line_references_module_import("", "Foo::Bar"));
        assert!(!line_references_module_import("use Foo::Bar;", ""));
    }
}
