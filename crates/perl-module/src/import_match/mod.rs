//! Single-line module-import match predicates.
//!
//! Determine whether a source line should be considered a target for
//! module-import rename rewrites.

use crate::boundary::contains_standalone_module_token;
use crate::import::{ModuleImportKind, parse_module_import_head};

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
