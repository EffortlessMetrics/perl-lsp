#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_parser::incremental::{apply_edits, Edit, IncrementalState};

const MAX_INITIAL_CHARS: usize = 256;
const MAX_EDIT_STEPS: usize = 32;
const MAX_INSERT_CHARS: usize = 32;

struct ByteCursor<'a> {
    data: &'a [u8],
    idx: usize,
}

impl<'a> ByteCursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, idx: 0 }
    }

    fn next_u8(&mut self) -> u8 {
        if self.data.is_empty() {
            return 0;
        }
        let b = self.data[self.idx % self.data.len()];
        self.idx = self.idx.saturating_add(1);
        b
    }

    fn next_bounded(&mut self, max_inclusive: usize) -> usize {
        if max_inclusive == 0 {
            return 0;
        }
        usize::from(self.next_u8()) % (max_inclusive + 1)
    }

    fn next_char(&mut self) -> char {
        const ALPHABET: &[u8] =
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_ ;(){}\n\t";
        char::from(ALPHABET[usize::from(self.next_u8()) % ALPHABET.len()])
    }

    fn next_text(&mut self, max_chars: usize) -> String {
        let len = self.next_bounded(max_chars);
        (0..len).map(|_| self.next_char()).collect()
    }
}

fn char_to_byte_offsets(s: &str) -> Vec<usize> {
    let mut offsets: Vec<usize> = s.char_indices().map(|(i, _)| i).collect();
    offsets.push(s.len());
    offsets
}

fn apply_to_ground_truth(
    source: &mut String,
    start_byte: usize,
    old_end_byte: usize,
    new_text: &str,
) {
    let mut new_source =
        String::with_capacity(source.len() - (old_end_byte - start_byte) + new_text.len());
    new_source.push_str(&source[..start_byte]);
    new_source.push_str(new_text);
    new_source.push_str(&source[old_end_byte..]);
    *source = new_source;
}

fuzz_target!(|data: &[u8]| {
    let mut cursor = ByteCursor::new(data);

    let initial = cursor.next_text(MAX_INITIAL_CHARS);
    let mut state = IncrementalState::new(initial.clone());
    let mut expected = initial;

    let steps = cursor.next_bounded(MAX_EDIT_STEPS);
    for _ in 0..steps {
        let offsets = char_to_byte_offsets(&expected);
        let char_count = offsets.len() - 1;

        let start_char = cursor.next_bounded(char_count);
        let max_delete = char_count - start_char;
        let delete_chars = cursor.next_bounded(max_delete);

        let start_byte = offsets[start_char];
        let old_end_byte = offsets[start_char + delete_chars];
        let new_text = cursor.next_text(MAX_INSERT_CHARS);
        let new_end_byte = start_byte + new_text.len();

        let edit = Edit { start_byte, old_end_byte, new_end_byte, new_text: new_text.clone() };

        if apply_edits(&mut state, &[edit]).is_err() {
            return;
        }

        apply_to_ground_truth(&mut expected, start_byte, old_end_byte, &new_text);

        assert_eq!(
            state.source, expected,
            "incremental state diverged from ground truth after random edit sequence"
        );
    }
});
