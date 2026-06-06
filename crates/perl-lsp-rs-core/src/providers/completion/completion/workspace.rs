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
use crate::providers::completion::module_scan_cache::{ModuleCompletionScanCache, ScanCacheKey};
use perl_module::path::module_name_to_path;
use perl_parser_core::SourceLocation;
use perl_semantic_analyzer::{
    Node, NodeKind, Parser,
    receiver_facts::{
        ReceiverFact, ReceiverFactContext, ReceiverFactFreshness, ReceiverFallbackState,
        ReceiverKind, receiver_fact_for_method_call,
    },
    semantic::SemanticModel,
    type_facts::TypeEvidence,
    type_inference::{PerlType, TypeInferenceEngine},
};
use perl_semantic_facts::{
    Confidence, DefinitionCandidate, EntityKind, FileId, PackageEdge, Provenance, VisibleSymbol,
    VisibleSymbolSource,
};
use perl_workspace::semantic::{
    imports::ImportExportIndex,
    package_graph::PackageGraphIndex,
    queries::{SemanticQueries, WorkspaceSemanticQueries},
    references::ReferenceIndex,
};
use perl_workspace::workspace_index::{
    SymbolKind as WsSymbolKind, VarKind, WorkspaceIndex, WorkspaceSymbol,
};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
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
                    label_details: None,
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
                    label_details: None,
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
                    commit_characters: Some(vec![":".to_string(), ";".to_string()]),
                    label_details: None,
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
                    label_details: None,
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
                    label_details: None,
                });
            }
            _ => {
                // Skip other symbol types
            }
        }
    }
}

/// Add live compiler visible-symbol completions for imported/exported symbols.
///
/// This is intentionally narrower than the shadow/cutover proof helpers:
/// only high-confidence import/export visibility facts are promoted into the
/// live completion list. Generated members, local symbols, external fallback
/// symbols, and dynamic-boundary candidates remain gated by their existing
/// provider-specific proof lanes.
pub fn add_visible_symbol_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
    filepath: Option<&str>,
) {
    if context.prefix.is_empty() || context.prefix.starts_with(['$', '@', '%', '&']) {
        return;
    }

    let Some(index) = workspace_index else {
        return;
    };
    let Some(uri) = filepath else {
        return;
    };
    let Ok(byte_offset) = u32::try_from(context.position) else {
        return;
    };

    let Some(visible_symbols) = index.with_semantic_queries_for_uri(uri, |file_id, queries| {
        queries.visible_symbols_at(file_id, byte_offset, None)
    }) else {
        return;
    };

    for symbol in visible_symbols
        .into_iter()
        .filter(is_live_visible_completion_candidate)
        .filter(|symbol| symbol.name.starts_with(&context.prefix))
    {
        let source_module = symbol.context.as_ref().and_then(|context| {
            context.source_module.as_deref().filter(|module| !module.is_empty())
        });
        let label = symbol.name.clone();
        completions.push(CompletionItem {
            label: label.clone(),
            kind: CompletionItemKind::Function,
            detail: Some(visible_symbol_completion_detail(&symbol, source_module)),
            documentation: Some(visible_symbol_completion_documentation(&symbol, source_module)),
            insert_text: Some(label.clone()),
            sort_text: Some(format!("2z_visible_{label}")),
            filter_text: Some(label),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
            commit_characters: None,
            label_details: None,
        });
    }
}

fn is_live_visible_completion_candidate(symbol: &VisibleSymbol) -> bool {
    symbol.confidence == Confidence::High
        && matches!(
            symbol.source,
            VisibleSymbolSource::ExplicitImport
                | VisibleSymbolSource::DefaultExport
                | VisibleSymbolSource::ExportTag
        )
}

fn visible_symbol_completion_detail(symbol: &VisibleSymbol, source_module: Option<&str>) -> String {
    let source = match symbol.source {
        VisibleSymbolSource::ExplicitImport => "imported",
        VisibleSymbolSource::DefaultExport => "default export",
        VisibleSymbolSource::ExportTag => "tag export",
        _ => "visible symbol",
    };

    match source_module {
        Some(module) => format!("{source} from {module} - compiler fact, high confidence"),
        None => format!("{source} - compiler fact, high confidence"),
    }
}

