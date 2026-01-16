//! Line index for efficient UTF-16 position calculations.
use ropey::Rope;
#[derive(Debug, Clone)]
pub struct LineStartsCache { line_starts: Vec<usize> }
impl LineStartsCache {
    pub fn new(text: &str) -> Self {
        let mut ls = vec![0];
        let mut i = 0;
        let b = text.as_bytes();
        while i < b.len() {
            if b[i] == b'\n' { ls.push(i + 1); }
            else if b[i] == b'\r' {
                if i + 1 < b.len() && b[i + 1] == b'\n' { ls.push(i + 2); i += 1; }
                else { ls.push(i + 1); }
            }
            i += 1;
        }
        Self { line_starts: ls }
    }
    pub fn new_rope(rope: &Rope) -> Self {
        let mut ls = vec![0];
        for li in 0..rope.len_lines() { if li > 0 { ls.push(rope.line_to_byte(li)); } }
        Self { line_starts: ls }
    }
    pub fn offset_to_position(&self, text: &str, offset: usize) -> (u32, u32) {
        let offset = offset.min(text.len());
        let line = self.line_starts.binary_search(&offset).unwrap_or_else(|i| i.saturating_sub(1));
        let ls = self.line_starts[line];
        (line as u32, text[ls..offset].chars().map(|c| c.len_utf16()).sum::<usize>() as u32)
    }
    pub fn position_to_offset(&self, text: &str, line: u32, character: u32) -> usize {
        let line = line as usize;
        if line >= self.line_starts.len() { return text.len(); }
        let ls = self.line_starts[line];
        let le = if line + 1 < self.line_starts.len() {
            let ns = self.line_starts[line + 1];
            let mut end = ns.saturating_sub(1);
            let b = text.as_bytes();
            while end > ls && (b[end] == b'\n' || b[end] == b'\r') { end = end.saturating_sub(1); }
            end + 1
        } else { text.len() };
        let lt = &text[ls..le];
        let mut uc = 0;
        let mut bo = 0;
        for ch in lt.chars() {
            if uc >= character as usize { break; }
            uc += ch.len_utf16();
            bo += ch.len_utf8();
        }
        ls + bo.min(lt.len())
    }
    pub fn offset_to_position_rope(&self, rope: &Rope, offset: usize) -> (u32, u32) {
        let offset = offset.min(rope.len_bytes());
        let line = self.line_starts.binary_search(&offset).unwrap_or_else(|i| i.saturating_sub(1));
        let ls = self.line_starts[line];
        (line as u32, rope.byte_slice(ls..offset).chars().map(|c| c.len_utf16()).sum::<usize>() as u32)
    }
    pub fn position_to_offset_rope(&self, rope: &Rope, line: u32, character: u32) -> usize {
        let line = line as usize;
        if line >= self.line_starts.len() { return rope.len_bytes(); }
        let ls = self.line_starts[line];
        let le = if line + 1 < self.line_starts.len() { self.line_starts[line + 1] } else { rope.len_bytes() };
        let sl = rope.byte_slice(ls..le);
        let mut uc = 0;
        let mut bo = 0;
        for ch in sl.chars() {
            if uc >= character as usize { break; }
            uc += ch.len_utf16();
            bo += ch.len_utf8();
        }
        ls + bo
    }
}
