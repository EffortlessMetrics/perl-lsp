//! POD (Plain Old Documentation) parser for Perl files.
//!
//! Extracts structured documentation from `=head1 NAME`, `=head1 SYNOPSIS`,
//! and `=head1 DESCRIPTION` sections of Perl modules.

use std::io;
use std::path::Path;

/// Structured POD documentation extracted from a Perl file.
#[derive(Debug, Clone, Default)]
pub struct PodDoc {
    /// Module name from `=head1 NAME`.
    pub name: Option<String>,
    /// Synopsis text from `=head1 SYNOPSIS`.
    pub synopsis: Option<String>,
    /// Description text from `=head1 DESCRIPTION`.
    pub description: Option<String>,
}

impl PodDoc {
    /// Returns `true` if no POD sections were found.
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.synopsis.is_none() && self.description.is_none()
    }
}

/// Extract POD documentation from a Perl file.
///
/// Parses `=head1 NAME`, `=head1 SYNOPSIS`, and `=head1 DESCRIPTION` sections.
/// Returns a `PodDoc` with the extracted content. Sections not found will be `None`.
pub fn extract_pod_from_file(path: &Path) -> io::Result<PodDoc> {
    let content = std::fs::read_to_string(path)?;
    Ok(extract_pod_from_str(&content))
}

/// Extract POD documentation from a string.
pub fn extract_pod_from_str(content: &str) -> PodDoc {
    let mut doc = PodDoc::default();
    let mut current_section: Option<&str> = None;
    let mut current_buf = String::new();
    let mut in_pod = false;

    for line in content.lines() {
        // Enter POD block
        if line.starts_with('=') {
            if line == "=cut" {
                // Flush current section before leaving POD
                flush_section(&mut doc, current_section, &current_buf);
                current_section = None;
                current_buf.clear();
                in_pod = false;
                continue;
            }

            // Any =command starts POD mode
            in_pod = true;

            if let Some(heading) = line.strip_prefix("=head1 ") {
                let heading = heading.trim();
                // Flush previous section
                flush_section(&mut doc, current_section, &current_buf);
                current_buf.clear();

                match heading {
                    "NAME" => current_section = Some("NAME"),
                    "SYNOPSIS" => current_section = Some("SYNOPSIS"),
                    "DESCRIPTION" => current_section = Some("DESCRIPTION"),
                    _ => current_section = None,
                }
            } else if current_section.is_some() {
                // Other POD commands within a tracked section — keep collecting
                current_buf.push_str(line);
                current_buf.push('\n');
            }
        } else if in_pod && current_section.is_some() {
            current_buf.push_str(line);
            current_buf.push('\n');
        }
    }

    // Flush any remaining section (file ended without =cut)
    flush_section(&mut doc, current_section, &current_buf);

    doc
}

/// Flush accumulated text into the appropriate `PodDoc` field.
fn flush_section(doc: &mut PodDoc, section: Option<&str>, buf: &str) {
    let text = strip_pod_formatting(buf.trim());
    if text.is_empty() {
        return;
    }
    match section {
        Some("NAME") => doc.name = Some(text),
        Some("SYNOPSIS") => doc.synopsis = Some(text),
        Some("DESCRIPTION") => doc.description = Some(text),
        _ => {}
    }
}

/// Strip common POD formatting codes: B<>, C<>, I<>, L<>, F<>, S<>, E<>, X<>, Z<>.
fn strip_pod_formatting(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        // Check for POD formatting code: single uppercase letter followed by '<'
        if c.is_ascii_uppercase() && chars.peek() == Some(&'<') {
            // Consume the '<'
            chars.next();

            // Check for E<> entity codes
            if c == 'E' {
                let entity: String = chars.by_ref().take_while(|&ch| ch != '>').collect();
                match entity.as_str() {
                    "lt" => result.push('<'),
                    "gt" => result.push('>'),
                    "sol" => result.push('/'),
                    "verbar" => result.push('|'),
                    "amp" => result.push('&'),
                    _ => {
                        result.push_str(&entity);
                    }
                }
                continue;
            }

            // Z<> produces nothing
            if c == 'Z' {
                for ch in chars.by_ref() {
                    if ch == '>' {
                        break;
                    }
                }
                continue;
            }

            // X<> (index entry) produces nothing
            if c == 'X' {
                for ch in chars.by_ref() {
                    if ch == '>' {
                        break;
                    }
                }
                continue;
            }

            // For L<>, extract display text or the link target
            if c == 'L' {
                let mut link_content = String::new();
                let mut depth: usize = 1;
                for ch in chars.by_ref() {
                    if ch == '<' {
                        depth = depth.saturating_add(1);
                        link_content.push(ch);
                    } else if ch == '>' {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            break;
                        }
                        link_content.push(ch);
                    } else {
                        link_content.push(ch);
                    }
                }
                // L<display|target> -> display; L<target> -> target
                if let Some(pipe_pos) = link_content.find('|') {
                    result.push_str(&link_content[..pipe_pos]);
                } else {
                    result.push_str(&link_content);
                }
                continue;
            }

            // B<>, C<>, I<>, F<>, S<> — extract inner content
            let mut depth: usize = 1;
            for ch in chars.by_ref() {
                if ch == '<' {
                    depth = depth.saturating_add(1);
                    result.push(ch);
                } else if ch == '>' {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                    result.push(ch);
                } else {
                    result.push(ch);
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_basic_pod() {
        let content = r#"
package Foo::Bar;

=head1 NAME

Foo::Bar - A sample module

=head1 SYNOPSIS

    use Foo::Bar;
    my $obj = Foo::Bar->new();

=head1 DESCRIPTION

This module does B<amazing> things with C<code>.

=cut

sub new { }
"#;
        let doc = extract_pod_from_str(content);
        assert_eq!(doc.name.as_deref(), Some("Foo::Bar - A sample module"));
        assert!(doc.synopsis.as_deref().is_some_and(|s| s.contains("use Foo::Bar;")));
        assert!(doc.description.as_deref().is_some_and(|s| s.contains("amazing")));
        // POD formatting should be stripped
        assert!(doc.description.as_deref().is_some_and(|s| !s.contains("B<")));
        assert!(doc.description.as_deref().is_some_and(|s| !s.contains("C<")));
    }

    #[test]
    fn test_empty_pod() {
        let content = "package Foo; sub bar { }";
        let doc = extract_pod_from_str(content);
        assert!(doc.is_empty());
    }

    #[test]
    fn test_strip_pod_formatting() {
        assert_eq!(strip_pod_formatting("B<bold>"), "bold");
        assert_eq!(strip_pod_formatting("C<code>"), "code");
        assert_eq!(strip_pod_formatting("I<italic>"), "italic");
        assert_eq!(strip_pod_formatting("L<Foo::Bar>"), "Foo::Bar");
        assert_eq!(strip_pod_formatting("L<display text|Foo::Bar>"), "display text");
        assert_eq!(strip_pod_formatting("E<lt>"), "<");
        assert_eq!(strip_pod_formatting("E<gt>"), ">");
        assert_eq!(strip_pod_formatting("X<index entry>"), "");
        assert_eq!(strip_pod_formatting("Z<>"), "");
    }

    #[test]
    fn test_pod_without_cut() {
        let content = r#"
=head1 NAME

NoEnd - Module without =cut

=head1 DESCRIPTION

Still valid POD.
"#;
        let doc = extract_pod_from_str(content);
        assert_eq!(doc.name.as_deref(), Some("NoEnd - Module without =cut"));
        assert!(doc.description.as_deref().is_some_and(|s| s.contains("Still valid POD.")));
    }
}
