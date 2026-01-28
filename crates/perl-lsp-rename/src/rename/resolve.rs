//! Rename symbol resolution logic
//!
//! This module provides symbol resolution for rename operations.

use perl_parser_core::SourceLocation;
use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};

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
    let chars: Vec<char> = source.chars().collect();
    if position >= chars.len() {
        return None;
    }

    // Check if we're on a sigil
    let (sigil, name_start) = if position > 0 {
        match chars.get(position - 1) {
            Some('$') => (Some(SymbolKind::scalar()), position),
            Some('@') => (Some(SymbolKind::array()), position),
            Some('%') => (Some(SymbolKind::hash()), position),
            Some('&') => (Some(SymbolKind::Subroutine), position),
            _ => (None, position),
        }
    } else {
        (None, position)
    };

    // If no sigil, check the current character
    let (sigil, name_start) = if sigil.is_none() && position < chars.len() {
        match chars[position] {
            '$' => (Some(SymbolKind::scalar()), position + 1),
            '@' => (Some(SymbolKind::array()), position + 1),
            '%' => (Some(SymbolKind::hash()), position + 1),
            '&' => (Some(SymbolKind::Subroutine), position + 1),
            _ => (sigil, name_start),
        }
    } else {
        (sigil, name_start)
    };

    // Extract the identifier
    let mut end = name_start;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }

    if end > name_start {
        let name: String = chars[name_start..end].iter().collect();
        let kind = sigil.unwrap_or(SymbolKind::Subroutine); // Default to sub if no sigil
        Some((name, kind))
    } else {
        None
    }
}

/// Get the range of a symbol at position
pub fn get_symbol_range_at_position(position: usize, source: &str) -> Option<SourceLocation> {
    let chars: Vec<char> = source.chars().collect();
    if position >= chars.len() {
        return None;
    }

    // Find start (including sigil if present)
    let mut start = position;
    if start > 0 && matches!(chars[start - 1], '$' | '@' | '%' | '&') {
        start -= 1;
    }

    // Find end
    let mut end = position;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }

    // Find start of identifier
    while start < position
        && start < chars.len()
        && (chars[start].is_alphanumeric() || chars[start] == '_')
    {
        start -= 1;
    }

    Some(SourceLocation { start, end })
}
