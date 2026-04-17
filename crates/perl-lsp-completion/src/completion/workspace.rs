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
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
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

/// Performance guard constants for include path scanning.
const SCAN_MAX_DEPTH: usize = 8;
const SCAN_MAX_ENTRIES_PER_PATH: usize = 20;

/// Convert a file path to a Perl module name.
///
/// Examples:
/// - `DBI.pm` → `Some("DBI")`
/// - `lib/DBD/MySQL.pm` → `Some("DBD::MySQL")`
/// - `lib/perl5/vendor_perl/Foo/Bar.pm` → `Some("Foo::Bar")` (strips vendor_perl)
/// - `some/nested/path.pm` → `Some("some::nested::path")`
///
/// Returns `None` if the file doesn't have a `.pm` extension or the module name
/// cannot be derived (e.g., empty path after stripping).
fn path_to_module_name(path: &Path, base_path: &Path) -> Option<String> {
    // Get the file stem (filename without extension)
    let file_stem = path.file_stem()?;
    let module_name = file_stem.to_str()?;

    // Get the parent directory relative to the base include path
    let parent = path.parent()?;
    let relative = parent.strip_prefix(base_path).ok()?;

    // Build the module name from path components
    let mut parts: Vec<&str> = Vec::new();

    // Split the relative path into components
    for component in relative.components() {
        match component {
            std::path::Component::Normal(name) => {
                let name_str = name.to_str()?;
                // Skip common Perl library directories
                if name_str == "lib" || name_str == "perl5" {
                    continue;
                }
                // Skip vendor_perl, site_perl, archlib components
                if name_str == "vendor_perl" || name_str == "site_perl" || name_str == "archlib" {
                    continue;
                }
                parts.push(name_str);
            }
            std::path::Component::ParentDir => {
                // Skip parent directory references
                continue;
            }
            _ => {
                // Skip other components (Prefix, RootDir, CurDir, etc.)
                continue;
            }
        }
    }

    // If we have path components, join them with ::
    if parts.is_empty() {
        // Module is directly in the include path root (e.g., DBI.pm at /path/DBI.pm)
        Some(module_name.to_string())
    } else {
        // Module is in a subdirectory (e.g., DBD/MySQL.pm at /path/DBD/MySQL.pm)
        parts.push(module_name);
        Some(parts.join("::"))
    }
}

/// Scan a single directory for `.pm` files matching a prefix.
///
/// Uses WalkDir with depth limit (8) and entry limit (20 per path) to prevent
/// excessive filesystem traversal. Results are cached in `include_path_cache`.
///
/// # Arguments
///
/// * `dir` - The directory to scan
/// * `prefix` - The module name prefix to filter on (e.g., `"DB"` matches `"DBI"`)
/// * `seen` - Modules already found (used for deduplication within this scan)
/// * `is_cancelled` - Cancellation callback
///
/// Returns a `Vec` of module names found in this directory.
#[cfg(not(target_arch = "wasm32"))]
fn scan_directory_for_modules<F>(
    dir: &Path,
    prefix: &str,
    seen: &mut HashSet<String>,
    is_cancelled: &F,
) -> Vec<String>
where
    F: Fn() -> bool + ?Sized,
{
    use walkdir::WalkDir;

    let mut results = Vec::new();

    // Create a WalkDir iterator with depth and entry limits
    let walker = WalkDir::new(dir).max_depth(SCAN_MAX_DEPTH).follow_links(false);

    for (entries_examined, entry) in walker
        .into_iter()
        .filter_map(|e: Result<walkdir::DirEntry, walkdir::Error>| e.ok())
        .enumerate()
    {
        // Check for cancellation periodically
        if entries_examined.is_multiple_of(10) && is_cancelled() {
            break;
        }

        // Enforce entry limit per path
        if results.len() >= SCAN_MAX_ENTRIES_PER_PATH {
            break;
        }

        // Only process files with .pm extension
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let extension = path.extension().and_then(|e: &std::ffi::OsStr| e.to_str());
        if extension != Some("pm") {
            continue;
        }

        // Convert path to module name
        let Some(module_name) = path_to_module_name(path, dir) else {
            continue;
        };

        // Filter by prefix - module name must start with the prefix
        if !prefix.is_empty() && !module_name.starts_with(prefix) {
            continue;
        }

        // Skip if already seen (handles deduplication across multiple paths)
        if !seen.insert(module_name.clone()) {
            continue;
        }

        results.push(module_name);
    }

    results
}

/// WASM32 fallback: no-op directory scanning.
#[cfg(target_arch = "wasm32")]
fn scan_directory_for_modules<F>(
    _dir: &Path,
    _prefix: &str,
    _seen: &mut HashSet<String>,
    _is_cancelled: &F,
) -> Vec<String>
where
    F: Fn() -> bool + ?Sized,
{
    Vec::new()
}

