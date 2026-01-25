//! Completion context analysis

use perl_semantic_analyzer::symbol::{ScopeKind, SymbolKind, SymbolTable};

/// Context for completion
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// The position where completion was triggered
    pub position: usize,
    /// The character that triggered completion (if any)
    pub trigger_character: Option<char>,
    /// Whether we're in a string literal
    pub in_string: bool,
    /// Whether we're in a regex
    pub in_regex: bool,
    /// Whether we're in a comment
    pub in_comment: bool,
    /// Current package context
    pub current_package: String,
    /// Prefix text before cursor
    pub prefix: String,
    /// Start position of the prefix (for text edit range calculation)
    pub prefix_start: usize,
}

impl CompletionContext {
    pub(crate) fn detect_current_package(symbol_table: &SymbolTable, position: usize) -> String {
        // First, check for innermost package scope containing the position
        let mut scope_start: Option<usize> = None;
        for scope in symbol_table.scopes.values() {
            if scope.kind == ScopeKind::Package
                && scope.location.start <= position
                && position <= scope.location.end
            {
                if scope_start.is_none_or(|s| scope.location.start >= s) {
                    scope_start = Some(scope.location.start);
                }
            }
        }

        if let Some(start) = scope_start {
            if let Some(sym) = symbol_table
                .symbols
                .values()
                .flat_map(|v| v.iter())
                .find(|sym| sym.kind == SymbolKind::Package && sym.location.start == start)
            {
                return sym.name.clone();
            }
        }

        // Fallback: find last package declaration without block before position
        let mut current = "main".to_string();
        let mut packages: Vec<&perl_semantic_analyzer::symbol::Symbol> = symbol_table
            .symbols
            .values()
            .flat_map(|v| v.iter())
            .filter(|sym| sym.kind == SymbolKind::Package)
            .collect();
        packages.sort_by_key(|sym| sym.location.start);
        for sym in packages {
            if sym.location.start <= position {
                let has_scope = symbol_table.scopes.values().any(|sc| {
                    sc.kind == ScopeKind::Package && sc.location.start == sym.location.start
                });
                if !has_scope {
                    current = sym.name.clone();
                }
            } else {
                break;
            }
        }
        current
    }

    pub(crate) fn new(
        symbol_table: &SymbolTable,
        position: usize,
        trigger_character: Option<char>,
        in_string: bool,
        in_regex: bool,
        in_comment: bool,
        prefix: String,
        prefix_start: usize,
    ) -> Self {
        let current_package = Self::detect_current_package(symbol_table, position);
        CompletionContext {
            position,
            trigger_character,
            in_string,
            in_regex,
            in_comment,
            current_package,
            prefix,
            prefix_start,
        }
    }
}