fn visible_symbol_completion_documentation(
    symbol: &VisibleSymbol,
    source_module: Option<&str>,
) -> String {
    let source = match symbol.source {
        VisibleSymbolSource::ExplicitImport => "explicit import",
        VisibleSymbolSource::DefaultExport => "default export",
        VisibleSymbolSource::ExportTag => "export tag",
        _ => "visible symbol",
    };
    let module = source_module.map(|module| format!("\nModule: `{module}`")).unwrap_or_default();

    format!(
        "Compiler visible-symbol completion.\n\nSource: {source}\nProvenance: ImportExportInference\nConfidence: High\nFreshness: Fresh{module}"
    )
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

const MAX_MODULE_SCAN_ROOTS: usize = 16;
const MAX_MODULES_PER_SCAN: usize = 512;
const MAX_SCAN_DEPTH: usize = 8;

/// Convert a module file path under `root` to a Perl module name.
///
/// Example: `lib/File/Spec.pm` under `lib` => `File::Spec`.
pub fn path_to_module_name(root: &Path, file_path: &Path) -> Option<String> {
    let rel = file_path.strip_prefix(root).ok()?;
    if rel.extension().and_then(|ext| ext.to_str()) != Some("pm") {
        return None;
    }

    let stem = rel.with_extension("");
    let mut parts: Vec<String> = Vec::new();
    for component in stem.components() {
        let part = component.as_os_str().to_str()?;
        if part.is_empty() {
            continue;
        }
        parts.push(part.to_string());
    }

    if parts.is_empty() { None } else { Some(parts.join("::")) }
}

/// Split a module prefix like `"Foo::Bar::Ba"` into a subdir and a leaf prefix.
///
/// Returns `(scan_dir, leaf_prefix, depth_consumed)` where:
/// - `scan_dir` is `root` joined with the path components before the last `::`,
/// - `leaf_prefix` is the last `::` segment (the partial name being typed),
/// - `depth_consumed` is the number of directory levels already descended into.
///
/// For a single-segment prefix such as `"Foo"`, `scan_dir == root`,
/// `leaf_prefix == "Foo"`, and `depth_consumed == 0` — the caller falls back
/// to a normal root scan.
///
/// For `"Foo::Bar::Ba"`:
/// - `scan_dir  == root.join("Foo/Bar")`
/// - `leaf_prefix == "Ba"`
/// - `depth_consumed == 2`  (two directory levels have been consumed)
///
/// This lets `scan_directory_for_modules` start the BFS at the narrowest
/// possible directory instead of scanning from the include root and filtering,
/// which is a significant speedup on large vendor / local / system `@INC` trees.
pub fn root_and_leaf_prefix(root: &Path, module_prefix: &str) -> (PathBuf, String, usize) {
    if module_prefix.is_empty() {
        return (root.to_path_buf(), String::new(), 0);
    }

    let mut parts: Vec<&str> = module_prefix.split("::").collect();
    if parts.len() <= 1 {
        // Single segment — scan from root, filter by that segment.
        return (root.to_path_buf(), module_prefix.to_string(), 0);
    }

    // The last element is the partial leaf being typed; everything before it
    // is a fully-typed namespace segment that maps to a real directory.
    let leaf = parts.pop().unwrap_or_default().to_string();
    let depth_consumed = parts.len(); // number of dir levels consumed
    let subdir = parts.iter().fold(root.to_path_buf(), |p, part| p.join(part));
    (subdir, leaf, depth_consumed)
}

/// Recursively scan a directory for `.pm` files and return module names.
///
/// When `prefix` contains `::` separators the scan starts from the narrowest
/// matching subdirectory (`prefix_dir`) instead of from `root`, which avoids
/// traversing unrelated subtrees on large include trees.  `path_to_module_name`
/// is still called with the original `root` so that the full qualified name
/// (`Foo::Bar::Baz`) is reconstructed correctly and the existing `starts_with`
/// filter continues to work unchanged.
pub fn scan_directory_for_modules(root: &Path, prefix: &str) -> Vec<String> {
    let mut modules = Vec::new();
    if !root.is_dir() {
        return modules;
    }

    // Prefix-directed optimisation: for namespaced prefixes like "Foo::Bar::Ba"
    // start BFS at `root/Foo/Bar/` rather than `root/`.  If that subdir does
    // not exist we fall back to a root scan (the prefix simply has no matches).
    let (scan_dir, _leaf_prefix, depth_consumed) = root_and_leaf_prefix(root, prefix);
    let start_dir = if scan_dir.is_dir() { scan_dir } else { root.to_path_buf() };
    let start_depth = if start_dir == root { 0 } else { depth_consumed };

    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::from([(start_dir, start_depth)]);
    while let Some((dir, depth)) = queue.pop_front() {
        if modules.len() >= MAX_MODULES_PER_SCAN {
            break;
        }

        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            if modules.len() >= MAX_MODULES_PER_SCAN {
                break;
            }

            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();

            if file_type.is_dir() {
                // Use path.is_symlink() rather than file_type.is_symlink() because
                // DirEntry::file_type() returns the entry's own type: on Unix a
                // symlinked directory has is_symlink()=true AND is_dir()=false,
                // so the file_type.is_symlink() guard inside is_dir() would be
                // dead code. path.is_symlink() correctly detects symlinks via lstat.
                if depth < MAX_SCAN_DEPTH && !path.is_symlink() {
                    queue.push_back((path, depth + 1));
                }
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            let Some(module_name) = path_to_module_name(root, &path) else {
                continue;
            };

            if prefix.is_empty() || module_name.starts_with(prefix) {
                modules.push(module_name);
            }
        }
    }

    modules
}

/// Add module name completions for `use` and `require` statements.
///
/// When the cursor is after `use ` or `require `, suggests package names from the
/// workspace index. This enables discovering available modules as you type.
///
/// For example, typing `use My` will suggest `MyApp`, `MyApp::Config`, etc.
fn workspace_module_symbol_matches_roots(symbol: &WorkspaceSymbol, active_roots: &[&Path]) -> bool {
    if active_roots.is_empty() {
        return true;
    }

    let Some(symbol_path) = perl_workspace::workspace_index::uri_to_fs_path(&symbol.uri) else {
        return false;
    };
    let module_path = module_name_to_path(&symbol.name);
    let symbol_key = normalized_path_key(&symbol_path);

    active_roots.iter().any(|root| {
        let candidate = root.join(&module_path);
        normalized_path_key(&candidate) == symbol_key
    })
}

fn normalized_path_key(path: &Path) -> String {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        if component == Component::CurDir {
            continue;
        }
        normalized.push(component.as_os_str());
    }

    let path = if normalized.as_os_str().is_empty() { PathBuf::from(".") } else { normalized };
    let key = path.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_string();
    if cfg!(windows) { key.to_ascii_lowercase() } else { key }
}

/// Collect module names from include roots using the same bounded directory
/// scanner and optional short-TTL cache used by completion-list module lookup.
pub fn collect_module_names_from_roots_with_cache(
    prefix: &str,
    include_paths: &[PathBuf],
    system_inc_paths: &[PathBuf],
    include_system_inc: bool,
    scan_cache: Option<&ModuleCompletionScanCache>,
    is_cancelled: &dyn Fn() -> bool,
) -> Vec<String> {
    let mut modules = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let cache_key = |root: &Path| -> ScanCacheKey {
        let (scan_dir, _, _) = root_and_leaf_prefix(root, prefix);
        let prefix_dir = match scan_dir.strip_prefix(root) {
            Ok(path) => path.to_path_buf(),
            Err(_) => scan_dir.to_path_buf(),
        };
        ScanCacheKey {
            // Inline completion runs on frequent keystrokes; keep this helper free
            // of canonicalization I/O and rely on stable include-root paths.
            canonical_root: root.to_path_buf(),
            prefix_dir,
            module_prefix: prefix.to_string(),
        }
    };

    let mut add_external_modules = |roots: &[PathBuf]| -> bool {
        for root in roots.iter().take(MAX_MODULE_SCAN_ROOTS) {
            if is_cancelled() {
                return false;
            }

            let scanned_modules = match scan_cache {
                Some(cache) => {
                    let key = cache_key(root);
                    if let Some(cached) = cache.get(&key) {
                        if is_cancelled() {
                            return false;
                        }
                        cached
                    } else {
                        let scanned = scan_directory_for_modules(root, prefix);
                        if is_cancelled() {
                            return false;
                        }
                        cache.insert(key, scanned.clone());
                        scanned
                    }
                }
                None => scan_directory_for_modules(root, prefix),
            };

            if is_cancelled() {
                return false;
            }

            for name in scanned_modules {
                if !seen.contains(&name) {
                    seen.insert(name.clone());
                    modules.push(name);
                }
            }
        }

        true
    };

    if !add_external_modules(include_paths) {
        return modules;
    }
    if include_system_inc {
        let _ = add_external_modules(system_inc_paths);
    }

    modules
}

/// Add module name completions for `use` and `require` statements.
///
/// Thin backward-compatible wrapper around [`add_use_module_completions_with_cache`]
/// that passes `None` for the cache.  Prefer the `_with_cache` variant when a
/// runtime-owned [`ModuleCompletionScanCache`] is available.
#[allow(dead_code)] // Public backward-compatibility API; callers in perl-lsp-rs use _with_cache
pub fn add_use_module_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
    include_paths: &[PathBuf],
    system_inc_paths: &[PathBuf],
    include_system_inc: bool,
) {
    add_use_module_completions_with_cache(
        completions,
        context,
        workspace_index,
        include_paths,
        system_inc_paths,
        include_system_inc,
        None,
        &|| false,
    );
}

