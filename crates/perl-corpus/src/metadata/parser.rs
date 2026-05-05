use crate::metadata::ids::{make_section_id, slugify_title};
use crate::metadata::{ExpectedBlock, ExpectedFormat, IdSource, Section};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

static SEC_RE: std::sync::LazyLock<Option<Regex>> =
    std::sync::LazyLock::new(|| Regex::new(r"(?m)^=+\s*$").ok());
static META_RE: std::sync::LazyLock<Option<Regex>> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^#\s*@(?P<k>id|tags|perl|flags):\s*(?P<v>.*)$").ok()
});

pub fn parse_sections(text: &str, path: &Path) -> Vec<Section> {
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
        return sections;
    };
    let meta_re = META_RE.as_ref();

    let raw_delims: Vec<usize> = sec_re.find_iter(text).map(|m| m.start()).collect();
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
        for (idx, line) in lines.iter().enumerate().skip(after_title_idx) {
            if let Some(meta_re) = meta_re {
                if let Some(cap) = meta_re.captures(line) {
                    meta.insert(cap["k"].to_string(), cap["v"].trim().to_string());
                    body_start_idx = idx + 1;
                    continue;
                }
            }

            if !line.starts_with('#') || line.trim().is_empty() {
                body_start_idx = idx;
                break;
            }
        }

        let explicit_id = meta.get("id").cloned().filter(|id| !id.trim().is_empty());
        let id = make_section_id(
            explicit_id.as_deref().unwrap_or_default(),
            &file_stem,
            &title,
            section_index,
            &mut auto_ids,
        );
        let (id_source, generated_id) = if explicit_id.is_some() {
            (IdSource::Explicit, None)
        } else {
            (IdSource::Generated, Some(id.clone()))
        };
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
                s.replace(',', " ").split_whitespace().map(ToString::to_string).collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let body_lines = if body_start_idx < lines.len() { &lines[body_start_idx..] } else { &[] };
        let body_end =
            body_lines.iter().position(|line| line.trim() == "---").unwrap_or(body_lines.len());
        let body = body_lines[..body_end].join("\n").trim().to_string();
        let expected = if body_end < body_lines.len() {
            let raw = body_lines[body_end + 1..].join("\n").trim().to_string();
            (!raw.is_empty()).then(|| ExpectedBlock { format: detect_expected_format(&raw), raw })
        } else {
            None
        };
        let line_num = text[..start].lines().count() + 1;
        let file_name = path.file_name().unwrap_or_default();

        sections.push(Section {
            id,
            id_source,
            explicit_id,
            generated_id,
            title,
            file: file_name.to_string_lossy().into(),
            tags,
            perl,
            flags,
            body,
            expected,
            line: Some(line_num),
        });
    }

    sections
}

fn detect_expected_format(raw: &str) -> ExpectedFormat {
    let trimmed = raw.trim();
    if trimmed.starts_with('(') && trimmed.contains(')') {
        return ExpectedFormat::TreeSitterSexp;
    }
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        if trimmed.contains("\"diagnostic\"") || trimmed.contains("\"diagnostics\"") {
            return ExpectedFormat::DiagnosticsJson;
        }
        return ExpectedFormat::AstJson;
    }
    if trimmed.is_empty() { ExpectedFormat::Unknown } else { ExpectedFormat::PlainText }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_sections_tracks_explicit_and_generated_ids() {
        let text = "=====\nExplicit\n# @id: explicit.case\nprint 1;\n=====\nGenerated\nprint 2;\n";
        let sections = parse_sections(text, Path::new("sample.txt"));
        assert_eq!(sections.len(), 2);
        assert!(matches!(sections[0].id_source, IdSource::Explicit));
        assert_eq!(sections[0].explicit_id.as_deref(), Some("explicit.case"));
        assert!(sections[0].generated_id.is_none());
        assert!(matches!(sections[1].id_source, IdSource::Generated));
        assert!(sections[1].explicit_id.is_none());
        assert_eq!(sections[1].generated_id.as_deref(), Some(sections[1].id.as_str()));
    }

    #[test]
    fn parse_sections_preserves_expected_block() {
        let text = "=====\nHas expected\nprint 1;\n---\n{\"diagnostics\":[]}\n";
        let sections = parse_sections(text, Path::new("sample.txt"));
        assert_eq!(sections.len(), 1);
        let expected = sections[0].expected.as_ref().expect("expected block");
        assert_eq!(expected.raw, "{\"diagnostics\":[]}");
        assert!(matches!(expected.format, ExpectedFormat::DiagnosticsJson));
    }
}
