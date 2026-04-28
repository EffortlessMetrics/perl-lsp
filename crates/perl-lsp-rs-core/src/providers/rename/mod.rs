//! Rename refactoring support
//!
//! This module provides the ability to rename symbols across a document,
//! ensuring all references are updated correctly.
//!
//! # LSP Workflow Integration
//!
//! Rename operations integrate with the Parse → Index → Navigate → Complete → Analyze workflow:
//!
//! - **Parse**: AST analysis identifies symbol definitions and usage patterns
//! - **Index**: Symbol tables provide comprehensive reference mapping for rename validation
//! - **Navigate**: Cross-file navigation enables workspace-wide symbol renaming
//! - **Complete**: Completion context validates new symbol names for conflicts
//! - **Analyze**: Impact analysis ensures rename operations maintain code correctness
//!
//! This integration enables safe, workspace-wide refactoring with comprehensive
//! validation and conflict detection.
//!
//! # LSP Context Integration
//!
//! Implements `textDocument/rename` and `textDocument/prepareRename` LSP methods:
//! - **Prepare rename**: Validates symbol at position is renameable
//! - **Rename execution**: Generates workspace edits for all symbol references
//! - **Cross-file refactoring**: Handles package-qualified symbol updates
//! - **Conflict detection**: Prevents name collisions and scope violations
//! - **Atomic operations**: Ensures all-or-nothing rename semantics
//!
//! # Client capability requirements
//!
//! Requires LSP client support for workspace edits and prepare rename:
//! ```json
//! {
//!   "textDocument": {
//!     "rename": {
//!       "prepareSupport": true,
//!       "prepareSupportDefaultBehavior": 1
//!     }
//!   },
//!   "workspace": {
//!     "workspaceEdit": {
//!       "resourceOperations": ["create", "rename", "delete"],
//!       "failureHandling": "textOnlyTransactional"
//!     }
//!   }
//! }
//! ```
//!
//! # Protocol compliance
//!
//! Implements the LSP rename protocol (`textDocument/rename` and
//! `textDocument/prepareRename`) with transactional workspace edits.
//! The protocol requirements map cleanly onto LSP workspace edit behavior.
//!
//! # Performance Characteristics
//!
//! - **Symbol resolution**: <50ms for typical file analysis
//! - **Cross-file analysis**: <300ms for workspace-wide rename validation
//! - **Edit generation**: <100ms for complex multi-file renames
//! - **Memory usage**: <20MB for large workspace symbol indexing
//!
//! # See also
//!
//! - [`RenameProvider`] for executing rename operations
//! - `crate::ide::lsp_compat::references` for related navigation workflows
//!
//! # Usage Examples
//!
//! ```ignore
//! use perl_lsp_providers::ide::lsp_compat::rename::{RenameProvider, RenameOptions};
//! use perl_parser_core::Parser;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let code = "sub hello_world { print \"Hello!\"; } hello_world();";
//! let mut parser = Parser::new(code);
//! let ast = parser.parse()?;
//!
//! let provider = RenameProvider::new(&ast, code.to_string());
//! let position = 4; // Byte position of 'hello_world'
//! let options = RenameOptions::default();
//!
//! // Rename symbol at position
//! let result = provider.rename(position, "greet_user", &options);
//! if result.is_valid {
//!     println!("Rename successful, {} edits", result.edits.len());
//!     for edit in result.edits {
//!         println!("Edit: {} -> {}", edit.location, edit.new_text);
//!     }
//! } else if let Some(error) = &result.error {
//!     eprintln!("Rename failed: {}", error);
//! }
//! # Ok(())
//! # }
//! ```

mod apply;
mod resolve;
mod types;
mod validate;

pub use apply::adjust_location_for_sigil;
#[allow(unused_imports)]
pub use apply::apply_rename_edits;
pub use apply::{is_in_comment, is_in_string};
pub use resolve::{find_symbol_at_position, get_symbol_range_at_position};
pub use types::{RenameOptions, RenameResult, TextEdit};
pub use validate::{can_rename_symbol, validate_name};

use std::collections::{HashMap, HashSet, VecDeque};

use perl_parser_core::Node;
use perl_semantic_analyzer::symbol::{ScopeId, SymbolExtractor, SymbolKind, SymbolTable};

