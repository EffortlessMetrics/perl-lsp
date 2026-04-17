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

/// Find undefined functions in the AST.
///
/// Walks the AST to collect function calls, then subtracts:
/// - Locally defined subroutines (`sub foo { ... }`)
/// - Explicitly imported names (`use Module qw(func)`)
/// - Perl built-in functions (`print`, `chomp`, etc.)
/// - Functions whose guessed module is already `use`d without an explicit import list
///
/// Known limitations:
/// - Cannot detect functions provided by `AUTOLOAD` (runtime mechanism)
/// - Cannot detect functions defined via `eval STRING` at runtime
/// - `use Module;` (no import list) is handled conservatively: functions that
///   map to an already-used module via `guess_module_for_function` are not flagged
pub fn find_undefined_functions(ast: &Node) -> Vec<String> {
    use perl_lexer::is_builtin;
    use perl_lsp_import_management::guess_module_for_function;
    use perl_parser_core::ast::NodeKind;
    use std::collections::HashSet;

    let mut function_calls: HashSet<String> = HashSet::new();
    let mut defined_subs: HashSet<String> = HashSet::new();
    let mut imported_names: HashSet<String> = HashSet::new();
    let mut used_modules_no_args: HashSet<String> = HashSet::new();

    fn walk(
        node: &Node,
        calls: &mut HashSet<String>,
        defs: &mut HashSet<String>,
        imports: &mut HashSet<String>,
        bare_uses: &mut HashSet<String>,
    ) {
        match &node.kind {
            NodeKind::FunctionCall { name, .. } => {
                // Skip qualified calls (e.g., Module::func)
                if !name.contains("::") {
                    calls.insert(name.clone());
                }
            }
            NodeKind::Subroutine { name: Some(n), .. } => {
                defs.insert(n.clone());
            }
            NodeKind::Use { module, args, .. } => {
                // Skip pragmas (lowercase first char)
                let is_module = module.chars().next().is_some_and(|c| c.is_ascii_uppercase());
                if is_module {
                    if args.is_empty() {
                        // Bare `use Module;` — can't know what's exported
                        bare_uses.insert(module.clone());
                    } else {
                        // Parse explicit import lists
                        for arg in args {
                            // Skip version numbers
                            let first_byte = arg.as_bytes().first().copied().unwrap_or(0);
                            if first_byte.is_ascii_digit() {
                                continue;
                            }
                            // Skip flag args (e.g., "-norequire")
                            if arg.starts_with('-') {
                                continue;
                            }
                            // Skip hash-ref style args
                            if arg.starts_with('{') {
                                continue;
                            }

                            if arg.starts_with("qw") {
                                let content = arg
                                    .trim_start_matches("qw")
                                    .trim_start_matches(|c: char| "([{/<|!".contains(c))
                                    .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                                for word in content.split_whitespace() {
                                    if !word.is_empty() {
                                        imports.insert(word.to_string());
                                    }
                                }
                            } else {
                                // Bare string arg: 'func' or "func"
                                let cleaned = arg.trim_matches(|c: char| c == '\'' || c == '"');
                                if !cleaned.is_empty() {
                                    imports.insert(cleaned.to_string());
                                }
                            }
                        }
                    }
                }
                // Handle `use constant NAME => value` — defines a callable name
                if module == "constant"
                    && let Some(first_arg) = args.first()
                {
                    let cleaned = first_arg.trim_matches(|c: char| c == '\'' || c == '"');
                    if !cleaned.is_empty()
                        && cleaned
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_uppercase() || c == '_')
                    {
                        defs.insert(cleaned.to_string());
                    }
                }
            }
            _ => {}
        }

        // Recurse into children
        node.for_each_child(|child| walk(child, calls, defs, imports, bare_uses));
    }

    walk(
        ast,
        &mut function_calls,
        &mut defined_subs,
        &mut imported_names,
        &mut used_modules_no_args,
    );

    // Filter: function_calls - defined_subs - imported_names - builtins
    // Also conservatively exclude functions whose guessed module is already used
    function_calls
        .into_iter()
        .filter(|name| {
            if defined_subs.contains(name) {
                return false;
            }
            if imported_names.contains(name) {
                return false;
            }
            if is_builtin(name) {
                return false;
            }
            // Conservative: if guess_module_for_function maps this to a bare-used module,
            // don't flag it (the module might export it via @EXPORT)
            if let Some(guessed_module) = guess_module_for_function(name)
                && used_modules_no_args.contains(&guessed_module)
            {
                return false;
            }
            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_tdd_support::must;

    #[test]
    fn undefined_function_detected() {
        // decode_json is not defined or imported — should be flagged
        let source = "use strict;\nmy $data = decode_json($text);";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let undef = find_undefined_functions(&ast);
        assert!(
            undef.contains(&"decode_json".to_string()),
            "Expected decode_json to be flagged as undefined, got: {:?}",
            undef
        );
    }

    #[test]
    fn builtin_not_flagged() {
        let source = "print 'hello';\nchomp $line;";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let undef = find_undefined_functions(&ast);
        assert!(undef.is_empty(), "builtins should not be flagged: {:?}", undef);
    }

    #[test]
    fn defined_sub_not_flagged() {
        let source = "sub greet { print 'hi'; }\ngreet();";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let undef = find_undefined_functions(&ast);
        assert!(
            !undef.contains(&"greet".to_string()),
            "defined sub should not be flagged: {:?}",
            undef
        );
    }

    #[test]
    fn explicitly_imported_function_not_flagged() {
        let source = "use JSON qw(decode_json);\nmy $d = decode_json($t);";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let undef = find_undefined_functions(&ast);
        assert!(
            !undef.contains(&"decode_json".to_string()),
            "explicitly imported function should not be flagged: {:?}",
            undef
        );
    }

    #[test]
    fn bare_use_module_conservative() {
        // `use JSON;` with no import list — can't know what's exported.
        // Since decode_json maps to JSON via guess_module_for_function,
        // and JSON is already used, don't flag it.
        let source = "use JSON;\nmy $d = decode_json($t);";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let undef = find_undefined_functions(&ast);
        assert!(
            !undef.contains(&"decode_json".to_string()),
            "functions from bare-used modules should not be flagged: {:?}",
            undef
        );
    }

    #[test]
    fn method_call_not_flagged() {
        let source = "my $obj = Foo->new();\n$obj->process();";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let undef = find_undefined_functions(&ast);
        assert!(undef.is_empty(), "method calls should not be flagged: {:?}", undef);
    }

    #[test]
    fn qualified_call_not_flagged() {
        let source = "File::Path::mkpath('/tmp/foo');";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let undef = find_undefined_functions(&ast);
        assert!(undef.is_empty(), "qualified calls should not be flagged: {:?}", undef);
    }

    #[test]
    fn empty_source_returns_empty() {
        let source = "";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let undef = find_undefined_functions(&ast);
        assert!(undef.is_empty());
    }
}