/// Scan all include paths for modules matching a prefix.
///
/// This function iterates through the configured include paths and scans
/// each directory for `.pm` files. Results are cached per-directory in
/// `include_path_cache` to avoid repeated filesystem scans.
///
/// # Arguments
///
/// * `prefix` - The module name prefix to filter on (e.g., `"DB"` matches `"DBI"`)
/// * `include_paths` - List of include path directories to scan
/// * `cache` - Per-directory cache of discovered module names
/// * `seen` - Modules already found (used for deduplication across paths)
/// * `is_cancelled` - Cancellation callback
///
/// Returns a `Vec` of unique module names found across all include paths.
#[cfg(not(target_arch = "wasm32"))]
fn scan_include_paths_for_modules(
    prefix: &str,
    include_paths: &[&PathBuf],
    cache: &RefCell<HashMap<PathBuf, Vec<String>>>,
    seen: &HashSet<String>,
    is_cancelled: &dyn Fn() -> bool,
) -> Vec<String> {
    let mut results = Vec::new();

    for inc_path in include_paths {
        // Check for cancellation before each path
        if is_cancelled() {
            break;
        }

        // Skip non-existent directories
        if !inc_path.exists() || !inc_path.is_dir() {
            continue;
        }

        // Check cache first
        let cached = {
            let cache_guard = cache.borrow();
            // Iterate through cache to find a matching path
            // inc_path is &&PathBuf, dereference to &PathBuf for comparison
            cache_guard.iter().find(|(k, _)| **k == **inc_path).map(|(_, v)| v.clone())
        };

        let modules = if let Some(cached_modules) = cached {
            cached_modules
        } else {
            // Scan the directory and cache results
            let mut temp_seen: HashSet<String> = HashSet::new();
            let scanned = scan_directory_for_modules(inc_path, "", &mut temp_seen, is_cancelled);

            // Store in cache
            let mut cache_guard = cache.borrow_mut();
            // inc_path is &&PathBuf, dereference twice to get PathBuf for the insert() call
            cache_guard.insert((**inc_path).clone(), scanned.clone());

            scanned
        };

        // Filter by prefix and add to results
        for module_name in modules {
            // Skip if already in the global `seen` set (workspace priority)
            if seen.contains(&module_name) {
                continue;
            }

            // Filter by prefix
            if !prefix.is_empty() && !module_name.starts_with(prefix) {
                continue;
            }

            results.push(module_name);
        }
    }

    results
}

/// WASM32 fallback: no-op include path scanning.
#[cfg(target_arch = "wasm32")]
fn scan_include_paths_for_modules(
    _prefix: &str,
    _include_paths: &[&PathBuf],
    _cache: &RefCell<HashMap<PathBuf, Vec<String>>>,
    _seen: &HashSet<String>,
    _is_cancelled: &dyn Fn() -> bool,
) -> Vec<String> {
    Vec::new()
}

/// Add module name completions for `use` and `require` statements.
///
/// When the cursor is after `use ` or `require `, suggests package names from the
/// workspace index AND include paths (from `.perl-lsp.toml` `includePaths`,
/// `PERL5LIB`, and system @INC when `useSystemInc: true`).
///
/// This implements a dual-tier search:
/// - Tier 1 (priority): Workspace index modules — sorted with prefix `1_`
/// - Tier 2 (fallback): Include path modules — sorted with prefix `2_`
///
/// Deduplication via `seen` HashSet ensures no duplicate module names.
pub fn add_use_module_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
    include_paths: &[PathBuf],
    system_inc_paths: &[PathBuf],
    include_path_cache: &RefCell<HashMap<PathBuf, Vec<String>>>,
) {
    let mut seen: HashSet<String> = HashSet::new();

    // Tier 1: Search workspace index (priority)
    if let Some(index) = workspace_index {
        if !index.has_symbols() {
            // No symbols in workspace, skip to include path scanning
        } else {
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
                    // Workspace modules always use "1_" prefix (tier-based sub-sorting
                    // within workspace is handled by include path tier, not workspace tier)
                    sort_text: Some(format!("1_{name}")),
                    filter_text: Some(name.clone()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: None,
                });
            }
        }
    }

    // Tier 2: Search include paths (fallback)
    // Combine both include_paths and system_inc_paths for scanning
    let all_inc_paths: Vec<&PathBuf> =
        include_paths.iter().chain(system_inc_paths.iter()).collect();

    if !all_inc_paths.is_empty() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let inc_modules = scan_include_paths_for_modules(
                &context.prefix,
                &all_inc_paths,
                include_path_cache,
                &seen,
                &|| false, // is_cancelled callback
            );

            for module_name in inc_modules {
                // Module already in `seen` from workspace index, skip
                if !seen.insert(module_name.clone()) {
                    continue;
                }

                completions.push(CompletionItem {
                    label: module_name.clone(),
                    kind: CompletionItemKind::Module,
                    detail: Some("module".to_string()),
                    documentation: None,
                    insert_text: Some(module_name.clone()),
                    // Tier 2: include path modules sorted with prefix "2_"
                    sort_text: Some(format!("2_{module_name}")),
                    filter_text: Some(module_name),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: None,
                });
            }
        }

        // WASM32: graceful no-op - include path scanning not available on WASM
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (include_paths, system_inc_paths, include_path_cache);
        }
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