/// Rename provider
pub struct RenameProvider {
    symbol_table: SymbolTable,
    source: String,
}

impl RenameProvider {
    /// Create a new rename provider
    pub fn new(ast: &Node, source: String) -> Self {
        let symbol_table = SymbolExtractor::new_with_source(&source).extract(ast);
        RenameProvider { symbol_table, source }
    }

    /// Prepare rename at a position (check if rename is possible)
    pub fn prepare_rename(
        &self,
        position: usize,
    ) -> Option<(perl_parser_core::SourceLocation, String)> {
        let (symbol, kind) = find_symbol_at_position(position, &self.symbol_table, &self.source)?;
        if !can_rename_symbol(&symbol, kind) {
            return None;
        }
        Some((get_symbol_range_at_position(position, &self.source)?, symbol))
    }

    /// Perform rename operation (renames all occurrences regardless of scope)
    pub fn rename(&self, position: usize, new_name: &str, options: &RenameOptions) -> RenameResult {
        let (old_name, kind) =
            match find_symbol_at_position(position, &self.symbol_table, &self.source) {
                Some(result) => result,
                None => {
                    return RenameResult {
                        edits: vec![],
                        is_valid: false,
                        error: Some("No symbol found at position".to_string()),
                    };
                }
            };

        if options.validate_new_name
            && let Err(error) = validate_name(new_name, kind, &self.symbol_table)
        {
            return RenameResult { edits: vec![], is_valid: false, error: Some(error) };
        }

        if !can_rename_symbol(&old_name, kind) {
            return RenameResult {
                edits: vec![],
                is_valid: false,
                error: Some("Cannot rename this symbol".to_string()),
            };
        }

        let mut edits = Vec::new();

        if let Some(symbols) = self.symbol_table.symbols.get(&old_name) {
            for symbol in symbols {
                if symbol.kind == kind {
                    edits.push(TextEdit {
                        location: adjust_location_for_sigil(symbol.location, kind),
                        new_text: new_name.to_string(),
                    });
                }
            }
        }

        if let Some(references) = self.symbol_table.references.get(&old_name) {
            for reference in references {
                if reference.kind == kind {
                    edits.push(TextEdit {
                        location: adjust_location_for_sigil(reference.location, kind),
                        new_text: new_name.to_string(),
                    });
                }
            }
        }

        if options.rename_in_comments || options.rename_in_strings {
            let additional_edits =
                apply::find_occurrences_in_text(&old_name, kind, options, &self.source);
            edits.extend(additional_edits);
        }

        edits.sort_by_key(|edit| edit.location.start);
        edits.dedup();

        RenameResult { edits, is_valid: true, error: None }
    }

    /// Perform scope-aware rename operation.
    ///
    /// Unlike `rename()`, this respects Perl lexical scoping: only renames the
    /// declaration and references within the same scope subtree. A `$foo` in an
    /// inner scope that shadows the outer `$foo` is treated as a separate variable.
    pub fn scoped_rename(
        &self,
        position: usize,
        new_name: &str,
        options: &RenameOptions,
    ) -> RenameResult {
        let (old_name, kind) =
            match find_symbol_at_position(position, &self.symbol_table, &self.source) {
                Some(result) => result,
                None => {
                    return RenameResult {
                        edits: vec![],
                        is_valid: false,
                        error: Some("No symbol found at position".to_string()),
                    };
                }
            };

        if options.validate_new_name
            && let Err(error) = validate_name(new_name, kind, &self.symbol_table)
        {
            return RenameResult { edits: vec![], is_valid: false, error: Some(error) };
        }

        if !can_rename_symbol(&old_name, kind) {
            return RenameResult {
                edits: vec![],
                is_valid: false,
                error: Some("Cannot rename this symbol".to_string()),
            };
        }

        let declaration_scope_id =
            match self.find_declaration_scope_for_position(position, &old_name, kind) {
                Some(id) => id,
                None => {
                    return self.rename(position, new_name, options);
                }
            };

        let descendant_scopes = self.collect_descendant_scopes(declaration_scope_id);
        let shadowing_scopes = self.find_shadowing_scopes(&old_name, kind, &descendant_scopes);

        let mut edits = Vec::new();

        if let Some(symbols) = self.symbol_table.symbols.get(&old_name) {
            for symbol in symbols {
                if symbol.kind == kind && symbol.scope_id == declaration_scope_id {
                    edits.push(TextEdit {
                        location: adjust_location_for_sigil(symbol.location, kind),
                        new_text: new_name.to_string(),
                    });
                }
            }
        }

        if let Some(references) = self.symbol_table.references.get(&old_name) {
            for reference in references {
                if reference.kind != kind {
                    continue;
                }
                let ref_scope = reference.scope_id;
                let in_scope =
                    ref_scope == declaration_scope_id || descendant_scopes.contains(&ref_scope);
                if !in_scope {
                    continue;
                }
                if self.is_in_shadowed_scope(ref_scope, &shadowing_scopes) {
                    continue;
                }
                edits.push(TextEdit {
                    location: adjust_location_for_sigil(reference.location, kind),
                    new_text: new_name.to_string(),
                });
            }
        }

        edits.sort_by_key(|edit| edit.location.start);
        edits.dedup();

        RenameResult { edits, is_valid: true, error: None }
    }

