//! POD documentation extractor for Perl `.pm` files.
//!
//! Parses POD (Plain Old Documentation) sections from Perl source files and
//! returns structured documentation suitable for hover display in an LSP.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

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
///
/// Parses POD markup from the source string and extracts structured documentation
/// for the NAME, SYNOPSIS, DESCRIPTION sections, and method documentation (head2).
///
/// # Arguments
///
/// * `source` - Perl source code containing POD documentation
///
/// # Returns
///
/// A `PodDoc` containing the extracted documentation fields. Empty fields indicate
/// the corresponding POD section was not present in the source.
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
            let heading = strip_pod_formatting(heading.trim());
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

/// Stores accumulated body text into the appropriate `PodDoc` field.
///
/// Called when a POD section ends (new section starts, `=cut`, or EOF).
/// The body text is cleaned of POD formatting and stored based on section type:
/// - `Name` → `PodDoc::name`
/// - `Synopsis` → `PodDoc::synopsis`
/// - `Description` → `PodDoc::description` (first paragraph only)
/// - `Method(name)` → `PodDoc::methods` entry
/// - `Other` → ignored
///
/// # Arguments
///
/// * `doc` - The `PodDoc` to store extracted content into
/// * `section` - The section type being flushed
/// * `body` - Accumulated raw text for the section
/// * `_in_over` - Whether inside an `=over`/`=back` block (unused, for future expansion)
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
        // Check for formatting code: X<...> or X<< ... >> where X is a POD code.
        if i + 2 < len
            && chars[i].is_ascii_alphabetic()
            && chars[i + 1] == '<'
            && is_pod_format_code(chars[i])
        {
            let code_char = chars[i];
            let delimiter_width = opening_delimiter_width(&chars, i + 1);
            i += 1 + delimiter_width; // skip X and the opening angle delimiter

            let start = i;
            let end = if delimiter_width == 1 {
                // Find matching > accounting for nested <> in classic X<...> codes.
                let mut depth = 1;
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
                let end = i;
                if i < len {
                    i += 1; // skip >
                }
                end
            } else {
                // POD permits doubled (or wider) delimiters so content can contain raw
                // angle brackets, for example C<< $obj->method >>.
                while i < len && !has_closing_delimiter(&chars, i, delimiter_width) {
                    i += 1;
                }
                let end = i;
                if i < len {
                    i += delimiter_width;
                }
                end
            };

            let inner = &chars[start..end];
            let mut inner_str: String = inner.iter().collect();
            if delimiter_width > 1 {
                inner_str = trim_multidelimiter_padding(&inner_str).to_string();
            }

            let display = match code_char {
                'L' => extract_link_display(&inner_str),
                'E' => decode_pod_entity(&inner_str),
                _ => strip_pod_formatting(&inner_str),
            };

            result.push_str(&display);
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

fn opening_delimiter_width(chars: &[char], start: usize) -> usize {
    chars[start..].iter().take_while(|ch| **ch == '<').count()
}

fn has_closing_delimiter(chars: &[char], start: usize, delimiter_width: usize) -> bool {
    chars
        .get(start..start + delimiter_width)
        .is_some_and(|candidate| candidate.iter().all(|ch| *ch == '>'))
}

fn trim_multidelimiter_padding(text: &str) -> &str {
    text.trim_matches(|ch: char| ch.is_ascii_whitespace())
}

/// Percent-encode characters that are invalid in a markdown link URL.
///
/// Encodes spaces (most common in POD section names like `L<Module/"Section Name">`)
/// and other characters that would break the markdown `[text](url)` parser.
fn encode_pod_link_target(target: &str) -> String {
    let mut encoded = String::with_capacity(target.len());
    for byte in target.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b':' | b'/') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn escape_markdown_link_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' | '[' | ']' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Extract a markdown link from a POD `L<>` formatting code.
///
/// Returns `[display](perl-module://target)` so LSP clients (VS Code) render
/// the link as clickable in hover tooltips.  Spaces in section names are
/// percent-encoded so the URL is well-formed.
///
/// Handles all standard POD link forms:
/// - `L<Module::Name>` → `[Module::Name](perl-module://Module::Name)`
/// - `L<text|Module::Name>` → `[text](perl-module://Module::Name)`
/// - `L<Module::Name/section>` → `[Module::Name](perl-module://Module::Name/section)`
/// - `L<text|Module::Name/section>` → `[text](perl-module://Module::Name/section)`
fn extract_link_display(link: &str) -> String {
    // L<text|target> — explicit display text before the pipe
    if let Some(pipe_pos) = link.find('|') {
        let display = escape_markdown_link_text(&strip_pod_formatting(&link[..pipe_pos]));
        let target = encode_pod_link_target(link[pipe_pos + 1..].trim());
        return format!("[{display}](perl-module://{target})");
    }
    // L<Module/section> — module + section, display is just the module part
    if let Some(slash_pos) = link.find('/') {
        let module = escape_markdown_link_text(&strip_pod_formatting(&link[..slash_pos]));
        let target = encode_pod_link_target(link.trim());
        return format!("[{module}](perl-module://{target})");
    }
    // L<Module::Name> — simple module reference
    let display = escape_markdown_link_text(&strip_pod_formatting(link));
    let target = encode_pod_link_target(link.trim());
    format!("[{display}](perl-module://{target})")
}

/// Decodes a POD E<> entity to its corresponding character.
///
/// Handles standard POD escape sequences:
/// - `E<lt>` → `<`
/// - `E<gt>` → `>`
/// - `E<amp>` → `&`
/// - `E<quot>` → `"`
/// - `E<apos>` → `'`
///
/// - `E<sol>` -> `/`
/// - `E<verbar>` -> `|`
/// - `E<number>`, `E<0xhex>`, and `E<0octal>` numeric codepoints
///
/// Unknown entities are returned as-is.
fn decode_pod_entity(entity: &str) -> String {
    match entity {
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "amp" => "&".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        "sol" => "/".to_string(),
        "verbar" => "|".to_string(),
        _ => decode_numeric_pod_entity(entity).unwrap_or_else(|| entity.to_string()),
    }
}

fn decode_numeric_pod_entity(entity: &str) -> Option<String> {
    if entity.is_empty() {
        return None;
    }

    let codepoint =
        if let Some(hex) = entity.strip_prefix("0x").or_else(|| entity.strip_prefix("0X")) {
            u32::from_str_radix(hex, 16).ok()?
        } else if entity.starts_with('0') && entity.len() > 1 {
            u32::from_str_radix(entity, 8).ok()?
        } else {
            entity.parse::<u32>().ok()?
        };

    char::from_u32(codepoint).map(|ch| ch.to_string())
}

fn is_pod_format_code(c: char) -> bool {
    matches!(c, 'B' | 'I' | 'C' | 'L' | 'F' | 'S' | 'E' | 'X' | 'Z')
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── decode_pod_entity ────────────────────────────────────────────────────

    #[test]
    fn decode_entity_unknown_returns_name_unchanged() {
        // Unknown entity names fall through to the `_` branch and are returned as-is.
        assert_eq!(decode_pod_entity("nbsp"), "nbsp");
        assert_eq!(decode_pod_entity("unknown"), "unknown");
    }

    #[test]
    fn decode_entity_empty_returns_empty() {
        // Empty entity name is unknown → returned unchanged (empty string).
        assert_eq!(decode_pod_entity(""), "");
    }

    #[test]
    fn decode_entity_numeric_codepoints() {
        assert_eq!(decode_pod_entity("32"), " ");
        assert_eq!(decode_pod_entity("0x20"), " ");
        assert_eq!(decode_pod_entity("0X3BB"), "λ");
        assert_eq!(decode_pod_entity("181"), "µ");
        assert_eq!(decode_pod_entity("0x201E"), "„");
        assert_eq!(decode_pod_entity("075"), "=");
    }

    #[test]
    fn decode_entity_invalid_numeric_returns_unchanged() {
        assert_eq!(decode_pod_entity("0x"), "0x");
        assert_eq!(decode_pod_entity("09"), "09");
        assert_eq!(decode_pod_entity("1114112"), "1114112");
        assert_eq!(decode_pod_entity("0x110000"), "0x110000");
    }

    #[test]
    fn decode_entity_known_entities() {
        assert_eq!(decode_pod_entity("lt"), "<");
        assert_eq!(decode_pod_entity("gt"), ">");
        assert_eq!(decode_pod_entity("amp"), "&");
        assert_eq!(decode_pod_entity("quot"), "\"");
        assert_eq!(decode_pod_entity("apos"), "'");
        assert_eq!(decode_pod_entity("sol"), "/");
        assert_eq!(decode_pod_entity("verbar"), "|");
    }

    // ── multi-angle POD formatting delimiters ────────────────────────────────

    #[test]
    fn strips_double_angle_code_formatting() {
        assert_eq!(strip_pod_formatting("C<< $obj->method >>"), "$obj->method");
    }

    #[test]
    fn double_angle_formatting_allows_single_angle_content() {
        assert_eq!(strip_pod_formatting("Use C<< <=> >> for comparison"), "Use <=> for comparison");
    }

    #[test]
    fn double_angle_link_renders_markdown() {
        assert_eq!(
            strip_pod_formatting("L<< display text|File::Find/The wanted function >>"),
            "[display text](perl-module://File::Find/The%20wanted%20function)"
        );
    }

    // ── encode_pod_link_target ───────────────────────────────────────────────

    #[test]
    fn encode_link_empty_string() {
        assert_eq!(encode_pod_link_target(""), "");
    }

    #[test]
    fn encode_link_pure_ascii_safe_chars_pass_through() {
        // The safe set includes alphanumerics and: - . _ ~ : /
        assert_eq!(encode_pod_link_target("-._~"), "-._~");
        assert_eq!(
            encode_pod_link_target("A::Module/section-name_v1.0~"),
            "A::Module/section-name_v1.0~"
        );
    }

    #[test]
    fn encode_link_multibyte_utf8_cafe() {
        // "café" — 'é' is U+00E9, encoded in UTF-8 as 0xC3 0xA9.
        let result = encode_pod_link_target("café");
        assert_eq!(result, "caf%C3%A9");
    }

    #[test]
    fn encode_link_multibyte_utf8_japanese() {
        // "日本語" — each kanji is three UTF-8 bytes.
        let result = encode_pod_link_target("日本語");
        // 日 = E6 97 A5, 本 = E6 9C AC, 語 = E8 AA 9E
        assert_eq!(result, "%E6%97%A5%E6%9C%AC%E8%AA%9E");
    }

    #[test]
    fn encode_link_consecutive_special_chars() {
        // Multiple consecutive non-safe characters are each percent-encoded.
        assert_eq!(encode_pod_link_target("a  b"), "a%20%20b");
        assert_eq!(encode_pod_link_target("((()))"), "%28%28%28%29%29%29");
    }

    #[test]
    fn encode_link_control_chars_tab_and_newline() {
        // Tab (0x09) and newline (0x0A) are not in the safe set → percent-encoded.
        assert_eq!(encode_pod_link_target("\t"), "%09");
        assert_eq!(encode_pod_link_target("\n"), "%0A");
        assert_eq!(encode_pod_link_target("a\tb"), "a%09b");
    }
}
