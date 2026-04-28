//! Scope-distance ranking for completion items.
//!
//! Computes a distance tier from the cursor's scope to each symbol's definition
//! scope. Closer symbols rank higher in completion results:
//!
//! - **Immediate**: symbol defined in the same scope as the cursor
//! - **Parent**: symbol defined in an enclosing scope (1+ hops up the chain)
//! - **Package**: symbol defined at package/global scope level
//! - **Workspace**: symbol from another file via the workspace index

use perl_semantic_analyzer::symbol::{ScopeId, ScopeKind, SymbolTable};

/// Scope-distance tier for sorting completions.
///
/// Variants are ordered from closest (highest priority) to farthest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeDistance {
    /// Symbol defined in the same scope as the cursor.
    Immediate,
    /// Symbol defined in an enclosing (parent) scope.
    Parent,
    /// Symbol defined at package or global scope.
    PackageLevel,
    /// Symbol from the workspace index (another file).
    Workspace,
}

impl ScopeDistance {
    /// Returns a single-character sort key for embedding in `sort_text`.
    ///
    /// Lexicographic ordering: `'a'` < `'b'` < `'c'` < `'d'` ensures that
    /// immediate-scope items sort before parent, before package, before workspace.
    pub fn sort_key(self) -> char {
        match self {
            Self::Immediate => 'a',
            Self::Parent => 'b',
            Self::PackageLevel => 'c',
            Self::Workspace => 'd',
        }
    }
}

