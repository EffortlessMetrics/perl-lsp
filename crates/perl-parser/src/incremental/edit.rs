use crate::incremental::LineIndex;

/// Edit description
#[derive(Clone, Debug)]
pub struct Edit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub new_text: String,
}

impl Edit {
    /// Returns the size of text touched by this edit (inserted or deleted).
    pub(crate) fn touched_bytes(&self) -> usize {
        let replaced_len = self.old_end_byte.saturating_sub(self.start_byte);
        replaced_len.max(self.new_text.len())
    }
}

#[cfg(feature = "lsp-compat")]
impl Edit {
    /// Convert LSP change to Edit
    pub fn from_lsp_change(
        change: &lsp_types::TextDocumentContentChangeEvent,
        line_index: &LineIndex,
        old_text: &str,
    ) -> Option<Self> {
        if let Some(range) = change.range {
            // `range.start.character` / `range.end.character` are UTF-16 code unit
            // offsets as specified by the LSP protocol.  Use the UTF-16-aware
            // conversion so that lines containing multibyte characters (e.g.
            // `my $café = 1;`) map to the correct byte offset rather than treating
            // the UTF-16 column as a raw byte count (fixes #750).
            let start_byte = line_index.position_to_byte_utf16(
                old_text,
                range.start.line as usize,
                range.start.character as usize,
            )?;
            let old_end_byte = line_index.position_to_byte_utf16(
                old_text,
                range.end.line as usize,
                range.end.character as usize,
            )?;
            let new_end_byte = start_byte + change.text.len();

            Some(Edit { start_byte, old_end_byte, new_end_byte, new_text: change.text.clone() })
        } else {
            Some(Edit {
                start_byte: 0,
                old_end_byte: old_text.len(),
                new_end_byte: change.text.len(),
                new_text: change.text.clone(),
            })
        }
    }
}
