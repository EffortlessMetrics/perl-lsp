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
    if max_len == 0 {
        return String::new();
    }

    let len = state.next_u64() as usize % (max_len + 1);
    const ALPHABET: &[u8] =
        b"abcdefghijklmnopqrstuvwxyz/::._ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789%?&#=+-\\";
    let mut out = String::with_capacity(len);

    for _ in 0..len {
        let idx = (state.next_u64() as usize) % ALPHABET.len();
        out.push(ALPHABET[idx] as char);
    }

    out
}

fn assert_no_file_uri_leak(input: &str) {
    let parsed = workspace_folder_to_path(input);
    assert!(
        !parsed.to_string_lossy().contains("file://"),
        "output path leaked URI scheme for input: {input:?}"
    );
}

#[test]
fn fuzz_workspace_folder_inputs_do_not_panic_or_produce_file_uris() {
    let mut rng = XorShift64::new(0x5EED_BEEF_DEAD_BEEF);

    for _ in 0..5_000 {
        let input = fuzz_string(&mut rng, 96);
        assert_no_file_uri_leak(&input);
    }
}

#[test]
fn fuzz_workspace_folder_targeted_uri_like_inputs() {
    let samples = [
        "",
        "file://",
        "file:///tmp/project",
        "FILE:///UPPER",
        "file://C:/Users/test/project",
        "vscode-remote://ssh-remote+host/workspace",
        "perl://module/Foo::Bar",
        "%66%69%6c%65://encoded",
        "./relative/path",
        "C:\\workspace\\project",
        "\\\\server\\share\\project",
        "file:///%2e%2e/%2e%2e/etc/passwd",
        "file://?query=1#frag",
    ];

    for sample in samples {
        assert_no_file_uri_leak(sample);
    }
}