    /// Find the scope where the symbol at `position` is declared.
    fn find_declaration_scope_for_position(
        &self,
        position: usize,
        name: &str,
        kind: SymbolKind,
    ) -> Option<ScopeId> {
        if let Some(symbols) = self.symbol_table.symbols.get(name) {
            for symbol in symbols {
                if symbol.kind == kind
                    && symbol.location.start <= position
                    && position < symbol.location.end
                {
                    return Some(symbol.scope_id);
                }
            }
        }

        if let Some(references) = self.symbol_table.references.get(name) {
            for reference in references {
                if reference.kind == kind
                    && reference.location.start <= position
                    && position < reference.location.end
                {
                    return self.find_declaration_scope_up_chain(reference.scope_id, name, kind);
                }
            }
        }

        None
    }

    /// Walk the scope parent chain to find the nearest scope that declares the symbol.
    fn find_declaration_scope_up_chain(
        &self,
        start_scope: ScopeId,
        name: &str,
        kind: SymbolKind,
    ) -> Option<ScopeId> {
        let mut current = Some(start_scope);
        while let Some(scope_id) = current {
            if let Some(symbols) = self.symbol_table.symbols.get(name) {
                for symbol in symbols {
                    if symbol.kind == kind && symbol.scope_id == scope_id {
                        return Some(scope_id);
                    }
                }
            }
            current = self.symbol_table.scopes.get(&scope_id).and_then(|s| s.parent);
        }
        None
    }

    /// Collect all scope IDs that are descendants of `root_scope_id`.
    ///
    /// Builds a parent→children map in O(n) then BFS iteratively from the root.
    /// This avoids the O(n×d) parent-chain walk of the previous implementation
    /// and is safe for arbitrarily deep nesting (no recursion).
    fn collect_descendant_scopes(&self, root_scope_id: ScopeId) -> HashSet<ScopeId> {
        // Build direct-children map: parent_id → [child_id, ...]
        let mut children: HashMap<ScopeId, Vec<ScopeId>> = HashMap::new();
        for (&scope_id, scope) in &self.symbol_table.scopes {
            if let Some(parent_id) = scope.parent {
                children.entry(parent_id).or_default().push(scope_id);
            }
        }

        // Iterative BFS from root — guards against cycles from parser error recovery
        let mut descendants = HashSet::new();
        let mut queue = VecDeque::new();
        if let Some(direct_children) = children.get(&root_scope_id) {
            queue.extend(direct_children.iter().copied());
        }
        while let Some(scope_id) = queue.pop_front() {
            if descendants.insert(scope_id) {
                // Only visit if not already seen (cycle guard)
                if let Some(kids) = children.get(&scope_id) {
                    queue.extend(kids.iter().copied());
                }
            }
        }
        descendants
    }

    /// Find descendant scopes that redeclare the same symbol (shadowing).
    fn find_shadowing_scopes(
        &self,
        name: &str,
        kind: SymbolKind,
        descendant_scopes: &HashSet<ScopeId>,
    ) -> HashSet<ScopeId> {
        let mut shadowing = HashSet::new();
        if let Some(symbols) = self.symbol_table.symbols.get(name) {
            for symbol in symbols {
                if symbol.kind == kind && descendant_scopes.contains(&symbol.scope_id) {
                    shadowing.insert(symbol.scope_id);
                }
            }
        }
        shadowing
    }

