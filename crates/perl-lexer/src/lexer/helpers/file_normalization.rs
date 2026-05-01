use crate::PerlLexer;

impl PerlLexer<'_> {
    /// Normalize file start by skipping BOM if present
    pub(crate) fn normalize_file_start(&mut self) {
        if self.position == 0 && self.matches_bytes(&[0xEF, 0xBB, 0xBF]) {
            self.position = 3;
            self.line_start_offset = 3;
        }
    }
}
