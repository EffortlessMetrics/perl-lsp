//! Deterministic module-import rename edit planning.
//!
//! Computes line edits for Perl module file-rename workflows.

mod detection;
mod rewriting;

use crate::import_match::line_references_module_import;
use crate::token::{module_variant_pairs, replace_module_token};
use detection::line_references_moose_moo_dsl;
pub use detection::{
    line_references_isa_assignment, line_references_package_declaration,
    line_references_qualified_call,
};
pub use rewriting::replace_module_name_prefix;

/// A full-line replacement edit for a module rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleLineEdit {
    /// Zero-based source line index.
    pub line: usize,
    /// Start column (always `0` for full-line replacement).
    pub start_character: usize,
    /// End column of the original line in bytes.
    pub end_character: usize,
    /// Replacement text for the full line.
    pub new_text: String,
}

/// Plan full-line edits needed to update module imports after file rename.
#[must_use]
pub fn plan_module_rename_edits(
    source: &str,
    old_module: &str,
    new_module: &str,
) -> Vec<ModuleLineEdit> {
    if source.is_empty()
        || old_module.is_empty()
        || new_module.is_empty()
        || old_module == new_module
    {
        return Vec::new();
    }

    let variants = module_variant_pairs(old_module, new_module);
    let mut edits = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let mut rewritten: Option<String> = None;

        for (old_variant, new_variant) in &variants {
            {
                let current_line = rewritten.as_deref().unwrap_or(line);
                if line_references_module_import(current_line, old_variant) {
                    let (candidate, changed) =
                        replace_module_token(current_line, old_variant, new_variant);
                    if changed {
                        rewritten = Some(candidate);
                    }
                }
            }

            {
                let current_line = rewritten.as_deref().unwrap_or(line);
                if line_references_moose_moo_dsl(current_line, old_variant) {
                    let (candidate, changed) =
                        replace_module_token(current_line, old_variant, new_variant);
                    if changed {
                        rewritten = Some(candidate);
                    }
                }
            }

            {
                let current_line = rewritten.as_deref().unwrap_or(line);
                if line_references_isa_assignment(current_line, old_variant) {
                    let (candidate, changed) =
                        replace_module_token(current_line, old_variant, new_variant);
                    if changed {
                        rewritten = Some(candidate);
                    }
                }
            }

            {
                let current_line = rewritten.as_deref().unwrap_or(line);
                if line_references_qualified_call(current_line, old_variant) {
                    let candidate =
                        replace_module_name_prefix(current_line, old_variant, new_variant);
                    if candidate != current_line {
                        rewritten = Some(candidate);
                    }
                }
            }

            {
                let current_line = rewritten.as_deref().unwrap_or(line);
                if line_references_package_declaration(current_line, old_variant) {
                    let (candidate, changed) =
                        replace_module_token(current_line, old_variant, new_variant);
                    if changed {
                        rewritten = Some(candidate);
                    }
                }
            }
        }

        if let Some(new_text) = rewritten {
            edits.push(ModuleLineEdit {
                line: line_idx,
                start_character: 0,
                end_character: line.len(),
                new_text,
            });
        }
    }

    edits
}

/// Apply full-line `ModuleLineEdit` replacements to source text.
#[must_use]
pub fn apply_module_rename_edits(source: &str, edits: &[ModuleLineEdit]) -> String {
    if edits.is_empty() {
        return source.to_string();
    }

    let mut lines: Vec<String> = source.split('\n').map(ToString::to_string).collect();

    let mut sorted = edits.to_vec();
    sorted.sort_by_key(|edit| edit.line);

    for edit in sorted {
        if let Some(line) = lines.get_mut(edit.line) {
            *line = edit.new_text;
        }
    }

    lines.join("\n")
}