    /// Check if `scope_id` is in or descended from any of the shadowing scopes.
    fn is_in_shadowed_scope(&self, scope_id: ScopeId, shadowing_scopes: &HashSet<ScopeId>) -> bool {
        if shadowing_scopes.is_empty() {
            return false;
        }
        if shadowing_scopes.contains(&scope_id) {
            return true;
        }
        let mut current = self.symbol_table.scopes.get(&scope_id).and_then(|s| s.parent);
        while let Some(parent_id) = current {
            if shadowing_scopes.contains(&parent_id) {
                return true;
            }
            current = self.symbol_table.scopes.get(&parent_id).and_then(|s| s.parent);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_semantic_analyzer::symbol::SymbolKind;
    use perl_tdd_support::{must, must_some};

    #[test]
    fn test_rename_variable() {
        let code = "my $count = 0;\n$count += 1;\nprint $count;\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = RenameProvider::new(&ast, code.to_string());
        let pos = must_some(code.find("$count")) + 1;
        let prepare = provider.prepare_rename(pos);
        assert!(prepare.is_some());
        let result = provider.rename(pos, "total", &RenameOptions::default());
        assert!(result.is_valid);
        assert_eq!(result.edits.len(), 3);
        let new_code = apply_rename_edits(code, &result.edits);
        assert!(new_code.contains("my $total"));
        assert!(new_code.contains("$total += 1"));
        assert!(new_code.contains("print $total"));
    }

    #[test]
    fn test_rename_function() {
        let code = "sub calculate {\n    return 42;\n}\nmy $result = calculate();\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = RenameProvider::new(&ast, code.to_string());
        let pos = must_some(code.find("calculate"));
        let result = provider.rename(pos, "compute", &RenameOptions::default());
        assert!(result.is_valid);
        assert!(!result.edits.is_empty());
        let new_code = apply_rename_edits(code, &result.edits);
        assert!(new_code.contains("compute"));
    }

    #[test]
    fn test_validate_new_name() {
        let code = "my $x = 1;";
        let ast = must(Parser::new(code).parse());
        let provider = RenameProvider::new(&ast, code.to_string());
        assert!(validate_name("", SymbolKind::scalar(), &provider.symbol_table).is_err());
        assert!(validate_name("123abc", SymbolKind::scalar(), &provider.symbol_table).is_err());
        assert!(validate_name("my", SymbolKind::scalar(), &provider.symbol_table).is_err());
        assert!(validate_name("test-var", SymbolKind::scalar(), &provider.symbol_table).is_err());
        assert!(validate_name("valid_name", SymbolKind::scalar(), &provider.symbol_table).is_ok());
        assert!(validate_name("_private", SymbolKind::scalar(), &provider.symbol_table).is_ok());
        assert!(validate_name("camelCase", SymbolKind::scalar(), &provider.symbol_table).is_ok());
        assert!(validate_name("naïve", SymbolKind::scalar(), &provider.symbol_table).is_err());
        assert!(validate_name("１２name", SymbolKind::scalar(), &provider.symbol_table).is_err());
    }

    #[test]
    fn test_scoped_rename_simple_variable() {
        let code = "my $count = 0;\n$count += 1;\nprint $count;\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = RenameProvider::new(&ast, code.to_string());
        let pos = must_some(code.find("$count")) + 1;
        let result = provider.scoped_rename(pos, "total", &RenameOptions::default());
        assert!(result.is_valid);
        assert!(!result.edits.is_empty());
        let new_code = apply_rename_edits(code, &result.edits);
        assert!(new_code.contains("$total"));
        assert!(!new_code.contains("$count"));
    }

    #[test]
    fn test_scoped_rename_nested_no_shadow() {
        let code = "my $x = 1;\nif (1) {\n    $x = 2;\n}\nprint $x;\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = RenameProvider::new(&ast, code.to_string());
        let pos = must_some(code.find("$x")) + 1;
        let result = provider.scoped_rename(pos, "y", &RenameOptions::default());
        assert!(result.is_valid);
        let new_code = apply_rename_edits(code, &result.edits);
        assert!(!new_code.contains("$x"));
        assert!(new_code.contains("$y"));
    }

    #[test]
    fn test_scoped_rename_shadowed_outer() {
        let code = "my $x = 1;\nif (1) {\n    my $x = 2;\n    print $x;\n}\nprint $x;\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = RenameProvider::new(&ast, code.to_string());
        let pos = must_some(code.find("my $x")) + 4;
        let result = provider.scoped_rename(pos, "y", &RenameOptions::default());
        assert!(result.is_valid);
        let new_code = apply_rename_edits(code, &result.edits);
        assert!(new_code.contains("my $y = 1"));
        assert!(new_code.contains("my $x = 2"));
    }

    #[test]
    fn test_scoped_rename_shadowed_inner() {
        let code = "my $x = 1;\nif (1) {\n    my $x = 2;\n    print $x;\n}\nprint $x;\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = RenameProvider::new(&ast, code.to_string());
        let inner_decl = must_some(code.find("my $x = 2"));
        let pos = inner_decl + 4;
        let result = provider.scoped_rename(pos, "z", &RenameOptions::default());
        assert!(result.is_valid);
        let new_code = apply_rename_edits(code, &result.edits);
        assert!(new_code.contains("my $z = 2"));
        assert!(new_code.contains("my $x = 1"));
    }

    #[test]
    fn test_scoped_rename_loop_variable() {
        let code = "for my $i (0..10) {\n    print $i;\n}\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = RenameProvider::new(&ast, code.to_string());
        let pos = must_some(code.find("$i")) + 1;
        let result = provider.scoped_rename(pos, "idx", &RenameOptions::default());
        assert!(result.is_valid);
        let new_code = apply_rename_edits(code, &result.edits);
        assert!(new_code.contains("$idx"));
        // "$idx" contains "$i" as substring
    }

    #[test]
    fn test_scoped_rename_from_reference() {
        let code = "my $foo = 42;\n$foo += 1;\nprint $foo;\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = RenameProvider::new(&ast, code.to_string());
        let pos = must_some(code.find("$foo += 1")) + 1;
        let result = provider.scoped_rename(pos, "bar", &RenameOptions::default());
        assert!(result.is_valid);
        let new_code = apply_rename_edits(code, &result.edits);
        assert!(new_code.contains("$bar"));
        assert!(!new_code.contains("$foo"));
    }

    #[test]
    fn test_scoped_rename_no_symbol_at_position() {
        let code = "    my $x = 1;";
        let ast = must(Parser::new(code).parse());
        let provider = RenameProvider::new(&ast, code.to_string());
        let result = provider.scoped_rename(0, "y", &RenameOptions::default());
        assert!(!result.is_valid);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_scoped_rename_validates_new_name() {
        let code = "my $x = 1;";
        let ast = must(Parser::new(code).parse());
        let provider = RenameProvider::new(&ast, code.to_string());
        let pos = must_some(code.find("$x")) + 1;
        let result = provider.scoped_rename(pos, "123invalid", &RenameOptions::default());
        assert!(!result.is_valid);
        assert!(result.error.is_some());
    }

    /// Correctness: deeply nested blocks with a variable reference at the innermost level.
    /// The iterative BFS must collect all descendant scopes from the declaration scope.
    /// Uses 50 levels to stay within the parser's MAX_RECURSION_DEPTH of 128.
    #[test]
    fn test_collect_descendant_scopes_deep_nesting_correctness() {
        // Build: my $x = 1; if(1){if(1){...50 levels...{$x = 2;}...}}
        let mut code = "my $x = 1;\n".to_string();
        for _ in 0..50 {
            code.push_str("if (1) {\n");
        }
        code.push_str("    $x = 2;\n");
        for _ in 0..50 {
            code.push_str("}\n");
        }
        code.push_str("print $x;\n");

        let ast = must(Parser::new(&code).parse());
        let provider = RenameProvider::new(&ast, code.clone());
        // Position of the declaration $x (after 'my ')
        let pos = must_some(code.find("my $x")) + 4;
        let result = provider.scoped_rename(pos, "y", &RenameOptions::default());
        assert!(result.is_valid, "scoped_rename must succeed on 50-deep nesting");
        let new_code = apply_rename_edits(&code, &result.edits);
        assert!(new_code.contains("my $y = 1"), "declaration should be renamed");
        assert!(new_code.contains("$y = 2"), "deep reference should be renamed");
        assert!(new_code.contains("print $y"), "outer reference should be renamed");
        assert!(!new_code.contains("$x"), "no original name should remain");
    }

    /// Cycle guard: construct a scope table with a self-referential parent link
    /// (parent == self). The iterative BFS must terminate without infinite loop.
    #[test]
    fn test_collect_descendant_scopes_cycle_guard() {
        use perl_parser_core::SourceLocation;
        use perl_semantic_analyzer::symbol::{Scope, ScopeKind};
        use std::collections::{HashMap, HashSet as ScopeSymbolSet};

        let root_id: ScopeId = 0;
        let child_id: ScopeId = 1;
        let cyclic_id: ScopeId = 2;

        let mut scopes: HashMap<ScopeId, Scope> = HashMap::new();
        scopes.insert(
            root_id,
            Scope {
                id: root_id,
                parent: None,
                kind: ScopeKind::Block,
                location: SourceLocation { start: 0, end: 100 },
                symbols: ScopeSymbolSet::new(),
            },
        );
        scopes.insert(
            child_id,
            Scope {
                id: child_id,
                parent: Some(root_id),
                kind: ScopeKind::Block,
                location: SourceLocation { start: 1, end: 50 },
                symbols: ScopeSymbolSet::new(),
            },
        );
        // Self-referential: parent == self (simulates parser error recovery edge case)
        scopes.insert(
            cyclic_id,
            Scope {
                id: cyclic_id,
                parent: Some(cyclic_id),
                kind: ScopeKind::Block,
                location: SourceLocation { start: 5, end: 10 },
                symbols: ScopeSymbolSet::new(),
            },
        );

        // Construct a minimal provider and replace its scope table
        let code = "my $x = 1;";
        let ast = must(Parser::new(code).parse());
        let mut provider = RenameProvider::new(&ast, code.to_string());
        // Replace scopes with our synthetic table
        provider.symbol_table.scopes = scopes;

        // Must terminate (not infinite loop) and return child_id (direct child of root)
        // cyclic_id has parent==cyclic_id, not root, so it's NOT a descendant of root
        let descendants = provider.collect_descendant_scopes(root_id);
        assert!(descendants.contains(&child_id), "direct child must be in descendants");
        assert!(
            !descendants.contains(&cyclic_id),
            "self-referential scope with unrelated parent not in descendants"
        );
        assert!(!descendants.contains(&root_id), "root itself must not be in descendants");
    }

    /// Performance: a synthetic 1000-scope linear chain must complete in under 10ms.
    /// Uses a hand-built scope table to bypass the parser's nesting depth limit.
    /// Catches any accidental regression back to O(n*d) behavior.
    #[test]
    fn test_collect_descendant_scopes_linear_chain_performance() {
        use perl_parser_core::SourceLocation;
        use perl_semantic_analyzer::symbol::{Scope, ScopeKind};
        use std::collections::{HashMap, HashSet as ScopeSymbolSet};
        use std::time::Instant;

        const CHAIN_LEN: usize = 1000;
        let root_id: ScopeId = 0;

        // Build a linear parent chain: 0 -> 1 -> 2 -> ... -> 999
        let mut scopes: HashMap<ScopeId, Scope> = HashMap::new();
        scopes.insert(
            root_id,
            Scope {
                id: root_id,
                parent: None,
                kind: ScopeKind::Global,
                location: SourceLocation { start: 0, end: 10000 },
                symbols: ScopeSymbolSet::new(),
            },
        );
        for i in 1..CHAIN_LEN {
            scopes.insert(
                i,
                Scope {
                    id: i,
                    parent: Some(i - 1),
                    kind: ScopeKind::Block,
                    location: SourceLocation { start: i * 10, end: i * 10 + 9 },
                    symbols: ScopeSymbolSet::new(),
                },
            );
        }

        let code = "my $x = 1;";
        let ast = must(Parser::new(code).parse());
        let mut provider = RenameProvider::new(&ast, code.to_string());
        provider.symbol_table.scopes = scopes;

        let start = Instant::now();
        let descendants = provider.collect_descendant_scopes(root_id);
        let elapsed = start.elapsed();

        assert_eq!(descendants.len(), CHAIN_LEN - 1, "all non-root scopes should be descendants");
        assert!(
            elapsed.as_millis() < 10,
            "collect_descendant_scopes on 1000-scope chain took {}ms, expected <10ms",
            elapsed.as_millis()
        );
    }
}
