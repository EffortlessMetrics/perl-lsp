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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_returns_empty_doc() {
        let doc = extract_pod("");
        assert!(doc.is_empty());
    }

    #[test]
    fn pure_code_no_pod() {
        let source = r#"
package Foo::Bar;
use strict;
sub new { bless {}, shift }
1;
"#;
        let doc = extract_pod(source);
        assert!(doc.is_empty());
    }

    #[test]
    fn extracts_name_section() {
        let source = r#"
=head1 NAME

Foo::Bar - A sample module

=cut
"#;
        let doc = extract_pod(source);
        assert_eq!(doc.name.as_deref(), Some("Foo::Bar - A sample module"));
    }

    #[test]
    fn extracts_synopsis() {
        let source = r#"
=head1 SYNOPSIS

    use Foo::Bar;
    my $obj = Foo::Bar->new();

=cut
"#;
        let doc = extract_pod(source);
        assert!(doc.synopsis.is_some());
        assert!(doc.synopsis.as_ref().is_some_and(|s| s.contains("use Foo::Bar")));
    }

    #[test]
    fn extracts_description_first_paragraph() {
        let source = r#"
=head1 DESCRIPTION

This module does amazing things.
It is very useful.

This second paragraph should not be included.

=cut
"#;
        let doc = extract_pod(source);
        assert_eq!(
            doc.description.as_deref(),
            Some("This module does amazing things.\nIt is very useful.")
        );
    }

    #[test]
    fn extracts_methods() {
        let source = r#"
=head2 new

Creates a new instance of the object.

=head2 process

Processes the input data.

=cut
"#;
        let doc = extract_pod(source);
        assert_eq!(doc.methods.len(), 2);
        assert!(doc.methods.contains_key("new"));
        assert!(doc.methods.contains_key("process"));
        assert!(doc.methods["new"].contains("Creates a new instance"));
        assert!(doc.methods["process"].contains("Processes the input data"));
    }

    #[test]
    fn strips_bold_formatting() {
        assert_eq!(strip_pod_formatting("B<bold text>"), "bold text");
    }

    #[test]
    fn strips_italic_formatting() {
        assert_eq!(strip_pod_formatting("I<italic text>"), "italic text");
    }

    #[test]
    fn strips_code_formatting() {
        assert_eq!(strip_pod_formatting("C<my $var>"), "my $var");
    }

    #[test]
    fn strips_link_simple() {
        assert_eq!(strip_pod_formatting("L<Module::Name>"), "Module::Name");
    }

    #[test]
    fn strips_link_with_display_text() {
        assert_eq!(strip_pod_formatting("L<click here|Module::Name>"), "click here");
    }

    #[test]
    fn strips_link_with_section() {
        assert_eq!(strip_pod_formatting("L<Module::Name/method>"), "Module::Name");
    }

    #[test]
    fn mixed_formatting() {
        assert_eq!(
            strip_pod_formatting("Use B<new> to create a C<Foo> object"),
            "Use new to create a Foo object"
        );
    }

    #[test]
    fn handles_cut_properly() {
        let source = r#"
=head1 NAME

First - Module

=cut

package First;

=head1 NAME

Second - Module

=cut
"#;
        let doc = extract_pod(source);
        // Second =head1 NAME overwrites the first
        assert_eq!(doc.name.as_deref(), Some("Second - Module"));
    }

    #[test]
    fn handles_pod_without_cut_at_eof() {
        let source = r#"
=head1 NAME

Foo::Bar - No cut at end
"#;
        let doc = extract_pod(source);
        assert_eq!(doc.name.as_deref(), Some("Foo::Bar - No cut at end"));
    }

    #[test]
    fn handles_over_item_back() {
        let source = r#"
=head2 options

Available options:

=over 4

=item B<verbose>

Enable verbose output.

=item B<quiet>

Suppress output.

=back

=cut
"#;
        let doc = extract_pod(source);
        assert!(doc.methods.contains_key("options"));
        let method_doc = &doc.methods["options"];
        assert!(method_doc.contains("Available options:"));
        assert!(method_doc.contains("- verbose"));
        assert!(method_doc.contains("- quiet"));
    }

    #[test]
    fn full_module_extraction() {
        let source = r#"
package DateTime::Format::Custom;

use strict;
use warnings;

=head1 NAME

DateTime::Format::Custom - Parse and format dates

=head1 SYNOPSIS

    use DateTime::Format::Custom;
    my $dt = DateTime::Format::Custom->parse("2024-01-01");

=head1 DESCRIPTION

This module provides custom date parsing and formatting
capabilities for the DateTime ecosystem.

It supports multiple input formats and can auto-detect
the format of input strings.

=head2 parse

    my $dt = DateTime::Format::Custom->parse($string);

Parses a date string and returns a L<DateTime> object.

=head2 format

    my $str = DateTime::Format::Custom->format($dt);

Formats a B<DateTime> object as a string.

=head1 AUTHOR

Jane Doe

=cut

sub parse { ... }
sub format { ... }

1;
"#;
        let doc = extract_pod(source);
        assert_eq!(doc.name.as_deref(), Some("DateTime::Format::Custom - Parse and format dates"));
        assert!(doc.synopsis.as_ref().is_some_and(|s| s.contains("use DateTime::Format::Custom")));
        assert!(doc.description.as_ref().is_some_and(|s| s.contains("custom date parsing")));
        // Description should only be first paragraph
        assert!(!doc.description.as_ref().is_none_or(|s| s.contains("auto-detect")));
        assert_eq!(doc.methods.len(), 2);
        assert!(doc.methods["parse"].contains("Parses a date string"));
        assert!(doc.methods["parse"].contains("DateTime"));
        assert!(doc.methods["format"].contains("Formats a DateTime object"));
    }

    #[test]
    fn extract_pod_from_file_missing_file() {
        let result = extract_pod_from_file(Path::new("/nonexistent/file.pm"));
        assert!(result.is_err());
    }

    #[test]
    fn nested_formatting_codes() {
        // Depth tracking handles nested angle brackets correctly
        assert_eq!(strip_pod_formatting("B<I<bold italic>>"), "bold italic");
    }

    #[test]
    fn no_formatting_passthrough() {
        assert_eq!(strip_pod_formatting("plain text here"), "plain text here");
    }

    #[test]
    fn empty_formatting_code() {
        assert_eq!(strip_pod_formatting("B<>"), "");
        assert_eq!(strip_pod_formatting("C<>"), "");
    }

    #[test]
    fn head1_other_sections_ignored() {
        let source = r#"
=head1 AUTHOR

John Doe

=head1 LICENSE

Same as Perl itself.

=cut
"#;
        let doc = extract_pod(source);
        assert!(doc.name.is_none());
        assert!(doc.synopsis.is_none());
        assert!(doc.description.is_none());
        assert!(doc.methods.is_empty());
    }

    #[test]
    fn pod_directive_starts_pod_mode() {
        let source = r#"
package Foo;

=pod

This is some POD text.

=head1 NAME

Foo - A module

=cut
"#;
        let doc = extract_pod(source);
        assert_eq!(doc.name.as_deref(), Some("Foo - A module"));
    }

    #[test]
    fn encoding_directive_starts_pod() {
        let source = r#"
=encoding utf-8

=head1 NAME

Encoded::Module - Uses UTF-8

=cut
"#;
        let doc = extract_pod(source);
        assert_eq!(doc.name.as_deref(), Some("Encoded::Module - Uses UTF-8"));
    }

    #[test]
    fn f_format_code_for_filenames() {
        assert_eq!(strip_pod_formatting("See F<config.yml>"), "See config.yml");
    }

    #[test]
    fn e_format_code_passthrough() {
        // E<lt> E<gt> etc. — we just strip the code, leaving the entity name
        assert_eq!(strip_pod_formatting("E<lt>"), "lt");
    }

    #[test]
    fn multiple_pod_blocks() {
        let source = r#"
package Multi;

=head1 NAME

Multi - Multiple POD blocks

=cut

sub helper { 1 }

=head2 run

Runs the main logic.

=cut

sub run { }

1;
"#;
        let doc = extract_pod(source);
        assert_eq!(doc.name.as_deref(), Some("Multi - Multiple POD blocks"));
        assert!(doc.methods.contains_key("run"));
    }
}
