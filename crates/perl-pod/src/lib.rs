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

/// Strip POD inline formatting codes: `B<bold>`, `I<italic>`, `C<code>`, `L<link>`.
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

            // For L<> links, extract display text
            let display = if code_char == 'L' {
                extract_link_display(&inner_str)
            } else {
                // Recursively strip formatting from inner content
                strip_pod_formatting(&inner_str)
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

fn is_pod_format_code(c: char) -> bool {
    matches!(c, 'B' | 'I' | 'C' | 'L' | 'F' | 'S' | 'E' | 'X' | 'Z')
}

/// Render a POD source string to markdown.
///
/// Converts both structural directives (`=head1`, `=head2`, `=over/=item/=back`)
/// and inline formatting codes (`B<>`, `I<>`, `C<>`, `L<>`, `F<>`, `E<>`) to
/// their markdown equivalents. Verbatim (indented) paragraphs become fenced
/// code blocks.
///
/// When the input has no POD directives (e.g. a plain inline string like
/// `"B<bold> text"`), only inline formatting conversion is applied.
///
/// # Examples
///
/// ```
/// use perl_pod::render_pod_to_markdown;
///
/// // Inline formatting codes
/// assert_eq!(render_pod_to_markdown("B<bold>"), "**bold**");
/// assert_eq!(render_pod_to_markdown("I<italic>"), "_italic_");
/// assert_eq!(render_pod_to_markdown("C<code>"), "`code`");
///
/// // Full POD documents are also handled
/// let pod = "=head1 NAME\n\nFoo - example module\n\n=cut\n";
/// let md = render_pod_to_markdown(pod);
/// assert!(md.contains("## NAME"));
/// ```
#[must_use]
pub fn render_pod_to_markdown(source: &str) -> String {
    // Detect whether source is a full POD document (has structural directives)
    let has_directives = source.lines().any(|l| {
        l.starts_with("=head")
            || l.starts_with("=pod")
            || l.starts_with("=over")
            || l.starts_with("=item")
            || l.starts_with("=back")
            || l.starts_with("=begin")
            || l.starts_with("=for")
            || l.starts_with("=encoding")
            || l.starts_with("=cut")
    });

    if has_directives { render_pod_document(source) } else { render_pod_inline(source) }
}

/// Render a full POD document (with directives) to markdown.
fn render_pod_document(source: &str) -> String {
    let mut output = String::new();
    let mut in_pod = false;
    let mut verbatim_lines: Vec<String> = Vec::new();
    let mut paragraph_lines: Vec<String> = Vec::new();

    for line in source.lines() {
        // Detect start of POD
        if !in_pod
            && (line.starts_with("=head")
                || line.starts_with("=pod")
                || line.starts_with("=over")
                || line.starts_with("=begin")
                || line.starts_with("=for")
                || line.starts_with("=encoding")
                || line.starts_with("=item"))
        {
            in_pod = true;
        }

        if !in_pod {
            continue;
        }

        // =cut ends POD
        if line.starts_with("=cut") {
            flush_verbatim_to_markdown(&mut verbatim_lines, &mut output);
            flush_paragraph_to_markdown(&mut paragraph_lines, &mut output);
            in_pod = false;
            continue;
        }

        // =head1 → ## heading
        if let Some(heading) = line.strip_prefix("=head1") {
            flush_verbatim_to_markdown(&mut verbatim_lines, &mut output);
            flush_paragraph_to_markdown(&mut paragraph_lines, &mut output);
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("## ");
            output.push_str(heading.trim());
            output.push('\n');
            continue;
        }

        // =head2 → ### heading
        if let Some(heading) = line.strip_prefix("=head2") {
            flush_verbatim_to_markdown(&mut verbatim_lines, &mut output);
            flush_paragraph_to_markdown(&mut paragraph_lines, &mut output);
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("### ");
            output.push_str(heading.trim());
            output.push('\n');
            continue;
        }

        // =over and =back produce no visible markdown
        if line.starts_with("=over") || line.starts_with("=back") {
            flush_verbatim_to_markdown(&mut verbatim_lines, &mut output);
            flush_paragraph_to_markdown(&mut paragraph_lines, &mut output);
            continue;
        }

        // =item → markdown bullet
        if let Some(item_text) = line.strip_prefix("=item") {
            flush_verbatim_to_markdown(&mut verbatim_lines, &mut output);
            flush_paragraph_to_markdown(&mut paragraph_lines, &mut output);
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("- ");
            output.push_str(&render_pod_inline(item_text.trim()));
            output.push('\n');
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

        // Blank lines flush accumulated content and separate paragraphs
        if line.trim().is_empty() {
            flush_verbatim_to_markdown(&mut verbatim_lines, &mut output);
            flush_paragraph_to_markdown(&mut paragraph_lines, &mut output);
            if !output.is_empty() && !output.ends_with("\n\n") {
                output.push('\n');
            }
            continue;
        }

        // Verbatim block: lines starting with whitespace
        if line.starts_with(' ') || line.starts_with('\t') {
            flush_paragraph_to_markdown(&mut paragraph_lines, &mut output);
            verbatim_lines.push(line.to_string());
        } else {
            // Regular paragraph text
            flush_verbatim_to_markdown(&mut verbatim_lines, &mut output);
            paragraph_lines.push(line.to_string());
        }
    }

    // Flush any trailing content (POD can end at EOF without =cut)
    flush_verbatim_to_markdown(&mut verbatim_lines, &mut output);
    flush_paragraph_to_markdown(&mut paragraph_lines, &mut output);

    output.trim_end().to_string()
}

/// Flush accumulated verbatim lines as a fenced code block.
fn flush_verbatim_to_markdown(verbatim_lines: &mut Vec<String>, output: &mut String) {
    if verbatim_lines.is_empty() {
        return;
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    output.push_str("```\n");
    for line in verbatim_lines.iter() {
        output.push_str(line.trim_start());
        output.push('\n');
    }
    output.push_str("```\n");
    verbatim_lines.clear();
}

/// Flush accumulated paragraph lines as rendered inline markdown.
fn flush_paragraph_to_markdown(paragraph_lines: &mut Vec<String>, output: &mut String) {
    if paragraph_lines.is_empty() {
        return;
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    let para = paragraph_lines.join("\n");
    output.push_str(&render_pod_inline(&para));
    output.push('\n');
    paragraph_lines.clear();
}

/// Render POD inline formatting codes to markdown.
///
/// Handles: `B<>` → `**`, `I<>` → `_`, `C<>` → backtick, `F<>` → backtick,
/// `L<>` → markdown link, `E<>` → entity decoding, `X<>` → empty,
/// `Z<>` → empty, `S<>` → passthrough.
fn render_pod_inline(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 16);
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Check for formatting code: X<...> where X is a known POD format code letter
        if i + 1 < len
            && chars[i].is_ascii_alphabetic()
            && chars[i + 1] == '<'
            && is_pod_format_code(chars[i])
        {
            let code_char = chars[i];
            i += 2; // skip X<

            // Find matching > accounting for nested angle brackets
            let mut depth = 1usize;
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
            let inner: String = chars[start..i].iter().collect();

            // Skip closing >
            if i < len {
                i += 1;
            }

            let rendered = render_format_code(code_char, &inner);
            result.push_str(&rendered);
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Render a single POD formatting code to its markdown equivalent.
fn render_format_code(code: char, inner: &str) -> String {
    match code {
        'B' => {
            // Bold: B<text> → **text**
            let content = render_pod_inline(inner);
            if content.is_empty() { String::new() } else { format!("**{content}**") }
        }
        'I' => {
            // Italic: I<text> → _text_
            let content = render_pod_inline(inner);
            if content.is_empty() { String::new() } else { format!("_{content}_") }
        }
        'C' => {
            // Code: C<text> → `text`
            let content = render_pod_inline(inner);
            if content.is_empty() { String::new() } else { format!("`{content}`") }
        }
        'F' => {
            // Filename: F<path> → `path` (same as code in markdown)
            let content = render_pod_inline(inner);
            if content.is_empty() { String::new() } else { format!("`{content}`") }
        }
        'L' => {
            // Link: various forms — see render_link_to_markdown
            render_link_to_markdown(inner)
        }
        'E' => {
            // Entity: E<lt> → <, E<gt> → >, E<amp> → &, etc.
            decode_pod_entity(inner)
        }
        'S' => {
            // Non-breaking space: S<text> → text (markdown has no direct equivalent)
            render_pod_inline(inner)
        }
        'X' => {
            // Index entry: X<> → empty string (invisible in rendered output)
            String::new()
        }
        'Z' => {
            // Null code: Z<> → empty string
            String::new()
        }
        _ => {
            // Unknown code: pass through unchanged
            format!("{code}<{inner}>")
        }
    }
}

/// Render a POD `L<>` link to a markdown link.
///
/// Forms handled:
/// - `L<Module>` → `[Module](Module)`
/// - `L<text|target>` → `[text](target)`
/// - `L<Module/section>` → `[Module](Module/section)`
/// - `L<text|Module/section>` → `[text](Module/section)`
fn render_link_to_markdown(inner: &str) -> String {
    if let Some(pipe_pos) = inner.find('|') {
        // L<display text|target>
        let display = render_pod_inline(&inner[..pipe_pos]);
        let target = &inner[pipe_pos + 1..];
        format!("[{display}]({target})")
    } else if let Some(slash_pos) = inner.find('/') {
        // L<Module/section> → [Module](Module/section)
        let module = &inner[..slash_pos];
        let display = render_pod_inline(module);
        format!("[{display}]({inner})")
    } else {
        // L<Module> → [Module](Module)
        let display = render_pod_inline(inner);
        format!("[{display}]({inner})")
    }
}

/// Decode a POD `E<>` entity to its character equivalent.
///
/// Handles common named entities and numeric entities (decimal and hex).
/// Unknown entities are returned unchanged, e.g. `E<unknown>` → `E<unknown>`.
fn decode_pod_entity(entity: &str) -> String {
    match entity {
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "amp" => "&".to_string(),
        "quot" => "\"".to_string(),
        "verbar" | "VERBAR" => "|".to_string(),
        "sol" | "SOL" => "/".to_string(),
        "apos" => "'".to_string(),
        "lchevron" | "laquo" => "\u{00AB}".to_string(),
        "rchevron" | "raquo" => "\u{00BB}".to_string(),
        "copy" => "\u{00A9}".to_string(),
        "reg" => "\u{00AE}".to_string(),
        "trade" => "\u{2122}".to_string(),
        "mdash" => "\u{2014}".to_string(),
        "ndash" => "\u{2013}".to_string(),
        "hellip" => "\u{2026}".to_string(),
        "nbsp" => "\u{00A0}".to_string(),
        _ => {
            // Numeric entity: E<0x263A> (hex) or E<9786> (decimal)
            if let Some(hex) = entity.strip_prefix("0x").or_else(|| entity.strip_prefix("0X")) {
                if let Ok(n) = u32::from_str_radix(hex, 16)
                    && let Some(c) = char::from_u32(n)
                {
                    return c.to_string();
                }
            } else if let Ok(n) = entity.parse::<u32>()
                && let Some(c) = char::from_u32(n)
            {
                return c.to_string();
            }
            // Unknown: pass through unchanged
            format!("E<{entity}>")
        }
    }
}
