//! Workspace symbol completion for Perl
//!
//! Provides completion for symbols from other files in the workspace using the workspace index.
//! Includes module name completion for `use`/`require` statements, workspace-aware method
//! completion for `->` expressions, and general cross-file symbol completion.

use super::{
    auto_import,
    context::CompletionContext,
    items::{CompletionItem, CompletionItemKind},
};
use perl_semantic_analyzer::type_inference::{PerlType, TypeInferenceEngine};
use perl_workspace::workspace_index::{
    SymbolKind as WsSymbolKind, VarKind, WorkspaceIndex, WorkspaceSymbol,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

/// Add workspace symbol completions for functions and variables
///
/// Queries the workspace index to provide completions for symbols from other files.
/// Uses the `import_map` to promote imported symbols and downrank explicitly
/// not-imported symbols for import-aware sort ordering.
pub fn add_workspace_symbol_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
    import_map: &HashMap<String, HashSet<String>>,
) {
    // Only proceed if we have a workspace index
    let Some(index) = workspace_index else {
        return;
    };

    // Only provide workspace completions if there's a reasonable prefix
    // to avoid overwhelming the user with all workspace symbols
    if context.prefix.is_empty() {
        return;
    }

    // Check if the workspace index has any symbols
    if !index.has_symbols() {
        return;
    }

    // Search for symbols matching the prefix
    let matching_symbols = index.find_symbols(&context.prefix);

    for symbol in matching_symbols {
        // Skip symbols that don't match the prefix
        if !symbol.name.starts_with(&context.prefix)
            && !symbol.qualified_name.as_ref().is_some_and(|qn| qn.contains(&context.prefix))
        {
            continue;
        }

        match symbol.kind {
            WsSymbolKind::Subroutine | WsSymbolKind::Method => {
                // Determine sort priority and detail based on import map
                let label = symbol.qualified_name.as_ref().unwrap_or(&symbol.name).clone();
                let module = symbol.container_name.as_deref().unwrap_or("");

                let (sort_prefix, detail) = match import_map.get(module) {
                    None => {
                        // Module not in import_map: not used or `use Module` (import all).
                        // Rank at tier 4 (after core builtins at tier 3).
                        let det = symbol
                            .container_name
                            .clone()
                            .unwrap_or_else(|| "workspace".to_string());
                        ("4_", det)
                    }
                    Some(imported_set) if imported_set.is_empty() => {
                        // Explicit empty import `use Module qw()` — not in namespace.
                        // Rank at tier 5 (lowest, after all useful completions).
                        ("5_", "not imported".to_string())
                    }
                    Some(imported_set) if imported_set.contains(&symbol.name) => {
                        // Symbol is explicitly imported — boost priority to tier 2
                        // (treated like a file-scope symbol).
                        let det = format!("imported from {module}");
                        ("2_", det)
                    }
                    Some(_) => {
                        // Module used with explicit list, but this symbol wasn't in it.
                        // Rank at tier 4 (workspace, after core builtins).
                        let det = symbol
                            .container_name
                            .clone()
                            .unwrap_or_else(|| "workspace".to_string());
                        ("4_", det)
                    }
                };

                completions.push(CompletionItem {
                    insert_text: Some(symbol.name.clone()),
                    sort_text: Some(format!("{sort_prefix}{label}")),
                    filter_text: Some(label.clone()),
                    label,
                    kind: CompletionItemKind::Function,
                    detail: Some(detail),
                    documentation: symbol.documentation.clone(),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: None,
                });
            }
            WsSymbolKind::Variable(var_kind) => {
                // Add variable completion with appropriate sigil
                let sigil = match var_kind {
                    VarKind::Scalar => "$",
                    VarKind::Array => "@",
                    VarKind::Hash => "%",
                };

                let label = if let Some(ref qname) = symbol.qualified_name {
                    format!("{}{}", sigil, qname)
                } else {
                    format!("{}{}", sigil, symbol.name)
                };

                // Only suggest if the prefix matches (considering sigil)
                if !label.starts_with(&context.prefix) {
                    continue;
                }

                completions.push(CompletionItem {
                    insert_text: Some(label.clone()),
                    sort_text: Some(format!("4_{}", label)), // Tier 4: after core builtins
                    filter_text: Some(label.clone()),
                    label,
                    kind: CompletionItemKind::Variable,
                    detail: symbol.container_name.clone().or_else(|| Some("workspace".to_string())),
                    documentation: symbol.documentation.clone(),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: None,
                });
            }
            WsSymbolKind::Package => {
                // Add package completion — tier 4 (workspace, after core builtins)
                let name = &symbol.name;
                completions.push(CompletionItem {
                    label: name.clone(),
                    kind: CompletionItemKind::Module,
                    detail: Some("package".to_string()),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(name.clone()),
                    sort_text: Some(format!("4_{name}")),
                    filter_text: Some(name.clone()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: None,
                });
            }
            WsSymbolKind::Constant => {
                // Add constant completion — tier 4 (workspace, after core builtins)
                let name = &symbol.name;
                completions.push(CompletionItem {
                    label: name.clone(),
                    kind: CompletionItemKind::Constant,
                    detail: symbol.container_name.clone().or_else(|| Some("workspace".to_string())),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(name.clone()),
                    sort_text: Some(format!("4_{name}")),
                    filter_text: Some(name.clone()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: None,
                });
            }
            WsSymbolKind::Export => {
                // Add exported symbol completion
                let name = &symbol.name;
                completions.push(CompletionItem {
                    label: name.clone(),
                    kind: CompletionItemKind::Function,
                    detail: Some("exported".to_string()),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(name.clone()),
                    sort_text: Some(format!("2_{name}")), // Prioritize exports
                    filter_text: Some(name.clone()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: None,
                });
            }
            _ => {
                // Skip other symbol types
            }
        }
    }
}

/// Ultra-common Perl pragmas and core modules that should surface first in `use` completions.
///
/// Tier 0: always-used pragmas and critical infrastructure modules.
const COMMON_MODULES_TIER_0: &[&str] = &[
    "strict",
    "warnings",
    "Carp",
    "Exporter",
    "File::Path",
    "File::Spec",
    "List::Util",
    "Scalar::Util",
    "Data::Dumper",
    "JSON",
    "POSIX",
    "Getopt::Long",
];

/// Common CPAN modules that are frequently used but less universal than tier-0.
///
/// Tier 1: widely-used libraries (DB, OOP, testing, filesystem).
const COMMON_MODULES_TIER_1: &[&str] =
    &["DBI", "Moo", "Moose", "Try::Tiny", "Path::Tiny", "Test::More", "Test::Exception"];

/// Returns the sort-text tier prefix for a module name.
///
/// Returns `"0"` for tier-0 (ultra-common), `"1"` for tier-1 (common), and `"9"` for
/// all other modules so they sort after the well-known ones.
fn module_sort_tier(name: &str) -> &'static str {
    if COMMON_MODULES_TIER_0.contains(&name) {
        "0"
    } else if COMMON_MODULES_TIER_1.contains(&name) {
        "1"
    } else {
        "9"
    }
}

/// Add module name completions for `use` and `require` statements.
///
/// When the cursor is after `use ` or `require `, suggests package names from the
/// workspace index. This enables discovering available modules as you type.
///
/// For example, typing `use My` will suggest `MyApp`, `MyApp::Config`, etc.
pub fn add_use_module_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
    include_paths: &[PathBuf],
    system_inc_paths: &[PathBuf],
) {
    let _ = (include_paths, system_inc_paths);
    let Some(index) = workspace_index else {
        return;
    };

    if !index.has_symbols() {
        return;
    }

    let mut seen: HashSet<String> = HashSet::new();

    // Search for package symbols matching the prefix
    let all_symbols = if context.prefix.is_empty() {
        index.all_symbols()
    } else {
        index.find_symbols(&context.prefix)
    };

    for symbol in all_symbols {
        if symbol.kind != WsSymbolKind::Package {
            continue;
        }

        // Match against the module name prefix
        if !context.prefix.is_empty() && !symbol.name.starts_with(&context.prefix) {
            continue;
        }

        if !seen.insert(symbol.name.clone()) {
            continue;
        }

        let name = &symbol.name;
        completions.push(CompletionItem {
            label: name.clone(),
            kind: CompletionItemKind::Module,
            detail: Some("module".to_string()),
            documentation: symbol
                .documentation
                .clone()
                .or_else(|| Some(format!("Package `{name}`"))),
            insert_text: Some(name.clone()),
            sort_text: Some(format!("1{}_{name}", module_sort_tier(name))),
            filter_text: Some(name.clone()),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
            commit_characters: None,
        });
    }
}

/// Add import completions for symbols inside `use Module qw(...)`.
///
/// When the cursor is inside the `qw()` import list of a `use` statement,
/// queries the workspace index for symbols exported by or defined in that
/// module and suggests matching function/variable/constant names.
///
/// For example, typing `use File::Basename qw(bas` will suggest `basename`,
/// `fileparse`, `dirname`, etc.
pub fn add_use_qw_import_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
    module_name: &str,
    qw_prefix: &str,
) {
    let Some(index) = workspace_index else {
        return;
    };

    if !index.has_symbols() {
        return;
    }

    let mut seen: HashSet<&str> = HashSet::new();
    let members = index.get_package_members(module_name);

    for symbol in &members {
        match symbol.kind {
            WsSymbolKind::Subroutine
            | WsSymbolKind::Method
            | WsSymbolKind::Export
            | WsSymbolKind::Constant => {}
            _ => continue,
        }

        // Filter by prefix typed inside qw()
        if !qw_prefix.is_empty() && !symbol.name.starts_with(qw_prefix) {
            continue;
        }

        // Deduplicate
        if !seen.insert(&symbol.name) {
            continue;
        }

        let kind_label = match symbol.kind {
            WsSymbolKind::Constant => "constant",
            WsSymbolKind::Export => "exported",
            _ => "function",
        };

        let name = &symbol.name;
        completions.push(CompletionItem {
            label: name.clone(),
            kind: match symbol.kind {
                WsSymbolKind::Constant => CompletionItemKind::Constant,
                _ => CompletionItemKind::Function,
            },
            detail: Some(format!("{module_name} {kind_label}")),
            documentation: symbol
                .documentation
                .clone()
                .or_else(|| Some(format!("`{module_name}::{name}`"))),
            insert_text: Some(name.clone()),
            sort_text: Some(format!("1_{name}")),
            filter_text: Some(name.clone()),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
            commit_characters: None,
        });
    }
}

