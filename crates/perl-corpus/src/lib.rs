//! Perl parser corpus - test data and property-based testing infrastructure
#![allow(clippy::pedantic)] // Corpus crate - focus on core clippy lints only

pub mod cases;
pub mod codegen;
pub mod files;
pub mod r#gen;
pub mod index;
pub mod lint;
pub mod meta;

use anyhow::{Context, Result};
pub use cases::{
    ComplexDataStructureCase, EdgeCase, EdgeCaseGenerator, complex_data_structure_cases,
    edge_cases, find_complex_case, get_complex_data_structure_tests, sample_complex_case,
};
pub use codegen::{
    CodegenOptions, StatementKind, generate_perl_code, generate_perl_code_with_options,
    generate_perl_code_with_seed, generate_perl_code_with_statements,
};
pub use files::{
    CORPUS_ROOT_ENV, CorpusFile, CorpusLayer, CorpusPaths, get_all_test_files, get_corpus_files,
    get_corpus_files_from, get_fuzz_files, get_test_files,
};
use meta::Section;
use regex::Regex;
use std::collections::HashMap;
use std::{fs, path::Path};

static SEC_RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"(?m)^=+\s*$")
        .unwrap_or_else(|_| panic!("SEC_RE regex is invalid - this is a bug in the corpus parser"))
});
static META_RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"(?m)^#\s*@(?P<k>id|tags|perl|flags):\s*(?P<v>.*)$")
        .unwrap_or_else(|_| panic!("META_RE regex is invalid - this is a bug in the corpus parser"))
});

fn slugify_title(title: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for ch in title.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    slug
}

/// Parse a corpus file into sections.
pub fn parse_file(path: &Path) -> Result<Vec<Section>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut sections = Vec::new();
    let file_stem = path
        .file_stem()
        .map(|stem| slugify_title(&stem.to_string_lossy()))
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "corpus".to_string());
    let mut auto_ids: HashMap<String, usize> = HashMap::new();
    let mut section_index = 0usize;

    // Find all section delimiters
    let mut offs = vec![0usize];
    for m in SEC_RE.find_iter(&text) {
        offs.push(m.start());
    }
    // Add EOF sentinel
    offs.push(text.len());

    for w in offs.windows(2) {
        let start = w[0];
        let end = w[1];
        if start == 0 {
            continue;
        } // skip prelude

        section_index += 1;

        // Extract section text
        let section_text = &text[start..end];
        let lines: Vec<&str> = section_text.lines().collect();

        if lines.len() < 2 {
            continue;
        } // malformed section

        // Title is the line after "===="
        let title = lines[1].trim().to_string();

        // Gather metadata lines following title
        let mut meta = HashMap::<String, String>::new();
        let mut body_start_idx = 2;

        for (i, line) in lines.iter().enumerate().skip(2) {
            if let Some(cap) = META_RE.captures(line) {
                meta.insert(cap["k"].to_string(), cap["v"].trim().to_string());
                body_start_idx = i + 1;
            } else if !line.starts_with('#') || line.trim().is_empty() {
                body_start_idx = i;
                break;
            }
        }

        // Extract metadata fields
        let mut id = meta.get("id").cloned().unwrap_or_default();
        let tags = meta
            .get("tags")
            .map(|s| {
                s.replace(',', " ").split_whitespace().map(|t| t.to_lowercase()).collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let perl = meta.get("perl").cloned().filter(|s| !s.is_empty());
        let flags = meta
            .get("flags")
            .map(|s| {
                s.replace(',', " ").split_whitespace().map(|t| t.to_string()).collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Extract body (code after metadata)
        let body_lines = if body_start_idx < lines.len() { &lines[body_start_idx..] } else { &[] };
        let body_end = body_lines
            .iter()
            .position(|line| line.trim() == "---")
            .unwrap_or(body_lines.len());
        let body = body_lines[..body_end].join("\n").trim().to_string();

        if id.is_empty() {
            let title_slug = slugify_title(&title);
            let base = if title_slug.is_empty() {
                format!("section-{}", section_index)
            } else {
                title_slug
            };
            let base_id = format!("{}.{}", file_stem, base);
            let count = auto_ids.entry(base_id.clone()).or_insert(0);
            id = if *count == 0 { base_id } else { format!("{}-{}", base_id, *count + 1) };
            *count += 1;
        }

        // Calculate line number (for error reporting)
        let line_num = text[..start].lines().count() + 1;

        sections.push(Section {
            id,
            title,
            file: path.file_name().unwrap_or_default().to_string_lossy().into(),
            tags,
            perl,
            flags,
            body,
            line: Some(line_num),
        });
    }

    Ok(sections)
}

/// Scan the `test_corpus/` directory.
pub fn parse_dir(dir: &Path) -> Result<Vec<Section>> {
    let mut all = Vec::new();

    // Build glob pattern
    let pattern = format!("{}/**/*.txt", dir.display());

    for entry in glob::glob(&pattern)? {
        let p = entry?;

        // Skip index/tag files
        let filename = p.file_name().unwrap_or_default().to_string_lossy();
        if filename.starts_with('_') || filename.starts_with('.') {
            continue;
        }

        all.extend(parse_file(&p)?);
    }

    // Sort by file and ID for stable output
    all.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.id.cmp(&b.id)));

    Ok(all)
}

/// Find sections by tag
pub fn find_by_tag<'a>(sections: &'a [Section], tag: &str) -> Vec<&'a Section> {
    sections.iter().filter(|s| s.has_tag(tag)).collect()
}

/// Find sections by flag
pub fn find_by_flag<'a>(sections: &'a [Section], flag: &str) -> Vec<&'a Section> {
    sections.iter().filter(|s| s.has_flag(flag)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        path.push(format!("{}_{}.txt", prefix, nanos));
        path
    }

    #[test]
    fn parse_file_strips_ast_and_generates_id() {
        let path = temp_file("perl_corpus_parse");
        let contents = r#"==========================================
Sample Section
==========================================

my $x = 1;

---
(source_file
  (expression_statement
    (assignment_expression
      (variable_declaration
        (scalar
          (varname)))
      (number))))

==========================================
Tagged Section
==========================================
# @id: custom.id
# @tags: alpha, Beta
# @flags: parser-sensitive
my $y = 2;
"#;

        fs::write(&path, contents).expect("write temp corpus file");
        let sections = parse_file(&path).expect("parse corpus file");
        fs::remove_file(&path).expect("cleanup temp corpus file");

        assert_eq!(sections.len(), 2);
        let file_stem = path.file_stem().unwrap().to_string_lossy();
        let expected_prefix = format!("{}.", slugify_title(&file_stem));
        assert!(sections[0].id.starts_with(&expected_prefix));
        assert_eq!(sections[0].body, "my $x = 1;");
        assert!(!sections[0].body.contains("---"));
        assert_eq!(sections[1].id, "custom.id");
        assert_eq!(sections[1].tags, vec!["alpha".to_string(), "beta".to_string()]);
        assert_eq!(sections[1].flags, vec!["parser-sensitive".to_string()]);
        assert_eq!(sections[1].body, "my $y = 2;");
    }
}
