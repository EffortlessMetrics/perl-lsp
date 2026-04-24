//! Workspace Rename Provider for LSP
//!
//! Provides cross-file renaming functionality using the workspace index.

use perl_parser::workspace_index::{SymKind, SymbolKey, WorkspaceIndex};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Represents a text edit for a single document
#[derive(Debug, Clone)]
pub struct TextEdit {
    /// Start position as (line, character) in UTF-16 code units
    pub start: (u32, u32),
    /// End position as (line, character) in UTF-16 code units
    pub end: (u32, u32),
    /// The replacement text to insert at this range
    pub new_text: String,
}

/// Represents edits to a single document
#[derive(Debug, Clone)]
pub struct RenameEdit {
    /// The document URI to apply edits to
    pub uri: String,
    /// The list of text edits for this document
    pub edits: Vec<TextEdit>,
}

/// Reasons a workspace rename request is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameRefusal {
    /// The symbol at cursor does not have a reliable definition in the workspace index.
    WeakIdentity,
    /// Multiple plausible symbols exist and at least one unqualified call site is ambiguous.
    AmbiguousIdentity,
}

impl RenameRefusal {
    /// Human-readable refusal message for LSP errors.
    pub fn message(&self) -> &'static str {
        match self {
            Self::WeakIdentity => {
                "Workspace rename refused: symbol identity is too weak to resolve safely"
            }
            Self::AmbiguousIdentity => {
                "Workspace rename refused: symbol identity is ambiguous across packages"
            }
        }
    }
}

/// Build a rename edit across the workspace.
///
/// Finds all references to the given symbol and builds text edits to rename them.
pub fn build_rename_edit(
    idx: &WorkspaceIndex,
    key: &SymbolKey,
    new_name_bare: &str,
) -> Vec<RenameEdit> {
    build_rename_edit_checked(idx, key, new_name_bare).unwrap_or_default()
}

/// Build a rename edit across the workspace, refusing weak or ambiguous identities.
pub fn build_rename_edit_checked(
    idx: &WorkspaceIndex,
    key: &SymbolKey,
    new_name_bare: &str,
) -> Result<Vec<RenameEdit>, RenameRefusal> {
    if idx.find_def(key).is_none() {
        return Err(RenameRefusal::WeakIdentity);
    }

    if is_ambiguous_identity(idx, key) {
        return Err(RenameRefusal::AmbiguousIdentity);
    }

    // 1) Get all references across the workspace
    let mut locs = idx.find_refs(key);

    // 2) Also include the definition itself
    if let Some(def) = idx.find_def(key) {
        locs.push(def);
    }

    // 3) Group edits by URI and compute replacement text
    let mut grouped: BTreeMap<String, Vec<TextEdit>> = BTreeMap::new();

    for loc in locs {
        let start_line = loc.range.start.line;
        let start_char = loc.range.start.column;
        let end_line = loc.range.end.line;
        let end_char = loc.range.end.column;

        if key.kind == SymKind::Sub
            && key.pkg.as_ref() != "main"
            && is_non_target_package_declaration(
                idx, key, &loc.uri, start_line, start_char, end_line, end_char,
            )
        {
            continue;
        }

        // Compute replacement text based on symbol kind
        let replacement = match key.kind {
            SymKind::Var => {
                // Preserve the sigil for variables
                let sigil = key.sigil.unwrap_or('$');
                format!("{}{}", sigil, new_name_bare)
            }
            SymKind::Sub => {
                // For subroutines, preserve any existing package qualifier
                let mut replacement = new_name_bare.to_string();

                if let Some(doc) = idx.document_store().get(&loc.uri) {
                    if let (Some(start_off), Some(end_off)) = (
                        doc.line_index.position_to_offset(start_line, start_char),
                        doc.line_index.position_to_offset(end_line, end_char),
                    ) {
                        if let Some(original) = doc.text.get(start_off..end_off) {
                            if let Some((qual, _)) = original.rsplit_once("::") {
                                replacement = format!("{}::{}", qual, new_name_bare);
                            }
                        }
                    }
                }

                replacement
            }
            SymKind::Pack => {
                // Package names are replaced as-is
                new_name_bare.to_string()
            }
        };

        grouped.entry(loc.uri.clone()).or_default().push(TextEdit {
            start: (start_line, start_char),
            end: (end_line, end_char),
            new_text: replacement,
        });
    }

    // Convert to RenameEdit structs
    Ok(grouped.into_iter().map(|(uri, edits)| RenameEdit { uri, edits }).collect())
}

