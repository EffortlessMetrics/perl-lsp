use crate::meta::Section;
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::{fs, path::Path};

// Regex patterns - use Option for graceful degradation if compilation fails
static SEC_RE: once_cell::sync::Lazy<Option<Regex>> =
    once_cell::sync::Lazy::new(|| Regex::new(r"(?m)^=+\s*$").ok());
static META_RE: once_cell::sync::Lazy<Option<Regex>> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"(?m)^#\s*@(?P<k>id|tags|perl|flags):\s*(?P<v>.*)$").ok()
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
        .and_then(|stem| {
            let slug = slugify_title(&stem.to_string_lossy());
            if slug.is_empty() { None } else { Some(slug) }
        })
        .unwrap_or_else(|| "corpus".to_string());
    let mut auto_ids: HashMap<String, usize> = HashMap::new();
    let mut section_index = 0usize;

    let Some(sec_re) = SEC_RE.as_ref() else {
        return Ok(sections);
    };

    let raw_delims: Vec<usize> = sec_re.find_iter(&text).map(|m| m.start()).collect();

    let mut opening_delims: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < raw_delims.len() {
        opening_delims.push(raw_delims[i]);
        if i + 1 < raw_delims.len() {
            let between = &text[raw_delims[i]..raw_delims[i + 1]];
            if between.lines().count() == 2 {
                i += 2;
                continue;
            }
        }
        i += 1;
    }

    let mut offs = vec![0usize];
    offs.extend(&opening_delims);
    offs.dedup();
    offs.push(text.len());

    for w in offs.windows(2) {
        let start = w[0];
        let end = w[1];

        let first_line = text[start..end].lines().next().unwrap_or("");
        if !sec_re.is_match(first_line) {
            continue;
        }

        section_index += 1;
        let section_text = &text[start..end];
        let lines: Vec<&str> = section_text.lines().collect();

        if lines.len() < 2 {
            continue;
        }

        let title = lines[1].trim().to_string();
        let after_title_idx = if lines.len() > 2 && sec_re.is_match(lines[2]) { 3 } else { 2 };

        let mut meta = HashMap::<String, String>::new();
        let mut body_start_idx = after_title_idx;

        if let Some(meta_re) = META_RE.as_ref() {
            for (i, line) in lines.iter().enumerate().skip(after_title_idx) {
                if let Some(cap) = meta_re.captures(line) {
                    meta.insert(cap["k"].to_string(), cap["v"].trim().to_string());
                    body_start_idx = i + 1;
                } else if !line.starts_with('#') || line.trim().is_empty() {
                    body_start_idx = i;
                    break;
                }
            }
        }

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

        let body_lines = if body_start_idx < lines.len() { &lines[body_start_idx..] } else { &[] };
        let body_end =
            body_lines.iter().position(|line| line.trim() == "---").unwrap_or(body_lines.len());
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

        let line_num = text[..start].lines().count() + 1;
        let file_name = path.file_name().unwrap_or_default();

        sections.push(Section {
            id,
            title,
            file: file_name.to_string_lossy().into(),
            tags,
            perl,
            flags,
            body,
            line: Some(line_num),
        });
    }

    Ok(sections)
}

pub fn parse_dir(dir: &Path) -> Result<Vec<Section>> {
    let mut all = Vec::new();
    let pattern = format!("{}/**/*.txt", dir.display());

    for entry in glob::glob(&pattern)? {
        let p = entry?;
        let filename = p.file_name().unwrap_or_default().to_string_lossy();
        if filename.starts_with('_') || filename.starts_with('.') {
            continue;
        }

        all.extend(parse_file(&p)?);
    }

    all.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.id.cmp(&b.id)));
    Ok(all)
}

pub fn find_by_tag<'a>(sections: &'a [Section], tag: &str) -> Vec<&'a Section> {
    sections.iter().filter(|s| s.has_tag(tag)).collect()
}

pub fn find_by_flag<'a>(sections: &'a [Section], flag: &str) -> Vec<&'a Section> {
    sections.iter().filter(|s| s.has_flag(flag)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::{must, must_some};
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

        must(std::fs::write(&path, contents));
        let sections = must(parse_file(&path));
        must(std::fs::remove_file(&path));

        assert!(sections.len() >= 2);

        let sample_section = must_some(sections.iter().find(|s| s.body.contains("my $x = 1;")));
        let tagged_section = must_some(sections.iter().find(|s| s.id == "custom.id"));

        assert_eq!(sample_section.body, "my $x = 1;");
        assert!(!sample_section.body.contains("---"));
        assert_eq!(tagged_section.id, "custom.id");
        assert_eq!(tagged_section.tags, vec!["alpha".to_string(), "beta".to_string()]);
        assert_eq!(tagged_section.flags, vec!["parser-sensitive".to_string()]);
        assert_eq!(tagged_section.body, "my $y = 2;");
    }
}
