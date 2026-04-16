//! Format string arity validation for printf/sprintf calls (PL405)
//!
//! Detects mismatches between the number of printf-style format specifiers
//! in the first string-literal argument and the number of remaining arguments.
//!
//! # Diagnostic codes
//!
//! | Code  | Severity | Description                                      |
//! |-------|----------|--------------------------------------------------|
//! | PL405 | Warning  | printf/sprintf specifier count != argument count |
//!
//! # Scope
//!
//! Only fires when the format argument is a static string literal. Skips:
//! - Variable format strings (e.g. `printf $fmt, @args`)
//! - `%*d` / `%.*f` width/precision-from-args (counted as one specifier, may
//!   produce false negatives but never false positives)
//! - `%1$s` positional specifiers (skip entire call if detected)
//!
//! Both `NodeKind::FunctionCall` and `NodeKind::IndirectCall` are handled.
//! The parser disambiguates: `printf FILEHANDLE FORMAT, LIST` becomes
//! `IndirectCall { method, object, args }` with format at `args[0]`.

use perl_diagnostics::codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};

use super::super::walker::walk_node;
use crate::internal_types::{Diagnostic, RelatedInformation};
use perl_diagnostics::codes::DiagnosticSeverity;

/// Check for printf/sprintf format specifier count mismatches (PL405).
pub fn check_printf_format(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    walk_node(node, &mut |n| match &n.kind {
        NodeKind::FunctionCall { name, args } if name == "printf" || name == "sprintf" => {
            check_format_args(name, args, n, diagnostics);
        }
        NodeKind::IndirectCall { method, args, .. } if method == "printf" => {
            // For IndirectCall the filehandle is `object`; `args` starts at FORMAT.
            check_format_args(method, args, n, diagnostics);
        }
        _ => {}
    });
}

fn check_format_args(name: &str, args: &[Node], node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    let Some(fmt_node) = args.first() else { return };

    // Only validate static string literals — skip variable/expression formats
    let raw_value = match &fmt_node.kind {
        NodeKind::String { value, .. } => value.as_str(),
        _ => return,
    };

    // Strip enclosing quotes (value stores raw token text including delimiters)
    let fmt_content = unquote_string(raw_value);

    // Skip if positional specifiers present (%1$s etc.) — arg counting breaks
    if fmt_content.contains('$') {
        return;
    }

    let specifier_count = count_format_specifiers(fmt_content);
    let arg_count = args.len().saturating_sub(1); // exclude the format arg itself

    if specifier_count != arg_count {
        let msg = format!(
            "`{}` format string has {} specifier{} but {} argument{} supplied",
            name,
            specifier_count,
            if specifier_count == 1 { "" } else { "s" },
            arg_count,
            if arg_count == 1 { "" } else { "s" },
        );
        diagnostics.push(Diagnostic {
            range: (node.location.start, node.location.end),
            severity: DiagnosticSeverity::Warning,
            code: Some(DiagnosticCode::PrintfFormatMismatch.as_str().to_string()),
            message: msg,
            related_information: vec![RelatedInformation {
                location: (fmt_node.location.start, fmt_node.location.end),
                message: format!(
                    "Format string contains {} specifier{}",
                    specifier_count,
                    if specifier_count == 1 { "" } else { "s" }
                ),
            }],
            tags: Vec::new(),
            suggestion: Some(format!(
                "Add {} argument{} to match {} format specifier{}, or adjust the format string",
                specifier_count,
                if specifier_count == 1 { "" } else { "s" },
                specifier_count,
                if specifier_count == 1 { "" } else { "s" },
            )),
        });
    }
}

/// Count argument-consuming format specifiers in a format string.
///
/// - `%%` is a literal percent; does NOT consume an argument — skipped.
/// - `%[flags][width][.precision]specifier` consumes one argument.
/// - `%*d` / `%.*f` (width/precision from arg via `*`) are counted as one
///   specifier each; may produce false negatives but not false positives.
fn count_format_specifiers(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }
        i += 1; // skip '%'
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'%' {
            // '%%' — literal percent, no argument consumed
            i += 1;
            continue;
        }
        // Skip optional flags: - + space 0 #
        while i < bytes.len() && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
            i += 1;
        }
        // Skip optional width (digits or *)
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'*' {
            i += 1;
        }
        // Skip optional precision (.digits or .*)
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'*' {
                i += 1;
            }
        }
        // Skip optional size modifier (h hh l ll L q v z t)
        if i < bytes.len() {
            match bytes[i] {
                b'h' | b'l' | b'L' | b'q' | b'v' | b'z' | b't' => {
                    i += 1;
                    if i < bytes.len() && (bytes[i] == b'h' || bytes[i] == b'l') {
                        i += 1; // hh or ll
                    }
                }
                _ => {}
            }
        }
        // Consume the conversion specifier character
        if i < bytes.len()
            && matches!(
                bytes[i],
                b's' | b'd'
                    | b'i'
                    | b'u'
                    | b'o'
                    | b'x'
                    | b'X'
                    | b'e'
                    | b'E'
                    | b'f'
                    | b'F'
                    | b'g'
                    | b'G'
                    | b'c'
                    | b'p'
                    | b'n'
                    | b'b'
            )
        {
            count += 1;
        }
        i += 1;
    }
    count
}

/// Strip enclosing quotes from a raw string token value.
///
/// `NodeKind::String { value }` stores the raw lexer token including delimiters.
/// Returns the content between the outermost quote pair, or the original string
/// if no recognized quote pair is found.
fn unquote_string(raw: &str) -> &str {
    if raw.len() >= 2 {
        let bytes = raw.as_bytes();
        let first = bytes[0];
        let last = bytes[raw.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &raw[1..raw.len() - 1];
        }
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_basic_specifiers() {
        assert_eq!(count_format_specifiers("%s %d"), 2);
        assert_eq!(count_format_specifiers("%s"), 1);
        assert_eq!(count_format_specifiers("no specifiers"), 0);
    }

    #[test]
    fn double_percent_not_counted() {
        assert_eq!(count_format_specifiers("%%"), 0);
        assert_eq!(count_format_specifiers("%s %%"), 1);
        assert_eq!(count_format_specifiers("%% %d"), 1);
    }

    #[test]
    fn flags_and_width_handled() {
        assert_eq!(count_format_specifiers("%-10s"), 1);
        assert_eq!(count_format_specifiers("%+.2f"), 1);
        assert_eq!(count_format_specifiers("%05d"), 1);
    }

    #[test]
    fn unquote_double_quotes() {
        assert_eq!(unquote_string(r#""hello""#), "hello");
    }

    #[test]
    fn unquote_single_quotes() {
        assert_eq!(unquote_string("'world'"), "world");
    }

    #[test]
    fn unquote_no_quotes_unchanged() {
        assert_eq!(unquote_string("bare"), "bare");
    }
}
