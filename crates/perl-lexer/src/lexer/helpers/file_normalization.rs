use crate::PerlLexer;

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

impl PerlLexer<'_> {
    /// Normalize file start by skipping BOM if present
    pub(crate) fn normalize_file_start(&mut self) {
        if self.position == 0 && self.matches_bytes(UTF8_BOM) {
            self.position = UTF8_BOM.len();
            self.line_start_offset = UTF8_BOM.len();
        }
    }
}