fn last_unmatched_open_brace(source: &str) -> Option<usize> {
    let mut stack = Vec::new();
    for (idx, ch) in source.char_indices() {
        match ch {
            '{' => stack.push(idx),
            '}' => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.pop()
}

fn scope_depth(symbol_table: &SymbolTable, scope_id: ScopeId) -> usize {
    let mut depth = 0usize;
    let mut current = scope_id;

    while let Some(scope) = symbol_table.scopes.get(&current) {
        let Some(parent) = scope.parent else {
            break;
        };
        depth += 1;
        current = parent;
    }

    depth
}

/// Find the innermost scope relevant to `position`.
///
/// Walks all scopes in the symbol table and returns the one whose location
/// range contains the position and has the latest start (i.e., most specific).
///
/// During interactive editing the semantic analyzer may leave an unfinished
/// child block with an `end` range before EOF even though the user is still
/// typing inside it. When the cursor is at EOF, we use the last unmatched `{`
/// as a scope hint and prefer the deepest scope that started before that brace.
pub fn scope_at_position(symbol_table: &SymbolTable, source: &str, position: usize) -> ScopeId {
    let mut containing: Option<(ScopeId, usize, usize)> = None;

    for scope in symbol_table.scopes.values() {
        if scope.location.start <= position && position <= scope.location.end {
            let depth = scope_depth(symbol_table, scope.id);
            let is_better_containing = containing.is_none_or(|(_, best_start, best_depth)| {
                scope.location.start > best_start
                    || (scope.location.start == best_start && depth > best_depth)
            });
            if is_better_containing {
                containing = Some((scope.id, scope.location.start, depth));
            }
        }
    }

    if position == source.len()
        && let Some(brace_pos) = last_unmatched_open_brace(source)
    {
        let mut eof_hint = containing;
        for scope in symbol_table.scopes.values() {
            if scope.location.start <= brace_pos {
                let depth = scope_depth(symbol_table, scope.id);
                let is_better_hint = eof_hint.is_none_or(|(_, best_start, best_depth)| {
                    scope.location.start > best_start
                        || (scope.location.start == best_start && depth > best_depth)
                });
                if is_better_hint {
                    eof_hint = Some((scope.id, scope.location.start, depth));
                }
            }
        }
        return eof_hint.map_or(0, |(id, _, _)| id);
    }

    containing.map_or(0, |(id, _, _)| id)
}

/// Compute the scope distance from `cursor_scope` to `symbol_scope`.
///
/// Walks the parent chain from `cursor_scope` upward:
/// - If `symbol_scope` is the cursor's own scope, returns `Immediate`.
/// - If `symbol_scope` is an ancestor, returns `Parent`.
/// - If the walk reaches a `Package` or `Global` scope that contains the symbol,
///   returns `PackageLevel`.
/// - Otherwise (e.g., different branch, different file), returns `Workspace`.
pub fn compute_scope_distance(
    symbol_table: &SymbolTable,
    cursor_scope: ScopeId,
    symbol_scope: ScopeId,
) -> ScopeDistance {
    if cursor_scope == symbol_scope {
        return ScopeDistance::Immediate;
    }

    // Walk up from cursor scope looking for the symbol's scope
    let mut current = cursor_scope;
    let mut hops = 0u32;

    while let Some(scope) = symbol_table.scopes.get(&current) {
        if let Some(parent_id) = scope.parent {
            hops += 1;

            if parent_id == symbol_scope {
                // Check if the symbol scope is a package/global scope
                if let Some(parent_scope) = symbol_table.scopes.get(&parent_id)
                    && matches!(parent_scope.kind, ScopeKind::Global | ScopeKind::Package)
                {
                    return ScopeDistance::PackageLevel;
                }
                return ScopeDistance::Parent;
            }

            current = parent_id;
        } else {
            // Reached the root without finding the symbol scope
            break;
        }

        // Safety limit to prevent infinite loops on malformed scope trees
        if hops > 100 {
            break;
        }
    }

    // The symbol scope was not found in our parent chain.
    // Check if the symbol is at package/global level.
    if let Some(sym_scope) = symbol_table.scopes.get(&symbol_scope)
        && matches!(sym_scope.kind, ScopeKind::Global | ScopeKind::Package)
    {
        return ScopeDistance::PackageLevel;
    }

    ScopeDistance::Workspace
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::SourceLocation;
    use perl_semantic_analyzer::symbol::{Scope, ScopeKind, SymbolTable};
    use std::collections::HashSet;

    /// Build a minimal symbol table with a known scope hierarchy for testing.
    fn build_test_table() -> SymbolTable {
        // Hierarchy:
        //   0 (Global, 0..100)
        //   +-- 1 (Package, 5..95)
        //       +-- 2 (Subroutine, 10..80)
        //           +-- 3 (Block, 20..50)
        let mut table = SymbolTable::new();

        // Override scope 0 to have end=100
        table.scopes.insert(
            0,
            Scope {
                id: 0,
                parent: None,
                kind: ScopeKind::Global,
                location: SourceLocation { start: 0, end: 100 },
                symbols: HashSet::new(),
            },
        );

        table.scopes.insert(
            1,
            Scope {
                id: 1,
                parent: Some(0),
                kind: ScopeKind::Package,
                location: SourceLocation { start: 5, end: 95 },
                symbols: HashSet::new(),
            },
        );

        table.scopes.insert(
            2,
            Scope {
                id: 2,
                parent: Some(1),
                kind: ScopeKind::Subroutine,
                location: SourceLocation { start: 10, end: 80 },
                symbols: HashSet::new(),
            },
        );

        table.scopes.insert(
            3,
            Scope {
                id: 3,
                parent: Some(2),
                kind: ScopeKind::Block,
                location: SourceLocation { start: 20, end: 50 },
                symbols: HashSet::new(),
            },
        );

        table
    }

    #[test]
    fn test_scope_at_position_innermost_block() {
        let table = build_test_table();
        // Position 30 is inside scope 3 (Block, 20..50)
        assert_eq!(scope_at_position(&table, "", 30), 3);
    }

    #[test]
    fn test_scope_at_position_subroutine() {
        let table = build_test_table();
        // Position 60 is inside scope 2 (Subroutine, 10..80) but outside scope 3
        assert_eq!(scope_at_position(&table, "", 60), 2);
    }

    #[test]
    fn test_scope_at_position_global() {
        let table = build_test_table();
        // Position 98 is inside scope 0 (Global) but outside scope 1 (Package, 5..95)
        assert_eq!(scope_at_position(&table, "", 98), 0);
    }

    #[test]
    fn test_scope_at_position_uses_eof_unmatched_brace_hint() {
        let mut table = build_test_table();
        // Simulate an incomplete block whose recorded end does not yet reach
        // the live cursor position.
        if let Some(scope) = table.scopes.get_mut(&3) {
            scope.location.end = 25;
        }

        let source = "sub process {\n    if (1) {\n        $";
        assert_eq!(scope_at_position(&table, source, source.len()), 3);
    }

    #[test]
    fn test_scope_at_position_does_not_revive_closed_child_block() {
        let mut table = build_test_table();
        if let Some(scope) = table.scopes.get_mut(&3) {
            scope.location.end = 25;
        }
        let source = "sub process {\n    if (1) {\n    }\n    $";
        assert_eq!(scope_at_position(&table, source, source.len()), 2);
    }

    #[test]
    fn test_scope_at_position_prefers_deeper_scope_when_ranges_match() {
        let mut table = SymbolTable::new();
        table.scopes.insert(
            0,
            Scope {
                id: 0,
                parent: None,
                kind: ScopeKind::Global,
                location: SourceLocation { start: 0, end: 100 },
                symbols: HashSet::new(),
            },
        );
        table.scopes.insert(
            1,
            Scope {
                id: 1,
                parent: Some(0),
                kind: ScopeKind::Block,
                location: SourceLocation { start: 10, end: 90 },
                symbols: HashSet::new(),
            },
        );
        table.scopes.insert(
            2,
            Scope {
                id: 2,
                parent: Some(1),
                kind: ScopeKind::Block,
                location: SourceLocation { start: 10, end: 90 },
                symbols: HashSet::new(),
            },
        );

        assert_eq!(scope_at_position(&table, "", 50), 2);
    }

    #[test]
    fn test_scope_distance_immediate() {
        let table = build_test_table();
        assert_eq!(compute_scope_distance(&table, 3, 3), ScopeDistance::Immediate);
    }

    #[test]
    fn test_scope_distance_parent_subroutine() {
        let table = build_test_table();
        // Cursor in scope 3, symbol in scope 2 (subroutine = parent)
        assert_eq!(compute_scope_distance(&table, 3, 2), ScopeDistance::Parent);
    }

    #[test]
    fn test_scope_distance_package_level() {
        let table = build_test_table();
        // Cursor in scope 3, symbol in scope 1 (package)
        assert_eq!(compute_scope_distance(&table, 3, 1), ScopeDistance::PackageLevel);
    }

    #[test]
    fn test_scope_distance_global_level() {
        let table = build_test_table();
        // Cursor in scope 3, symbol in scope 0 (global)
        assert_eq!(compute_scope_distance(&table, 3, 0), ScopeDistance::PackageLevel);
    }

    #[test]
    fn test_scope_distance_different_branch() {
        let mut table = build_test_table();
        // Add a sibling scope 4 under Package scope 1
        table.scopes.insert(
            4,
            Scope {
                id: 4,
                parent: Some(1),
                kind: ScopeKind::Subroutine,
                location: SourceLocation { start: 82, end: 94 },
                symbols: HashSet::new(),
            },
        );

        // Cursor in scope 3 (under scope 2), symbol in scope 4 (sibling branch)
        // scope 4 is Subroutine, not Global/Package, so it returns Workspace
        assert_eq!(compute_scope_distance(&table, 3, 4), ScopeDistance::Workspace);
    }

    #[test]
    fn test_sort_key_ordering() {
        assert!(ScopeDistance::Immediate.sort_key() < ScopeDistance::Parent.sort_key());
        assert!(ScopeDistance::Parent.sort_key() < ScopeDistance::PackageLevel.sort_key());
        assert!(ScopeDistance::PackageLevel.sort_key() < ScopeDistance::Workspace.sort_key());
    }
}