/// Add module name completions for `use` and `require` statements, optionally
/// using a runtime-owned TTL cache to avoid repeated filesystem scans on each
/// keystroke (issue #8514).
///
/// ## Cache contract
///
/// - When `scan_cache` is `Some`, each
///   `(canonical_root, prefix_dir, full_module_prefix)` tuple is looked up before
///   scanning. On a miss the scan proceeds normally and the result is stored in
///   the cache. On a hit the cached `Vec<String>` is used directly.
/// - `is_cancelled` is checked **before returning any cached hit** so that a
///   cancelled LSP request does not deliver results to the editor.
/// - The workspace-index path is not cached — it is already in-memory.
/// - Cache population uses the canonical form of the root path when available
///   (`std::fs::canonicalize`); falls back to the raw path on error.
pub fn add_use_module_completions_with_cache(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
    include_paths: &[PathBuf],
    system_inc_paths: &[PathBuf],
    include_system_inc: bool,
    scan_cache: Option<&ModuleCompletionScanCache>,
    is_cancelled: &dyn Fn() -> bool,
) {
    let mut seen: HashSet<String> = HashSet::new();
    let mut active_module_roots: Vec<&Path> = include_paths.iter().map(PathBuf::as_path).collect();
    if include_system_inc {
        active_module_roots.extend(system_inc_paths.iter().map(PathBuf::as_path));
    }

    if let Some(index) = workspace_index
        && index.has_symbols()
    {
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

            if !workspace_module_symbol_matches_roots(&symbol, &active_module_roots) {
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
                label_details: None,
            });
        }
    }

    // Helper: resolve the canonical form of `root` for use as a cache key.
    // Falls back to the raw path when canonicalization fails (e.g. non-existent dir).
    let canonical_root = |root: &Path| -> PathBuf {
        std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf())
    };

    // Helper: build the ScanCacheKey for (root, prefix).
    //
    // The prefix_dir is the subdirectory that the scan starts from
    // (e.g. `Mojo/` for prefix `Mojo::Controller`). Using a relative
    // path under the canonical root keeps the key stable across callers
    // that might pass in slightly different root representations. The full
    // prefix is part of the key because cached values are prefix-filtered.
    let cache_key = |root: &Path| -> ScanCacheKey {
        let (scan_dir, _, _) = root_and_leaf_prefix(root, &context.prefix);
        let prefix_dir = scan_dir.strip_prefix(root).unwrap_or(&scan_dir).to_path_buf();
        ScanCacheKey {
            canonical_root: canonical_root(root),
            prefix_dir,
            module_prefix: context.prefix.clone(),
        }
    };

    let mut add_external_modules = |roots: &[PathBuf], detail: &str| -> bool {
        for root in roots.iter().take(MAX_MODULE_SCAN_ROOTS) {
            if is_cancelled() {
                return false;
            }

            let modules = match scan_cache {
                Some(cache) => {
                    let key = cache_key(root);
                    if let Some(cached) = cache.get(&key) {
                        // Cancellation check before returning cached result.
                        if is_cancelled() {
                            return false;
                        }
                        cached
                    } else {
                        let scanned = scan_directory_for_modules(root, &context.prefix);
                        if is_cancelled() {
                            return false;
                        }
                        cache.insert(key, scanned.clone());
                        scanned
                    }
                }
                None => scan_directory_for_modules(root, &context.prefix),
            };

            if is_cancelled() {
                return false;
            }

            for name in modules {
                if !seen.insert(name.clone()) {
                    continue;
                }

                completions.push(CompletionItem {
                    label: name.clone(),
                    kind: CompletionItemKind::Module,
                    detail: Some(detail.to_string()),
                    documentation: Some(format!("Package `{name}`")),
                    insert_text: Some(name.clone()),
                    sort_text: Some(format!("2{}_{name}", module_sort_tier(&name))),
                    filter_text: Some(name),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: Some(vec![":".to_string(), ";".to_string()]),
                    label_details: None,
                });
            }
        }
        true
    };

    if !add_external_modules(include_paths, "external module") {
        return;
    }
    if include_system_inc {
        let _ = add_external_modules(system_inc_paths, "system module");
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
            label_details: None,
        });
    }
}

/// Classification of how a method-completion receiver was inferred at the
/// call site.
///
/// This is *typed receiver-evidence provenance*: source-backed exact facts may
/// drive the narrow semantic method-completion pilot, while weaker evidence
/// keeps the existing fallback path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReceiverEvidence {
    /// Literal package name on the left of `->`, e.g. `Foo->method` or
    /// `Foo::Bar->method`. High confidence.
    StaticPackage(String),
    /// `$self->` or `$this->` inside a non-`main` `package Foo;` block.
    /// Resolves to the enclosing package via `context.current_package`.
    /// High confidence.
    SelfOrThis(String),
    /// Variable-method call against `$x` assigned earlier in the source as
    /// `my $x = Foo->new(...)`. The constructor convention pins the
    /// receiver to `Foo`. High confidence.
    ConstructorAssignment(String),
    /// Variable-method call against `$x` assigned earlier as
    /// `my $x = bless ..., "Foo"`. Literal `bless` with a literal class.
    /// Medium confidence — strong static evidence, still a Perl runtime
    /// construct.
    LiteralBless(String),
    /// Receiver type was resolved by [`TypeInferenceEngine`] for a `$var`
    /// call site. Medium confidence — confidence ultimately follows the
    /// engine's source, but at this layer we treat all engine results as
    /// medium.
    TypeEngine(String),
    /// Receiver was resolved by the semantic receiver-fact layer and met the
    /// exact/fresh/high-confidence provider cutover bar.
    ObjectFact(String),
    /// Static hash slot, e.g. `$services{db}->`, resolved through a fresh
    /// source-backed receiver fact.
    HashSlotFact(String),
    /// No receiver evidence found, OR a positively-detected dynamic form
    /// (e.g. `bless {}, $class`, expression-tail class, nested call,
    /// Positively-detected dynamic / fail-closed receiver form — e.g.
    /// `bless {}, $class`, `bless {}, "Foo" . $suffix`,
    /// `wrapper(bless {}, "Foo")`, `bless::class {}, "Foo"`. The user
    /// typed something we *can* see is a `bless` expression but the
    /// resulting class is not a literal we can pin down. Method
    /// completion stays fail-closed for these — no exact receiver
    /// completions and no Unknown-receiver fallback (#7929 outcome A).
    Dynamic,
    /// No receiver evidence found at all (e.g. `$obj->` where `$obj` is
    /// a sub parameter with no assignment in scope). Eligible for the
    /// bounded low-confidence fallback added in #7929 outcome A.
    Unknown,
}

impl ReceiverEvidence {
    /// Returns the inferred receiver package, if any. `Dynamic` and
    /// `Unknown` both return `None`.
    pub(super) fn package(&self) -> Option<&str> {
        match self {
            Self::StaticPackage(p)
            | Self::SelfOrThis(p)
            | Self::ConstructorAssignment(p)
            | Self::LiteralBless(p)
            | Self::TypeEngine(p)
            | Self::ObjectFact(p)
            | Self::HashSlotFact(p) => Some(p.as_str()),
            Self::Dynamic | Self::Unknown => None,
        }
    }

