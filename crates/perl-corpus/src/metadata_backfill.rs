use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Summary returned after backfilling metadata in a corpus directory.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataBackfillReport {
    pub scanned: usize,
    pub updated: Vec<PathBuf>,
}

impl MetadataBackfillReport {
    pub fn updated_count(&self) -> usize {
        self.updated.len()
    }
}

/// Add generated metadata to every section that lacks an immediate metadata block.
pub fn backfill_dir(root: &Path) -> Result<MetadataBackfillReport> {
    let mut files = Vec::new();
    if root.exists() {
        for entry in fs::read_dir(root).with_context(|| format!("reading {}", root.display()))? {
            let entry = entry.with_context(|| format!("reading entry in {}", root.display()))?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("txt")
                && !path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with('_'))
            {
                files.push(path);
            }
        }
    }
    files.sort();

    let mut updated = Vec::new();
    for path in &files {
        if backfill_file(path)? {
            updated.push(path.clone());
        }
    }

    Ok(MetadataBackfillReport { scanned: files.len(), updated })
}

/// Add generated metadata to sections in one corpus file.
pub fn backfill_file(path: &Path) -> Result<bool> {
    let content =
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let base_tags = file_tag_map()
        .get(path.file_name().and_then(|name| name.to_str()).unwrap_or_default())
        .cloned()
        .unwrap_or_default();

    let mut modified = false;
    let mut new_content = String::new();
    let mut index = 0;
    let mut section_count = 0;

    while index < lines.len() {
        if is_separator(lines[index]) {
            section_count += 1;
            new_content.push_str(lines[index]);
            index += 1;

            let title = if index < lines.len() {
                let title = trim_line_ending(lines[index]).trim().to_string();
                new_content.push_str(lines[index]);
                index += 1;
                title
            } else {
                "(untitled)".to_string()
            };

            if index < lines.len() && is_separator(lines[index]) {
                new_content.push_str(lines[index]);
                index += 1;
            }

            if index < lines.len() && has_metadata_marker(lines[index]) {
                while index < lines.len() && !is_separator(lines[index]) {
                    new_content.push_str(lines[index]);
                    index += 1;
                }
                continue;
            }

            let file_base = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("corpus")
                .replace(['-', '.'], "_");
            let id = format!("{file_base}.{section_count:03}");
            let generated_tags = generated_tags(&title, &base_tags);
            new_content.push_str(&format!("# @id: {id}\n"));
            new_content.push_str(&format!("# @tags: {}\n", generated_tags.join(" ")));
            new_content.push_str("# @perl: 5.8+\n");
            if title.to_lowercase().contains("error") {
                new_content.push_str("# @flags: error-node-expected\n");
            } else if path.file_name().and_then(|name| name.to_str()) == Some("fuzz-tripwires.txt")
            {
                new_content.push_str("# @flags: lexer-sensitive tripwire\n");
            }
            new_content.push('\n');
            modified = true;

            while index < lines.len() && !is_separator(lines[index]) {
                new_content.push_str(lines[index]);
                index += 1;
            }
        } else {
            new_content.push_str(lines[index]);
            index += 1;
        }
    }

    if modified {
        fs::write(path, new_content).with_context(|| format!("writing {}", path.display()))?;
    }

    Ok(modified)
}

fn generated_tags(title: &str, base_tags: &[&'static str]) -> Vec<&'static str> {
    let title = title.to_lowercase();
    let mut tags = base_tags.to_vec();

    for (needle, tag) in [
        ("math", "arithmetic"),
        ("string", "string"),
        ("list", "list"),
        ("array", "list"),
        ("hash", "hash"),
        ("file", "file-test"),
        ("magic", "punctuation-var"),
        ("special", "punctuation-var"),
        ("error", "error"),
        ("regex", "regex"),
        ("match", "regex"),
        ("pod", "comment"),
    ] {
        if title.contains(needle) {
            tags.push(tag);
        }
    }
    if title.contains("time") {
        tags.extend(["localtime", "gmtime"]);
    }

    tags.sort_unstable();
    tags.dedup();
    tags
}

fn file_tag_map() -> BTreeMap<&'static str, Vec<&'static str>> {
    BTreeMap::from([
        ("builtins-core.txt", vec!["builtin", "function"]),
        ("special-vars-magic.txt", vec!["special-var", "magic"]),
        ("operators-augassign.txt", vec!["operator", "augmented-assignment"]),
        ("cli-env-pod.txt", vec!["argv", "env", "pod", "documentation"]),
        ("time-and-caller.txt", vec!["time", "caller", "wantarray", "eval"]),
        ("sysproc-and-net.txt", vec!["system", "exec", "fork", "pipe", "socket"]),
        ("vec-bitwise-dbm.txt", vec!["vec", "bitwise", "dbm", "tie"]),
        ("debugger-b-backend.txt", vec!["debugger", "B", "Devel"]),
        ("fuzz-tripwires.txt", vec!["tripwire", "regex-code", "qw", "indirect"]),
    ])
}

fn has_metadata_marker(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('#')
        && ["@id:", "@tags:", "@perl:", "@flags:"].iter().any(|marker| trimmed.contains(marker))
}

fn is_separator(line: &str) -> bool {
    let trimmed = trim_line_ending(line).trim();
    !trimmed.is_empty() && trimmed.chars().all(|ch| ch == '=')
}

fn trim_line_ending(line: &str) -> &str {
    line.strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .or_else(|| line.strip_suffix('\r'))
        .unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        path.push(format!("{prefix}_{nanos}.txt"));
        path
    }

    #[test]
    fn backfill_file_adds_metadata_to_unannotated_sections() {
        let path = temp_file("perl_corpus_backfill");
        must(fs::write(&path, "====\nString match\n====\nmy $value = 'x';\n"));

        let modified = must(backfill_file(&path));
        let rewritten = must(fs::read_to_string(&path));
        must(fs::remove_file(&path));

        assert!(modified);
        assert!(rewritten.contains("# @id: perl_corpus_backfill_"));
        assert!(rewritten.contains(".001\n"));
        assert!(rewritten.contains("# @tags: regex string"));
        assert!(rewritten.contains("# @perl: 5.8+"));
    }

    #[test]
    fn backfill_file_preserves_existing_metadata() {
        let path = temp_file("perl_corpus_backfill_existing");
        let original = "====\nAlready tagged\n====\n# @id: custom.case\nmy $value = 1;\n";
        must(fs::write(&path, original));

        let modified = must(backfill_file(&path));
        let rewritten = must(fs::read_to_string(&path));
        must(fs::remove_file(&path));

        assert!(!modified);
        assert_eq!(rewritten, original);
    }
}
