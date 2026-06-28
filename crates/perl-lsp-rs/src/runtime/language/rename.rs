//! Rename handlers for symbol renaming
//!
//! Handles textDocument/prepareRename and textDocument/rename requests.
//! Supports both single-file and workspace-wide renaming.
//!
//! # Lifecycle-Aware Behavior
//!
//! Uses the routing module for state-aware dispatch:
//! - **Ready state**: Full workspace rename across all indexed files
//! - **Building/Degraded state**: Bounded readiness wait, then safe same-file rename only

use super::super::*;
use crate::protocol::{req_position, req_uri};
#[cfg(feature = "workspace")]
use crate::runtime::routing::{IndexAccessMode, route_index_access};
#[cfg(feature = "workspace")]
use perl_lsp_rs_core::providers::navigation::rename_shadow::{
    RenamePackagePilotIneligibleReason, RenamePackagePilotResult, rename_package_pilot_proof,
};
use perl_lsp_rs_core::providers::rename::{
    RenameOptions, RenameProvider, TextEdit as RenameEdit, is_in_comment, is_in_string,
};
#[cfg(feature = "workspace")]
use perl_semantic_facts::{EntityId, FileId, PlannedEdit};
#[cfg(feature = "workspace")]
use perl_workspace::semantic::queries::{QueryContext, SemanticQueries};
#[cfg(feature = "workspace")]
use std::collections::{BTreeMap, BTreeSet};

/// Returns true if `c` is a Perl variable sigil (`$`, `@`, or `%`).
fn is_perl_sigil(c: char) -> bool {
    matches!(c, '$' | '@' | '%')
}

fn strip_perl_sigil(name: &str) -> &str {
    match name.chars().next() {
        Some(c) if is_perl_sigil(c) => &name[c.len_utf8()..],
        _ => name,
    }
}

fn perl_word_split_boundary(c: char) -> bool {
    !c.is_alphanumeric() && c != '_'
}

fn lexical_declaration_keyword_before(source: &str, symbol_start: usize) -> bool {
    let line_start =
        if symbol_start == 0 { 0 } else { source[..symbol_start].rfind('\n').map_or(0, |p| p + 1) };
    let prefix = source[line_start..symbol_start].trim_end();
    let previous_word = prefix.split(perl_word_split_boundary).rfind(|word| !word.is_empty());
    matches!(previous_word, Some("my" | "state"))
}

fn sub_declaration_keyword_before(source: &str, symbol_start: usize) -> bool {
    let line_start =
        if symbol_start == 0 { 0 } else { source[..symbol_start].rfind('\n').map_or(0, |p| p + 1) };
    let prefix = source[line_start..symbol_start].trim_end();
    let previous_word = prefix.split(perl_word_split_boundary).rfind(|word| !word.is_empty());
    matches!(previous_word, Some("sub"))
}

fn range_starts_with_sub_declaration_name(
    source_range: &str,
    absolute_start: usize,
    symbol: &str,
) -> Option<(usize, usize)> {
    let leading_ws = source_range.len().saturating_sub(source_range.trim_start().len());
    let after_ws = source_range.get(leading_ws..)?;
    let after_sub = after_ws.strip_prefix("sub")?;
    if after_sub.chars().next().is_some_and(|ch| ch.is_alphanumeric() || ch == '_') {
        return None;
    }

    let name_prefix_len = after_sub.len().saturating_sub(after_sub.trim_start().len());
    let name_start = leading_ws + "sub".len() + name_prefix_len;
    let name_end = name_start + symbol.len();
    if source_range.get(name_start..name_end)? != symbol {
        return None;
    }
    if source_range
        .get(name_end..)
        .and_then(|tail| tail.chars().next())
        .is_some_and(|ch| ch.is_alphanumeric() || ch == '_')
    {
        return None;
    }

    Some((absolute_start + name_start, absolute_start + name_end))
}

fn range_contains_single_bare_function_call_name(
    source_range: &str,
    absolute_start: usize,
    symbol: &str,
) -> Option<(usize, usize)> {
    let mut match_span = None;

    for (relative_start, _) in source_range.match_indices(symbol) {
        let relative_end = relative_start + symbol.len();
        let before_ok = source_range
            .get(..relative_start)
            .and_then(|prefix| prefix.chars().next_back())
            .is_none_or(|ch| !ch.is_alphanumeric() && ch != '_');
        let after = source_range.get(relative_end..)?;
        let after_ok = after.chars().next().is_none_or(|ch| !ch.is_alphanumeric() && ch != '_');
        if !before_ok || !after_ok || !after.trim_start().starts_with('(') {
            continue;
        }
        if match_span.is_some() {
            return None;
        }
        match_span = Some((absolute_start + relative_start, absolute_start + relative_end));
    }

    match_span
}

fn same_file_rename_span(
    source: &str,
    start: usize,
    end: usize,
    current_symbol: &str,
    current_symbol_bare: &str,
) -> Option<(usize, usize)> {
    let source_range = source.get(start..end)?;
    if source_range == current_symbol
        || (current_symbol_bare != current_symbol && source_range == current_symbol_bare)
    {
        return Some((start, end));
    }
    if current_symbol_bare != current_symbol {
        return None;
    }
    range_starts_with_sub_declaration_name(source_range, start, current_symbol).or_else(|| {
        range_contains_single_bare_function_call_name(source_range, start, current_symbol)
    })
}

fn workspace_edit_uri_key(uri: &str) -> String {
    #[cfg(windows)]
    {
        let mut bytes = uri.as_bytes().to_vec();
        if bytes.len() > "file:///c:".len()
            && uri.starts_with("file:///")
            && bytes[8].is_ascii_lowercase()
            && bytes[9] == b':'
        {
            bytes[8] = bytes[8].to_ascii_uppercase();
            return String::from_utf8(bytes).unwrap_or_else(|_| uri.to_string());
        }
    }
    uri.to_string()
}

#[cfg(feature = "workspace")]
// This edit collector has several independent range inputs so callers can use it for live and indexed documents.
#[allow(clippy::too_many_arguments)]
fn add_qualified_document_rename_edits<F>(
    grouped: &mut BTreeMap<String, Vec<Value>>,
    seen: &mut BTreeSet<String>,
    edit_uri: &str,
    source: &str,
    qualified_name: &str,
    package_len: usize,
    symbol_len: usize,
    new_name_bare: &str,
    offset_to_pos16: F,
) where
    F: Fn(usize) -> (u32, u32),
{
    for (match_start, _) in source.match_indices(qualified_name) {
        let before_ok = source
            .get(..match_start)
            .and_then(|prefix| prefix.chars().next_back())
            .is_none_or(|ch| !ch.is_alphanumeric() && ch != '_' && ch != ':' && ch != '\'');
        let name_start = match_start + package_len + "::".len();
        let name_end = name_start + symbol_len;
        let after_ok = source
            .get(name_end..)
            .and_then(|suffix| suffix.chars().next())
            .is_none_or(|ch| !ch.is_alphanumeric() && ch != '_' && ch != ':' && ch != '\'');
        if !before_ok
            || !after_ok
            || is_in_comment(name_start, source)
            || is_in_string(name_start, source)
        {
            continue;
        }

        let (start_line, start_char) = offset_to_pos16(name_start);
        let (end_line, end_char) = offset_to_pos16(name_end);
        let edit_key = format!("{edit_uri}:{start_line}:{start_char}:{end_line}:{end_char}");
        if !seen.insert(edit_key) {
            continue;
        }

        grouped.entry(edit_uri.to_string()).or_default().push(json!({
            "range": {
                "start": { "line": start_line, "character": start_char },
                "end": { "line": end_line, "character": end_char }
            },
            "newText": new_name_bare
        }));
    }
}

impl LspServer {
    #[cfg(feature = "workspace")]
    fn package_rename_pilot_entity_id<Q: SemanticQueries>(
        queries: &Q,
        file_id: FileId,
        byte_offset: u32,
        symbol: &str,
    ) -> Option<EntityId> {
        queries
            .symbol_at(file_id, byte_offset)
            .and_then(|(_, occurrence)| occurrence.entity_id)
            .or_else(|| {
                let context = QueryContext::new(file_id, None, Some(byte_offset));
                queries.definitions(symbol, &context).first().map(|candidate| candidate.entity_id)
            })
    }

    #[cfg(feature = "workspace")]
    fn package_rename_pilot_edits_to_workspace_edit(
        workspace_index: &crate::workspace_index::WorkspaceIndex,
        edits: Vec<PlannedEdit>,
    ) -> Option<(Value, usize)> {
        if edits.is_empty() {
            return None;
        }

        let mut grouped: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        let mut edit_count = 0_usize;

        for edit in edits {
            // GA promotion (#1386): all PlannedEditCategory variants (Definition,
            // Reference, ImportList, ExportList, and any future #[non_exhaustive]
            // additions) are accepted here.  The anchor byte-range guard below
            // (start >= end || old_text mismatch) is the correctness safety valve
            // for every category — if the anchor resolves to the wrong text the
            // whole workspace edit is aborted rather than emitting a corrupt edit.
            let location = workspace_index
                .semantic_anchor_wire_location_for_file(edit.file_id, edit.anchor_id)?;
            let doc = workspace_index.document_store().get(&location.uri)?;
            let start = location.range.start.to_byte_offset(&doc.text);
            let end = location.range.end.to_byte_offset(&doc.text);
            if start >= end || doc.text.get(start..end)? != edit.old_text {
                return None;
            }

            grouped.entry(location.uri).or_default().push(json!({
                "range": {
                    "start": {
                        "line": location.range.start.line,
                        "character": location.range.start.character
                    },
                    "end": {
                        "line": location.range.end.line,
                        "character": location.range.end.character
                    }
                },
                "newText": edit.new_text
            }));
            edit_count += 1;
        }

        Some((json!({ "changes": grouped }), edit_count))
    }