/// Infer the package type of a `->` receiver from the source context.
///
/// Looks for patterns like `My::Package->method` (static call) or attempts to
/// find the type from variable assignment context like `my $obj = My::Package->new`.
fn infer_receiver_package(context: &CompletionContext, source: &str) -> Option<String> {
    let arrow_prefix = context.prefix.trim_end_matches("->");

    // Case 1: Static method call like `My::Package->meth` or `Package->meth`
    // The prefix already contains the package name (starts with uppercase, no sigil)
    if !arrow_prefix.starts_with('$')
        && !arrow_prefix.starts_with('@')
        && !arrow_prefix.starts_with('%')
        && arrow_prefix.chars().next().is_some_and(|c| c.is_ascii_uppercase())
    {
        return Some(arrow_prefix.to_string());
    }

    // Case 3: Self-call inside a method — `$self->` or `$this->` resolves to the
    // current package. Standard Perl OO convention: the invocant of `bless` is
    // assigned to `$self` (or `$this`) via `my $self = shift`.  The RHS is just
    // `shift`, so Case 2 would not match any constructor pattern.  Instead we
    // fall back to `context.current_package` which the context analyser already
    // sets correctly from the surrounding `package` declaration.
    if matches!(arrow_prefix, "$self" | "$this")
        && !context.current_package.is_empty()
        && context.current_package != "main"
    {
        return Some(context.current_package.clone());
    }

    // Case 2: Variable method call like `$obj->meth`
    // Try to find the type from assignment context
    if arrow_prefix.starts_with('$') {
        let var_name = arrow_prefix;
        // Look for assignment pattern: `my $var = Package->new`
        // Search backwards in source for the variable assignment
        let before = &source[..context.position.min(source.len())];

        // Find the most recent assignment to this variable
        for line in before.lines().rev() {
            let trimmed = line.trim();
            // Match patterns like: `my $var = Package::Name->new(...)`
            // or `$var = Package::Name->new(...)`
            // We need a single `=` that is not part of `==`, `!=`, `<=`, `>=`, `=~`.
            let assign_pos = find_assignment_eq(trimmed);
            if let Some(assign_pos) = assign_pos {
                let lhs = trimmed[..assign_pos].trim();
                if lhs.ends_with(var_name) || lhs.contains(&format!("{var_name} ")) {
                    let rhs = trimmed[assign_pos + 1..].trim();
                    // Extract package name from `Package::Name->new(...)` pattern
                    if let Some(arrow_pos) = rhs.find("->") {
                        let pkg = rhs[..arrow_pos].trim();
                        if pkg.contains("::")
                            || pkg.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                        {
                            return Some(pkg.to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

fn infer_receiver_package_from_type_engine(
    context: &CompletionContext,
    type_engine: Option<&TypeInferenceEngine>,
) -> Option<String> {
    let arrow_prefix = context.prefix.trim_end_matches("->");
    let var_name = arrow_prefix.strip_prefix('$')?;
    let ty = type_engine?.get_type_at(var_name)?;

    match ty {
        PerlType::Object(class) => Some(class),
        PerlType::Reference(inner) => match inner.as_ref() {
            PerlType::Object(class) => Some(class.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Add method completions from the workspace index for `->` expressions.
///
/// When the user types `$obj->` or `Package->`, queries the workspace index for
/// methods defined in the receiver's package and suggests them.
///
/// Auto-import edits are attached when the receiver package is not yet imported.
pub fn add_workspace_method_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    type_engine: Option<&TypeInferenceEngine>,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
) {
    let Some(index) = workspace_index else {
        return;
    };

    if !index.has_symbols() {
        return;
    }

    let package_name = infer_receiver_package_from_type_engine(context, type_engine)
        .or_else(|| infer_receiver_package(context, source));

    let Some(package_name) = package_name else {
        return;
    };

    // Collect labels already present to avoid duplicates with local method completions
    let existing_labels: HashSet<String> =
        completions.iter().map(|item| item.label.clone()).collect();

    let method_prefix = context.prefix.rsplit("->").next().unwrap_or("");

    // Collect all methods from the receiver package AND its ancestor chain (parents + roles).
    // Child methods take priority: collect_all_package_members deduplicates by keeping the
    // first occurrence (closest to the receiver in the BFS order).
    let members = collect_all_package_members(index, &package_name);

    // Build an auto-import edit once for all methods from this package.
    let auto_import_edit = auto_import::build_auto_import_edit(source, &package_name);

    for symbol in members {
        match symbol.kind {
            WsSymbolKind::Subroutine | WsSymbolKind::Method => {}
            _ => continue,
        }

        if !method_prefix.is_empty() && !symbol.name.starts_with(method_prefix) {
            continue;
        }

        // Skip if already provided by local method completion
        if existing_labels.contains(&symbol.name) {
            continue;
        }

        let additional_edits =
            auto_import_edit.as_ref().map(|e| vec![e.clone()]).unwrap_or_default();

        // Show which package actually defines the method for inherited completions
        let defining_pkg = symbol.container_name.as_deref().unwrap_or(package_name.as_str());
        let detail = if defining_pkg == package_name {
            format!("{package_name} method")
        } else {
            format!("{package_name} method (from {defining_pkg})")
        };

        // Own-class methods rank above inherited: tier 2 for own, tier 3 for inherited.
        // This ensures $obj->zoom (own) sorts before $obj->abstract_method (inherited)
        // even when the own method name is alphabetically after the inherited name.
        let method_tier = if defining_pkg == package_name { "2" } else { "3" };

        completions.push(CompletionItem {
            label: symbol.name.clone(),
            kind: CompletionItemKind::Function,
            detail: Some(detail),
            documentation: symbol.documentation.clone().or_else(|| {
                Some(format!("Method `{}::{}` from workspace index.", defining_pkg, symbol.name))
            }),
            insert_text: Some(format!("{}()", symbol.name)),
            sort_text: Some(format!("{method_tier}_{}", symbol.name)), // tier 2=own, 3=inherited, after local (tier 1)
            filter_text: Some(symbol.name.clone()),
            additional_edits,
            text_edit_range: Some((context.prefix_start, context.position)),
            commit_characters: None,
        });
    }
}

/// Collect all method symbols accessible from a package, following parent/role chains.
///
/// Performs BFS over the inheritance graph starting at `package_name`, collecting
/// subroutine and method symbols from each package in the resolution order.
/// Child-defined methods shadow parent methods — the first occurrence of each name wins.
///
/// Edge-case handling:
/// - Diamond inheritance: BFS visited-set prevents duplicate traversal.
/// - Circular `@ISA`: visited-set prevents infinite loops.
/// - Package not indexed: `get_package_members` returns `Vec::new()` gracefully.
/// - `use parent -norequire`: already handled by `ClassModelBuilder`; model.parents
///   contains the parent names regardless.
///
/// NOTE: C3 MRO ordering is NOT honoured — this uses BFS (breadth-first), which
/// approximates but does not exactly match C3 for complex diamond hierarchies.
/// This is a pre-existing approximation shared with `navigation.rs`. A follow-up
/// issue should address strict C3 ordering if it becomes important (see issue #3482).
fn collect_all_package_members(index: &WorkspaceIndex, package_name: &str) -> Vec<WorkspaceSymbol> {
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut result: Vec<WorkspaceSymbol> = Vec::new();

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    // Cache of package_name → list of parent/role package names, populated lazily.
    let mut related_cache: HashMap<String, Vec<String>> = HashMap::new();

    // Collect related packages (parents + roles) for a given package by parsing
    // its source file from the workspace index or filesystem.
    let collect_related = |pkg: &str, cache: &mut HashMap<String, Vec<String>>| -> Vec<String> {
        cache
            .entry(pkg.to_string())
            .or_insert_with(|| {
                let Some(pkg_location) = index.find_definition(pkg) else {
                    return Vec::new();
                };

                let text = index.document_store().get_text(&pkg_location.uri).or_else(|| {
                    perl_workspace::workspace_index::uri_to_fs_path(&pkg_location.uri)
                        .and_then(|path| std::fs::read_to_string(path).ok())
                });

                let Some(text) = text else {
                    return Vec::new();
                };

                let mut parser = perl_semantic_analyzer::Parser::new(&text);
                let Ok(ast) = parser.parse() else {
                    return Vec::new();
                };

                perl_semantic_analyzer::semantic::SemanticAnalyzer::analyze_with_source(&ast, &text)
                    .class_models
                    .into_iter()
                    .find(|model| model.name == pkg)
                    .map(|model| {
                        model.parents.iter().chain(model.roles.iter()).cloned().collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .clone()
    };

    // Start with the receiver package itself
    queue.push_back(package_name.to_string());
    visited.insert(package_name.to_string());

    while let Some(pkg) = queue.pop_front() {
        // Collect direct members for this package
        let members = index.get_package_members(&pkg);
        for symbol in members {
            // Only include subroutines and methods
            match symbol.kind {
                WsSymbolKind::Subroutine | WsSymbolKind::Method => {}
                _ => continue,
            }
            // Child wins: skip if a closer ancestor already provided this name
            if seen_names.insert(symbol.name.clone()) {
                result.push(symbol);
            }
        }

        // Enqueue ancestor packages
        let related = collect_related(&pkg, &mut related_cache);
        for ancestor in related {
            if visited.insert(ancestor.clone()) {
                queue.push_back(ancestor);
            }
        }
    }

    result
}

/// Find the position of a single assignment `=` in a line, skipping compound
/// operators like `==`, `!=`, `<=`, `>=`, `=~`, and `=>`.
///
/// Returns `None` if no assignment operator is found.
fn find_assignment_eq(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'=' {
            continue;
        }
        // Skip if preceded by !, <, >, or = (compound operators)
        if i > 0 && matches!(bytes[i - 1], b'!' | b'<' | b'>' | b'=') {
            continue;
        }
        // Skip if followed by = or ~ or > (==, =~, =>)
        if i + 1 < bytes.len() && matches!(bytes[i + 1], b'=' | b'~' | b'>') {
            continue;
        }
        return Some(i);
    }
    None
}
