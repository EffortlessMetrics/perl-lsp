//! POD documentation facts.
//!
//! Combines two views of a file's Plain Old Documentation:
//! - a **structured** summary (module name / description / documented method
//!   names) reused from the zero-dependency `perl-pod` leaf crate, and
//! - **ranged sections** (`=head1`/`=head2`/…/`=item`) from a lightweight line
//!   scan, so consumers get both semantic content and spans.
//!
//! Implements the POD fact class (PLSP-ADR-0006 follow-up), taking the
//! substrate to full fact-class coverage.

use serde::{Deserialize, Serialize};

use crate::id::FileId;
use crate::provenance::Confidence;
use crate::range::{SourceRange, Utf8LineIndex};

/// The kind of a POD section directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PodSectionKind {
    /// `=head1`
    Head1,
    /// `=head2`
    Head2,
    /// `=head3` / `=head4`
    Head3,
    /// `=item`
    Item,
    /// `=pod` / `=begin` / `=for` / other block start.
    Block,
}

/// One POD section directive with its title and span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodSection {
    /// The directive kind.
    pub kind: PodSectionKind,
    /// The title/text after the directive (e.g. `NAME`, a method name).
    pub title: String,
    /// Span of the directive line.
    pub range: SourceRange,
}

/// POD facts for one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodFact {
    /// The documented file.
    pub file_id: FileId,
    /// Module name / one-liner from `=head1 NAME`, if present.
    pub name: Option<String>,
    /// First paragraph of `=head1 DESCRIPTION`, if present.
    pub description: Option<String>,
    /// Names of methods documented via `=head2 <name>`, sorted.
    pub documented_methods: Vec<String>,
    /// Section directives with spans, in source order.
    pub sections: Vec<PodSection>,
    /// Confidence in the fact.
    pub confidence: Confidence,
}

/// Extract POD facts from a file's source, or `None` if it contains no POD.
#[must_use]
pub fn extract_pod_facts(
    file_id: &FileId,
    source: &str,
    line_index: &Utf8LineIndex,
) -> Option<PodFact> {
    let sections = scan_sections(source, line_index);
    let doc = perl_pod::extract_pod(source);

    if sections.is_empty() && doc.is_empty() {
        return None;
    }

    let mut documented_methods: Vec<String> = doc.methods.keys().cloned().collect();
    documented_methods.sort();

    Some(PodFact {
        file_id: file_id.clone(),
        name: doc.name,
        description: doc.description,
        documented_methods,
        sections,
        confidence: Confidence::High,
    })
}

/// Scan raw source for POD section directives, recording each directive line's
/// span. Byte offsets are tracked so ranges land on the real source, and POD
/// state is toggled by `=…`/`=cut` so `=item`-like text inside code is ignored.
fn scan_sections(source: &str, line_index: &Utf8LineIndex) -> Vec<PodSection> {
    let mut sections = Vec::new();
    let mut byte = 0u32;
    let mut in_pod = false;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        let line_len = u32::try_from(line.len()).unwrap_or(u32::MAX);
        let content_len = u32::try_from(trimmed.len()).unwrap_or(u32::MAX);

        if trimmed.starts_with('=') && !in_pod && !is_cut_directive(trimmed) {
            in_pod = true;
        }
        if in_pod {
            if is_cut_directive(trimmed) {
                in_pod = false;
            } else if let Some((kind, title)) = classify_directive(trimmed) {
                let range = line_index.source_range(byte, byte.saturating_add(content_len));
                sections.push(PodSection { kind, title, range });
            }
        }
        byte = byte.saturating_add(line_len);
    }
    sections
}

/// True when a line's directive token is exactly `=cut` — not merely
/// *prefixed* by it. A prefix match (`starts_with("=cut")`) would wrongly
/// treat an unrelated/unknown directive like `=cutlery` as closing the POD
/// block; real Perl parses the directive as the first whitespace-delimited
/// token, so only an exact `=cut` token ends the block.
fn is_cut_directive(line: &str) -> bool {
    line.split_whitespace().next() == Some("=cut")
}

