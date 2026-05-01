use super::{generate_id, normalize_flags, normalize_tags, Section, META_RE, OUTPUT_SEPARATOR, SECTION_RE};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

pub fn parse_sections(text: &str, path: &Path) -> Result<Vec<Section>> {
    let mut sections = Vec::new();
    let file_stem = path
        .file_stem()
        .and_then(|stem| {
            let slug = super::slugify_title(&stem.to_string_lossy());
            if slug.is_empty() { None } else { Some(slug) }
        })
        .unwrap_or_else(|| "corpus".to_string());
    let mut auto_ids: HashMap<String, usize> = HashMap::new();
    let mut section_index = 0usize;

    let Some(sec_re) = SECTION_RE.as_ref() else {
        return Ok(sections);
    };

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

        let id = generate_id(meta.get("id").map(String::as_str), &file_stem, &title, section_index, &mut auto_ids);
        let tags = meta.get("tags").map_or_else(Vec::new, |s| normalize_tags(s));
        let perl = meta.get("perl").cloned().filter(|s| !s.is_empty());
        let flags = meta.get("flags").map_or_else(Vec::new, |s| normalize_flags(s));

        let body_lines = if body_start_idx < lines.len() { &lines[body_start_idx..] } else { &[] };
        let body_end =
            body_lines.iter().position(|line| line.trim() == OUTPUT_SEPARATOR).unwrap_or(body_lines.len());
        let body = body_lines[..body_end].join("\n").trim().to_string();
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
