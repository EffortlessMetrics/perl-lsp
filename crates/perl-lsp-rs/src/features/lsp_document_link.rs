//! textDocument/documentLink handler - clickable module/file links
//!
//! This module creates clickable links for:
//! - Module names (use/require) -> MetaCPAN
//! - File paths in require/do -> local files

use lsp_types::{DocumentLink, Position, Range, Uri};
use std::path::PathBuf;
use url::Url;

fn line_start_offsets(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

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

/// Collects clickable document links from Perl source code.
///
/// Scans the document for `use`, `require`, and `do` statements, creating links for:
/// - Module names (e.g., `use Foo::Bar;`) → MetaCPAN URLs
/// - File paths in quotes (e.g., `require 'path/file.pl'`) → local file URLs
///
/// # Arguments
/// * `text` - The Perl source code to scan
/// * `uri` - The document's URI, used to resolve relative file paths
///
/// # Returns
/// A vector of `DocumentLink` objects with range, target URL, and tooltip.
pub fn collect_document_links(text: &str, uri: &Url) -> Result<Vec<DocumentLink>, String> {
    let mut links = Vec::new();

    let line_starts = line_start_offsets(text);

    for (line_idx, line) in text.lines().enumerate() {
        let line_start = line_starts.get(line_idx).copied().unwrap_or(text.len());
        // `use Foo::Bar;`
        if let Some(idx) = line.find("use ") {
            let rest = &line[idx + 4..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == ':' || *c == '_')
                .collect();
            if !name.is_empty() && name.contains("::") {
                let s = line_start + idx + 4;
                let e = s + name.len();
                links.push(DocumentLink {
                    range: to_range(text, s, e),
                    target: Url::parse(&format!("https://metacpan.org/pod/{}", name))
                        .map_err(|e| {
                            tracing::debug!(module = %name, error = %e, "document link: failed to build MetaCPAN URL");
                        })
                        .ok()
                        .and_then(|url| url.to_string().parse::<Uri>().map_err(|e| {
                            tracing::debug!(module = %name, error = %e, "document link: failed to parse MetaCPAN URI");
                        }).ok()),
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
                    let s = line_start + idx + 8;
                    let e = s + name.len();
                    links.push(DocumentLink {
                        range: to_range(text, s, e),
                        target: Url::parse(&format!("https://metacpan.org/pod/{}", name))
                            .map_err(|e| {
                                tracing::debug!(module = %name, error = %e, "document link: failed to build MetaCPAN URL");
                            })
                            .ok()
                            .and_then(|url| url.to_string().parse::<Uri>().map_err(|e| {
                                tracing::debug!(module = %name, error = %e, "document link: failed to parse MetaCPAN URI");
                            }).ok()),
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
                        let s = line_start + idx + kw.len() + 1;
                        let e = s + path.len();
                        // Try to resolve relative to current file
                        let target = if PathBuf::from(path).is_absolute() {
                            // Absolute path - works on both Unix and Windows
                            Url::from_file_path(path)
                                .map_err(|()| {
                                    tracing::debug!(
                                        path,
                                        "document link: failed to convert absolute path to URL"
                                    );
                                })
                                .ok()
                        } else {
                            // Relative to current file's directory
                            uri.to_file_path().map_err(|()| {
                                tracing::debug!(uri = %uri, "document link: URI is not a file path");
                            }).ok().and_then(|base_path| {
                                base_path.parent().and_then(|parent| {
                                    let resolved = parent.join(path);
                                    // Normalize the path for the current OS
                                    Url::from_file_path(&resolved).map_err(|()| {
                                        tracing::debug!(path = %resolved.display(), "document link: failed to convert resolved path to URL");
                                    }).ok()
                                })
                            })
                        };
                        if let Some(target_url) = target {
                            // Get display path for tooltip
                            let display_path = if let Ok(file_path) = target_url.to_file_path() {
                                file_path.display().to_string()
                            } else {
                                path.to_string()
                            };
                            links.push(DocumentLink {
                                range: to_range(text, s, e),
                                target: target_url.to_string().parse::<Uri>().map_err(|e| {
                                    tracing::debug!(error = %e, "document link: failed to parse file URI");
                                }).ok(),
                                tooltip: Some(format!("Open {}", display_path)),
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

#[cfg(test)]
mod tests {
    use super::{collect_document_links, line_start_offsets};
    use lsp_types::Position;
    use url::Url;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn line_start_offsets_preserve_crlf_byte_starts() {
        let text = "# before\r\nuse Foo::Bar;\r\nrequire Baz::Qux;\r\n";

        assert_eq!(line_start_offsets(text), vec![0, 10, 25, 44]);
    }

    #[test]
    fn collect_document_links_uses_crlf_line_starts_for_module_ranges() -> TestResult {
        let uri = Url::parse("file:///workspace/main.pl")?;
        let text = "# before\r\nuse Foo::Bar;\r\nrequire Baz::Qux;\r\n";

        let links = collect_document_links(text, &uri)?;
        let foo = links
            .iter()
            .find(|link| link.tooltip.as_deref() == Some("Open Foo::Bar on MetaCPAN"))
            .ok_or("missing Foo::Bar document link")?;
        let baz = links
            .iter()
            .find(|link| link.tooltip.as_deref() == Some("Open Baz::Qux on MetaCPAN"))
            .ok_or("missing Baz::Qux document link")?;

        assert_eq!(foo.range.start, Position::new(1, 4));
        assert_eq!(foo.range.end, Position::new(1, 12));
        assert_eq!(baz.range.start, Position::new(2, 8));
        assert_eq!(baz.range.end, Position::new(2, 16));
        Ok(())
    }
}