/// Classify a POD directive line into a section kind + title.
fn classify_directive(line: &str) -> Option<(PodSectionKind, String)> {
    let (directive, rest) = match line.split_once(char::is_whitespace) {
        Some((d, r)) => (d, r.trim().to_string()),
        None => (line, String::new()),
    };
    let kind = match directive {
        "=head1" => PodSectionKind::Head1,
        "=head2" => PodSectionKind::Head2,
        "=head3" | "=head4" => PodSectionKind::Head3,
        "=item" => PodSectionKind::Item,
        "=pod" | "=begin" | "=for" | "=encoding" | "=over" => PodSectionKind::Block,
        _ => return None,
    };
    Some((kind, rest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Digest;

    fn facts(src: &str) -> Option<PodFact> {
        let idx = Utf8LineIndex::new(src);
        extract_pod_facts(&FileId::new("lib/App.pm", &Digest::of(src)), src, &idx)
    }

    #[test]
    fn extracts_name_description_and_sections() {
        let src = "package App;\n\n=head1 NAME\n\nApp - does things\n\n=head1 DESCRIPTION\n\nA longer description.\n\n=head2 run\n\nRuns it.\n\n=cut\n\nsub run { 1 }\n1;\n";
        let f = facts(src).unwrap();
        assert_eq!(f.name.as_deref(), Some("App - does things"));
        assert!(f.description.is_some());
        assert!(f.documented_methods.contains(&"run".to_string()), "method run documented");
        assert!(
            f.sections.iter().any(|s| s.kind == PodSectionKind::Head1 && s.title == "NAME"),
            "NAME head1 section with range; sections={:?}",
            f.sections
        );
    }

    #[test]
    fn ranges_track_real_lines() {
        let src = "package App;\n=head1 NAME\n=cut\n1;\n";
        let f = facts(src).unwrap();
        let name = f.sections.iter().find(|s| s.title == "NAME").unwrap();
        assert_eq!(name.range.start_line, 1, "=head1 NAME is on line 1 (0-based)");
    }

    #[test]
    fn no_pod_yields_none() {
        assert!(facts("package App;\nsub run { 1 }\n1;\n").is_none());
    }

    #[test]
    fn item_text_in_code_is_not_treated_as_pod() {
        // A line starting with `=` only enters POD mode via a real directive;
        // ordinary code never starts a line with `=item`, but guard anyway.
        let src = "my $x = 1;\nmy $y = 2;\n";
        assert!(facts(src).is_none());
    }

    #[test]
    fn cut_lookalike_directive_does_not_close_pod() {
        // Regression: `=cutlery` (or any directive merely *prefixed* by "cut")
        // must not be mistaken for the `=cut` terminator via a starts_with
        // prefix match. Perl parses the directive as the first
        // whitespace-delimited token, so only an exact `=cut` line closes POD.
        let src = "package App;\n\n=head1 NAME\n\nApp - does things\n\n=cutlery not a real directive\n\n=head2 run\n\nRuns it.\n\n=cut\n\nsub run { 1 }\n1;\n";
        let f = facts(src).unwrap();
        assert!(
            f.sections.iter().any(|s| s.kind == PodSectionKind::Head2 && s.title == "run"),
            "the =head2 after the =cutlery lookalike is still inside POD; sections={:?}",
            f.sections
        );
        assert!(
            f.documented_methods.contains(&"run".to_string()),
            "run is still documented despite the =cutlery lookalike line"
        );
    }

    #[test]
    fn is_cut_directive_matches_exact_token_only() {
        // Direct unit coverage of the token-boundary fix: `=cut` (with or
        // without trailing text — only the first token is the directive name)
        // matches; a lookalike directive or ordinary code does not.
        assert!(is_cut_directive("=cut"));
        assert!(is_cut_directive("=cut trailing junk is ignored"));
        assert!(!is_cut_directive("=cutlery not a real directive"));
        assert!(!is_cut_directive("=customs"));
        assert!(!is_cut_directive("code();"));
    }
}