    /// Returns `true` only when this evidence is eligible for the
    /// bounded Unknown-receiver fallback (#7929). `Dynamic` is
    /// explicitly *not* eligible — dynamic boundaries stay fail-closed.
    pub(super) fn is_unknown_fallback_eligible(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// Returns the confidence level for this evidence kind, using the
    /// shared `perl_semantic_facts::Confidence` vocabulary so the rest of
    /// the semantic stack speaks the same language. `Dynamic` and
    /// `Unknown` return `None` — there is no confidence when there is
    /// no exact evidence.
    ///
    /// Today the production method-completion callsite reads this only
    /// to decide medium-confidence labelling on detail text (#7925
    /// outcome C). It does not yet drive ranking.
    #[allow(dead_code)]
    pub(super) fn confidence(&self) -> Option<Confidence> {
        match self {
            Self::StaticPackage(_) | Self::SelfOrThis(_) | Self::ConstructorAssignment(_) => {
                Some(Confidence::High)
            }
            Self::ObjectFact(_) | Self::HashSlotFact(_) => Some(Confidence::High),
            Self::LiteralBless(_) | Self::TypeEngine(_) => Some(Confidence::Medium),
            Self::Dynamic | Self::Unknown => None,
        }
    }

    /// Short, user-facing suffix describing the evidence source, suitable
    /// for appending to a `CompletionItem.detail` string. `Dynamic` and
    /// `Unknown` return `None` — when there is no exact evidence, there
    /// is nothing to label. Issue #7918: explanatory only, no ranking /
    /// inclusion change.
    pub(super) fn detail_suffix(&self) -> Option<&'static str> {
        match self {
            Self::StaticPackage(_) => Some("receiver: static package"),
            Self::SelfOrThis(_) => Some("receiver: self/this"),
            Self::ConstructorAssignment(_) => Some("receiver: constructor assignment"),
            Self::LiteralBless(_) => Some("receiver: literal bless"),
            Self::TypeEngine(_) => Some("receiver: type engine"),
            Self::ObjectFact(_) => Some("receiver: source-backed object"),
            Self::HashSlotFact(_) => Some("receiver: hash slot"),
            Self::Dynamic | Self::Unknown => None,
        }
    }
}

/// Apply the receiver-evidence detail suffix to an existing base detail
/// string. Returns the unchanged base when the evidence carries no suffix
/// (e.g. `Unknown`). Issue #7918.
pub(super) fn detail_with_evidence(base: String, evidence: &ReceiverEvidence) -> String {
    let Some(suffix) = evidence.detail_suffix() else {
        return base;
    };
    // Outcome C from #7925: append `, medium confidence` only to medium-
    // confidence evidence. High-confidence evidence (the common case)
    // stays clean. `Unknown` already returns `None` from `detail_suffix`
    // and was handled by the early return above.
    match evidence.confidence() {
        Some(Confidence::Medium) => format!("{base} — {suffix}, medium confidence"),
        _ => format!("{base} — {suffix}"),
    }
}

/// Classify the receiver of a `->` method-completion call site.
///
/// Tries exact source-backed receiver facts first, then falls back to the
/// historical type-engine and text-pattern receiver inference paths. Returns
/// [`ReceiverEvidence::Unknown`] when no evidence is found.
pub(super) fn classify_receiver(
    context: &CompletionContext,
    source: &str,
    type_engine: Option<&TypeInferenceEngine>,
) -> ReceiverEvidence {
    if let Some(evidence) = source_backed_receiver_fact_evidence(context, source, type_engine) {
        return evidence;
    }
    if let Some(pkg) = type_engine_receiver(context, type_engine) {
        return ReceiverEvidence::TypeEngine(pkg);
    }
    classify_text_pattern_receiver(context, source)
}

fn source_backed_receiver_fact_evidence(
    context: &CompletionContext,
    source: &str,
    type_engine: Option<&TypeInferenceEngine>,
) -> Option<ReceiverEvidence> {
    let current_prefix = source.get(context.prefix_start..context.position)?;
    if current_prefix != context.prefix.as_str() {
        return None;
    }

    let receiver_source = context.prefix.strip_suffix("->")?.trim();
    if receiver_source.is_empty() {
        return None;
    }

    let fact = receiver_fact_for_arrow_receiver(receiver_source, type_engine?)?;
    exact_receiver_fact_evidence(&fact)
}

fn receiver_fact_for_arrow_receiver(
    receiver_source: &str,
    type_engine: &TypeInferenceEngine,
) -> Option<ReceiverFact> {
    const PROBE_METHOD: &str = "__plsp_receiver_probe";
    let probe_source = format!("{receiver_source}->{PROBE_METHOD}();");
    let mut parser = Parser::new(&probe_source);
    let ast = parser.parse().ok()?;
    let call = method_call_named(&ast, PROBE_METHOD)?;
    Some(receiver_fact_for_method_call(
        call,
        ReceiverFactContext::new(Some(type_engine.environment())).with_source(&probe_source),
    ))
}

fn method_call_named<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if let NodeKind::MethodCall { method, .. } = &node.kind
        && method == name
    {
        return Some(node);
    }

    match &node.kind {
        NodeKind::Program { statements } => {
            statements.iter().find_map(|child| method_call_named(child, name))
        }
        NodeKind::ExpressionStatement { expression } => method_call_named(expression, name),
        NodeKind::VariableDeclaration { initializer, .. } => {
            initializer.as_deref().and_then(|child| method_call_named(child, name))
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            method_call_named(lhs, name).or_else(|| method_call_named(rhs, name))
        }
        NodeKind::MethodCall { object, args, .. } => method_call_named(object, name)
            .or_else(|| args.iter().find_map(|child| method_call_named(child, name))),
        NodeKind::Binary { left, right, .. } => {
            method_call_named(left, name).or_else(|| method_call_named(right, name))
        }
        NodeKind::ArrayLiteral { elements } => {
            elements.iter().find_map(|child| method_call_named(child, name))
        }
        NodeKind::HashLiteral { pairs } => pairs.iter().find_map(|(key, value)| {
            method_call_named(key, name).or_else(|| method_call_named(value, name))
        }),
        _ => None,
    }
}

fn exact_receiver_fact_evidence(fact: &ReceiverFact) -> Option<ReceiverEvidence> {
    if fact.confidence == Confidence::Medium
        && fact.freshness == ReceiverFactFreshness::Fresh
        && fact.dynamic_boundary.is_none()
        && fact.source_range.is_some()
        && let Some(package) = literal_bless_package_from_fact(fact)
    {
        return Some(ReceiverEvidence::LiteralBless(package));
    }

    if fact.confidence != Confidence::High
        || fact.freshness != ReceiverFactFreshness::Fresh
        || fact.fallback_state != ReceiverFallbackState::Exact
        || fact.dynamic_boundary.is_some()
        || fact.source_range.is_none()
    {
        return None;
    }

    let package = fact.package.clone()?;
    match fact.kind {
        ReceiverKind::StaticPackage => Some(ReceiverEvidence::StaticPackage(package)),
        ReceiverKind::SelfReceiver => Some(ReceiverEvidence::SelfOrThis(package)),
        ReceiverKind::ObjectVariable => Some(ReceiverEvidence::ObjectFact(package)),
        ReceiverKind::HashSlot => Some(ReceiverEvidence::HashSlotFact(package)),
        ReceiverKind::HashRefSlot
        | ReceiverKind::ArrayIndex
        | ReceiverKind::DynamicKey
        | ReceiverKind::Unknown => None,
        _ => None,
    }
}

fn literal_bless_package_from_fact(fact: &ReceiverFact) -> Option<String> {
    let package = fact.package.as_ref()?;
    fact.evidence.iter().find_map(|evidence| match evidence {
        TypeEvidence::BlessLiteral { package: evidence_package } if evidence_package == package => {
            Some(package.clone())
        }
        _ => None,
    })
}