    #[cfg(feature = "workspace")]
    fn package_rename_open_document_qualified_workspace_edit(
        &self,
        workspace_index: Option<&crate::workspace_index::WorkspaceIndex>,
        request_uri: &str,
        key: &crate::workspace_index::SymbolKey,
        new_name_bare: &str,
    ) -> Option<(Value, usize, &'static str)> {
        let mut grouped: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        let qualified_name = format!("{}::{}", key.pkg, key.name);
        let package_len = key.pkg.as_ref().len();
        let symbol_len = key.name.as_ref().len();
        let documents = self.documents_guard();
        let request_doc = self.get_document(&documents, request_uri)?;
        let request_ast = request_doc.ast.as_ref()?;
        let request_edit_uri = workspace_edit_uri_key(request_uri);
        let mut live_document_keys = BTreeSet::new();
        let mut indexed_document_keys = BTreeSet::new();

        let mut line_start = 0_usize;
        for line in request_doc.text.split_inclusive('\n') {
            if let Some((name_start, name_end)) =
                range_starts_with_sub_declaration_name(line, line_start, key.name.as_ref())
                && crate::declaration::current_package_at(request_ast, name_start)
                    == key.pkg.as_ref()
                && !is_in_comment(name_start, &request_doc.text)
                && !is_in_string(name_start, &request_doc.text)
            {
                let (start_line, start_char) = self.offset_to_pos16(request_doc, name_start);
                let (end_line, end_char) = self.offset_to_pos16(request_doc, name_end);
                let edit_key =
                    format!("{request_edit_uri}:{start_line}:{start_char}:{end_line}:{end_char}");
                seen.insert(edit_key);
                grouped.entry(request_edit_uri.clone()).or_default().push(json!({
                    "range": {
                        "start": { "line": start_line, "character": start_char },
                        "end": { "line": end_line, "character": end_char }
                    },
                    "newText": new_name_bare
                }));
                break;
            }
            line_start += line.len();
        }

        if grouped.is_empty() {
            return None;
        }

        for (uri, doc) in documents.iter() {
            live_document_keys.insert(self.normalize_uri_key(uri));
            let edit_uri = if self.normalize_uri_key(uri) == self.normalize_uri_key(request_uri) {
                request_edit_uri.clone()
            } else {
                workspace_edit_uri_key(uri)
            };
            add_qualified_document_rename_edits(
                &mut grouped,
                &mut seen,
                &edit_uri,
                &doc.text,
                &qualified_name,
                package_len,
                symbol_len,
                new_name_bare,
                |offset| self.offset_to_pos16(doc, offset),
            );
        }
        drop(documents);

        let mut used_indexed_document = false;
        if let Some(workspace_index) = workspace_index {
            for doc in workspace_index.document_store().all_documents() {
                let normalized_uri = self.normalize_uri_key(&doc.uri);
                indexed_document_keys.insert(normalized_uri.clone());
                if live_document_keys.contains(&normalized_uri) {
                    continue;
                }
                let edit_uri = workspace_edit_uri_key(&doc.uri);
                let edit_count_before: usize = grouped.values().map(Vec::len).sum();
                add_qualified_document_rename_edits(
                    &mut grouped,
                    &mut seen,
                    &edit_uri,
                    &doc.text,
                    &qualified_name,
                    package_len,
                    symbol_len,
                    new_name_bare,
                    |offset| doc.line_index.offset_to_position(offset),
                );
                let edit_count_after: usize = grouped.values().map(Vec::len).sum();
                if edit_count_after > edit_count_before {
                    used_indexed_document = true;
                }
            }
        }

        let mut used_disk_document = false;
        let scanned_document_count = live_document_keys.len() + indexed_document_keys.len();
        if grouped.values().map(Vec::len).sum::<usize>() <= 1 && scanned_document_count <= 8 {
            for root in self.package_rename_disk_scan_roots(request_uri) {
                let discovered_files =
                    super::super::file_discovery::discover_perl_files(&root).files;
                if discovered_files.len() > 512 {
                    continue;
                }
                for path in discovered_files {
                    let Ok(uri) = perl_uri::fs_path_to_uri(&path) else {
                        continue;
                    };
                    let normalized_uri = self.normalize_uri_key(&uri);
                    if live_document_keys.contains(&normalized_uri)
                        || indexed_document_keys.contains(&normalized_uri)
                    {
                        continue;
                    }
                    let Ok(text) = std::fs::read_to_string(&path) else {
                        continue;
                    };
                    let edit_uri = workspace_edit_uri_key(&uri);
                    let edit_count_before: usize = grouped.values().map(Vec::len).sum();
                    add_qualified_document_rename_edits(
                        &mut grouped,
                        &mut seen,
                        &edit_uri,
                        &text,
                        &qualified_name,
                        package_len,
                        symbol_len,
                        new_name_bare,
                        |offset| perl_position_tracking::offset_to_utf16_line_col(&text, offset),
                    );
                    let edit_count_after: usize = grouped.values().map(Vec::len).sum();
                    if edit_count_after > edit_count_before {
                        used_disk_document = true;
                    }
                }
            }
        }

        let edit_count: usize = grouped.values().map(Vec::len).sum();
        if edit_count <= 1 {
            return None;
        }

        let fallback_state = if used_disk_document {
            "workspace_disk"
        } else if used_indexed_document {
            "workspace_index"
        } else {
            "current_source"
        };
        Some((json!({ "changes": grouped }), edit_count, fallback_state))
    }

    #[cfg(feature = "workspace")]
    fn package_rename_disk_scan_roots(&self, request_uri: &str) -> Vec<std::path::PathBuf> {
        let mut roots = Vec::new();
        {
            let folders = self.workspace_folders.lock();
            if let Some(folder) = best_workspace_folder_for_doc(&folders, request_uri)
                && let Some(path) = workspace_folder_path(folder)
            {
                roots.push(path);
            }
        }
        if roots.is_empty()
            && let Some(path) = self.root_path.lock().clone()
        {
            roots.push(path);
        }
        roots
    }

    #[cfg(feature = "workspace")]
    fn package_rename_live_pilot_workspace_edit(
        &self,
        workspace_index: &crate::workspace_index::WorkspaceIndex,
        uri: &str,
        byte_offset: usize,
        symbol: &str,
        new_name_bare: &str,
    ) -> Option<Result<(Value, usize), RenamePackagePilotIneligibleReason>> {
        let byte_offset = u32::try_from(byte_offset).ok()?;
        workspace_index
            .with_semantic_queries_for_uri(uri, |file_id, queries| {
                let entity_id =
                    Self::package_rename_pilot_entity_id(&queries, file_id, byte_offset, symbol)?;
                let outcome = rename_package_pilot_proof(true, &queries, entity_id, new_name_bare);
                match outcome.result {
                    RenamePackagePilotResult::Eligible { edits } => {
                        Self::package_rename_pilot_edits_to_workspace_edit(workspace_index, edits)
                            .map(Ok)
                    }
                    RenamePackagePilotResult::Ineligible {
                        reason: RenamePackagePilotIneligibleReason::EmptyPlan,
                        ..
                    } => None,
                    RenamePackagePilotResult::Ineligible { reason, .. } => Some(Err(reason)),
                    _ => None,
                }
            })
            .flatten()
    }

    fn record_rename_provider_decision_trace(
        &self,
        uri: Option<&str>,
        symbol: Option<&str>,
        reason: &'static str,
        edit_count: usize,
        fallback_state: &'static str,
    ) {
        self.record_provider_decision_trace(
            "rename",
            &json!({
                "provider": "rename",
                "provider_action": "textDocument/rename",
                "decision": if edit_count > 0 { "acted" } else { "fallback" },
                "reason": reason,
                "uri": uri,
                "symbol": symbol,
                "live_provider_edit_count": edit_count,
                "fallback_state": fallback_state,
                "claim_boundary": "package-local compiler facts require exact live guardrails; broader compiler-backed refactor facts remain gated by receipt proof"
            }),
        );
    }

    fn token_byte_span_in_line(line: &str, offset: usize) -> Option<(usize, usize)> {
        let is_ident_char = |ch: char| ch.is_alphanumeric() || ch == '_';
        let clamped_offset = offset.min(line.len());

        let mut probe = None;
        let mut previous = None;
        for (idx, ch) in line.char_indices() {
            let end = idx + ch.len_utf8();
            if idx <= clamped_offset && clamped_offset < end {
                probe = Some((idx, ch));
                break;
            }
            if idx >= clamped_offset {
                break;
            }
            previous = Some((idx, ch));
        }

        if probe.is_none()
            && let Some((idx, ch)) = previous
            && idx + ch.len_utf8() == clamped_offset
        {
            probe = Some((idx, ch));
        }

        let (probe_idx, probe_char) = probe?;
        if !is_ident_char(probe_char) {
            return None;
        }

        let mut start = probe_idx;
        while start > 0 {
            let prefix = &line[..start];
            let Some((prev_idx, prev_char)) = prefix.char_indices().next_back() else {
                break;
            };
            if !is_ident_char(prev_char) {
                break;
            }
            start = prev_idx;
        }

        let mut end = probe_idx + probe_char.len_utf8();
        while end < line.len() {
            let suffix = &line[end..];
            let Some(next_char) = suffix.chars().next() else {
                break;
            };
            if !is_ident_char(next_char) {
                break;
            }
            end += next_char.len_utf8();
        }

        Some((start, end))
    }

    fn generated_accessor_prefix_matches(prefix: &str) -> bool {
        let mut rest = prefix.trim_start();

        if let Some(after_paren) = rest.strip_prefix('(') {
            rest = after_paren.trim_start();
        }

        if let Some(after_quote) = rest.strip_prefix(['\'', '"']) {
            rest = after_quote.trim_start();
        }

        if let Some(after_plus) = rest.strip_prefix('+') {
            rest = after_plus.trim_start();
        }

        if let Some(after_quote) = rest.strip_prefix(['\'', '"']) {
            rest = after_quote.trim_start();
        }

        rest.is_empty()
    }

    fn generated_accessor_arrow_follows(suffix: &str) -> bool {
        let mut rest = suffix.trim_start();
        if let Some(quote) = rest.chars().next().filter(|ch| matches!(ch, '\'' | '"')) {
            rest = rest[quote.len_utf8()..].trim_start();
        }
        if let Some(after_paren) = rest.strip_prefix(')') {
            rest = after_paren.trim_start();
        }
        rest.starts_with("=>")
    }

    fn has_generated_accessor_marker_before(line: &str, token_start: usize) -> bool {
        let mut search_from = 0;

        while let Some(relative_idx) = line[search_from..token_start].find("has") {
            let idx = search_from + relative_idx;
            let after_idx = idx + "has".len();
            let before_is_boundary = idx == 0
                || line[..idx]
                    .chars()
                    .next_back()
                    .is_none_or(|ch| !ch.is_alphanumeric() && ch != '_');
            if before_is_boundary
                && Self::generated_accessor_prefix_matches(&line[after_idx..token_start])
            {
                return true;
            }
            search_from = after_idx;
        }

        false
    }

    fn offset_is_generated_accessor_declaration(source: &str, offset: usize) -> bool {
        let clamped_offset = offset.min(source.len());
        let line_start = if clamped_offset == 0 {
            0
        } else {
            source[..clamped_offset].rfind('\n').map_or(0, |p| p + 1)
        };
        let line_end =
            source[clamped_offset..].find('\n').map_or(source.len(), |p| clamped_offset + p);
        let Some(line) = source.get(line_start..line_end) else {
            return false;
        };
        let relative_offset = clamped_offset.saturating_sub(line_start);
        let Some((token_start, token_end)) = Self::token_byte_span_in_line(line, relative_offset)
        else {
            return false;
        };

        if !Self::has_generated_accessor_marker_before(line, token_start) {
            return false;
        }

        Self::generated_accessor_arrow_follows(&line[token_end..])
    }

    fn rename_blocked_at(doc: &crate::state::DocumentState, offset: usize) -> bool {
        Self::offset_is_generated_accessor_declaration(&doc.text, offset)
            || Self::offset_is_inside_quoted_string(&doc.text, offset)
    }

    fn scoped_lexical_rename_edits(
        &self,
        doc: &crate::state::DocumentState,
        ast: &perl_parser_core::Node,
        offset: usize,
        normalized_name: &str,
    ) -> Option<Vec<Value>> {
        if normalized_name.chars().next().is_none_or(|c| !is_perl_sigil(c)) {
            return None;
        }

        let provider = RenameProvider::new(ast, doc.text.clone());
        let result = provider.scoped_rename(
            offset,
            strip_perl_sigil(normalized_name),
            &RenameOptions::default(),
        );
        if !result.is_valid || result.edits.is_empty() {
            return None;
        }
        let lexical_declaration_edit_count =
            result.edits.iter().filter(|edit| self.is_lexical_declaration_edit(doc, edit)).count();
        if lexical_declaration_edit_count != 1 {
            return None;
        }

        Some(
            result
                .edits
                .iter()
                .map(|edit| self.rename_edit_to_lsp_text_edit(doc, edit, normalized_name))
                .collect(),
        )
    }

