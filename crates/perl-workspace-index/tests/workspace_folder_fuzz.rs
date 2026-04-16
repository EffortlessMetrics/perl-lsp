use perl_workspace::folder::workspace_folder_to_path;

#[derive(Debug)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }
}

fn fuzz_string(state: &mut XorShift64, max_len: usize) -> String {
    let len = (state.next_u64() as usize % max_len).saturating_add(1);
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz/::._ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789%";
    let mut out = String::with_capacity(len);

    for _ in 0..len {
        let idx = (state.next_u64() as usize) % ALPHABET.len();
        out.push(ALPHABET[idx] as char);
    }

    out
}

#[test]
fn fuzz_workspace_folder_inputs_do_not_panic_or_produce_file_uris() {
    let mut rng = XorShift64::new(0x5EED_BEEF_DEAD_BEEF);

    for _ in 0..2_500 {
        let input = fuzz_string(&mut rng, 48);
        let parsed = workspace_folder_to_path(&input);

        assert!(!parsed.to_string_lossy().contains("file://"));
    }
}