/// Type-engine arm of [`classify_receiver`]. Extracted from the legacy
/// `infer_receiver_package_from_type_engine` body, behavior preserved.
fn type_engine_receiver(
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

/// Text-pattern arm of [`classify_receiver`]. Looks for `Foo->method`
/// (static), `$self->` / `$this->` (self), `my $x = Foo->new` (constructor
/// assignment), and `my $x = bless ..., "Foo"` (literal bless).
pub(super) fn classify_text_pattern_receiver(
    context: &CompletionContext,
    source: &str,
) -> ReceiverEvidence {
    let arrow_prefix = context.prefix.trim_end_matches("->");

    // Case 1: Static method call like `My::Package->meth` or `Package->meth`.
    // The prefix already contains the package name (starts with uppercase, no sigil).
    if !arrow_prefix.starts_with('$')
        && !arrow_prefix.starts_with('@')
        && !arrow_prefix.starts_with('%')
        && arrow_prefix.chars().next().is_some_and(|c| c.is_ascii_uppercase())
    {
        return ReceiverEvidence::StaticPackage(arrow_prefix.to_string());
    }

    // Case 3: Self-call inside a method — `$self->` or `$this->` resolves to
    // the current package. Standard Perl OO convention: the invocant of `bless`
    // is assigned to `$self` (or `$this`) via `my $self = shift`. The RHS is
    // just `shift`, so Case 2 below would not match any constructor pattern.
    // Instead we fall back to `context.current_package` which the context
    // analyser already sets correctly from the surrounding `package`
    // declaration.
    if matches!(arrow_prefix, "$self" | "$this")
        && !context.current_package.is_empty()
        && context.current_package != "main"
    {
        return ReceiverEvidence::SelfOrThis(context.current_package.clone());
    }

    // Case 2: Variable method call like `$obj->meth` — try to find the
    // receiver type from a recent assignment.
    if arrow_prefix.starts_with('$') {
        let var_name = arrow_prefix;
        let before = &source[..context.position.min(source.len())];

        let lines: Vec<&str> = before.lines().collect();
        for (line_idx, line) in lines.iter().enumerate().rev() {
            let trimmed = line.trim();
            let assign_pos = find_assignment_eq(trimmed);
            if let Some(assign_pos) = assign_pos {
                let lhs = trimmed[..assign_pos].trim();
                if lhs.ends_with(var_name) || lhs.contains(&format!("{var_name} ")) {
                    let rhs = collect_assignment_rhs(&lines, line_idx, assign_pos);
                    let rhs = rhs.trim();
                    // Pattern: `Package::Name->new(...)`
                    if let Some(arrow_pos) = rhs.find("->") {
                        let pkg = rhs[..arrow_pos].trim();
                        if pkg.contains("::")
                            || pkg.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                        {
                            return ReceiverEvidence::ConstructorAssignment(pkg.to_string());
                        }
                    }
                    // Pattern: `bless REF, "Class"` / `bless REF, 'Class'`.
                    // Only literal-string class names produce inference; dynamic
                    // forms like `bless {}, $class` intentionally fall through
                    // (extract_bless_literal_class is fail-closed).
                    if let Some(class) = extract_bless_literal_class(rhs) {
                        return ReceiverEvidence::LiteralBless(class);
                    }
                    // Pattern: dynamic / fail-closed `bless` form (#7929).
                    // We saw a `bless` keyword in the RHS (outside string
                    // literals) but could not extract a literal class —
                    // covers `bless {}, $class`, `bless {}, "Foo" . $suffix`,
                    // `wrapper(bless {}, "Foo")`, `bless::class {}, "Foo"`,
                    // etc. The receiver is dynamic; fail closed (no exact
                    // package and no Unknown-receiver fallback).
                    if rhs_has_bless_keyword_outside_strings(rhs) {
                        return ReceiverEvidence::Dynamic;
                    }
                }
            }
        }
    }

    ReceiverEvidence::Unknown
}

fn collect_assignment_rhs(lines: &[&str], line_idx: usize, assign_pos: usize) -> String {
    let first_line = lines[line_idx].trim();
    let mut rhs = first_line[assign_pos + 1..].trim().to_string();
    if truncate_after_top_level_semicolon(&rhs).len() < rhs.len() {
        return truncate_after_top_level_semicolon(&rhs).to_string();
    }

    for continuation in lines.iter().skip(line_idx + 1) {
        if !rhs.is_empty() {
            rhs.push('\n');
        }
        rhs.push_str(continuation.trim_end());
        let truncated = truncate_after_top_level_semicolon(&rhs);
        if truncated.len() < rhs.len() {
            return truncated.to_string();
        }
    }

    rhs
}

fn truncate_after_top_level_semicolon(s: &str) -> &str {
    let mut depth_paren: i32 = 0;
    let mut depth_brace: i32 = 0;
    let mut depth_bracket: i32 = 0;
    let mut in_string: Option<char> = None;
    let mut prev_was_backslash = false;

    for (idx, ch) in s.char_indices() {
        if let Some(q) = in_string {
            if !prev_was_backslash && ch == q {
                in_string = None;
            }
            prev_was_backslash = !prev_was_backslash && ch == '\\';
            continue;
        }

        prev_was_backslash = false;
        match ch {
            '"' | '\'' => in_string = Some(ch),
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '{' => depth_brace += 1,
            '}' => depth_brace -= 1,
            '[' => depth_bracket += 1,
            ']' => depth_bracket -= 1,
            ';' if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 => {
                return &s[..idx + ch.len_utf8()];
            }
            _ => {}
        }
    }

    s
}

/// Returns `true` when the RHS contains a *call-like* `bless` keyword
/// outside string literals and comments. Used by
/// `classify_text_pattern_receiver` to detect dynamic / fail-closed bless
/// expressions that could not be resolved to a literal class — these
/// include nested calls (`wrapper(bless {}, "Foo")`), expression-tail
/// forms (`bless {}, "Foo" . $suffix`), dynamic class
/// (`bless {}, $class`), and non-builtin `bless`-prefixed identifiers
/// (`bless::class {}, "Foo"`). Issue #7929.
///
/// "Call-like" means `bless` is followed by ASCII whitespace, `(`, or
/// `::` (the qualified non-builtin form), and is not preceded by a Perl
/// sigil (`$`/`@`/`%`/`&`), an identifier byte, or a hash-key opener
/// (`{`). This rejects harmless mentions like `$bless`, `$obj->{bless}`,
/// and `# bless ...` comments which should remain Unknown / fallback-
/// eligible rather than fail-closed Dynamic.
fn rhs_has_bless_keyword_outside_strings(rhs: &str) -> bool {
    let bytes = rhs.as_bytes();
    let needle = b"bless";
    let mut in_string: Option<u8> = None;
    let mut prev_was_backslash = false;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = in_string {
            if !prev_was_backslash && b == q {
                in_string = None;
            }
            prev_was_backslash = !prev_was_backslash && b == b'\\';
            i += 1;
            continue;
        }
        prev_was_backslash = false;
        // Outside strings, `#` starts a comment that runs to end-of-line.
        // The RHS scan is single-line, so terminate here.
        if b == b'#' {
            return false;
        }
        if b == b'"' || b == b'\'' {
            in_string = Some(b);
            i += 1;
            continue;
        }
        if i + needle.len() <= bytes.len()
            && &bytes[i..i + needle.len()] == needle
            && is_call_like_bless(bytes, i)
        {
            return true;
        }
        i += 1;
    }
    false
}