    fn scoped_main_sub_rename_edits(
        &self,
        doc: &crate::state::DocumentState,
        ast: &perl_parser_core::Node,
        offset: usize,
        current_symbol: &str,
        normalized_name: &str,
    ) -> Option<Vec<Value>> {
        if normalized_name.chars().next().is_some_and(is_perl_sigil) {
            return None;
        }
        if crate::declaration::current_package_at(ast, offset) != "main" {
            return None;
        }

        let provider = RenameProvider::new(ast, doc.text.clone());
        let result = provider.scoped_rename(
            offset,
            strip_perl_sigil(normalized_name),
            &RenameOptions::default(),
        );
        if !result.is_valid || result.edits.is_empty() {
            return None;
        }

        let mut edits = Vec::new();
        let mut has_sub_declaration_edit = false;
        for edit in &result.edits {
            if crate::declaration::current_package_at(ast, edit.location.start) != "main" {
                return None;
            }
            let (start, end) = same_file_rename_span(
                &doc.text,
                edit.location.start,
                edit.location.end,
                current_symbol,
                current_symbol,
            )?;
            let narrowed = RenameEdit {
                location: perl_parser_core::SourceLocation { start, end },
                new_text: edit.new_text.clone(),
            };
            if sub_declaration_keyword_before(&doc.text, start) {
                has_sub_declaration_edit = true;
            }
            edits.push(self.rename_edit_to_lsp_text_edit(doc, &narrowed, normalized_name));
        }

        has_sub_declaration_edit.then_some(edits)
    }

    fn is_lexical_declaration_edit(
        &self,
        doc: &crate::state::DocumentState,
        edit: &RenameEdit,
    ) -> bool {
        if edit.location.start == 0 || edit.location.start > doc.text.len() {
            return false;
        }
        let Some(prefix) = doc.text.get(..edit.location.start) else {
            return false;
        };
        let Some(previous) = prefix.chars().next_back() else {
            return false;
        };
        if !is_perl_sigil(previous) {
            return false;
        }
        lexical_declaration_keyword_before(&doc.text, edit.location.start - previous.len_utf8())
    }

    fn rename_edit_to_lsp_text_edit(
        &self,
        doc: &crate::state::DocumentState,
        edit: &RenameEdit,
        normalized_name: &str,
    ) -> Value {
        let mut start = edit.location.start;
        let mut new_text = edit.new_text.clone();

        if start > 0
            && let Some(prefix) = doc.text.get(..start)
            && let Some(previous) = prefix.chars().next_back()
            && is_perl_sigil(previous)
        {
            start = start.saturating_sub(previous.len_utf8());
            new_text = normalized_name.to_string();
        }

        let (start_line, start_char) = self.offset_to_pos16(doc, start);
        let (end_line, end_char) = self.offset_to_pos16(doc, edit.location.end);

        json!({
            "range": {
                "start": { "line": start_line, "character": start_char },
                "end": { "line": end_line, "character": end_char }
            },
            "newText": new_text
        })
    }

    fn token_span_at(content: &str, offset: usize) -> Option<(usize, usize)> {
        // Build (byte_offset, char) pairs so all index arithmetic stays in byte space.
        // Callers receive a byte offset from pos16_to_offset and the returned
        // (start, end) are passed to offset_to_pos16, which also expects byte offsets.
        let pairs: Vec<(usize, char)> = content.char_indices().collect();
        if pairs.is_empty() {
            return None;
        }

        let is_ident_char = |ch: char| ch.is_alphanumeric() || ch == '_';
        let is_sigil = |ch: char| ch == '$' || ch == '@' || ch == '%';

        // Map the byte offset to a char-index in pairs via binary search.
        // partition_point returns the first index where byte_offset >= offset.
        let ci = pairs.partition_point(|(b, _)| *b < offset);
        // Clamp to last valid index.
        let ci = ci.min(pairs.len().saturating_sub(1));

        // Allow cursor-at-end and cursor-next-to-token positions by probing the
        // previous character when needed.
        let mut probe = ci;
        let at_logical_end = offset >= content.len()
            || (ci == pairs.len().saturating_sub(1) && offset > pairs[ci].0);
        if at_logical_end
            || (!is_ident_char(pairs[probe].1)
                && !is_sigil(pairs[probe].1)
                && probe > 0
                && (is_ident_char(pairs[probe - 1].1) || is_sigil(pairs[probe - 1].1)))
        {
            probe = probe.saturating_sub(1);
        }

        if !is_ident_char(pairs[probe].1) && !is_sigil(pairs[probe].1) {
            return None;
        }

        // Skip from sigil to identifier body when the cursor is on sigil.
        let mut start = probe;
        if is_sigil(pairs[start].1) && start + 1 < pairs.len() && is_ident_char(pairs[start + 1].1)
        {
            start += 1;
        }

        while start > 0 && is_ident_char(pairs[start - 1].1) {
            start -= 1;
        }
        if start > 0 && is_sigil(pairs[start - 1].1) {
            start -= 1;
        }

        let mut end = start;
        if is_sigil(pairs[end].1) {
            end += 1;
        }
        while end < pairs.len() && is_ident_char(pairs[end].1) {
            end += 1;
        }

        // Require at least one identifier character so we don't rename standalone sigils.
        let body_start = if is_sigil(pairs[start].1) { start + 1 } else { start };
        if body_start >= end {
            return None;
        }

        // Convert char indices back to byte offsets for the return value.
        let start_byte = pairs[start].0;
        let end_byte = if end < pairs.len() { pairs[end].0 } else { content.len() };

        Some((start_byte, end_byte))
    }

    fn offset_is_inside_quoted_string(content: &str, offset: usize) -> bool {
        let mut in_single = false;
        let mut in_double = false;
        let mut escaped = false;
        let mut in_comment = false;

        for (byte_offset, ch) in content.char_indices() {
            if byte_offset >= offset {
                break;
            }

            if in_comment {
                if ch == '\n' {
                    in_comment = false;
                }
                continue;
            }

            if escaped {
                escaped = false;
                continue;
            }

            if in_single {
                match ch {
                    '\\' => escaped = true,
                    '\'' => in_single = false,
                    _ => {}
                }
                continue;
            }

            if in_double {
                match ch {
                    '\\' => escaped = true,
                    '"' => in_double = false,
                    _ => {}
                }
                continue;
            }

            match ch {
                '#' => in_comment = true,
                '\'' => in_single = true,
                '"' => in_double = true,
                _ => {}
            }
        }

        in_single || in_double
    }

    /// Normalize a rename target against the current symbol, validating the sigil and identifier.
    ///
    /// If `current_symbol` starts with a sigil, the returned name is sigil-prefixed.
    /// If `requested_name` is missing its sigil, the current symbol's sigil is applied.
    /// If `requested_name` has a mismatching sigil, this returns an error.
    fn normalize_rename_target(
        &self,
        current_symbol: Option<&str>,
        requested_name: &str,
    ) -> Result<String, JsonRpcError> {
        if requested_name.is_empty() {
            return Err(JsonRpcError {
                code: -32602,
                message: "Invalid identifier: empty rename target".to_string(),
                data: None,
            });
        }

        let current_sigil =
            current_symbol.and_then(|symbol| symbol.chars().next()).filter(|c| is_perl_sigil(*c));

        match current_sigil {
            Some(sigil) => {
                let mut requested_chars = requested_name.chars();
                let requested_first = requested_chars.next();
                let bare_name = if let Some(first) = requested_first {
                    if is_perl_sigil(first) {
                        if first != sigil {
                            return Err(JsonRpcError {
                                code: -32602,
                                message: format!(
                                    "Invalid identifier: sigil '{}' does not match '{}'",
                                    first, sigil
                                ),
                                data: None,
                            });
                        }
                        requested_chars.collect::<String>()
                    } else {
                        requested_name.to_string()
                    }
                } else {
                    String::new()
                };

                if !self.is_valid_identifier(&bare_name) {
                    return Err(JsonRpcError {
                        code: -32602,
                        message: format!("Invalid identifier: {}", requested_name),
                        data: None,
                    });
                }

                Ok(format!("{}{}", sigil, bare_name))
            }
            None => {
                if !self.is_valid_identifier(requested_name) {
                    return Err(JsonRpcError {
                        code: -32602,
                        message: format!("Invalid identifier: {}", requested_name),
                        data: None,
                    });
                }
                Ok(requested_name.to_string())
            }
        }
    }

    /// Handle textDocument/prepareRename request
    pub(crate) fn handle_prepare_rename(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(_ast) = &doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);
                    if Self::rename_blocked_at(doc, offset) {
                        return Ok(Some(json!(null)));
                    }