fn package_name_for_line(text: &str, target_line: u32) -> &str {
    let mut current_pkg = "main";

    for (line_index, raw_line) in text.lines().enumerate() {
        if (line_index as u32) > target_line {
            break;
        }

        let trimmed = raw_line.trim_start();
        if !trimmed.starts_with("package ") {
            continue;
        }

        let package_decl = trimmed.trim_start_matches("package ").trim_start();
        let package_name = package_decl
            .split(|ch: char| ch.is_whitespace() || ch == ';')
            .next()
            .unwrap_or_default();

        if !package_name.is_empty() {
            current_pkg = package_name;
        }
    }

    current_pkg
}

fn is_sub_declaration_line(line_text: &str, name: &str) -> bool {
    let trimmed = line_text.trim_start();
    if !trimmed.starts_with("sub ") {
        return false;
    }

    let tail = trimmed.trim_start_matches("sub ").trim_start();
    let declared_name = tail
        .split(|ch: char| ch.is_whitespace() || ch == '(' || ch == '{' || ch == ';')
        .next()
        .unwrap_or_default();

    declared_name == name || declared_name.rsplit_once("::").is_some_and(|(_, bare)| bare == name)
}

fn is_non_target_package_declaration(
    idx: &WorkspaceIndex,
    key: &SymbolKey,
    uri: &str,
    start_line: u32,
    start_char: u32,
    end_line: u32,
    end_char: u32,
) -> bool {
    let Some(doc) = idx.document_store().get(uri) else {
        return false;
    };

    // Try to read the original source span to detect a qualified name like "Bar::process_data".
    // When the index returns an inverted range (end < start, a known indexer edge case), we fall
    // through to the package-context check below rather than conservatively returning false.
    let maybe_original = doc
        .line_index
        .position_to_offset(start_line, start_char)
        .and_then(|start_off| {
            doc.line_index
                .position_to_offset(end_line, end_char)
                .and_then(|end_off| doc.text.get(start_off..end_off))
        });

    if let Some(original) = maybe_original {
        if let Some((qualifier, _)) = original.rsplit_once("::") {
            return qualifier != key.pkg.as_ref();
        }
        // Unqualified bare name — rely on package context below.
    }

    let package_at_line = package_name_for_line(&doc.text, start_line);
    if package_at_line == key.pkg.as_ref() {
        return false;
    }

    let line_text = doc.text.lines().nth(start_line as usize).unwrap_or_default();
    is_sub_declaration_line(line_text, key.name.as_ref())
}

fn is_ambiguous_identity(idx: &WorkspaceIndex, key: &SymbolKey) -> bool {
    if key.kind != SymKind::Sub {
        return false;
    }

    // Only ambiguous if there are multiple package-level definitions for the same bare name.
    let def_count = idx
        .search_symbols(key.name.as_ref())
        .iter()
        .filter(|symbol| {
            symbol.name == key.name.as_ref()
                && symbol.qualified_name.is_some()
                && matches!(
                    symbol.kind,
                    perl_parser::workspace_index::SymbolKind::Subroutine
                        | perl_parser::workspace_index::SymbolKind::Method
                )
        })
        .count();

    if def_count <= 1 {
        return false;
    }

    // If there are unqualified references outside the defining package, rename is ambiguous.
    idx.find_refs(key).into_iter().any(|loc| {
        let Some(doc) = idx.document_store().get(&loc.uri) else {
            return true;
        };

        let start_line = loc.range.start.line;
        let start_char = loc.range.start.column;
        let end_line = loc.range.end.line;
        let end_char = loc.range.end.column;

        let original = doc
            .line_index
            .position_to_offset(start_line, start_char)
            .and_then(|start_off| {
                doc.line_index
                    .position_to_offset(end_line, end_char)
                    .and_then(|end_off| doc.text.get(start_off..end_off))
            })
            .unwrap_or_default();

        // Qualified references are explicit and therefore not ambiguous.
        if original.contains("::") {
            return false;
        }

        let line_text = doc.text.lines().nth(start_line as usize).unwrap_or_default();
        if is_sub_declaration_line(line_text, key.name.as_ref()) {
            return false;
        }

        let package_at_line = package_name_for_line(&doc.text, start_line);
        package_at_line != key.pkg.as_ref()
    })
}

