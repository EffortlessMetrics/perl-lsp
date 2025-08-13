//! textDocument/documentLink handler - clickable module/file links
//!
//! This module creates clickable links for:
//! - Module names (use/require) -> MetaCPAN
//! - File paths in require/do -> local files

use lsp_types::{DocumentLink, Position, Range, Uri};

fn to_range(content: &str, start: usize, end: usize) -> Range {
    // Simple byte->(line,col) translator
    let (mut line, mut col, mut i) = (0u32, 0u32, 0usize);
    let mut start_pos = Position::new(0, 0);
    let mut end_pos = Position::new(0, 0);
    for ch in content.chars() {
        if i == start {
            start_pos = Position::new(line, col);
        }
        if i == end {
            end_pos = Position::new(line, col);
            break;
        }
        i += ch.len_utf8();
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += ch.len_utf16() as u32;
        }
    }
    if end_pos == Position::new(0, 0) {
        end_pos = Position::new(line, col);
    }
    Range::new(start_pos, end_pos)
}

pub fn collect_document_links(text: &str, uri_str: &str) -> Result<Vec<DocumentLink>, String> {
    let mut links = Vec::new();

    for (line_idx, line) in text.lines().enumerate() {
        // `use Foo::Bar;`
        if let Some(idx) = line.find("use ") {
            let rest = &line[idx + 4..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == ':' || *c == '_')
                .collect();
            if !name.is_empty() && name.contains("::") {
                let s = text[..]
                    .lines()
                    .take(line_idx)
                    .map(|l| l.len() + 1)
                    .sum::<usize>()
                    + idx
                    + 4;
                let e = s + name.len();
                links.push(DocumentLink {
                    range: to_range(text, s, e),
                    target: format!("https://metacpan.org/pod/{}", name)
                        .parse::<Uri>()
                        .ok(),
                    tooltip: Some(format!("Open {} on MetaCPAN", name)),
                    data: None,
                });
            }
        }

        // `require Module::Name`
        if let Some(idx) = line.find("require ") {
            let rest = &line[idx + 8..];
            // Check if it's a module name (not a file path)
            if !rest.trim_start().starts_with(['\'', '"']) {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == ':' || *c == '_')
                    .collect();
                if !name.is_empty() && name.contains("::") {
                    let s = text[..]
                        .lines()
                        .take(line_idx)
                        .map(|l| l.len() + 1)
                        .sum::<usize>()
                        + idx
                        + 8;
                    let e = s + name.len();
                    links.push(DocumentLink {
                        range: to_range(text, s, e),
                        target: format!("https://metacpan.org/pod/{}", name)
                            .parse::<Uri>()
                            .ok(),
                        tooltip: Some(format!("Open {} on MetaCPAN", name)),
                        data: None,
                    });
                }
            }
        }

        // `require 'path'` / `do "path"`
        for kw in ["require ", "do "] {
            if let Some(idx) = line.find(kw) {
                let rest = &line[idx + kw.len()..];
                let quote = rest.chars().next().unwrap_or(' ');
                if quote == '\'' || quote == '"' {
                    if let Some(endq) = rest[1..].find(quote) {
                        let path = &rest[1..1 + endq];
                        let s = text[..]
                            .lines()
                            .take(line_idx)
                            .map(|l| l.len() + 1)
                            .sum::<usize>()
                            + idx
                            + kw.len()
                            + 1;
                        let e = s + path.len();
                        // Try to resolve relative to current file
                        let target = if path.starts_with('/') {
                            // Absolute path
                            format!("file://{}", path).parse::<Uri>().ok()
                        } else {
                            // Relative to current file's directory
                            // Extract the directory from the URI string
                            if let Some(last_slash) = uri_str.rfind('/') {
                                let base_dir = &uri_str[..last_slash];
                                format!("{}/{}", base_dir, path).parse::<Uri>().ok()
                            } else {
                                None
                            }
                        };
                        if let Some(target_url) = target {
                            links.push(DocumentLink {
                                range: to_range(text, s, e),
                                target: Some(target_url),
                                tooltip: Some(format!("Open {}", path)),
                                data: None,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(links)
}
