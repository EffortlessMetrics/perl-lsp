use crate::metadata::{
    flags::normalize_flags,
    ids::{auto_id, slugify_title},
    section::Section,
    syntax::{EXPECTED_SEPARATOR, META_RE, SEC_RE},
    tags::normalize_tags,
};
use std::{collections::HashMap, path::Path};

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
            for (line_index, line) in lines.iter().enumerate().skip(after_title_idx) {
                if let Some(cap) = meta_re.captures(line) {
                    meta.insert(cap["k"].to_string(), cap["v"].trim().to_string());
                    body_start_idx = line_index + 1;
                } else if !line.starts_with('#') || line.trim().is_empty() {
                    body_start_idx = line_index;
                    break;
                }
            }
        }

        let mut id = meta.get("id").cloned().unwrap_or_default();
        let tags = normalize_tags(meta.get("tags"));
        let perl = meta.get("perl").cloned().filter(|s| !s.is_empty());
        let flags = normalize_flags(meta.get("flags"));

        let body_lines = if body_start_idx < lines.len() { &lines[body_start_idx..] } else { &[] };
        let body_end = body_lines
            .iter()
            .position(|line| line.trim() == EXPECTED_SEPARATOR)
            .unwrap_or(body_lines.len());
        let body = body_lines[..body_end].join("\n").trim().to_string();

        auto_id(&mut id, &title, section_index, &file_stem, &mut auto_ids);

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

    sections
}
