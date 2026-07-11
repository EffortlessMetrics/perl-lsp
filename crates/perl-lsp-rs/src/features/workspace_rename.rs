//! Workspace Rename Provider for LSP
//!
//! Provides cross-file renaming functionality using the workspace index.

use perl_parser::workspace_index::{SymKind, SymbolKey, WorkspaceIndex};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt;

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

/// Reasons why workspace rename is refused with a hard error.
///
/// Graceful-degradation cases (`main`-package symbols, unsupported kinds,
/// missing definitions) return `Ok(vec![])` so the LSP handler falls through
/// to same-file rename.  Only `AmbiguousIdentity` produces a hard refusal
/// because it indicates an unsafe rename the user must resolve manually.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameRefusal {
    /// Workspace index produced references that cannot be safely attributed
    /// to a single declaration (e.g. unqualified cross-package call site).
    AmbiguousIdentity(String),
}

impl fmt::Display for RenameRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AmbiguousIdentity(reason) => {
                write!(f, "Workspace rename refused: ambiguous symbol identity ({reason})")
            }
        }
    }
}

impl std::error::Error for RenameRefusal {}

/// Build a rename edit across the workspace.
///
/// Returns `Ok(edits)` on success (edits may be empty — caller falls through to
/// same-file rename when empty).  Returns `Err(RenameRefusal::AmbiguousIdentity)`
/// when the workspace index finds an unqualified reference to this symbol from a
/// different package, making it unsafe to rename automatically.
///
/// Graceful-degradation paths (symbol in `main`, kind not Sub, definition not in
/// index) return `Ok(vec![])` rather than errors so the handler can fall back to
/// same-file rename without surfacing a JSON-RPC error to the editor.
pub fn build_rename_edit(
    idx: &WorkspaceIndex,
    key: &SymbolKey,
    new_name_bare: &str,
) -> Result<Vec<RenameEdit>, RenameRefusal> {
    // Only Sub symbols have stable enough workspace identity for cross-file rename.
    // Var/Pack fall through to same-file rename.
    if key.kind != SymKind::Sub {
        return Ok(vec![]);
    }

    // `main`-package symbols may be called unqualified from any file; workspace
    // rename cannot safely attribute them.  Fall through to same-file rename.
    if key.pkg.as_ref() == "main" {
        return Ok(vec![]);
    }

    // If the workspace index has no definition for this key, we can't do a
    // cross-file rename.  Fall through to same-file rename.
    let Some(def) = idx.find_def(key) else {
        return Ok(vec![]);
    };

    // 1) Get all references across the workspace
    let mut locs = idx.find_refs(key);

    // 2) Also include the definition itself
    locs.push(def);

    // 3) Group edits by URI and compute replacement text
    let mut grouped: BTreeMap<String, Vec<TextEdit>> = BTreeMap::new();

    for loc in locs {
        let start_line = loc.range.start.line;
        let start_char = loc.range.start.column;
        let end_line = loc.range.end.line;
        let end_char = loc.range.end.column;

        if is_non_target_package_declaration(
            idx, key, &loc.uri, start_line, start_char, end_line, end_char,
        ) {
            continue;
        }

        // Guard: unqualified bare call to this sub from a different package is
        // ambiguous — the workspace can't tell if it's intentionally calling our
        // sub or a same-package sub with the same name.
        if is_ambiguous_sub_reference(
            idx, key, &loc.uri, start_line, start_char, end_line, end_char,
        ) {
            return Err(RenameRefusal::AmbiguousIdentity(format!(
                "unqualified `{}` reference outside package `{}`",
                key.name, key.pkg
            )));
        }

        let mut edit_start_line = start_line;
        let mut edit_start_char = start_char;
        let mut edit_end_line = end_line;
        let mut edit_end_char = end_char;
        let mut narrowed_sub_name = false;

        if key.kind == SymKind::Sub
            && let Some(((name_start_line, name_start_char), (name_end_line, name_end_char))) =
                sub_name_range_in_source_span(
                    idx,
                    &loc.uri,
                    start_line,
                    start_char,
                    end_line,
                    end_char,
                    key.name.as_ref(),
                )
        {
            edit_start_line = name_start_line;
            edit_start_char = name_start_char;
            edit_end_line = name_end_line;
            edit_end_char = name_end_char;
            narrowed_sub_name = true;
        }

        // Compute replacement text based on symbol kind
        let replacement = match key.kind {
            SymKind::Var => {
                let sigil = key.sigil.unwrap_or('$');
                format!("{}{}", sigil, new_name_bare)
            }
            SymKind::Sub => {
                // For subroutines, preserve any existing package qualifier
                let mut replacement = new_name_bare.to_string();

                if !narrowed_sub_name && let Some(doc) = idx.document_store().get(&loc.uri) {
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
            SymKind::Pack => new_name_bare.to_string(),
        };

        grouped.entry(loc.uri.clone()).or_default().push(TextEdit {
            start: (edit_start_line, edit_start_char),
            end: (edit_end_line, edit_end_char),
            new_text: replacement,
        });
    }

    Ok(grouped.into_iter().map(|(uri, edits)| RenameEdit { uri, edits }).collect())
}

/// Returns true when `loc` is an unqualified bare call to `key.name` from a
/// package other than `key.pkg`.  Such references are ambiguous: a Perl
/// program importing a same-named sub from a different package would be
/// incorrectly renamed.
///
/// Arrow method calls (`$self->method`, `$obj->method`) are NOT ambiguous: the
/// receiver's class is determined at dispatch time via `@ISA`/Perl OO, and the
/// workspace index already validated that this reference belongs to `key.pkg`'s
/// inheritance chain.  The workspace index returns the full expression span
/// (`$self->shared`), so detecting `->` inside the span itself is sufficient to
/// identify an arrow method call.
fn is_ambiguous_sub_reference(
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

    let Some(start_off) = doc.line_index.position_to_offset(start_line, start_char) else {
        return false;
    };
    let Some(end_off) = doc.line_index.position_to_offset(end_line, end_char) else {
        return false;
    };
    let Some(original) = doc.text.get(start_off..end_off) else {
        return false;
    };

    // Only explicitly qualified calls (`Pkg::name` or `&Pkg::name`) are unambiguous.
    // A bare `&name` call (without `::`) is still subject to package resolution and
    // is just as ambiguous as `name()` when called from a different package.
    if original.contains("::") {
        return false;
    }

    let package_at_line = package_name_for_line(&doc.text, start_line);

    // Arrow method calls (`$self->name`, `$obj->name`) are not bare function calls,
    // but they are only safe for workspace rename when the caller package is the
    // defining package or explicitly inherits from it.  Dynamic receiver chains such
    // as `$self->app->dispatcher->method` must fail closed.
    if original.contains("->") {
        return package_at_line != key.pkg.as_ref()
            && !package_explicitly_inherits(&doc.text, package_at_line, key.pkg.as_ref());
    }

    package_at_line != key.pkg.as_ref()
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

fn package_explicitly_inherits(text: &str, package: &str, parent: &str) -> bool {
    let mut in_target_package = false;

    for raw_line in text.lines() {
        let trimmed = raw_line.trim_start();
        if let Some(declared_package) = package_declared_on_line(trimmed) {
            in_target_package = declared_package == package;
            continue;
        }

        if !in_target_package || trimmed.starts_with('#') {
            continue;
        }

        if line_declares_parent(trimmed, parent) {
            return true;
        }
    }

    false
}

fn package_declared_on_line(trimmed: &str) -> Option<&str> {
    if !trimmed.starts_with("package ") {
        return None;
    }

    let package_decl = trimmed.trim_start_matches("package ").trim_start();
    let package_name =
        package_decl.split(|ch: char| ch.is_whitespace() || ch == ';').next().unwrap_or_default();
    (!package_name.is_empty()).then_some(package_name)
}

fn line_declares_parent(trimmed: &str, parent: &str) -> bool {
    (trimmed.starts_with("use parent ")
        || trimmed.starts_with("use base ")
        || trimmed.starts_with("extends ")
        || trimmed.contains("@ISA"))
        && contains_module_name(trimmed, parent)
}

fn contains_module_name(text: &str, module: &str) -> bool {
    let mut search_from = 0;
    while let Some(relative_start) = text[search_from..].find(module) {
        let start = search_from + relative_start;
        let end = start + module.len();
        let before_ok = text[..start].chars().next_back().is_none_or(|ch| !is_module_name_char(ch));
        let after_ok = text[end..].chars().next().is_none_or(|ch| !is_module_name_char(ch));
        if before_ok && after_ok {
            return true;
        }
        search_from = end;
    }

    false
}

fn is_module_name_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == ':'
}

fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn find_sub_name_in_text(text: &str, name: &str) -> Option<(usize, usize)> {
    let mut search_from = 0;
    while let Some(relative_start) = text[search_from..].find(name) {
        let start = search_from + relative_start;
        let end = start + name.len();
        let before_ok = text[..start].chars().next_back().is_none_or(|ch| !is_ident_char(ch));
        let after_ok = text[end..].chars().next().is_none_or(|ch| !is_ident_char(ch));
        if before_ok && after_ok {
            return Some((start, end));
        }
        search_from = end;
    }

    None
}

fn sub_name_range_in_source_span(
    idx: &WorkspaceIndex,
    uri: &str,
    start_line: u32,
    start_char: u32,
    end_line: u32,
    end_char: u32,
    name: &str,
) -> Option<((u32, u32), (u32, u32))> {
    let doc = idx.document_store().get(uri)?;
    let start_off = doc.line_index.position_to_offset(start_line, start_char)?;
    let end_off = doc.line_index.position_to_offset(end_line, end_char)?;
    let original = doc.text.get(start_off..end_off)?;
    let (name_start, name_end) = find_sub_name_in_text(original, name)?;

    Some((
        doc.line_index.offset_to_position(start_off + name_start),
        doc.line_index.offset_to_position(start_off + name_end),
    ))
}

fn sub_declaration_name_span(line_text: &str, name: &str) -> Option<(usize, usize)> {
    let leading_ws = line_text.len() - line_text.trim_start().len();
    let trimmed = &line_text[leading_ws..];
    let after_sub = trimmed.strip_prefix("sub")?;
    if !after_sub.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }

    let whitespace_bytes: usize =
        after_sub.chars().take_while(|ch| ch.is_whitespace()).map(char::len_utf8).sum();
    let name_start = leading_ws + "sub".len() + whitespace_bytes;
    let tail = &line_text[name_start..];
    let declared_name = tail
        .split(|ch: char| ch.is_whitespace() || ch == '(' || ch == '{' || ch == ';')
        .next()
        .unwrap_or_default();

    if declared_name.is_empty() {
        return None;
    }

    if declared_name == name
        || declared_name.rsplit_once("::").is_some_and(|(_, bare)| bare == name)
    {
        Some((name_start, name_start + declared_name.len()))
    } else {
        None
    }
}

fn is_sub_declaration_line(line_text: &str, name: &str) -> bool {
    sub_declaration_name_span(line_text, name).is_some()
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
    let maybe_original =
        doc.line_index.position_to_offset(start_line, start_char).and_then(|start_off| {
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
    // Tests are permitted to use `.expect()` on Result/Option per the repo's
    // coding standards (unlike production code, where it is banned).
    #![allow(clippy::expect_used)]

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

        let edits = build_rename_edit(&idx, &key, "new_name")?;
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
        replacements.sort_by_key(|r| std::cmp::Reverse(r.0));
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

        let edits = build_rename_edit(&idx, &key, "renamed_target")?;

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

        let edits = build_rename_edit(&idx, &key, "process_records")?;

        // Bar.pm must not appear in the edit list
        let bar_edit = edits.iter().find(|e| e.uri.contains("Bar.pm"));
        assert!(
            bar_edit.is_none(),
            "renaming Foo::process_data must not touch Bar.pm; got edits: {:?}",
            edits.iter().map(|e| (&e.uri, &e.edits)).collect::<Vec<_>>()
        );

        Ok(())
    }

    /// Renaming a sub with an unqualified cross-package call site must return
    /// `Err(AmbiguousIdentity)` so the LSP handler can report the refusal to
    /// the editor instead of silently producing incorrect edits.
    #[test]
    fn rename_refuses_ambiguous_unqualified_cross_package_call()
    -> Result<(), Box<dyn std::error::Error>> {
        let idx = WorkspaceIndex::new();

        let foo_text = "package Foo;\nsub process_data { return 1; }\n1;\n";
        // Bar calls process_data without qualification — ambiguous cross-package ref.
        let bar_text = "package Bar;\nsub run { return process_data(); }\n1;\n";

        index_text(&idx, "file:///Foo.pm", foo_text)?;
        index_text(&idx, "file:///Bar.pm", bar_text)?;

        let key = SymbolKey {
            pkg: Arc::from("Foo"),
            name: Arc::from("process_data"),
            sigil: None,
            kind: SymKind::Sub,
        };

        let refusal = build_rename_edit(&idx, &key, "process_records")
            .expect_err("workspace rename should refuse ambiguous unqualified cross-package refs");
        assert!(
            matches!(refusal, RenameRefusal::AmbiguousIdentity(_)),
            "expected AmbiguousIdentity refusal, got: {refusal:?}"
        );

        Ok(())
    }

    /// A bare `&name` call (without `::`) from a different package is still an
    /// unqualified reference and must be refused, just like a bare `name()` call.
    /// Only `&Pkg::name` (which contains `::`) is unambiguous.
    #[test]
    fn rename_refuses_ampersand_sigil_cross_package_call() -> Result<(), Box<dyn std::error::Error>>
    {
        let idx = WorkspaceIndex::new();

        let foo_text = "package Foo;\nsub process_data { return 1; }\n1;\n";
        // Bar calls &process_data — still unqualified, still ambiguous.
        let bar_text = "package Bar;\nsub run { return &process_data(); }\n1;\n";

        index_text(&idx, "file:///Foo.pm", foo_text)?;
        index_text(&idx, "file:///Bar.pm", bar_text)?;

        let key = SymbolKey {
            pkg: Arc::from("Foo"),
            name: Arc::from("process_data"),
            sigil: None,
            kind: SymKind::Sub,
        };

        let refusal = build_rename_edit(&idx, &key, "process_records")
            .expect_err("workspace rename should refuse ambiguous &name cross-package refs");
        assert!(
            matches!(refusal, RenameRefusal::AmbiguousIdentity(_)),
            "expected AmbiguousIdentity refusal for &name cross-package call, got: {refusal:?}"
        );

        Ok(())
    }

    /// `main`-package symbols fall through gracefully (empty edits) rather than
    /// returning a hard error, allowing the LSP handler to do same-file rename.
    #[test]
    fn rename_main_package_sub_returns_empty_not_error() -> Result<(), Box<dyn std::error::Error>> {
        let idx = WorkspaceIndex::new();

        // No package declaration → everything is in `main`.
        let text = "sub greet { return 'hello'; }\nmy $x = greet();\n";
        index_text(&idx, "file:///script.pl", text)?;

        let key = SymbolKey {
            pkg: Arc::from("main"),
            name: Arc::from("greet"),
            sigil: None,
            kind: SymKind::Sub,
        };

        let edits = build_rename_edit(&idx, &key, "welcome")?;
        assert!(
            edits.is_empty(),
            "main-package rename must return empty edits (fall-through to same-file), got: {edits:?}"
        );

        Ok(())
    }

    /// Arrow method calls (`$self->name`) must NOT be treated as ambiguous
    /// unqualified cross-package references.  The `->` dispatch operator makes the
    /// receiver explicit even when the method name itself has no `::` qualifier.
    ///
    /// Fixture: `Base` defines `shared`; `Child` extends `Base` and calls
    /// `$self->shared` inside `run`.  Cross-file rename of `Base::shared` must
    /// produce edits for both `Base.pm` (definition) and `Child.pm` (call site).
    #[test]
    fn rename_arrow_method_call_cross_package_is_not_ambiguous()
    -> Result<(), Box<dyn std::error::Error>> {
        let idx = WorkspaceIndex::new();

        let base_text = "package Base;\n\nsub shared {\n    return 'shared';\n}\n\n1;\n";
        let child_text = "package Child;\nuse parent 'Base';\n\nsub run {\n    my ($self) = @_;\n    return $self->shared;\n}\n\n1;\n";

        index_text(&idx, "file:///Base.pm", base_text)?;
        index_text(&idx, "file:///Child.pm", child_text)?;

        let key = SymbolKey {
            pkg: Arc::from("Base"),
            name: Arc::from("shared"),
            sigil: None,
            kind: SymKind::Sub,
        };

        // Must NOT return AmbiguousIdentity — `$self->shared` is arrow dispatch.
        let edits = build_rename_edit(&idx, &key, "shared_renamed").map_err(|e| {
            format!(
                "expected Ok but got refusal: {e}. \
                 `$self->shared` in Child.pm is arrow method dispatch — must not be treated as \
                 ambiguous unqualified cross-package call."
            )
        })?;

        // Base.pm (definition) and Child.pm (inherited call site) must both be edited.
        let base_edit = edits.iter().find(|e| e.uri.contains("Base.pm"));
        assert!(
            base_edit.is_some(),
            "rename must produce edits for Base.pm (definition). Got URIs: {:?}",
            edits.iter().map(|e| &e.uri).collect::<Vec<_>>()
        );
        let child_edit = edits.iter().find(|e| e.uri.contains("Child.pm"));
        assert!(
            child_edit.is_some(),
            "rename must produce edits for Child.pm inherited call site. Got URIs: {:?}",
            edits.iter().map(|e| &e.uri).collect::<Vec<_>>()
        );

        Ok(())
    }

    /// Dynamic arrow receiver chains (`$self->app->dispatcher->method`) must fail
    /// closed unless the caller package explicitly inherits from the target package.
    #[test]
    fn rename_dynamic_arrow_receiver_without_inheritance_is_ambiguous()
    -> Result<(), Box<dyn std::error::Error>> {
        let idx = WorkspaceIndex::new();

        let dispatcher_text = "package Catalyst::Dispatcher;\nsub get_action { return 1; }\n1;\n";
        let controller_text = concat!(
            "package Catalyst::Controller;\n",
            "use parent 'Catalyst::Component';\n",
            "sub action_for {\n",
            "    my ($self, $action) = @_;\n",
            "    return $self->_application->dispatcher->get_action($action);\n",
            "}\n",
            "1;\n",
        );

        index_text(&idx, "file:///Dispatcher.pm", dispatcher_text)?;
        index_text(&idx, "file:///Controller.pm", controller_text)?;

        let key = SymbolKey {
            pkg: Arc::from("Catalyst::Dispatcher"),
            name: Arc::from("get_action"),
            sigil: None,
            kind: SymKind::Sub,
        };

        let result = build_rename_edit(&idx, &key, "renamed_get_action");
        assert!(
            matches!(result, Err(RenameRefusal::AmbiguousIdentity(_))),
            "dynamic arrow receiver without explicit inheritance must fail closed; got: {result:?}"
        );

        Ok(())
    }

    /// Bare unqualified cross-package calls (`process_data()` from a foreign package)
    /// must still be refused — the arrow-method exemption must not open that path.
    #[test]
    fn rename_bare_unqualified_cross_package_call_still_refused()
    -> Result<(), Box<dyn std::error::Error>> {
        let idx = WorkspaceIndex::new();

        let foo_text = "package Foo;\nsub process_data { return 1; }\n1;\n";
        // Bar calls process_data without `->` — truly unqualified bare function call.
        let bar_text = "package Bar;\nsub run { return process_data(); }\n1;\n";

        index_text(&idx, "file:///Foo.pm", foo_text)?;
        index_text(&idx, "file:///Bar.pm", bar_text)?;

        let key = SymbolKey {
            pkg: Arc::from("Foo"),
            name: Arc::from("process_data"),
            sigil: None,
            kind: SymKind::Sub,
        };

        let result = build_rename_edit(&idx, &key, "process_records");
        assert!(
            matches!(result, Err(RenameRefusal::AmbiguousIdentity(_))),
            "bare unqualified cross-package call must still produce AmbiguousIdentity refusal; \
             got: {result:?}"
        );

        Ok(())
    }
}