/// Convert RenameEdit to LSP WorkspaceEdit JSON.
///
/// Transforms the internal rename edit representation to the LSP protocol format.
pub fn to_workspace_edit(edits: Vec<RenameEdit>) -> Value {
    let mut changes: BTreeMap<String, Vec<Value>> = BTreeMap::new();

    for rename_edit in edits {
        let text_edits: Vec<Value> = rename_edit
            .edits
            .into_iter()
            .map(|te| {
                json!({
                    "range": {
                        "start": { "line": te.start.0, "character": te.start.1 },
                        "end": { "line": te.end.0, "character": te.end.1 }
                    },
                    "newText": te.new_text
                })
            })
            .collect();

        changes.insert(rename_edit.uri, text_edits);
    }

    json!({ "changes": changes })
}

/// Check if a rename is valid for the given symbol.
///
/// Validates that the new name is a valid Perl identifier.
pub fn validate_rename(_key: &SymbolKey, new_name: &str) -> Result<(), String> {
    // Basic validation
    if new_name.is_empty() {
        return Err("New name cannot be empty".to_string());
    }

    // Check for valid Perl identifier
    if !new_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(
            "Invalid identifier: must contain only alphanumeric characters and underscores"
                .to_string(),
        );
    }

    // Check first character is not a digit
    if new_name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return Err("Identifier cannot start with a digit".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use url::Url;

    fn index_text(
        idx: &WorkspaceIndex,
        uri: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = Url::parse(uri)?;
        idx.index_file(url, text.to_string())?;
        Ok(())
    }

    #[test]
    fn rename_sub_preserves_package_qualifier() -> Result<(), Box<dyn std::error::Error>> {
        let idx = WorkspaceIndex::new();
        let uri = "file:///test.pl";
        let text = r#"
package Package;
my $var = 0;
sub name { }
Package::name();
name();
$var;
"#;
        index_text(&idx, uri, text)?;

        let key = SymbolKey {
            pkg: Arc::from("Package"),
            name: Arc::from("name"),
            sigil: None,
            kind: SymKind::Sub,
        };

        let edits = build_rename_edit(&idx, &key, "new_name");
        assert_eq!(edits.len(), 1);

        let texts: Vec<String> = edits[0].edits.iter().map(|e| e.new_text.clone()).collect();

        // Workspace indexing now finds the declaration plus both qualified and unqualified calls
        // Enhanced dual indexing may find additional references due to improved coverage
        assert_eq!(texts.len(), 3);
        assert!(texts.contains(&"new_name".to_string()));

        // Apply edits and verify other symbols remain unchanged
        let doc = idx.document_store().get(uri).ok_or("document not found")?;
        let mut replacements: Vec<(usize, usize, &str)> = edits[0]
            .edits
            .iter()
            .filter_map(|e| {
                let start = doc.line_index.position_to_offset(e.start.0, e.start.1)?;
                let end = doc.line_index.position_to_offset(e.end.0, e.end.1)?;
                Some((start, end, e.new_text.as_str()))
            })
            .collect();
        replacements.sort_by(|a, b| b.0.cmp(&a.0));
        let mut new_text = text.to_string();
        for (start, end, rep) in replacements {
            new_text.replace_range(start..end, rep);
        }

        assert!(new_text.contains("package Package;"));
        assert!(new_text.contains("$var"));
        // Workspace indexing now works correctly - should rename function calls too
        assert!(new_text.contains("new_name")); // Declaration and calls should be renamed
        Ok(())
    }

    /// Cross-folder rename: sub defined in root_a/lib/A.pm, called from root_b/lib/B.pm.
    ///
    /// Verifies that build_rename_edit produces edits in BOTH files when the
    /// WorkspaceIndex has indexed files from two separate workspace roots.
    /// This is the alpha slice of issue #3522 cross-folder rename support.
    #[test]
    fn rename_sub_spans_two_workspace_roots() -> Result<(), Box<dyn std::error::Error>> {
        let idx = WorkspaceIndex::new();

        // root_a: defines A::target_name
        let a_uri = "file:///root_a/lib/A.pm";
        let a_text =
            "package A;\n\nsub target_name {\n    my ($self) = @_;\n    return 42;\n}\n\n1;\n";
        index_text(&idx, a_uri, a_text)?;

        // root_b: calls A::target_name
        let b_uri = "file:///root_b/lib/B.pm";
        let b_text = "package B;\n\nuse A;\n\nsub run {\n    my $obj = A->new();\n    return A::target_name($obj);\n}\n\n1;\n";
        index_text(&idx, b_uri, b_text)?;

        let key = SymbolKey {
            pkg: Arc::from("A"),
            name: Arc::from("target_name"),
            sigil: None,
            kind: SymKind::Sub,
        };

        let edits = build_rename_edit(&idx, &key, "renamed_target");

        // The rename must produce at least one edit (for the definition in A.pm)
        assert!(
            !edits.is_empty(),
            "build_rename_edit must return at least one RenameEdit for A::target_name"
        );

        // At minimum, A.pm (the definition file) must be included
        let a_edit = edits.iter().find(|e| e.uri.contains("A.pm"));
        assert!(
            a_edit.is_some(),
            "WorkspaceEdit must include edits for A.pm (definition). Got URIs: {:?}",
            edits.iter().map(|e| &e.uri).collect::<Vec<_>>()
        );

        // B.pm (the call site) must also be included — this is the core of the cross-folder test.
        // A soft `if let Some(b) = b_edit` would let the test pass even if cross-folder rename
        // is broken.  The rename indexes both files, so B.pm must always appear.
        let b_edit = edits.iter().find(|e| e.uri.contains("B.pm"));
        assert!(
            b_edit.is_some(),
            "WorkspaceEdit must include edits for B.pm (call site). Got URIs: {:?}",
            edits.iter().map(|e| &e.uri).collect::<Vec<_>>()
        );
        if let Some(b) = b_edit {
            // All edits in B.pm must use the new name
            for edit in &b.edits {
                assert!(
                    edit.new_text.contains("renamed_target"),
                    "B.pm edit must use 'renamed_target', got: {:?}",
                    edit.new_text
                );
            }
        }

        // Verify A.pm edits use the new name
        if let Some(a) = a_edit {
            for edit in &a.edits {
                assert!(
                    edit.new_text.contains("renamed_target"),
                    "A.pm edit must use 'renamed_target', got: {:?}",
                    edit.new_text
                );
            }
        }

        Ok(())
    }

    #[test]
    fn is_sub_declaration_line_handles_forward_declaration() {
        // "sub foo;" is a valid Perl forward declaration — semicolon must be treated
        // as a delimiter so the name is extracted correctly.
        assert!(
            is_sub_declaration_line("sub process_data;", "process_data"),
            "forward declaration 'sub process_data;' should match name 'process_data'"
        );
        assert!(
            is_sub_declaration_line("sub process_data ;", "process_data"),
            "forward declaration with space before semicolon should match"
        );
        // A declaration in another package must still not match a different name
        assert!(
            !is_sub_declaration_line("sub process_data;", "other_name"),
            "forward decl with wrong name must not match"
        );
    }

    #[test]
    fn rename_skips_forward_decl_in_other_package() -> Result<(), Box<dyn std::error::Error>> {
        // If package Bar has a forward declaration "sub process_data;" and we rename
        // Foo::process_data, the forward decl in Bar must not be touched.
        let idx = WorkspaceIndex::new();

        let foo_text = "package Foo;\nsub process_data { return 1; }\n1;\n";
        let bar_text = "package Bar;\nsub process_data;\nsub process_data { return 2; }\n1;\n";

        index_text(&idx, "file:///Foo.pm", foo_text)?;
        index_text(&idx, "file:///Bar.pm", bar_text)?;

        let key = SymbolKey {
            pkg: Arc::from("Foo"),
            name: Arc::from("process_data"),
            sigil: None,
            kind: SymKind::Sub,
        };

        let edits = build_rename_edit(&idx, &key, "process_records");

        // Bar.pm must not appear in the edit list
        let bar_edit = edits.iter().find(|e| e.uri.contains("Bar.pm"));
        assert!(
            bar_edit.is_none(),
            "renaming Foo::process_data must not touch Bar.pm; got edits: {:?}",
            edits.iter().map(|e| (&e.uri, &e.edits)).collect::<Vec<_>>()
        );

        Ok(())
    }
}