/// Returns `true` when the `bless` token at `bytes[i..i+5]` is *call-like*:
/// not preceded by a sigil/ident-byte/hash-key opener, and followed by
/// whitespace, `(`, or `::`. See [`rhs_has_bless_keyword_outside_strings`].
fn is_call_like_bless(bytes: &[u8], i: usize) -> bool {
    let prev_ok = match i.checked_sub(1).map(|j| bytes[j]) {
        None => true,
        Some(p) => {
            !is_perl_ident_byte_local(p)
                && p != b'$'
                && p != b'@'
                && p != b'%'
                && p != b'&'
                && p != b'{'
        }
    };
    if !prev_ok {
        return false;
    }
    let next_idx = i + b"bless".len();
    match bytes.get(next_idx).copied() {
        None => false,
        Some(n) => {
            n.is_ascii_whitespace()
                || n == b'('
                || (n == b':' && bytes.get(next_idx + 1).copied() == Some(b':'))
        }
    }
}

fn is_perl_ident_byte_local(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Extract the literal class name from a `bless REF, "Class"` expression.
///
/// Anchored to RHS-as-builtin-bless-expression: only succeeds when the
/// RHS, after trimming leading whitespace, *starts* with `bless` followed
/// by end-of-string, ASCII whitespace, or `(`. This means the helper
/// returns `None` for nested forms like `wrapper(bless {}, "Foo")` (where
/// the assignment result is not necessarily the blessed object) and for
/// non-builtin punctuation-suffixed forms like `bless::factory {}, "Foo"`
/// (where the call target is a different sub that merely shares the
/// `bless` prefix).
///
/// Trailing content after the closing quote of the class literal must be
/// only whitespace, the matching closing paren if a leading `(` was
/// consumed, and an optional terminating `;`. Anything else — `. $suffix`,
/// `|| "Bar"`, `, $extra`, etc. — disables inference (fails closed) so
/// non-literal class expressions never produce false-precision evidence.
///
/// Returns `Some(class)` only for the conservative literal form. Returns
/// `None` for dynamic forms (`bless {}, $class`), 1-arg forms
/// (`bless {}` — defaults to caller package, intentionally not inferred
/// here), nested forms, expression-tail forms, and anything that fails
/// to parse cleanly.
fn extract_bless_literal_class(rhs: &str) -> Option<String> {
    // Anchor: RHS must START with the builtin `bless` expression.
    let trimmed = rhs.trim_start();
    if !starts_with_bless_expression(trimmed) {
        return None;
    }
    let after_bless = &trimmed["bless".len()..];

    // Allow optional `(` and whitespace.
    let scan = after_bless.trim_start();
    let (scan, expect_rparen) = match scan.strip_prefix('(') {
        Some(rest) => (rest, true),
        None => (scan, false),
    };

    // Find the comma separating the two args, respecting balanced delimiters
    // and string literals so that hash/array contents like
    // `bless { a => 1, b => 2 }, "Foo"` and `bless [1, 2, 3], "Foo"` parse.
    let comma_pos = find_top_level_comma(scan)?;
    let after_comma = scan[comma_pos + 1..].trim_start();

    // Require a literal quoted string for the class.
    let bytes = after_comma.as_bytes();
    let close_char = match bytes.first()? {
        b'"' => '"',
        b'\'' => '\'',
        _ => return None,
    };
    let body = &after_comma[1..];
    let close_pos = body.find(close_char)?;
    let class = &body[..close_pos];

    if !is_valid_perl_package_name(class) {
        return None;
    }

    // Validate trailing content: only whitespace, the matching `)` if a
    // leading `(` was consumed, and an optional `;` then EOF. Anything else
    // (concatenation, logical-or, extra arg, expression continuation) means
    // the class expression is not a plain literal — fail closed.
    let mut tail = body[close_pos + 1..].trim_start();
    if expect_rparen {
        tail = tail.strip_prefix(')')?.trim_start();
    }
    let tail = tail.strip_prefix(';').unwrap_or(tail).trim();
    if !tail.is_empty() {
        return None;
    }

    Some(class.to_string())
}

/// Returns `true` when `s` begins with the builtin `bless` expression.
///
/// Stricter than a generic word-boundary check: after `bless`, only end
/// of string, ASCII whitespace, or `(` are accepted. This rejects
/// non-builtin forms like `bless::factory(...)`, `bless+REF`, `bless.foo`,
/// and similar punctuation-suffixed identifiers that happen to share the
/// `bless` prefix but are not the Perl builtin.
fn starts_with_bless_expression(s: &str) -> bool {
    if !s.starts_with("bless") {
        return false;
    }
    match s.as_bytes().get("bless".len()).copied() {
        None => true,
        Some(b) => b.is_ascii_whitespace() || b == b'(',
    }
}

/// Find the first top-level `,` outside of `()`, `{}`, `[]`, and string
/// literals. Used to identify the arg separator in `bless REF, CLASS`.
fn find_top_level_comma(s: &str) -> Option<usize> {
    let mut depth_paren: i32 = 0;
    let mut depth_brace: i32 = 0;
    let mut depth_bracket: i32 = 0;
    let mut in_string: Option<char> = None;
    let mut prev_was_backslash = false;

    for (i, c) in s.char_indices() {
        if let Some(q) = in_string {
            if !prev_was_backslash && c == q {
                in_string = None;
            }
            prev_was_backslash = !prev_was_backslash && c == '\\';
            continue;
        }
        prev_was_backslash = false;
        match c {
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '{' => depth_brace += 1,
            '}' => depth_brace -= 1,
            '[' => depth_bracket += 1,
            ']' => depth_bracket -= 1,
            '"' => in_string = Some('"'),
            '\'' => in_string = Some('\''),
            ',' if depth_paren <= 0 && depth_brace <= 0 && depth_bracket <= 0 => {
                return Some(i);
            }
            _ => {}
        }
    }
    None
}

/// Validate a Perl package name: identifier segments separated by `::`.
fn is_valid_perl_package_name(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut start_of_segment = true;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if start_of_segment {
            if !(c.is_ascii_alphabetic() || c == '_') {
                return false;
            }
            start_of_segment = false;
            continue;
        }
        if c == ':' {
            // Must be `::`, then a fresh segment.
            if chars.next() != Some(':') {
                return false;
            }
            start_of_segment = true;
            continue;
        }
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return false;
        }
    }
    !start_of_segment
}

