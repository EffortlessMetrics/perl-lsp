//! POD documentation extractor for Perl `.pm` files.
//!
//! Parses POD (Plain Old Documentation) sections from Perl source files and
//! returns structured documentation suitable for hover display in an LSP.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::collections::HashMap;
use std::io;
use std::path::Path;

/// Extracted POD documentation from a Perl module.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodDoc {
    /// Module name and optional one-line description from `=head1 NAME`.
    pub name: Option<String>,
    /// Usage example from `=head1 SYNOPSIS`.
    pub synopsis: Option<String>,
    /// First paragraph of `=head1 DESCRIPTION`.
    pub description: Option<String>,
    /// Method/function docs keyed by name, from `=head2 method_name`.
    pub methods: HashMap<String, String>,
}

impl PodDoc {
    /// Returns `true` if no documentation was extracted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.synopsis.is_none()
            && self.description.is_none()
            && self.methods.is_empty()
    }
}

/// Read a file and extract its POD documentation.
///
/// # Errors
///
/// Returns an I/O error if the file cannot be read.
pub fn extract_pod_from_file(path: &Path) -> io::Result<PodDoc> {
    let content = std::fs::read_to_string(path)?;
    Ok(extract_pod(&content))
}

/// Extract POD documentation from a string of Perl source code.
#[must_use]
pub fn extract_pod(source: &str) -> PodDoc {
    let mut doc = PodDoc::default();
    let mut current_section: Option<Section> = None;
    let mut body = String::new();
    let mut in_pod = false;
    let mut in_over = false;

    for line in source.lines() {
        // Detect POD start directives
        if line.starts_with("=head")
            || line.starts_with("=pod")
            || line.starts_with("=over")
            || line.starts_with("=begin")
            || line.starts_with("=for")
            || line.starts_with("=encoding")
            || line.starts_with("=item")
        {
            in_pod = true;
        }

        if !in_pod {
            continue;
        }

        // =cut ends POD
        if line.starts_with("=cut") {
            flush_section(&mut doc, &current_section, &body, in_over);
            current_section = None;
            body.clear();
            in_pod = false;
            in_over = false;
            continue;
        }

        // =over / =item / =back for lists
        if line.starts_with("=over") {
            in_over = true;
            body.push('\n');
            continue;
        }
        if line.starts_with("=back") {
            in_over = false;
            body.push('\n');
            continue;
        }
        if line.starts_with("=item") {
            let item_text = line.strip_prefix("=item").map(str::trim).unwrap_or("");
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str("- ");
            body.push_str(&strip_pod_formatting(item_text));
            body.push('\n');
            continue;
        }

        // New head1 section
        if let Some(heading) = line.strip_prefix("=head1") {
            flush_section(&mut doc, &current_section, &body, false);
            body.clear();
            let heading = heading.trim();
            current_section = Some(match heading {
                "NAME" => Section::Name,
                "SYNOPSIS" => Section::Synopsis,
                "DESCRIPTION" => Section::Description,
                _ => Section::Other(()),
            });
            continue;
        }

        // New head2 section — treated as method documentation
        if let Some(heading) = line.strip_prefix("=head2") {
            flush_section(&mut doc, &current_section, &body, false);
            body.clear();
            let heading = heading.trim().to_string();
            current_section = Some(Section::Method(heading));
            continue;
        }

        // Skip other directives
        if line.starts_with("=pod")
            || line.starts_with("=encoding")
            || line.starts_with("=begin")
            || line.starts_with("=end")
            || line.starts_with("=for")
        {
            continue;
        }

        // Accumulate body text
        if current_section.is_some() && (!body.is_empty() || !line.is_empty()) {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(line);
        }
    }

    // Flush any remaining section (POD can end at EOF without =cut)
    flush_section(&mut doc, &current_section, &body, in_over);

    doc
}

#[derive(Debug)]
enum Section {
    Name,
    Synopsis,
    Description,
    Method(String),
    Other(()),
}

fn flush_section(doc: &mut PodDoc, section: &Option<Section>, body: &str, _in_over: bool) {
    let section = match section {
        Some(s) => s,
        None => return,
    };

    let trimmed = body.trim();
    if trimmed.is_empty() {
        return;
    }

    let cleaned = strip_pod_formatting(trimmed);

    match section {
        Section::Name => {
            doc.name = Some(cleaned);
        }
        Section::Synopsis => {
            doc.synopsis = Some(cleaned);
        }
        Section::Description => {
            // Take only the first paragraph
            let first_para = first_paragraph(&cleaned);
            doc.description = Some(first_para);
        }
        Section::Method(name) => {
            doc.methods.insert(name.clone(), cleaned);
        }
        Section::Other(_) => {
            // Ignore other head1 sections for now
        }
    }
}

/// Extract the first paragraph (text before the first blank line).
fn first_paragraph(text: &str) -> String {
    let mut result = String::new();
    for line in text.lines() {
        if line.trim().is_empty() && !result.is_empty() {
            break;
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line);
    }
    result
}

/// Strip POD inline formatting codes: `B<bold>`, `I<italic>`, `C<code>`, `L<link>`,
/// and decode common `E<>` entities.
///
/// Handles simple (non-nested) formatting codes. Nested codes like `B<I<text>>`
/// are handled by stripping outer codes first.
fn strip_pod_formatting(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Check for formatting code: X<...> where X is a letter
        if i + 2 < len
            && chars[i].is_ascii_alphabetic()
            && chars[i + 1] == '<'
            && is_pod_format_code(chars[i])
        {
            let code_char = chars[i];
            i += 2; // skip X<

            // Find matching > accounting for nested <>
            let mut depth = 1;
            let start = i;
            while i < len && depth > 0 {
                if chars[i] == '<' {
                    depth += 1;
                } else if chars[i] == '>' {
                    depth -= 1;
                }
                if depth > 0 {
                    i += 1;
                }
            }
            let inner = &chars[start..i];
            let inner_str: String = inner.iter().collect();

            let display = match code_char {
                'L' => extract_link_display(&inner_str),
                'E' => decode_pod_entity(&inner_str),
                _ => strip_pod_formatting(&inner_str),
            };

            result.push_str(&display);
            if i < len {
                i += 1; // skip >
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Extract display text from a POD L<> link.
///
/// Handles common forms:
/// - `L<Module::Name>` -> `Module::Name`
/// - `L<text|Module::Name>` -> `text`
/// - `L<text|Module::Name/section>` -> `text`
/// - `L<Module::Name/section>` -> `Module::Name`
fn extract_link_display(link: &str) -> String {
    // L<text|target> -> show text
    if let Some(pipe_pos) = link.find('|') {
        return strip_pod_formatting(&link[..pipe_pos]);
    }
    // L<Module/section> -> show Module
    if let Some(slash_pos) = link.find('/') {
        return strip_pod_formatting(&link[..slash_pos]);
    }
    strip_pod_formatting(link)
}

fn decode_pod_entity(entity: &str) -> String {
    match entity {
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "amp" => "&".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        _ => entity.to_string(),
    }
}

fn is_pod_format_code(c: char) -> bool {
    matches!(c, 'B' | 'I' | 'C' | 'L' | 'F' | 'S' | 'E' | 'X' | 'Z')
}
