//! Rename symbol resolution logic
//!
//! This module provides symbol resolution for rename operations.

use perl_parser_core::SourceLocation;
use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};
use perl_symbol::cursor;

/// Find the symbol at a given position
pub fn find_symbol_at_position(
    position: usize,
    symbol_table: &SymbolTable,
    source: &str,
) -> Option<(String, SymbolKind)> {
    // First check if we're on a definition
    for (name, symbols) in &symbol_table.symbols {
        for symbol in symbols {
            if symbol.location.start <= position && position <= symbol.location.end {
                return Some((name.clone(), symbol.kind));
            }
        }
    }

    // Then check references
    for (name, references) in &symbol_table.references {
        for reference in references {
            if reference.location.start <= position && position <= reference.location.end {
                return Some((name.clone(), reference.kind));
            }
        }
    }

    // Try to extract from source text
    extract_symbol_from_source(position, source)
}

/// Extract symbol from source text at position
pub fn extract_symbol_from_source(position: usize, source: &str) -> Option<(String, SymbolKind)> {
    let (name, kind) = cursor::extract_symbol_from_source(position, source)?;
    Some((name, map_cursor_kind(kind)))
}

/// Get the range of a symbol at position
pub fn get_symbol_range_at_position(position: usize, source: &str) -> Option<SourceLocation> {
    let (start, end) = cursor::get_symbol_range_at_position(position, source)?;
    Some(SourceLocation { start, end })
}

fn map_cursor_kind(kind: cursor::CursorSymbolKind) -> SymbolKind {
    match kind {
        cursor::CursorSymbolKind::Scalar => SymbolKind::scalar(),
        cursor::CursorSymbolKind::Array => SymbolKind::array(),
        cursor::CursorSymbolKind::Hash => SymbolKind::hash(),
        cursor::CursorSymbolKind::Subroutine => SymbolKind::Subroutine,
    }
}