/// Add method completions from the workspace index for `->` expressions.
///
/// When the user types `$obj->` or `Package->`, queries the workspace index for
/// methods defined in the receiver's package and suggests them.
///
/// Auto-import edits are attached when the receiver package is not yet imported.
///
/// When receiver inference returns [`ReceiverEvidence::Unknown`] (no exact
/// receiver evidence found), the bounded low-confidence fallback added in
/// #7929 fires: methods from imported / visible packages plus the current
/// file's package and its `@ISA` chain are offered with a low-confidence
/// detail label and a sort tier that puts them below all exact-receiver
/// completions. [`ReceiverEvidence::Dynamic`] (positively-detected dynamic
/// `bless` forms) is *not* fallback-eligible and stays fail-closed.
pub fn add_workspace_method_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    type_engine: Option<&TypeInferenceEngine>,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
    used_modules: &HashSet<String>,
) {
    let Some(index) = workspace_index else {
        return;
    };

    if !index.has_symbols() {
        return;
    }

    // Prefer semantic receiver facts only when they meet the narrow live pilot
    // bar. Medium, dynamic, unknown, and unsupported facts fall back through the
    // existing receiver classifier instead of suppressing legacy behavior.
    let evidence = classify_receiver(context, source, type_engine);
    let Some(package_name) = evidence.package().map(str::to_string) else {
        // No exact receiver package. Trigger bounded Unknown-receiver
        // fallback (#7929) only for `Unknown` evidence; `Dynamic` stays
        // fail-closed.
        if evidence.is_unknown_fallback_eligible() {
            add_unknown_receiver_fallback(completions, context, index, used_modules);
        }
        return;
    };

    // Collect labels already present to avoid duplicates with local method completions
    let existing_labels: HashSet<String> =
        completions.iter().map(|item| item.label.clone()).collect();

    let method_prefix = context.prefix.rsplit("->").next().unwrap_or("");

    // Collect all methods from the receiver package AND its ancestor chain
    // (parents + roles). Child methods take priority.
    let members = collect_all_package_members(index, &package_name);

    // Build an auto-import edit once for all methods from this package.
    let auto_import_edit = auto_import::build_auto_import_edit(source, &package_name);
    let method_symbols = workspace_method_symbols(&members, &existing_labels, method_prefix);

    if add_semantic_method_completions(
        completions,
        context,
        index,
        &package_name,
        method_prefix,
        &method_symbols,
        auto_import_edit.as_ref(),
        &evidence,
    ) {
        return;
    }

    for symbol in method_symbols {
        let additional_edits =
            auto_import_edit.as_ref().map(|e| vec![e.clone()]).unwrap_or_default();

        // Show which package actually defines the method for inherited completions
        let defining_pkg = symbol.container_name.as_deref().unwrap_or(package_name.as_str());
        let base_detail = if defining_pkg == package_name {
            format!("{package_name} method")
        } else {
            format!("{package_name} method (from {defining_pkg})")
        };
        // Append receiver-evidence suffix from #7918. Detail-only — no
        // change to label, insert_text, filter_text, sort_text, or the
        // candidate set.
        let detail = detail_with_evidence(base_detail, &evidence);

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
            label_details: None,
        });
    }
}

/// Bounded low-confidence fallback for method completion when receiver
/// evidence is [`ReceiverEvidence::Unknown`] (#7929 outcome A).
///
/// Sources are restricted to:
/// - imported / visible packages from the current file's `import_map`
/// - the current package and its `@ISA` chain (via
///   [`collect_all_package_members`]) when `current_package` is set and
///   not `main`
///
/// All-workspace fallback is intentionally not used. Fallback candidates
/// carry a `receiver: unknown, low confidence` detail suffix and use sort
/// tier 6 so they always sort below exact-receiver completions (which
/// use tiers 1–4).
fn add_unknown_receiver_fallback(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    index: &WorkspaceIndex,
    used_modules: &HashSet<String>,
) {
    let mut allowed_packages: HashSet<String> = used_modules.clone();
    if !context.current_package.is_empty() && context.current_package != "main" {
        allowed_packages.insert(context.current_package.clone());
    }
    if allowed_packages.is_empty() {
        return;
    }

    let method_prefix = context.prefix.rsplit("->").next().unwrap_or("");
    let existing_labels: HashSet<String> =
        completions.iter().map(|item| item.label.clone()).collect();
    let mut emitted: HashSet<String> = HashSet::new();

    for package_name in &allowed_packages {
        let members = collect_all_package_members(index, package_name);
        for symbol in members {
            if !matches!(symbol.kind, WsSymbolKind::Subroutine | WsSymbolKind::Method) {
                continue;
            }
            if !method_prefix.is_empty() && !symbol.name.starts_with(method_prefix) {
                continue;
            }
            if existing_labels.contains(&symbol.name) {
                continue;
            }
            if !emitted.insert(symbol.name.clone()) {
                continue;
            }

            let defining_pkg = symbol.container_name.as_deref().unwrap_or(package_name.as_str());
            let detail = format!(
                "workspace method — receiver: unknown, low confidence (from {defining_pkg})"
            );

            completions.push(CompletionItem {
                label: symbol.name.clone(),
                kind: CompletionItemKind::Function,
                detail: Some(detail),
                documentation: symbol.documentation.clone().or_else(|| {
                    Some(format!(
                        "Workspace method `{}::{}` (low-confidence fallback for unknown receiver).",
                        defining_pkg, symbol.name
                    ))
                }),
                insert_text: Some(format!("{}()", symbol.name)),
                // Tier 6: below all exact-receiver completion tiers
                // (existing tiers 1–4) and below other tier-5 catch-alls.
                sort_text: Some(format!("6_{}", symbol.name)),
                filter_text: Some(symbol.name.clone()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
                label_details: None,
            });
        }
    }
}

fn workspace_method_symbols<'a>(
    members: &'a [WorkspaceSymbol],
    existing_labels: &HashSet<String>,
    method_prefix: &str,
) -> Vec<&'a WorkspaceSymbol> {
    members
        .iter()
        .filter(|symbol| matches!(symbol.kind, WsSymbolKind::Subroutine | WsSymbolKind::Method))
        .filter(|symbol| method_prefix.is_empty() || symbol.name.starts_with(method_prefix))
        .filter(|symbol| !existing_labels.contains(&symbol.name))
        .collect()
}

fn add_semantic_method_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    index: &WorkspaceIndex,
    package_name: &str,
    method_prefix: &str,
    method_symbols: &[&WorkspaceSymbol],
    auto_import_edit: Option<&(SourceLocation, String)>,
    evidence: &ReceiverEvidence,
) -> bool {
    let legacy_names = method_symbol_names(method_symbols);
    if legacy_names.is_empty() {
        return false;
    }

    let Some(candidates) = semantic_method_candidates_for_legacy_methods(
        index,
        package_name,
        &legacy_names,
        method_symbols,
    ) else {
        return false;
    };

    let mut candidate_names: HashSet<String> = HashSet::new();
    for candidate in &candidates {
        candidate_names.insert(candidate.display_name.clone());
    }

    // Cut over only when semantic candidates cover the current legacy workspace
    // method set for this prefix. Otherwise the provider keeps the proven
    // fallback path and avoids dropping completions.
    if !legacy_names.iter().all(|name| candidate_names.contains(name)) {
        return false;
    }

    let mut seen = HashSet::new();
    for candidate in candidates {
        if !method_prefix.is_empty() && !candidate.display_name.starts_with(method_prefix) {
            continue;
        }
        if !seen.insert(candidate.display_name.clone()) {
            continue;
        }

        let additional_edits = auto_import_edit.map(|e| vec![e.clone()]).unwrap_or_default();
        completions.push(CompletionItem {
            label: candidate.display_name.clone(),
            kind: CompletionItemKind::Function,
            detail: Some(semantic_method_detail(package_name, &candidate, evidence)),
            documentation: Some(semantic_method_documentation(package_name, &candidate)),
            insert_text: Some(format!("{}()", candidate.display_name)),
            sort_text: Some(format!(
                "{}_{}",
                semantic_method_sort_tier(package_name, &candidate),
                candidate.display_name
            )),
            filter_text: Some(candidate.display_name.clone()),
            additional_edits,
            text_edit_range: Some((context.prefix_start, context.position)),
            commit_characters: None,
            label_details: None,
        });
    }

    true
}

fn method_symbol_names(method_symbols: &[&WorkspaceSymbol]) -> Vec<String> {
    let mut names: Vec<String> = method_symbols.iter().map(|symbol| symbol.name.clone()).collect();
    names.sort();
    names.dedup();
    names
}

