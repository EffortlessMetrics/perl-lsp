//! Label lookup helpers for `goto LABEL` diagnostics.

use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};

pub(crate) fn has_label(symbol_table: &SymbolTable, label: &str) -> bool {
    symbol_table
        .symbols
        .get(label)
        .is_some_and(|symbols| symbols.iter().any(|symbol| symbol.kind == SymbolKind::Label))
}
