//! Completion context analysis

use perl_semantic_analyzer::symbol::{ScopeId, ScopeKind, SymbolKind, SymbolTable};

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
    /// Whether we're completing a module name after `use` or `require`
    pub in_use_statement: bool,
    /// Current package context
    pub current_package: String,
    /// Prefix text before cursor
    pub prefix: String,
    /// Start position of the prefix (for text edit range calculation)
    pub prefix_start: usize,
    /// The innermost scope containing the cursor position
    pub cursor_scope_id: ScopeId,
}

impl CompletionContext {
    /// Return the start of the method token for an arrow-form completion.
    ///
    /// Arrow receivers remain part of `prefix` for receiver classification, but
    /// completion edits replace only the method token after `->`.
    pub(crate) fn method_text_edit_start(&self, source: &str) -> usize {
        if source.get(self.prefix_start..self.position) == Some(self.prefix.as_str())
            && let Some(arrow_start) = self.prefix.rfind("->")
        {
            return self.prefix_start + arrow_start + 2;
        }

        self.prefix_start
    }

    pub(crate) fn detect_current_package(symbol_table: &SymbolTable, position: usize) -> String {
        // First, check for innermost package scope containing the position
        let mut scope_start: Option<usize> = None;
        for scope in symbol_table.scopes.values() {
            if scope.kind == ScopeKind::Package
                && scope.location.start <= position
                && position <= scope.location.end
                && scope_start.is_none_or(|s| scope.location.start >= s)
            {
                scope_start = Some(scope.location.start);
            }
        }

        if let Some(start) = scope_start
            && let Some(sym) = symbol_table
                .symbols
                .values()
                .flat_map(|v| v.iter())
                .find(|sym| sym.kind == SymbolKind::Package && sym.location.start == start)
        {
            return sym.name.clone();
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

    // Completion context is assembled from independent lexical/provider facts.
    #[allow(clippy::too_many_arguments)]
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
            in_use_statement: false,
            current_package,
            prefix,
            prefix_start,
            cursor_scope_id: 0,
        }
    }
}
