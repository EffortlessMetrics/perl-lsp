//! Fuzz-style randomized stress tests for workspace discovery.

use perl_source_file::is_perl_source_path;
use perl_workspace_discovery::discover_perl_files;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Clone)]
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

    fn next_usize(&mut self, upper_bound: usize) -> usize {
        if upper_bound == 0 {
            return 0;
        }
        (self.next_u64() as usize) % upper_bound
    }
}

fn create_file(root: &Path, relative: &str) -> TestResult {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, "# fuzz fixture\n")?;
    Ok(())
}

#[test]
fn fuzz_randomized_workspace_layouts_preserve_discovery_invariants() -> TestResult {
    let directories = [
        "src",
        "lib",
        "t",
        "scripts",
        "nested/a",
        "nested/b",
        "node_modules/vendor",
        "target/build",
        ".cache/precompiled",
        ".git/hooks",
    ];
    let extensions = ["pl", "pm", "t", "psgi", "txt", "md", "json", "rs"];

    let mut rng = XorShift64::new(0xA11C_E55D_1234_5678);

    for case_idx in 0..128 {
        let tmp = TempDir::new()?;
        let root = tmp.path();

        let file_count = 8 + rng.next_usize(48);
        for file_idx in 0..file_count {
            let directory = directories[rng.next_usize(directories.len())];
            let extension = extensions[rng.next_usize(extensions.len())];
            let suffix = rng.next_usize(10_000);
            let relative = format!("{directory}/f_{case_idx}_{file_idx}_{suffix}.{extension}");
            create_file(root, &relative)?;
        }

        let result = discover_perl_files(root);
        let mut seen = HashSet::new();

        for path in &result.files {
            assert!(path.starts_with(root));
            assert!(is_perl_source_path(path));
            assert!(seen.insert(path.clone()));
        }
    }

    Ok(())
}