                    // Get the token at the current position
                    let token = self.get_token_at_position(&doc.text, offset);
                    if !token.is_empty()
                        && (token.starts_with('$')
                            || token.starts_with('@')
                            || token.starts_with('%')
                            || token.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_'))
                    {
                        // Find the token bounds
                        let (start_offset, end_offset) = self.get_token_bounds(&doc.text, offset);
                        let (start_line, start_char) = self.offset_to_pos16(doc, start_offset);
                        let (end_line, end_char) = self.offset_to_pos16(doc, end_offset);

                        // Return the range and placeholder text
                        return Ok(Some(json!({
                            "range": {
                                "start": {
                                    "line": start_line,
                                    "character": start_char
                                },
                                "end": {
                                    "line": end_line,
                                    "character": end_char
                                }
                            },
                            "placeholder": token
                        })));
                    }
                }
            }
        }

        // Return null if rename is not possible at this position
        Ok(Some(json!(null)))
    }

    /// Handle textDocument/rename request with workspace support
    ///
    /// Uses routing helper for lifecycle-aware behavior:
    /// - **Ready state**: Full workspace rename across all indexed files
    /// - **Building/Degraded state**: Same-file rename only with warning log
    pub(crate) fn handle_rename_workspace(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_rename_workspace_inner(params, true)
    }

    pub(crate) fn handle_rename_workspace_for_receipt_noise(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_rename_workspace_inner(params, false)
    }

    fn workspace_edit_change_count(workspace_edit: &Value) -> usize {
        workspace_edit
            .get("changes")
            .and_then(Value::as_object)
            .map(|changes| changes.values().filter_map(Value::as_array).map(Vec::len).sum())
            .unwrap_or(0)
    }

    #[cfg(feature = "workspace")]
    fn workspace_edit_change_keys(workspace_edit: &Value) -> BTreeSet<String> {
        let mut keys = BTreeSet::new();
        let Some(changes) = workspace_edit.get("changes").and_then(Value::as_object) else {
            return keys;
        };

        for (uri, edits) in changes {
            let Some(edits) = edits.as_array() else {
                continue;
            };
            for edit in edits {
                let Some(range) = edit.get("range") else {
                    continue;
                };
                let key = format!(
                    "{}:{}:{}:{}:{}:{}",
                    uri,
                    range
                        .get("start")
                        .and_then(|start| start.get("line"))
                        .and_then(Value::as_u64)
                        .unwrap_or_default(),
                    range
                        .get("start")
                        .and_then(|start| start.get("character"))
                        .and_then(Value::as_u64)
                        .unwrap_or_default(),
                    range
                        .get("end")
                        .and_then(|end| end.get("line"))
                        .and_then(Value::as_u64)
                        .unwrap_or_default(),
                    range
                        .get("end")
                        .and_then(|end| end.get("character"))
                        .and_then(Value::as_u64)
                        .unwrap_or_default(),
                    edit.get("newText").and_then(Value::as_str).unwrap_or_default(),
                );
                keys.insert(key);
            }
        }

        keys
    }

    #[cfg(feature = "workspace")]
    fn workspace_edit_covers_required_changes(candidate: &Value, required: &Value) -> bool {
        let required_keys = Self::workspace_edit_change_keys(required);
        if required_keys.is_empty() {
            return false;
        }
        let candidate_keys = Self::workspace_edit_change_keys(candidate);
        required_keys.is_subset(&candidate_keys)
    }

    fn package_rename_guard_accepts_workspace_edit(
        guard_workspace_edit: &Value,
        semantic_workspace_edit: &Value,
        semantic_edit_count: usize,
    ) -> bool {
        Self::workspace_edit_change_count(guard_workspace_edit) == semantic_edit_count
            && guard_workspace_edit == semantic_workspace_edit
    }

    #[cfg(feature = "workspace")]
    pub(super) fn wait_for_rename_index_ready(&self) {
        use perl_parser::workspace_index::IndexState;
        use std::sync::atomic::Ordering;
        use std::time::{Duration, Instant};

        let indexing_scan_in_progress = self.indexing_in_progress.load(Ordering::Acquire);
        let pending_index_tasks = self.pending_index_task_count.load(Ordering::Acquire);
        if !indexing_scan_in_progress && pending_index_tasks == 0 {
            return;
        }

        let Some(coordinator) = self.coordinator() else {
            return;
        };

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let pending_index_tasks = self.pending_index_task_count.load(Ordering::Acquire);
            match coordinator.state() {
                IndexState::Ready { .. } if pending_index_tasks == 0 => break,
                IndexState::Degraded { .. } => break,
                _ if Instant::now() >= deadline => break,
                _ => std::thread::sleep(Duration::from_millis(1)),
            }
        }
    }

    fn handle_rename_workspace_inner(
        &self,
        params: Option<Value>,
        package_local_live_pilot_enabled: bool,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            if let (Some(uri), Some(line), Some(ch), Some(new_name)) = (
                p.get("textDocument").and_then(|t| t.get("uri")).and_then(|s| s.as_str()),
                p.get("position").and_then(|p| p.get("line")).and_then(|n| n.as_u64()),
                p.get("position").and_then(|p| p.get("character")).and_then(|n| n.as_u64()),
                p.get("newName").and_then(|s| s.as_str()),
            ) {
                let rename_starts_in_blocked_context = {
                    let documents = self.documents_guard();
                    self.get_document(&documents, uri)
                        .map(|doc| {
                            let offset = self.pos16_to_offset(doc, line as u32, ch as u32);
                            Self::rename_blocked_at(doc, offset)
                        })
                        .unwrap_or(false)
                };
                if rename_starts_in_blocked_context {
                    self.record_rename_provider_decision_trace(
                        Some(uri),
                        None,
                        "blocked_context",
                        0,
                        "no_edit",
                    );
                    return Ok(Some(json!({"changes": {}})));
                }

                // Check index access mode using routing helper
                #[cfg(feature = "workspace")]
                {
                    self.wait_for_rename_index_ready();
                    let access_mode = route_index_access(self.coordinator());
                    let (symbol_key, rename_byte_offset, rename_is_package_scoped) = {
                        let documents = self.documents_guard();
                        self.get_document(&documents, uri).and_then(|doc| {
                            doc.ast.as_ref().and_then(|ast| {
                                let offset = self.pos16_to_offset(doc, line as u32, ch as u32);
                                let current_pkg =
                                    crate::declaration::current_package_at(ast, offset);
                                crate::declaration::symbol_at_cursor_with_source(
                                    ast,
                                    offset,
                                    current_pkg,
                                    &doc.text,
                                )
                                .map(|key| (key, offset, current_pkg != "main"))
                            })
                        })
                    }
                    .map_or((None, None, false), |(key, offset, package_scoped)| {
                        (Some(key), Some(offset), package_scoped)
                    });
                    let current_symbol = {
                        let documents = self.documents_guard();
                        self.get_document(&documents, uri).map(|doc| {
                            let offset = self.pos16_to_offset(doc, line as u32, ch as u32);
                            self.get_token_at_position(&doc.text, offset)
                        })
                    };
                    let normalized_name =
                        self.normalize_rename_target(current_symbol.as_deref(), new_name)?;
                    let normalized_bare = strip_perl_sigil(&normalized_name);
                    let workspace_symbol_key =
                        symbol_key.as_ref().map(super::to_workspace_symbol_key);

                    match access_mode {
                        IndexAccessMode::Partial(reason) => {
                            tracing::debug!(
                                reason,
                                "Rename: partial-index workspace facts cannot authorize package-local live edits, using same-file only"
                            );
                            if rename_is_package_scoped
                                && current_symbol.as_deref().is_some_and(|symbol| {
                                    !symbol.is_empty()
                                        && symbol.chars().next().is_some_and(|c| !is_perl_sigil(c))
                                })
                                && let Some(key) = workspace_symbol_key.as_ref()
                                && let Some((
                                    open_doc_ws_edit,
                                    open_doc_edit_count,
                                    open_doc_fallback_state,
                                )) = self.package_rename_open_document_qualified_workspace_edit(
                                    None,
                                    uri,
                                    key,
                                    normalized_bare,
                                )
                            {
                                self.record_rename_provider_decision_trace(
                                    Some(uri),
                                    current_symbol.as_deref(),
                                    "open_document_qualified_workspace_edit",
                                    open_doc_edit_count,
                                    open_doc_fallback_state,
                                );
                                return Ok(Some(open_doc_ws_edit));
                            }
                            self.record_rename_provider_decision_trace(
                                Some(uri),
                                current_symbol.as_deref(),
                                "partial_index_package_local_live_pilot_blocked",
                                0,
                                "same_file",
                            );
                            // Fall through to same-file rename
                        }
                        IndexAccessMode::None => {
                            tracing::debug!("Rename: no workspace feature, using same-file only");
                            if rename_is_package_scoped
                                && current_symbol.as_deref().is_some_and(|symbol| {
                                    !symbol.is_empty()
                                        && symbol.chars().next().is_some_and(|c| !is_perl_sigil(c))
                                })
                                && let Some(key) = workspace_symbol_key.as_ref()
                                && let Some((
                                    open_doc_ws_edit,
                                    open_doc_edit_count,
                                    open_doc_fallback_state,
                                )) = self.package_rename_open_document_qualified_workspace_edit(
                                    None,
                                    uri,
                                    key,
                                    normalized_bare,
                                )
                            {
                                self.record_rename_provider_decision_trace(
                                    Some(uri),
                                    current_symbol.as_deref(),
                                    "open_document_qualified_workspace_edit",
                                    open_doc_edit_count,
                                    open_doc_fallback_state,
                                );
                                return Ok(Some(open_doc_ws_edit));
                            }
                            // Fall through to same-file rename
                        }
                        IndexAccessMode::Full(coordinator) => {
                            let idx = coordinator.index();
                            let workspace_index_matches_request_doc = {
                                let documents = self.documents_guard();
                                self.get_document(&documents, uri).is_none_or(|doc| {
                                    idx.document_store()
                                        .get(uri)
                                        .is_some_and(|indexed| indexed.text == doc.text)
                                })
                            };
                            if package_local_live_pilot_enabled {
                                if let (Some(offset), Some(symbol)) =
                                    (rename_byte_offset, current_symbol.as_deref())
                                    && !symbol.is_empty()
                                    && symbol.chars().next().is_some_and(|c| !is_perl_sigil(c))
                                    && rename_is_package_scoped
                                {
                                    match self.package_rename_live_pilot_workspace_edit(
                                        idx.as_ref(),
                                        uri,
                                        offset,
                                        symbol,
                                        normalized_bare,
                                    ) {
                                        Some(Ok((semantic_ws_edit, semantic_edit_count))) => {
                                            let Some(key) = workspace_symbol_key.as_ref() else {
                                                self.record_rename_provider_decision_trace(
                                                    Some(uri),
                                                    Some(symbol),
                                                    "package_local_live_pilot_blocked",
                                                    0,
                                                    "no_edit",
                                                );
                                                return Ok(Some(json!({"changes": {}})));
                                            };

                                            let guard_edits =
                                                crate::features::workspace_rename::build_rename_edit(
                                                    idx.as_ref(),
                                                    key,
                                                    normalized_bare,
                                                )
                                                .map_err(|refusal| {
                                                    self.record_rename_provider_decision_trace(
                                                        Some(uri),
                                                        Some(symbol),
                                                        "package_local_live_pilot_ambiguous",
                                                        0,
                                                        "ambiguous_identity",
                                                    );
                                                    JsonRpcError {
                                                        code: -32602,
                                                        message: refusal.to_string(),
                                                        data: None,
                                                    }
                                                })?;

                                            if !guard_edits.is_empty() {
                                                let guard_ws_edit =
                                                    crate::features::workspace_rename::to_workspace_edit(
                                                        guard_edits,
                                                    );
                                                if let Some((
                                                    open_doc_ws_edit,
                                                    open_doc_edit_count,
                                                    open_doc_fallback_state,
                                                )) = self
                                                    .package_rename_open_document_qualified_workspace_edit(
                                                        Some(idx.as_ref()),
                                                        uri,
                                                        key,
                                                        normalized_bare,
                                                    )
                                                    && Self::workspace_edit_covers_required_changes(
                                                        &open_doc_ws_edit,
                                                        &guard_ws_edit,
                                                    )
                                                {
                                                    self.record_rename_provider_decision_trace(
                                                        Some(uri),
                                                        Some(symbol),
                                                        "open_document_qualified_workspace_edit",
                                                        open_doc_edit_count,
                                                        open_doc_fallback_state,
                                                    );
                                                    return Ok(Some(open_doc_ws_edit));
                                                }

                                                if Self::package_rename_guard_accepts_workspace_edit(
                                                    &guard_ws_edit,
                                                    &semantic_ws_edit,
                                                    semantic_edit_count,
                                                ) {
                                                    self.record_rename_provider_decision_trace(
                                                        Some(uri),
                                                        Some(symbol),
                                                        "package_local_live_pilot",
                                                        semantic_edit_count,
                                                        "none",
                                                    );
                                                    return Ok(Some(semantic_ws_edit));
                                                }

                                                let guard_edit_count =
                                                    Self::workspace_edit_change_count(
                                                        &guard_ws_edit,
                                                    );
                                                self.record_rename_provider_decision_trace(
                                                    Some(uri),
                                                    Some(symbol),
                                                    "full_index_workspace_edit",
                                                    guard_edit_count,
                                                    "workspace_index",
                                                );
                                                return Ok(Some(guard_ws_edit));
                                            }

                                            self.record_rename_provider_decision_trace(
                                                Some(uri),
                                                Some(symbol),
                                                "package_local_live_pilot_guard_mismatch",
                                                0,
                                                "no_edit",
                                            );
                                            return Ok(Some(json!({"changes": {}})));
                                        }
                                        Some(Err(
                                            RenamePackagePilotIneligibleReason::UnsupportedEditCategory,
                                        )) => {
                                            self.record_rename_provider_decision_trace(
                                                Some(uri),
                                                Some(symbol),
                                                "package_local_live_pilot_unsupported",
                                                0,
                                                "workspace_index",
                                            );
                                        }
                                        Some(Err(_)) => {
                                            self.record_rename_provider_decision_trace(
                                                Some(uri),
                                                Some(symbol),
                                                "package_local_live_pilot_blocked",
                                                0,
                                                "no_edit",
                                            );
                                            return Ok(Some(json!({"changes": {}})));
                                        }
                                        None => {}
                                    }
                                }
                            }

                            if let Some(key) = workspace_symbol_key.as_ref() {
                                if package_local_live_pilot_enabled
                                    && !workspace_index_matches_request_doc
                                {
                                    self.record_rename_provider_decision_trace(
                                        Some(uri),
                                        current_symbol.as_deref(),
                                        "workspace_index_stale",
                                        0,
                                        "same_file",
                                    );
                                } else {
                                    let edits =
                                        crate::features::workspace_rename::build_rename_edit(
                                            idx.as_ref(),
                                            key,
                                            normalized_bare,
                                        )
                                        .map_err(
                                            |refusal| JsonRpcError {
                                                code: -32602,
                                                message: refusal.to_string(),
                                                data: None,
                                            },
                                        )?;
                                    if edits.is_empty() {
                                        // Fall through to same-file rename.
                                    } else {
                                        let edit_count = edits.len();
                                        let ws_edit =
                                            crate::features::workspace_rename::to_workspace_edit(
                                                edits,
                                            );
                                        if let Some((
                                            open_doc_ws_edit,
                                            open_doc_edit_count,
                                            open_doc_fallback_state,
                                        )) = self
                                            .package_rename_open_document_qualified_workspace_edit(
                                                Some(idx.as_ref()),
                                                uri,
                                                key,
                                                normalized_bare,
                                            )
                                            && Self::workspace_edit_covers_required_changes(
                                                &open_doc_ws_edit,
                                                &ws_edit,
                                            )
                                        {
                                            self.record_rename_provider_decision_trace(
                                                Some(uri),
                                                current_symbol.as_deref(),
                                                "open_document_qualified_workspace_edit",
                                                open_doc_edit_count,
                                                open_doc_fallback_state,
                                            );
                                            return Ok(Some(open_doc_ws_edit));
                                        }
                                        self.record_rename_provider_decision_trace(
                                            Some(uri),
                                            current_symbol.as_deref(),
                                            "full_index_workspace_edit",
                                            edit_count,
                                            "workspace_index",
                                        );
                                        return Ok(Some(ws_edit));
                                    }
                                }
                            }
                        }
                    }
                }

                // Same-file fallback for degraded/partial modes
                let documents = self.documents_guard();
                if let Some(doc) = self.get_document(&documents, uri) {
                    if let Some(ref ast) = doc.ast {
                        let offset = self.pos16_to_offset(doc, line as u32, ch as u32);
                        let current_symbol = self.get_token_at_position(&doc.text, offset);
                        let normalized_name =
                            self.normalize_rename_target(Some(current_symbol.as_str()), new_name)?;
                        let current_symbol_bare = strip_perl_sigil(&current_symbol);

                        if let Some(edits) =
                            self.scoped_lexical_rename_edits(doc, ast, offset, &normalized_name)
                        {
                            let edit_count = edits.len();
                            self.record_rename_provider_decision_trace(
                                Some(uri),
                                Some(current_symbol.as_str()),
                                "same_file_lexical",
                                edit_count,
                                "none",
                            );
                            return Ok(Some(json!({
                                "changes": {
                                    uri: edits
                                }
                            })));
                        }

                        if let Some(edits) = self.scoped_main_sub_rename_edits(
                            doc,
                            ast,
                            offset,
                            &current_symbol,
                            &normalized_name,
                        ) {
                            let edit_count = edits.len();
                            self.record_rename_provider_decision_trace(
                                Some(uri),
                                Some(current_symbol.as_str()),
                                "same_file_main_sub",
                                edit_count,
                                "none",
                            );
                            return Ok(Some(json!({
                                "changes": {
                                    uri: edits
                                }
                            })));
                        }

                        // Create semantic analyzer for same-file rename
                        let analyzer = crate::semantic::SemanticAnalyzer::analyze(ast);

                        // Find all references (including definition)
                        let references = analyzer.find_all_references(offset, true);

                        if !references.is_empty() {
                            let edit_count = references.len();
                            // Create text edits for all references
                            let mut edits = Vec::new();
                            let mut has_sub_declaration_edit =
                                current_symbol_bare != current_symbol;
                            for location in references {
                                let Some((edit_start, edit_end)) = same_file_rename_span(
                                    &doc.text,
                                    location.start,
                                    location.end,
                                    &current_symbol,
                                    current_symbol_bare,
                                ) else {
                                    self.record_rename_provider_decision_trace(
                                        Some(uri),
                                        Some(current_symbol.as_str()),
                                        "same_file_semantic_range_mismatch",
                                        0,
                                        "no_edit",
                                    );
                                    return Ok(Some(json!({"changes": {}})));
                                };
                                if current_symbol_bare == current_symbol
                                    && sub_declaration_keyword_before(&doc.text, edit_start)
                                {
                                    has_sub_declaration_edit = true;
                                }

                                let narrowed = RenameEdit {
                                    location: perl_parser_core::SourceLocation {
                                        start: edit_start,
                                        end: edit_end,
                                    },
                                    new_text: normalized_name.to_string(),
                                };
                                edits.push(self.rename_edit_to_lsp_text_edit(
                                    doc,
                                    &narrowed,
                                    &normalized_name,
                                ));
                            }

                            if !has_sub_declaration_edit {
                                self.record_rename_provider_decision_trace(
                                    Some(uri),
                                    Some(current_symbol.as_str()),
                                    "same_file_semantic_range_mismatch",
                                    0,
                                    "no_edit",
                                );
                                return Ok(Some(json!({"changes": {}})));
                            }

                            // Return WorkspaceEdit with same-file changes only
                            self.record_rename_provider_decision_trace(
                                Some(uri),
                                Some(current_symbol.as_str()),
                                "same_file_semantic",
                                edit_count,
                                "none",
                            );
                            return Ok(Some(json!({
                                "changes": {
                                    uri: edits
                                }
                            })));
                        }
                    }
                }
            }
        }
        // Explicit blocker paths return empty edits above. If no safe edit path
        // resolved, return null so clients can treat this as unavailable rather
        // than as an empty successful refactor.
        Ok(None)
    }

    /// Validate if a string is a valid Perl identifier
    pub(crate) fn is_valid_identifier(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        let chars: Vec<char> = name.chars().collect();

        // First character must be letter or underscore
        let first_char = match chars.first() {
            Some(c) => c,
            None => return false, // Empty string is not a valid identifier
        };
        if !first_char.is_alphabetic() && *first_char != '_' {
            return false;
        }

        // Rest must be alphanumeric or underscore
        for ch in &chars[1..] {
            if !ch.is_alphanumeric() && *ch != '_' {
                return false;
            }
        }

        true
    }

    /// Get token at position (simple implementation)
    pub(crate) fn get_token_at_position(&self, content: &str, offset: usize) -> String {
        if content.is_empty() || offset > content.len() {
            return String::new();
        }
        match Self::token_span_at(content, offset) {
            Some((start, end)) => content[start..end].to_string(),
            None => String::new(),
        }
    }

    /// Get the bounds of the token at the given position.
    ///
    /// Returns byte offsets `(start, end)` into `content`, suitable for
    /// passing directly to `offset_to_pos16`.
    pub(crate) fn get_token_bounds(&self, content: &str, offset: usize) -> (usize, usize) {
        if content.is_empty() || offset > content.len() {
            return (offset, offset);
        }
        Self::token_span_at(content, offset).unwrap_or((offset, offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position_of(text: &str, needle: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
        for (line_idx, line) in text.lines().enumerate() {
            if let Some(byte_offset) = line.find(needle) {
                let line_number = u32::try_from(line_idx)?;
                let character = line[..byte_offset].chars().map(char::len_utf16).sum::<usize>();
                let character = u32::try_from(character)?;
                return Ok((line_number, character));
            }
        }

        Err(format!("needle `{needle}` not found").into())
    }

    fn rename_params(uri: &str, line: u32, character: u32, new_name: &str) -> Value {
        serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "newName": new_name
        })
    }

    #[test]
    fn token_helpers_support_cursor_on_sigil() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let text = "my $value = 1;";
        let offset = text.find('$').ok_or("missing sigil")?;

        let token = server.get_token_at_position(text, offset);
        let (start, end) = server.get_token_bounds(text, offset);

        assert_eq!(token, "$value");
        // bounds are byte offsets; slice with &text[start..end]
        assert_eq!(&text[start..end], "$value");
        Ok(())
    }

    #[test]
    fn token_helpers_support_cursor_after_identifier() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let text = "my $value = 1;";
        let offset = text.find("$value").ok_or("missing variable")? + "$value".len();

        let token = server.get_token_at_position(text, offset);
        let (start, end) = server.get_token_bounds(text, offset);

        assert_eq!(token, "$value");
        // bounds are byte offsets; slice with &text[start..end]
        assert_eq!(&text[start..end], "$value");
        Ok(())
    }

    #[test]
    fn perl_word_split_boundary_discriminator_c_eq_underscore() {
        assert_eq!(perl_word_split_boundary('_'), false, "input that hits the boundary: c == '_'");
    }

    #[test]
    fn perl_word_split_boundary_discriminator_c_ne_underscore() {
        assert_eq!(perl_word_split_boundary(' '), true, "input that hits the boundary: c != '_'");
        assert_eq!(
            perl_word_split_boundary('a'),
            false,
            "input that hits the boundary: c.is_alphanumeric()"
        );
    }

    #[test]
    fn sub_declaration_keyword_before_boundary_discriminator_symbol_start_zero()
    -> Result<(), Box<dyn std::error::Error>> {
        let boundary_source = "target();\nsub target { 1 }\n";
        assert_eq!(
            sub_declaration_keyword_before(boundary_source, 0),
            false,
            "symbol_start == 0 must not slice before the document start"
        );

        let sub_name_offset = boundary_source.find("target {").ok_or("missing sub target")?;
        assert_eq!(
            sub_declaration_keyword_before(boundary_source, sub_name_offset),
            true,
            "normal sub declaration names should still detect the preceding sub keyword"
        );

        Ok(())
    }

    #[test]
    fn sub_declaration_keyword_before_boundary_discriminator_c_ne_underscore()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "sub_ target { 1 }\nsub target { 2 }\n";
        let invalid_offset = source.find("target { 1 }").ok_or("missing invalid sub target")?;
        let valid_offset = source.find("target { 2 }").ok_or("missing valid sub target")?;

        assert_eq!(
            sub_declaration_keyword_before(source, invalid_offset),
            false,
            "input that hits the boundary: c != '_'"
        );
        assert_eq!(
            sub_declaration_keyword_before(source, valid_offset),
            true,
            "plain sub declarations should still detect the sub keyword"
        );

        Ok(())
    }

    #[test]
    fn sub_declaration_keyword_before_boundary_discriminator_not_c_is_alphanumeric_and_c_ne_underscore()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "sub\t target { 1 }\nmethod target { 2 }\n";
        let tab_offset = source.find("target { 1 }").ok_or("missing tab-separated sub target")?;
        let method_offset = source.find("target { 2 }").ok_or("missing method target")?;

        assert_eq!(
            sub_declaration_keyword_before(source, tab_offset),
            true,
            "input that hits the boundary: !c.is_alphanumeric() && c != '_'"
        );
        assert_eq!(
            sub_declaration_keyword_before(source, method_offset),
            false,
            "other declaration-like keywords must not be treated as sub declarations"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn wait_for_rename_index_ready_observes_ready_transition()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = std::sync::Arc::new(LspServer::default());
        let uri = "file:///workspace/lib/WaitReady.pm";
        let source = "package WaitReady;\nsub target { 1 }\n1;\n";
        server.test_index_file_in_building_state(uri, source).map_err(std::io::Error::other)?;
        server.test_simulate_indexing_start();

        let worker = std::sync::Arc::clone(&server);
        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(5));
            worker.test_simulate_indexing_complete();
        });

        server.wait_for_rename_index_ready();
        handle.join().map_err(|_| std::io::Error::other("index-ready worker panicked"))?;

        Ok(())
    }

    #[test]
    fn range_starts_with_sub_declaration_name_boundary_discriminator_call_observation()
    -> Result<(), Box<dyn std::error::Error>> {
        let absolute_start = 19;
        let source_range = "  sub target { 1 }";
        let relative_name_start = source_range.find("target").ok_or("missing target")?;
        let expected_start = absolute_start + relative_name_start;
        let expected_end = expected_start + "target".len();

        assert_eq!(
            range_starts_with_sub_declaration_name(source_range, absolute_start, "target"),
            Some((expected_start, expected_end)),
            "input that observes the range_starts_with_sub_declaration_name call"
        );
        Ok(())
    }

    #[test]
    fn range_starts_with_sub_declaration_name_boundary_discriminator_ch_is_alphanumeric_or_ch_eq_underscore()
    -> Result<(), Box<dyn std::error::Error>> {
        let absolute_start = 19;

        assert_eq!(
            range_starts_with_sub_declaration_name("subtarget { 1 }", absolute_start, "target"),
            None,
            "input that hits the boundary: ch.is_alphanumeric() || ch == '_'"
        );
        assert_eq!(
            range_starts_with_sub_declaration_name("sub_target { 1 }", absolute_start, "target"),
            None,
            "input that hits the boundary: ch == '_'"
        );

        Ok(())
    }

    #[test]
    fn range_starts_with_sub_declaration_name_boundary_discriminator_source_range_get_name_start_name_end_ne_symbol()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            range_starts_with_sub_declaration_name("sub target { 1 }", 19, "other"),
            None,
            "input that hits the boundary: source_range.get(name_start..name_end)? != symbol"
        );

        Ok(())
    }

    #[test]
    fn range_starts_with_sub_declaration_name_boundary_discriminator_tail_ch_is_alphanumeric_or_ch_eq_underscore()
    -> Result<(), Box<dyn std::error::Error>> {
        let absolute_start = 19;

        assert_eq!(
            range_starts_with_sub_declaration_name(
                "sub targetSuffix { 1 }",
                absolute_start,
                "target"
            ),
            None,
            "input that hits the boundary: source_range.get(name_end..).and_then(|tail| tail.chars().next()).is_some_and(|ch| ch.is_alphanumeric() || ch == '_')"
        );
        assert_eq!(
            range_starts_with_sub_declaration_name(
                "sub target_suffix { 1 }",
                absolute_start,
                "target"
            ),
            None,
            "input that hits the boundary: ch == '_'"
        );

        Ok(())
    }

    #[test]
    fn range_contains_single_bare_function_call_name_rejects_non_calls_and_duplicates()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            range_contains_single_bare_function_call_name("target + 1", 7, "target"),
            None,
            "bare symbol ranges that are not call sites must not become rename edits"
        );
        assert_eq!(
            range_contains_single_bare_function_call_name("target() + target()", 7, "target"),
            None,
            "ambiguous duplicate call names inside one provider range must be rejected"
        );
        assert_eq!(
            range_contains_single_bare_function_call_name("target()", 7, "target"),
            Some((7, 13)),
            "single bare call names should still narrow to the token span"
        );

        Ok(())
    }

    #[test]
    fn same_file_rename_span_rejects_sigiled_symbol_range_mismatch()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "my $value_suffix = $value;";
        let bare_start = source.find("value;").ok_or("missing bare value")?;
        let bare_end = bare_start + "value".len();
        let mismatch_start = source.find("value_suffix").ok_or("missing value_suffix")?;
        let mismatch_end = mismatch_start + "value_suffix".len();

        assert_eq!(
            same_file_rename_span(source, bare_start, bare_end, "$value", "value"),
            Some((bare_start, bare_end)),
            "sigiled rename plans may target the bare identifier span"
        );
        assert_eq!(
            same_file_rename_span(source, mismatch_start, mismatch_end, "$value", "value"),
            None,
            "sigiled rename plans must reject coarse ranges that do not match the current token"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn handle_rename_partial_index_uses_open_document_qualified_workspace_edit()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let request_uri = "file:///workspace/lib/Partial/Pkg.pm";
        let caller_uri = "file:///workspace/lib/Partial/Caller.pm";
        let request_source = "package Partial::Pkg;\nsub target { 1 }\n1;\n";
        let caller_source = "package Partial::Caller;\nsub run { Partial::Pkg::target(); }\n1;\n";
        server.test_apply_did_open(request_uri, request_source, 1)?;
        server.test_apply_did_open(caller_uri, caller_source, 1)?;
        server
            .test_index_file_in_building_state(request_uri, request_source)
            .map_err(std::io::Error::other)?;

        let (line, character) = position_of(request_source, "target {")?;
        let rename_result = server
            .handle_rename_workspace(Some(rename_params(request_uri, line, character, "renamed")))?
            .ok_or("missing partial-index package rename result")?;

        let changes = rename_result
            .get("changes")
            .and_then(Value::as_object)
            .ok_or("missing partial-index workspace edit changes")?;
        let edit_count: usize = changes.values().filter_map(Value::as_array).map(Vec::len).sum();
        assert_eq!(edit_count, 2);
        assert!(
            changes.contains_key(request_uri),
            "request declaration edit should be present: {rename_result}"
        );
        assert!(
            changes.contains_key(caller_uri),
            "open caller qualified call edit should be present: {rename_result}"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn handle_rename_partial_index_falls_back_when_open_document_guard_has_no_callers()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let request_uri = "file:///workspace/lib/PartialSolo/Pkg.pm";
        let request_source = "package PartialSolo::Pkg;\nsub target { 1 }\n1;\n";
        server.test_apply_did_open(request_uri, request_source, 1)?;
        server
            .test_index_file_in_building_state(request_uri, request_source)
            .map_err(std::io::Error::other)?;

        let (line, character) = position_of(request_source, "target {")?;
        let rename_result = server.handle_rename_workspace(Some(rename_params(
            request_uri,
            line,
            character,
            "renamed",
        )))?;

        assert!(
            rename_result.is_none()
                || rename_result
                    .as_ref()
                    .and_then(|value| value.get("changes"))
                    .and_then(Value::as_object)
                    .is_some_and(|changes| !changes.is_empty()),
            "partial-index fallthrough should either produce a safe same-file edit or refuse: {rename_result:?}"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn handle_rename_without_coordinator_uses_open_document_qualified_workspace_edit()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut server = LspServer::default();
        server.index_coordinator = None;
        server.test_simulate_indexing_start();

        let request_uri = "file:///workspace/lib/NoCoord/Pkg.pm";
        let caller_uri = "file:///workspace/lib/NoCoord/Caller.pm";
        let request_source = "package NoCoord::Pkg;\nsub target { 1 }\n1;\n";
        let caller_source = "package NoCoord::Caller;\nsub run { NoCoord::Pkg::target(); }\n1;\n";
        server.test_apply_did_open(request_uri, request_source, 1)?;
        server.test_apply_did_open(caller_uri, caller_source, 1)?;

        let (line, character) = position_of(request_source, "target {")?;
        let rename_result = server
            .handle_rename_workspace(Some(rename_params(request_uri, line, character, "renamed")))?
            .ok_or("missing no-coordinator package rename result")?;

        let changes = rename_result
            .get("changes")
            .and_then(Value::as_object)
            .ok_or("missing no-coordinator workspace edit changes")?;
        let edit_count: usize = changes.values().filter_map(Value::as_array).map(Vec::len).sum();
        assert_eq!(edit_count, 2);
        assert!(
            changes.contains_key(request_uri),
            "request declaration edit should be present: {rename_result}"
        );
        assert!(
            changes.contains_key(caller_uri),
            "open caller qualified call edit should be present without a coordinator: {rename_result}"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn handle_rename_full_index_uses_indexed_open_document_guard()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let request_uri = "file:///workspace/lib/Full/Pkg.pm";
        let caller_uri = "file:///workspace/lib/Full/Caller.pm";
        let request_source = "package Full::Pkg;\nsub target { 1 }\n1;\n";
        let caller_source = "package Full::Caller;\nsub run { Full::Pkg::target(); }\n1;\n";
        server.test_apply_did_open(request_uri, request_source, 1)?;

        let coordinator = server.coordinator().ok_or("missing workspace index coordinator")?;
        coordinator
            .index()
            .index_file(url::Url::parse(request_uri)?, request_source.to_string())
            .map_err(std::io::Error::other)?;
        coordinator
            .index()
            .index_file(url::Url::parse(caller_uri)?, caller_source.to_string())
            .map_err(std::io::Error::other)?;
        coordinator.transition_to_ready(
            coordinator.index().file_count(),
            coordinator.index().symbol_count(),
        );

        let (line, character) = position_of(request_source, "target {")?;
        let rename_result = server
            .handle_rename_workspace_for_receipt_noise(Some(rename_params(
                request_uri,
                line,
                character,
                "renamed",
            )))?
            .ok_or("missing full-index package rename result")?;

        let changes = rename_result
            .get("changes")
            .and_then(Value::as_object)
            .ok_or("missing full-index workspace edit changes")?;
        let edit_count: usize = changes.values().filter_map(Value::as_array).map(Vec::len).sum();
        assert_eq!(edit_count, 2);
        assert!(
            changes.contains_key(request_uri),
            "request declaration edit should be present: {rename_result}"
        );
        assert!(
            changes.contains_key(caller_uri),
            "indexed caller qualified call edit should be present: {rename_result}"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn handle_rename_full_index_stale_request_falls_back_to_same_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let request_uri = "file:///workspace/lib/Stale/Pkg.pm";
        let stale_index_source = "package Stale::Pkg;\nsub other { 1 }\n1;\n";
        let live_source = "package Stale::Pkg;\nsub target { 1 }\ntarget();\n1;\n";
        server.test_apply_did_open(request_uri, live_source, 1)?;

        let coordinator = server.coordinator().ok_or("missing workspace index coordinator")?;
        coordinator
            .index()
            .index_file(url::Url::parse(request_uri)?, stale_index_source.to_string())
            .map_err(std::io::Error::other)?;
        coordinator.transition_to_ready(
            coordinator.index().file_count(),
            coordinator.index().symbol_count(),
        );

        let (line, character) = position_of(live_source, "target {")?;
        let rename_result = server
            .handle_rename_workspace(Some(rename_params(request_uri, line, character, "renamed")))?
            .ok_or("missing stale-index same-file fallback rename result")?;

        let changes = rename_result
            .get("changes")
            .and_then(Value::as_object)
            .ok_or("missing stale-index workspace edit changes")?;
        let edits = changes
            .get(request_uri)
            .and_then(Value::as_array)
            .ok_or("missing stale-index same-file edits")?;
        assert!(
            edits.len() >= 2,
            "stale workspace index should fall back to same-file edits: {rename_result}"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn add_qualified_document_rename_edits_filters_contexts_and_deduplicates()
    -> Result<(), Box<dyn std::error::Error>> {
        let uri = "file:///workspace/lib/Caller.pm";
        let source = concat!(
            "My::Pkg::target();\n",
            "My::Pkg::target();\n",
            "Other::My::Pkg::target();\n",
            "My::Pkg::target_suffix();\n",
            "My::Pkg::target::child();\n",
            "Other'My::Pkg::target();\n",
            "My::Pkg::target'child();\n",
            "# My::Pkg::target();\n",
            "my $s = \"My::Pkg::target()\";\n",
        );
        let mut grouped: std::collections::BTreeMap<String, Vec<serde_json::Value>> =
            std::collections::BTreeMap::new();
        let mut seen = std::collections::BTreeSet::new();

        add_qualified_document_rename_edits(
            &mut grouped,
            &mut seen,
            uri,
            source,
            "My::Pkg::target",
            "My::Pkg".len(),
            "target".len(),
            "renamed",
            |offset| perl_position_tracking::offset_to_utf16_line_col(source, offset),
        );

        let edits = grouped.get(uri).ok_or("missing qualified call edits")?;
        assert_eq!(
            edits.len(),
            2,
            "only standalone qualified calls outside comments and strings should be edited"
        );
        assert!(
            edits.iter().all(|edit| edit.get("newText").and_then(Value::as_str) == Some("renamed")),
            "all accepted qualified call edits should use the requested bare replacement"
        );

        add_qualified_document_rename_edits(
            &mut grouped,
            &mut seen,
            uri,
            source,
            "My::Pkg::target",
            "My::Pkg".len(),
            "target".len(),
            "renamed",
            |offset| perl_position_tracking::offset_to_utf16_line_col(source, offset),
        );

        let deduped_edits = grouped.get(uri).ok_or("missing deduped qualified call edits")?;
        assert_eq!(
            deduped_edits.len(),
            2,
            "re-scanning the same document must not produce duplicate edits"
        );

        Ok(())
    }

    #[test]
    fn rename_edit_to_lsp_text_edit_expands_sigil_for_bare_span()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let uri = "file:///workspace/lib/Sigil.pm";
        let source = "my $value = $value;\n";
        server.test_apply_did_open(uri, source, 1)?;
        let bare_start = source.rfind("value").ok_or("missing bare variable reference")?;
        let bare_end = bare_start + "value".len();
        let edit = RenameEdit {
            location: perl_parser_core::SourceLocation { start: bare_start, end: bare_end },
            new_text: "$renamed".to_string(),
        };
        let documents = server.documents_guard();
        let doc = server.get_document(&documents, uri).ok_or("missing opened document")?;

        let lsp_edit = server.rename_edit_to_lsp_text_edit(doc, &edit, "$renamed");
        assert_eq!(lsp_edit.get("newText").and_then(Value::as_str), Some("$renamed"));
        assert_eq!(lsp_edit["range"]["start"]["character"], serde_json::json!(12));
        assert_eq!(lsp_edit["range"]["end"]["character"], serde_json::json!(18));

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn workspace_edit_covers_required_changes_requires_superset()
    -> Result<(), Box<dyn std::error::Error>> {
        let required = serde_json::json!({
            "changes": {
                "file:///workspace/lib/Pkg.pm": [{
                    "range": {
                        "start": { "line": 1, "character": 4 },
                        "end": { "line": 1, "character": 10 }
                    },
                    "newText": "renamed"
                }]
            }
        });
        let candidate = serde_json::json!({
            "changes": {
                "file:///workspace/lib/Pkg.pm": [{
                    "range": {
                        "start": { "line": 1, "character": 4 },
                        "end": { "line": 1, "character": 10 }
                    },
                    "newText": "renamed"
                }],
                "file:///workspace/lib/Caller.pm": [{
                    "range": {
                        "start": { "line": 2, "character": 20 },
                        "end": { "line": 2, "character": 26 }
                    },
                    "newText": "renamed"
                }]
            }
        });
        let missing = serde_json::json!({
            "changes": {
                "file:///workspace/lib/Caller.pm": [{
                    "range": {
                        "start": { "line": 2, "character": 20 },
                        "end": { "line": 2, "character": 26 }
                    },
                    "newText": "renamed"
                }]
            }
        });
        let wrong_text = serde_json::json!({
            "changes": {
                "file:///workspace/lib/Pkg.pm": [{
                    "range": {
                        "start": { "line": 1, "character": 4 },
                        "end": { "line": 1, "character": 10 }
                    },
                    "newText": "other"
                }]
            }
        });

        assert!(LspServer::workspace_edit_covers_required_changes(&candidate, &required));
        assert!(!LspServer::workspace_edit_covers_required_changes(&missing, &required));
        assert!(!LspServer::workspace_edit_covers_required_changes(&wrong_text, &required));

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn package_rename_open_document_qualified_workspace_edit_uses_indexed_non_live_docs()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let request_uri = "file:///workspace/lib/Indexed/Pkg.pm";
        let caller_uri = "file:///workspace/lib/Indexed/Caller.pm";
        let request_source = "package Indexed::Pkg;\nsub target { 1 }\n1;\n";
        let caller_source = "package Indexed::Caller;\nsub run { Indexed::Pkg::target(); }\n1;\n";

        server.test_handle_did_open(Some(serde_json::json!({
            "textDocument": {
                "uri": request_uri,
                "text": request_source,
                "languageId": "perl",
                "version": 1
            }
        })))?;

        let index = crate::workspace_index::WorkspaceIndex::new();
        index.index_file_str(request_uri, request_source)?;
        index.index_file_str(caller_uri, caller_source)?;
        let key = crate::workspace_index::SymbolKey {
            pkg: "Indexed::Pkg".into(),
            name: "target".into(),
            sigil: None,
            kind: crate::workspace_index::SymKind::Sub,
        };

        let (workspace_edit, edit_count, fallback_state) = server
            .package_rename_open_document_qualified_workspace_edit(
                Some(&index),
                request_uri,
                &key,
                "renamed",
            )
            .ok_or("missing indexed qualified workspace edit")?;

        assert_eq!(edit_count, 2);
        assert_eq!(fallback_state, "workspace_index");
        let changes = workspace_edit
            .get("changes")
            .and_then(Value::as_object)
            .ok_or("missing workspace edit changes")?;
        assert!(
            changes.contains_key(request_uri),
            "request document declaration edit should be preserved: {workspace_edit}"
        );
        assert!(
            changes.contains_key(caller_uri),
            "indexed non-live caller edit should be included: {workspace_edit}"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn package_rename_open_document_qualified_workspace_edit_rejects_comment_string_anchors()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let request_uri = "file:///workspace/lib/Comment/Pkg.pm";
        let caller_uri = "file:///workspace/lib/Comment/Caller.pm";
        let request_source = concat!(
            "package Comment::Pkg;\n",
            "# sub target { 1 }\n",
            "my $s = 'sub target { 1 }';\n",
            "1;\n",
        );
        let caller_source = "package Comment::Caller;\nsub run { Comment::Pkg::target(); }\n1;\n";
        server.test_apply_did_open(request_uri, request_source, 1)?;
        server.test_apply_did_open(caller_uri, caller_source, 1)?;

        let key = crate::workspace_index::SymbolKey {
            pkg: "Comment::Pkg".into(),
            name: "target".into(),
            sigil: None,
            kind: crate::workspace_index::SymKind::Sub,
        };

        assert!(
            server
                .package_rename_open_document_qualified_workspace_edit(
                    None,
                    request_uri,
                    &key,
                    "renamed",
                )
                .is_none(),
            "comment or string text must not seed package rename declaration anchors"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn package_rename_open_document_qualified_workspace_edit_rejects_block_scoped_package_mismatch()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let request_uri = "file:///workspace/lib/Block/Pkg.pm";
        let caller_uri = "file:///workspace/lib/Block/Caller.pm";
        let request_source = concat!(
            "package Block::Outer;\n",
            "{\n",
            "    package Block::Inner;\n",
            "}\n",
            "sub target { 1 }\n",
            "1;\n",
        );
        let caller_source = "package Block::Caller;\nsub run { Block::Inner::target(); }\n1;\n";
        server.test_handle_did_open(Some(serde_json::json!({
            "textDocument": {
                "uri": request_uri,
                "text": request_source,
                "languageId": "perl",
                "version": 1
            }
        })))?;
        server.test_apply_did_open(caller_uri, caller_source, 1)?;

        let key = crate::workspace_index::SymbolKey {
            pkg: "Block::Inner".into(),
            name: "target".into(),
            sigil: None,
            kind: crate::workspace_index::SymKind::Sub,
        };

        assert!(
            server
                .package_rename_open_document_qualified_workspace_edit(
                    None,
                    request_uri,
                    &key,
                    "renamed",
                )
                .is_none(),
            "a block-scoped package declaration must not claim later outer-scope sub declarations"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn package_rename_open_document_qualified_workspace_edit_uses_disk_fallback()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let request_path = temp.path().join("lib").join("Disk").join("Pkg.pm");
        let caller_path = temp.path().join("lib").join("Disk").join("Caller.pm");
        let request_parent = request_path.parent().ok_or("missing request parent")?;
        let caller_parent = caller_path.parent().ok_or("missing caller parent")?;
        std::fs::create_dir_all(request_parent)?;
        std::fs::create_dir_all(caller_parent)?;

        let request_source = "package Disk::Pkg;\nsub target { 1 }\n1;\n";
        let caller_source = "package Disk::Caller;\nsub run { Disk::Pkg::target(); }\n1;\n";
        std::fs::write(&request_path, request_source)?;
        std::fs::write(&caller_path, caller_source)?;

        let server = LspServer::default();
        let root_uri = perl_uri::fs_path_to_uri(temp.path())?;
        let request_uri = perl_uri::fs_path_to_uri(&request_path)?;
        let caller_uri = perl_uri::fs_path_to_uri(&caller_path)?;
        server.test_set_workspace_folder_uris(&[root_uri.as_str()]);
        server.test_handle_did_open(Some(serde_json::json!({
            "textDocument": {
                "uri": request_uri,
                "text": request_source,
                "languageId": "perl",
                "version": 1
            }
        })))?;

        let key = crate::workspace_index::SymbolKey {
            pkg: "Disk::Pkg".into(),
            name: "target".into(),
            sigil: None,
            kind: crate::workspace_index::SymKind::Sub,
        };

        let (workspace_edit, edit_count, fallback_state) = server
            .package_rename_open_document_qualified_workspace_edit(
                None,
                &request_uri,
                &key,
                "renamed",
            )
            .ok_or("missing disk qualified workspace edit")?;

        assert_eq!(edit_count, 2);
        assert_eq!(fallback_state, "workspace_disk");
        let changes = workspace_edit
            .get("changes")
            .and_then(Value::as_object)
            .ok_or("missing workspace edit changes")?;
        assert!(
            changes.contains_key(&workspace_edit_uri_key(&request_uri)),
            "request document declaration edit should be preserved: {workspace_edit}"
        );
        assert!(
            changes.contains_key(&workspace_edit_uri_key(&caller_uri)),
            "disk-discovered caller edit should be included: {workspace_edit}"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn package_rename_open_document_qualified_workspace_edit_skips_truncated_disk_fallback()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let request_path = temp.path().join("lib").join("DiskCap").join("Pkg.pm");
        let caller_dir = temp.path().join("lib").join("DiskCap").join("callers");
        let request_parent = request_path.parent().ok_or("missing request parent")?;
        std::fs::create_dir_all(request_parent)?;
        std::fs::create_dir_all(&caller_dir)?;

        let request_source = "package DiskCap::Pkg;\nsub target { 1 }\n1;\n";
        std::fs::write(&request_path, request_source)?;
        for idx in 0..513 {
            let caller_path = caller_dir.join(format!("Caller{idx}.pm"));
            std::fs::write(
                caller_path,
                format!(
                    "package DiskCap::Caller{idx};\nsub run {{ DiskCap::Pkg::target(); }}\n1;\n"
                ),
            )?;
        }

        let server = LspServer::default();
        let root_uri = perl_uri::fs_path_to_uri(temp.path())?;
        let request_uri = perl_uri::fs_path_to_uri(&request_path)?;
        server.test_set_workspace_folder_uris(&[root_uri.as_str()]);
        server.test_handle_did_open(Some(serde_json::json!({
            "textDocument": {
                "uri": request_uri,
                "text": request_source,
                "languageId": "perl",
                "version": 1
            }
        })))?;

        let key = crate::workspace_index::SymbolKey {
            pkg: "DiskCap::Pkg".into(),
            name: "target".into(),
            sigil: None,
            kind: crate::workspace_index::SymKind::Sub,
        };

        assert!(
            server
                .package_rename_open_document_qualified_workspace_edit(
                    None,
                    &request_uri,
                    &key,
                    "renamed",
                )
                .is_none(),
            "disk fallback must refuse rather than emit edits from a truncated file set"
        );

        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn package_rename_disk_scan_roots_use_workspace_folder_or_root_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let workspace_temp = tempfile::tempdir()?;
        let root_temp = tempfile::tempdir()?;
        let workspace_doc = workspace_temp.path().join("lib").join("Folder").join("Doc.pm");
        let workspace_doc_parent = workspace_doc.parent().ok_or("missing workspace doc parent")?;
        std::fs::create_dir_all(workspace_doc_parent)?;

        let server = LspServer::default();
        let workspace_root_uri = perl_uri::fs_path_to_uri(workspace_temp.path())?;
        let workspace_doc_uri = perl_uri::fs_path_to_uri(&workspace_doc)?;
        server.test_set_workspace_folder_uris(&[workspace_root_uri.as_str()]);
        assert_eq!(
            server.package_rename_disk_scan_roots(&workspace_doc_uri),
            vec![workspace_temp.path().to_path_buf()],
            "workspace folder provenance should win when the request URI belongs to it"
        );

        let server = LspServer::default();
        server.test_set_root_path(root_temp.path().to_path_buf());
        assert_eq!(
            server.package_rename_disk_scan_roots("file:///outside/Doc.pm"),
            vec![root_temp.path().to_path_buf()],
            "root_path should be the disk-scan fallback when no workspace folder matches"
        );

        Ok(())
    }

    #[test]
    fn normalize_rename_target_rejects_invalid_targets() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();

        assert_eq!(server.normalize_rename_target(Some("$value"), "renamed")?, "$renamed");
        assert_eq!(server.normalize_rename_target(Some("$value"), "$renamed")?, "$renamed");
        assert_eq!(server.normalize_rename_target(Some("target"), "renamed")?, "renamed");

        assert!(
            server.normalize_rename_target(Some("$value"), "").is_err(),
            "empty requested names must be rejected"
        );
        assert!(
            server.normalize_rename_target(Some("$value"), "@renamed").is_err(),
            "sigiled renames must preserve the current symbol sigil"
        );
        assert!(
            server.normalize_rename_target(Some("$value"), "bad-name").is_err(),
            "sigiled symbols still require valid Perl identifier bodies"
        );
        assert!(
            server.normalize_rename_target(Some("target"), "bad-name").is_err(),
            "bare symbols still require valid Perl identifiers"
        );

        Ok(())
    }

    #[test]
    fn is_valid_identifier_rejects_invalid_edges() {
        let server = LspServer::default();

        assert!(!server.is_valid_identifier(""));
        assert!(!server.is_valid_identifier("1bad"));
        assert!(!server.is_valid_identifier("bad-name"));
        assert!(server.is_valid_identifier("_good1"));
    }

    #[test]
    fn token_helpers_reject_empty_standalone_sigil_and_non_identifier_offsets()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();

        assert_eq!(server.get_token_at_position("", 0), "");
        assert_eq!(server.get_token_bounds("", 4), (4, 4));
        assert_eq!(server.get_token_at_position("$", 0), "");
        assert_eq!(server.get_token_bounds("$", 0), (0, 0));
        assert_eq!(server.get_token_at_position(";", 0), "");
        assert_eq!(server.get_token_bounds(";", 0), (0, 0));
        assert_eq!(server.get_token_at_position("target;", "target".len()), "target");
        assert_eq!(server.get_token_bounds("target;", "target".len()), (0, "target".len()));

        assert_eq!(LspServer::token_byte_span_in_line("", 0), None);
        assert_eq!(LspServer::token_byte_span_in_line("   ", 1), None);
        assert_eq!(
            LspServer::token_byte_span_in_line("prefix target", "prefix target".len()),
            Some(("prefix ".len(), "prefix target".len())),
            "line-local token spans should support cursor-at-line-end token offsets"
        );

        Ok(())
    }

    #[test]
    fn rename_guard_tracks_escaped_quotes_and_comment_boundaries()
    -> Result<(), Box<dyn std::error::Error>> {
        let single = r#"my $s = 'don\'t stop'; my $x = 1;"#;
        let single_inside = single.find("stop").ok_or("missing single quoted text")?;
        let single_after = single.find("my $x").ok_or("missing code after single string")?;
        assert!(LspServer::offset_is_inside_quoted_string(single, single_inside));
        assert!(!LspServer::offset_is_inside_quoted_string(single, single_after));

        let double = r#"my $s = "he said \"ok\""; my $x = 1;"#;
        let double_inside = double.find("ok").ok_or("missing escaped double quoted text")?;
        let double_after = double.find("my $x").ok_or("missing code after double string")?;
        assert!(LspServer::offset_is_inside_quoted_string(double, double_inside));
        assert!(!LspServer::offset_is_inside_quoted_string(double, double_after));

        let comment = "# \"commented\"\nmy $x = 1;";
        let code_after_comment = comment.find("my $x").ok_or("missing code after comment")?;
        assert!(!LspServer::offset_is_inside_quoted_string(comment, code_after_comment));

        Ok(())
    }

    #[test]
    fn prepare_rename_returns_null_for_blocked_or_missing_symbols()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let uri = "file:///workspace/lib/Blocked.pm";
        let source = "use Moo;\nhas name => (is => 'ro');\nsub run { 1 }\n";
        server.test_apply_did_open(uri, source, 1)?;

        let blocked = server.handle_prepare_rename(Some(serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": 1, "character": 5 }
        })))?;
        assert_eq!(blocked, Some(serde_json::json!(null)));

        let punctuation = server.handle_prepare_rename(Some(serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": 2, "character": 10 }
        })))?;
        assert_eq!(punctuation, Some(serde_json::json!(null)));

        let missing_document = server.handle_prepare_rename(Some(serde_json::json!({
            "textDocument": { "uri": "file:///workspace/lib/Missing.pm" },
            "position": { "line": 0, "character": 0 }
        })))?;
        assert_eq!(missing_document, Some(serde_json::json!(null)));

        Ok(())
    }

    #[test]
    fn token_helpers_work_with_non_ascii_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        // "# café\n" — 'é' (U+00E9) is 2 UTF-8 bytes; the line is 9 bytes, 8 chars.
        // Byte offset of '$' on line 2 = 9 + 3 = 12 (after "# café\nmy ").
        let text = "# café\nmy $foo = 1;";
        let dollar_offset = text.find('$').ok_or("missing sigil")?; // byte 12

        let token = server.get_token_at_position(text, dollar_offset);
        assert_eq!(token, "$foo", "byte offset must not be treated as char index");

        let (start, end) = server.get_token_bounds(text, dollar_offset);
        assert_eq!(&text[start..end], "$foo");
        Ok(())
    }

    #[test]
    fn rename_guard_detects_dynamic_typeglob_string_positions()
    -> Result<(), Box<dyn std::error::Error>> {
        let text = r#"*{"Mojolicious::Routes::Route::$name"} = sub { $cb->(@_) };"#;
        let string_offset = text.find("Routes::Route").ok_or("missing dynamic package")? + 2;
        let code_offset = text.find("$cb").ok_or("missing callback")? + 1;

        assert!(LspServer::offset_is_inside_quoted_string(text, string_offset));
        assert!(!LspServer::offset_is_inside_quoted_string(text, code_offset));
        Ok(())
    }

    #[test]
    fn rename_guard_uses_byte_offsets_with_unicode_prefix() -> Result<(), Box<dyn std::error::Error>>
    {
        let text = "my $emoji = \"🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀\";\n*{\"Mojolicious::Routes::Route::$name\"} = sub { $cb->(@_) };";
        let string_offset = text.find("Routes::Route").ok_or("missing dynamic package")? + 2;
        let code_offset = text.find("$cb").ok_or("missing callback")? + 1;

        assert!(LspServer::offset_is_inside_quoted_string(text, string_offset));
        assert!(!LspServer::offset_is_inside_quoted_string(text, code_offset));
        Ok(())
    }

    #[test]
    fn rename_guard_detects_unquoted_generated_accessors() -> Result<(), Box<dyn std::error::Error>>
    {
        let text = "use Moo;\nhas routes      => (is => 'ro');\nhas title => 1; has slug => 1;\nsub routes { 1 }\n";
        let accessor_offset = text.find("routes      =>").ok_or("missing has accessor")? + 2;
        let same_line_accessor_offset =
            text.find("slug =>").ok_or("missing same-line has accessor")? + 2;
        let sub_offset = text.rfind("routes").ok_or("missing sub name")? + 2;

        assert!(LspServer::offset_is_generated_accessor_declaration(text, accessor_offset));
        assert!(LspServer::offset_is_generated_accessor_declaration(
            text,
            same_line_accessor_offset
        ));
        assert!(!LspServer::offset_is_generated_accessor_declaration(text, sub_offset));
        Ok(())
    }

    #[test]
    fn rename_guard_detects_quoted_generated_accessors() -> Result<(), Box<dyn std::error::Error>> {
        let text = "use Moose;\nhas 'name' => (is => 'rw');\nhas'compact' => 1;\nhas '+extended' => 1;\nhas + 'spaced' => 1;\nhas ('wrapped') => 1;\nhash name => 1;\n";
        let accessor_offset = text.find("name' =>").ok_or("missing quoted accessor")? + 1;
        let compact_accessor_offset =
            text.find("compact' =>").ok_or("missing compact accessor")? + 1;
        let extended_accessor_offset =
            text.find("extended' =>").ok_or("missing extended accessor")? + 1;
        let spaced_accessor_offset = text.find("spaced' =>").ok_or("missing spaced accessor")? + 1;
        let wrapped_accessor_offset =
            text.find("wrapped') =>").ok_or("missing wrapped accessor")? + 1;
        let hash_offset = text.rfind("name").ok_or("missing hash key")? + 1;

        assert!(LspServer::offset_is_generated_accessor_declaration(text, accessor_offset));
        assert!(LspServer::offset_is_generated_accessor_declaration(text, compact_accessor_offset));
        assert!(LspServer::offset_is_generated_accessor_declaration(
            text,
            extended_accessor_offset
        ));
        assert!(LspServer::offset_is_generated_accessor_declaration(text, spaced_accessor_offset));
        assert!(LspServer::offset_is_generated_accessor_declaration(text, wrapped_accessor_offset));
        assert!(!LspServer::offset_is_generated_accessor_declaration(text, hash_offset));
        Ok(())
    }
}
