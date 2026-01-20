//! Corpus file discovery helpers.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Environment variable used to override corpus root discovery.
pub const CORPUS_ROOT_ENV: &str = "PERL_CORPUS_ROOT";

/// Common corpus paths anchored at a root directory.
#[derive(Debug, Clone)]
pub struct CorpusPaths {
    /// Workspace root used for discovery.
    pub root: PathBuf,
    /// Directory containing gap coverage corpus files.
    pub test_corpus: PathBuf,
    /// Directory containing fuzz regression fixtures.
    pub fuzz: PathBuf,
}

impl CorpusPaths {
    /// Discover corpus paths from environment or workspace layout.
    pub fn discover() -> Self {
        if let Ok(root) = env::var(CORPUS_ROOT_ENV) {
            return Self::from_root(PathBuf::from(root));
        }

        Self::from_root(find_workspace_root())
    }

    /// Build corpus paths from an explicit root.
    pub fn from_root(root: PathBuf) -> Self {
        Self {
            test_corpus: root.join("test_corpus"),
            fuzz: root.join("crates/perl-corpus/fuzz"),
            root,
        }
    }
}

/// Return test corpus files (gap coverage fixtures).
pub fn get_test_files() -> Vec<PathBuf> {
    get_test_files_from(&CorpusPaths::discover())
}

/// Return test corpus files using a specific root.
pub fn get_test_files_from(paths: &CorpusPaths) -> Vec<PathBuf> {
    collect_files(&paths.test_corpus, &["pl", "pm"])
}

/// Return fuzz regression fixtures (Perl sources only).
pub fn get_fuzz_files() -> Vec<PathBuf> {
    get_fuzz_files_from(&CorpusPaths::discover())
}

/// Return fuzz regression fixtures from an explicit root.
pub fn get_fuzz_files_from(paths: &CorpusPaths) -> Vec<PathBuf> {
    collect_files(&paths.fuzz, &["pl"])
}

/// Return all available Perl sources across corpus layers.
pub fn get_all_test_files() -> Vec<PathBuf> {
    let paths = CorpusPaths::discover();
    let mut files = get_test_files_from(&paths);
    files.extend(get_fuzz_files_from(&paths));
    files.sort();
    files.dedup();
    files
}

fn find_workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest_dir.ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if !cargo_toml.exists() {
            continue;
        }

        if let Ok(contents) = fs::read_to_string(&cargo_toml) {
            if contents.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }

    manifest_dir
}

fn collect_files(root: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !root.exists() {
        return files;
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            if file_name.starts_with('.') || file_name.starts_with('_') {
                continue;
            }

            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            if file_type.is_file() && has_allowed_extension(&path, extensions) {
                files.push(path);
            }
        }
    }

    files.sort();
    files.dedup();
    files
}

fn has_allowed_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.iter().any(|allowed| ext.eq_ignore_ascii_case(allowed)))
        .unwrap_or(false)
}
