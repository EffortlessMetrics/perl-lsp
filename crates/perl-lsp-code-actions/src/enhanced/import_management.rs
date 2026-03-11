//! Import management code actions

use crate::types::{CodeAction, CodeActionEdit, CodeActionKind};
use perl_lsp_import_management::{
    collect_imports, find_imports_range, guess_module_for_function, sort_imports,
};
use perl_lsp_rename::TextEdit;
use perl_parser_core::ast::{Node, SourceLocation};

use super::helpers::Helpers;

/// Add missing imports for undefined functions
pub fn add_missing_imports(ast: &Node, _source: &str, helpers: &Helpers<'_>) -> Option<CodeAction> {
    let undefined = find_undefined_functions(ast);
    if undefined.is_empty() {
        return None;
    }

    let mut imports = Vec::new();

    // Map common functions to their modules
    for func in &undefined {
        if let Some(module) = guess_module_for_function(func) {
            imports.push(format!("use {};", module));
        }
    }

    if imports.is_empty() {
        return None;
    }

    // Find insert position (after shebang and existing pragmas)
    let insert_pos = helpers.find_import_insert_position();

    Some(CodeAction {
        title: "Add missing imports".to_string(),
        kind: CodeActionKind::QuickFix,
        diagnostics: Vec::new(),
        edit: CodeActionEdit {
            changes: vec![TextEdit {
                location: SourceLocation { start: insert_pos, end: insert_pos },
                new_text: format!("{}\n", imports.join("\n")),
            }],
        },
        is_preferred: false,
    })
}

/// Organize import statements
pub fn organize_imports(_ast: &Node, source: &str, helpers: &Helpers<'_>) -> Option<CodeAction> {
    let imports = collect_imports(helpers.lines());
    if imports.len() <= 1 {
        return None;
    }

    // Sort imports: pragmas first, then core, then CPAN, then local
    let organized = sort_imports(imports);

    // Find the range of existing imports
    if let Some((start, end)) = find_imports_range(source, helpers.lines()) {
        return Some(CodeAction {
            title: "Organize imports".to_string(),
            kind: CodeActionKind::SourceOrganizeImports,
            diagnostics: Vec::new(),
            edit: CodeActionEdit {
                changes: vec![TextEdit {
                    location: SourceLocation { start, end },
                    new_text: organized.join("\n") + "\n",
                }],
            },
            is_preferred: false,
        });
    }

    None
}

/// Find undefined functions in the AST
pub fn find_undefined_functions(_ast: &Node) -> Vec<String> {
    // This would require full semantic analysis
    // For now, return empty
    Vec::new()
}
