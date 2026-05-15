//! Single-responsibility helpers for [`super::WorkspaceRefactor::rename_symbol`].
//!
//! The public rename entry point delegates to focused functions in this module so
//! each phase of the operation — input validation, symbol identification,
//! location lookup, and edit construction — can evolve independently and be
//! reasoned about on its own.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use super::{FileEdit, RefactorError, TextEdit};
use crate::document_store::DocumentStore;
use crate::workspace_index::{
    Location, SymKind, SymbolKey, WorkspaceIndex, normalize_var, uri_to_fs_path,
};

/// Soft caps for the naive fallback search to keep workspace-wide scans bounded.
const FALLBACK_MAX_TOTAL_MATCHES: usize = 1000;
const FALLBACK_PER_DOC_EARLY_EXIT: usize = 500;

/// Description of the symbol being renamed.
pub(super) struct RenameTarget {
    pub(super) sigil: Option<char>,
    pub(super) kind: SymKind,
    pub(super) key: SymbolKey,
}

/// Validate user-supplied rename inputs before doing any lookup work.
pub(super) fn validate_inputs(old_name: &str, new_name: &str) -> Result<(), RefactorError> {
    if old_name.is_empty() {
        return Err(RefactorError::InvalidInput("Symbol name cannot be empty".to_string()));
    }
    if new_name.is_empty() {
        return Err(RefactorError::InvalidInput("New name cannot be empty".to_string()));
    }
    if old_name == new_name {
        return Err(RefactorError::InvalidInput("Old and new names are identical".to_string()));
    }
    Ok(())
}

/// Derive the symbol kind and lookup key from the user-supplied name.
pub(super) fn build_target(old_name: &str) -> RenameTarget {
    let (sigil, bare) = normalize_var(old_name);
    let kind = if sigil.is_some() { SymKind::Var } else { SymKind::Sub };
    let key = SymbolKey {
        pkg: Arc::from("main".to_string()),
        name: Arc::from(bare.to_string()),
        sigil,
        kind,
    };
    RenameTarget { sigil, kind, key }
}

/// Look up known references and definition for a symbol via the workspace index.
///
/// The definition is always appended when present so callers do not need to
/// special-case it. Returns an empty vector when the index has no matches.
pub(super) fn collect_indexed_locations(
    index: &WorkspaceIndex,
    target: &RenameTarget,
) -> Vec<Location> {
    let mut locations = index.find_refs(&target.key);
    if let Some(def) = index.find_def(&target.key) {
        if !locations.iter().any(|loc| loc.uri == def.uri && loc.range == def.range) {
            locations.push(def);
        }
    }
    locations
}

/// Heuristic byte-based text search used when the index returned no matches.
///
/// This is deliberately naive — it scans every open document for raw occurrences
/// of `old_name` and is bounded by [`FALLBACK_MAX_TOTAL_MATCHES`] and
/// [`FALLBACK_PER_DOC_EARLY_EXIT`] to keep workspace-wide scans tractable.
pub(super) fn fallback_text_search(store: &DocumentStore, old_name: &str) -> Vec<Location> {
    let mut locations = Vec::new();
    for doc in store.all_documents() {
        if !doc.text.contains(old_name) {
            continue;
        }

        let idx = doc.line_index.clone();
        let mut pos = 0;

        while let Some(found) = doc.text[pos..].find(old_name) {
            let start = pos + found;
            let end = start + old_name.len();

            if start >= doc.text.len() || end > doc.text.len() {
                break;
            }

            let (start_line, start_col) = idx.offset_to_position(start);
            let (end_line, end_col) = idx.offset_to_position(end);
            let start_byte = idx.position_to_offset(start_line, start_col).unwrap_or(0);
            let end_byte = idx.position_to_offset(end_line, end_col).unwrap_or(0);
            locations.push(Location {
                uri: doc.uri.clone(),
                range: crate::position::Range {
                    start: crate::position::Position {
                        byte: start_byte,
                        line: start_line,
                        column: start_col,
                    },
                    end: crate::position::Position {
                        byte: end_byte,
                        line: end_line,
                        column: end_col,
                    },
                },
            });
            pos = end;

            if locations.len() >= FALLBACK_MAX_TOTAL_MATCHES {
                break;
            }
        }

        if locations.len() >= FALLBACK_PER_DOC_EARLY_EXIT {
            break;
        }
    }
    locations
}

/// Build the per-file text edits that perform the rename.
pub(super) fn build_file_edits(
    store: &DocumentStore,
    locations: Vec<Location>,
    target: &RenameTarget,
    new_name: &str,
) -> Result<Vec<FileEdit>, RefactorError> {
    let mut edits: BTreeMap<PathBuf, Vec<TextEdit>> = BTreeMap::new();

    for loc in locations {
        let path = uri_to_fs_path(&loc.uri).ok_or_else(|| {
            RefactorError::UriConversion(format!("Failed to convert URI to path: {}", loc.uri))
        })?;
        let Some(doc) = store.get(&loc.uri) else {
            continue;
        };
        let start_off =
            doc.line_index.position_to_offset(loc.range.start.line, loc.range.start.column);
        let end_off = doc.line_index.position_to_offset(loc.range.end.line, loc.range.end.column);
        if let (Some(start_off), Some(end_off)) = (start_off, end_off) {
            edits.entry(path).or_default().push(TextEdit {
                start: start_off,
                end: end_off,
                new_text: replacement_text(target, new_name),
            });
        }
    }

    Ok(edits.into_iter().map(|(file_path, edits)| FileEdit { file_path, edits }).collect())
}

/// Compute the replacement text for a single occurrence, preserving sigils for variables.
fn replacement_text(target: &RenameTarget, new_name: &str) -> String {
    match target.kind {
        SymKind::Var => {
            let sig = target.sigil.unwrap_or('$');
            format!("{}{}", sig, new_name.trim_start_matches(['$', '@', '%']))
        }
        _ => new_name.to_string(),
    }
}