fn semantic_method_candidates_for_legacy_methods(
    index: &WorkspaceIndex,
    package_name: &str,
    method_names: &[String],
    method_symbols: &[&WorkspaceSymbol],
) -> Option<Vec<DefinitionCandidate>> {
    let mut shards = HashMap::new();
    let mut source_uris = HashSet::new();
    let legacy_defining_packages = method_symbol_defining_packages(method_symbols, package_name);

    if let Some(package_location) = index.find_definition(package_name) {
        source_uris.insert(package_location.uri);
    }

    for symbol in method_symbols {
        if let Some(shard) = index.file_fact_shard(&symbol.uri) {
            source_uris.insert(symbol.uri.clone());
            shards.entry(shard.source_uri.clone()).or_insert(shard);
        }
    }

    if shards.is_empty() {
        return None;
    }

    let package_graph = build_completion_package_graph(index, &source_uris);
    let reference_index = ReferenceIndex::new();
    let import_export_index = ImportExportIndex::new();
    let queries = WorkspaceSemanticQueries::with_package_graph(
        &reference_index,
        &import_export_index,
        &shards,
        &package_graph,
    );

    let mut candidates = Vec::new();
    for method_name in method_names {
        let expected_package = legacy_defining_packages.get(method_name)?;
        candidates.extend(
            queries
                .method_candidates(package_name, method_name)
                .into_iter()
                .filter(is_confident_method_candidate)
                .filter(|candidate| {
                    candidate.package.as_deref() == Some(expected_package.as_str())
                }),
        );
    }

    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|left, right| {
        semantic_method_candidate_sort_key(package_name, left)
            .cmp(&semantic_method_candidate_sort_key(package_name, right))
    });
    candidates.dedup_by(|left, right| {
        left.display_name == right.display_name && left.package == right.package
    });

    Some(candidates)
}

fn method_symbol_defining_packages(
    method_symbols: &[&WorkspaceSymbol],
    receiver_package: &str,
) -> HashMap<String, String> {
    let mut packages = HashMap::new();
    for symbol in method_symbols {
        packages.entry(symbol.name.clone()).or_insert_with(|| {
            symbol.container_name.clone().unwrap_or_else(|| receiver_package.to_string())
        });
    }
    packages
}

fn is_confident_method_candidate(candidate: &DefinitionCandidate) -> bool {
    candidate.confidence == Confidence::High
        && matches!(candidate.kind, EntityKind::Method | EntityKind::Subroutine)
}

fn semantic_method_candidate_sort_key(
    receiver_package: &str,
    candidate: &DefinitionCandidate,
) -> (u8, String, String) {
    (
        semantic_method_sort_rank(receiver_package, candidate),
        candidate.display_name.clone(),
        candidate.package.clone().unwrap_or_default(),
    )
}

fn semantic_method_sort_tier(
    receiver_package: &str,
    candidate: &DefinitionCandidate,
) -> &'static str {
    match semantic_method_sort_rank(receiver_package, candidate) {
        0 => "2",
        1 => "3",
        _ => "4",
    }
}

fn semantic_method_sort_rank(receiver_package: &str, candidate: &DefinitionCandidate) -> u8 {
    if candidate.package.as_deref() == Some(receiver_package) {
        match candidate.kind {
            EntityKind::GeneratedMember => 1,
            _ => 0,
        }
    } else {
        2
    }
}

fn semantic_method_detail(
    receiver_package: &str,
    candidate: &DefinitionCandidate,
    evidence: &ReceiverEvidence,
) -> String {
    let defining_pkg = candidate.package.as_deref().unwrap_or(receiver_package);
    let base = match candidate.kind {
        EntityKind::GeneratedMember => format!("generated accessor from {defining_pkg}"),
        _ if defining_pkg == receiver_package => format!("method from {receiver_package}"),
        _ => format!("inherited method from {defining_pkg}"),
    };
    detail_with_evidence(base, evidence)
}

fn semantic_method_documentation(
    receiver_package: &str,
    candidate: &DefinitionCandidate,
) -> String {
    let defining_pkg = candidate.package.as_deref().unwrap_or(receiver_package);
    match candidate.kind {
        EntityKind::GeneratedMember => {
            format!("Generated method `{}` from `{defining_pkg}`.", candidate.display_name)
        }
        _ => format!("Method `{}::{}`.", defining_pkg, candidate.display_name),
    }
}

fn build_completion_package_graph(
    index: &WorkspaceIndex,
    source_uris: &HashSet<String>,
) -> PackageGraphIndex {
    let mut graph = PackageGraphIndex::new();

    for uri in source_uris {
        let Some(text) = workspace_text_for_uri(index, uri) else {
            continue;
        };
        let Ok(ast) = parse_workspace_source(&text) else {
            continue;
        };
        let model = SemanticModel::build(&ast, &text);
        let edges: Vec<PackageEdge> = model
            .package_edges()
            .iter()
            .filter(|edge| {
                matches!(edge.confidence, Confidence::High | Confidence::Medium)
                    && !matches!(edge.provenance, Provenance::DynamicBoundary)
            })
            .cloned()
            .collect();
        if !edges.is_empty() {
            graph.add_edges(uri, semantic_file_id(uri), edges);
        }
    }

    graph
}

fn parse_workspace_source(text: &str) -> Result<perl_parser_core::ast::Node, String> {
    let mut parser = perl_semantic_analyzer::Parser::new(text);
    parser.parse().map_err(|err| err.to_string())
}

fn workspace_text_for_uri(index: &WorkspaceIndex, uri: &str) -> Option<String> {
    index.document_store().get_text(uri).or_else(|| {
        perl_workspace::workspace_index::uri_to_fs_path(uri)
            .and_then(|path| std::fs::read_to_string(path).ok())
    })
}

fn semantic_file_id(uri: &str) -> FileId {
    let mut hasher = DefaultHasher::new();
    uri.hash(&mut hasher);
    FileId(hasher.finish())
}

#[cfg(test)]
mod visible_symbol_completion_tests {
    use super::{VisibleSymbol, VisibleSymbolSource, is_live_visible_completion_candidate};
    use perl_semantic_facts::{Confidence, EntityId};

    fn visible(source: VisibleSymbolSource, confidence: Confidence) -> VisibleSymbol {
        VisibleSymbol {
            name: "candidate".to_string(),
            entity_id: Some(EntityId(1)),
            source,
            confidence,
            context: None,
        }
    }

    #[test]
    fn live_visible_completion_filter_accepts_only_high_confidence_import_export_sources() {
        assert!(is_live_visible_completion_candidate(&visible(
            VisibleSymbolSource::ExplicitImport,
            Confidence::High,
        )));
        assert!(is_live_visible_completion_candidate(&visible(
            VisibleSymbolSource::DefaultExport,
            Confidence::High,
        )));
        assert!(is_live_visible_completion_candidate(&visible(
            VisibleSymbolSource::ExportTag,
            Confidence::High,
        )));

        assert!(!is_live_visible_completion_candidate(&visible(
            VisibleSymbolSource::ExplicitImport,
            Confidence::Medium,
        )));
        assert!(!is_live_visible_completion_candidate(&visible(
            VisibleSymbolSource::Generated,
            Confidence::High,
        )));
        assert!(!is_live_visible_completion_candidate(&visible(
            VisibleSymbolSource::DynamicUnknown,
            Confidence::High,
        )));
        assert!(!is_live_visible_completion_candidate(&visible(
            VisibleSymbolSource::LocalLexical,
            Confidence::High,
        )));
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
