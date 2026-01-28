//! Import management code actions

use crate::ide::lsp_compat::code_actions::{CodeAction, CodeActionEdit, CodeActionKind};
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
    let imports = collect_imports(helpers.lines);
    if imports.len() <= 1 {
        return None;
    }

    // Sort imports: pragmas first, then core, then CPAN, then local
    let organized = sort_imports(imports);

    // Find the range of existing imports
    if let Some((start, end)) = find_imports_range(source, helpers.lines) {
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

/// Guess module for a function
pub fn guess_module_for_function(func: &str) -> Option<String> {
    match func {
        "dumper" => Some("Data::Dumper"),
        "encode" | "decode" => Some("Encode"),
        "basename" | "dirname" => Some("File::Basename"),
        "mkpath" | "rmtree" => Some("File::Path"),
        "slurp" => Some("File::Slurp"),
        "decode_json" | "encode_json" => Some("JSON"),
        _ => None,
    }
    .map(|s| s.to_string())
}

/// Collect all import statements
pub fn collect_imports(lines: &Vec<String>) -> Vec<String> {
    let mut imports = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("use ") || trimmed.starts_with("require ") {
            imports.push(line.clone());
        }
    }

    imports
}

/// Sort imports by category
pub fn sort_imports(imports: Vec<String>) -> Vec<String> {
    let mut pragmas = Vec::new();
    let mut core = Vec::new();
    let mut cpan = Vec::new();
    let mut local = Vec::new();

    for import in imports {
        if import.contains("strict")
            || import.contains("warnings")
            || import.contains("utf8")
            || import.contains("feature")
        {
            pragmas.push(import);
        } else if import.contains("::") {
            cpan.push(import);
        } else if import.starts_with("use lib") || import.contains("./") {
            local.push(import);
        } else {
            core.push(import);
        }
    }

    pragmas.sort();
    core.sort();
    cpan.sort();
    local.sort();

    let mut result = Vec::new();
    result.extend(pragmas);
    result.extend(core);
    result.extend(cpan);
    result.extend(local);

    result
}

/// Find the range of import statements
pub fn find_imports_range(source: &str, lines: &Vec<String>) -> Option<(usize, usize)> {
    let imports = collect_imports(lines);
    if imports.is_empty() {
        return None;
    }

    let first = source.find(&imports[0])?;
    let last = source.find(&imports[imports.len() - 1])?;
    let last_end = last + imports[imports.len() - 1].len();

    Some((first, last_end))
}
