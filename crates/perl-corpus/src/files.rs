//! Corpus file discovery helpers.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Environment variable used to override corpus root discovery.
pub const CORPUS_ROOT_ENV: &str = "PERL_CORPUS_ROOT";
const TEST_EXTENSIONS: &[&str] = &["pl", "pm", "t", "psgi", "cgi"];

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
    collect_files(&paths.test_corpus, TEST_EXTENSIONS)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(prefix: &str) -> PathBuf {
        let mut root = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        root.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    #[test]
    fn collect_files_filters_extensions_and_skips_hidden() {
        let root = temp_root("perl_corpus_files");
        let keep_dir = root.join("keep");
        fs::create_dir_all(&keep_dir).expect("create keep dir");
        fs::create_dir_all(root.join("_skip")).expect("create skip dir");
        fs::create_dir_all(root.join(".hidden_dir")).expect("create hidden dir");

        let fixtures = [
            root.join("case.pl"),
            root.join("case.pm"),
            root.join("case.t"),
            root.join("case.psgi"),
            root.join("case.cgi"),
            keep_dir.join("nested.pl"),
        ];
        for fixture in &fixtures {
            fs::write(fixture, "print 1;\n").expect("write fixture");
        }

        fs::write(root.join("case.txt"), "ignore\n").expect("write ignored");
        fs::write(root.join(".hidden.pl"), "ignore\n").expect("write hidden");
        fs::write(root.join("_skip/inner.pl"), "ignore\n").expect("write skipped");
        fs::write(root.join(".hidden_dir/inner.pm"), "ignore\n").expect("write hidden dir");

        let files = collect_files(&root, TEST_EXTENSIONS);
        let mut names: Vec<_> = files
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        names.sort();

        let expected = vec![
            "case.cgi",
            "case.pl",
            "case.pm",
            "case.psgi",
            "case.t",
            "nested.pl",
        ];
        assert_eq!(names, expected);

        fs::remove_dir_all(&root).expect("cleanup temp root");
    }
}
