const TEMPLATE_EXTENSIONS: [&str; 4] = ["ep", "tt", "tt2", "mason"];

pub(super) fn is_embedded_template_uri(uri: &str) -> bool {
    perl_uri::uri_extension(uri)
        .is_some_and(|ext| TEMPLATE_EXTENSIONS.iter().any(|t| t.eq_ignore_ascii_case(ext)))
}

pub(super) fn is_perl_language_id(language_id: &str) -> bool {
    matches!(
        language_id.to_ascii_lowercase().as_str(),
        "perl" | "perl5" | "perl-cpanfile" | "embedded-perl" | "mojolicious"
    )
}

#[cfg(feature = "incremental")]
pub(super) fn build_incremental_edit_set(
    original_rope: &ropey::Rope,
    lsp_changes: &[lsp_types::TextDocumentContentChangeEvent],
) -> Option<perl_parser::incremental::incremental_edit::IncrementalEditSet> {
    use crate::textdoc::{PosEnc, safe_range_mapping};
    use perl_parser::incremental::incremental_edit::{IncrementalEdit, IncrementalEditSet};

    fn map_offset_to_original_space(evolving: usize, cumulative_shift: isize) -> Option<usize> {
        if cumulative_shift >= 0 {
            evolving.checked_sub(cumulative_shift as usize)
        } else {
            evolving.checked_add((-cumulative_shift) as usize)
        }
    }

    let mut working_rope = original_rope.clone();
    let mut edit_set = IncrementalEditSet::new();
    let mut cumulative_shift: isize = 0;

    for change in lsp_changes {
        let range = change.range.as_ref()?;
        let mapping = safe_range_mapping(&working_rope, range, PosEnc::Utf16)?;
        let evolving_start = mapping.start_byte;
        let evolving_end = mapping.end_byte;

        let (Some(orig_start), Some(orig_end)) = (
            map_offset_to_original_space(evolving_start, cumulative_shift),
            map_offset_to_original_space(evolving_end, cumulative_shift),
        ) else {
            tracing::debug!(
                "Incremental edit batch cannot be mapped to original space; falling back to full reparse"
            );
            return None;
        };
        edit_set.add(IncrementalEdit::new(orig_start, orig_end, change.text.clone()));

        working_rope.remove(mapping.start_char..mapping.end_char);
        working_rope.insert(mapping.start_char, &change.text);

        cumulative_shift +=
            change.text.len() as isize - (evolving_end as isize - evolving_start as isize);
    }

    if edit_set.is_empty() { None } else { Some(edit_set) }
}
