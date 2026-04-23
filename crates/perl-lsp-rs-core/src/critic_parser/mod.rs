//! Perl::Critic output parser.
//!
//! Parses the verbose output format from `perlcritic` into structured records.
//!
//! Previously the standalone `perl-lsp-critic-parser` crate; absorbed into
//! `perl-lsp-rs-core::critic_parser` in Wave G3 (#4535).
//!
//! # Line Format
//!
//! ```text
//! file:line:column:severity:policy:message
//! ```
//!
//! Where `policy` is a `::` separated Perl package name (e.g.
//! `Perl::Critic::Policy::ProhibitComplexMappings`).

/// A parsed Perl::Critic output line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCriticLine {
    /// Source file path.
    pub file: String,
    /// 1-indexed line number.
    pub line: u32,
    /// 1-indexed column number.
    pub column: u32,
    /// Numeric Perl::Critic severity.
    pub severity: u8,
    /// Perl::Critic policy identifier.
    pub policy: String,
    /// Human-readable violation message.
    pub message: String,
}

/// Parse all valid Perl::Critic lines from a UTF-8 string.
pub fn parse_perlcritic_output(output: &str) -> Vec<ParsedCriticLine> {
    output.lines().filter_map(parse_perlcritic_line).collect()
}

/// Parse one Perl::Critic verbose output line.
pub fn parse_perlcritic_line(line: &str) -> Option<ParsedCriticLine> {
    let line = line.trim_end_matches('\r');
    if line.trim().is_empty() {
        return None;
    }

    let parts: Vec<&str> = line.split(':').collect();

    let mut numeric_idx = None;
    let max_start = parts.len().saturating_sub(4);
    for idx in 1..=max_start {
        if parts.get(idx).and_then(|v| v.parse::<u32>().ok()).is_some()
            && parts.get(idx + 1).and_then(|v| v.parse::<u32>().ok()).is_some()
            && parts.get(idx + 2).and_then(|v| v.parse::<u8>().ok()).is_some()
        {
            numeric_idx = Some(idx);
            break;
        }
    }

    let start = numeric_idx?;
    let file = parts[..start].join(":");
    if file.is_empty() {
        return None;
    }

    let line_num = parts[start].parse::<u32>().ok()?;
    let column = parts[start + 1].parse::<u32>().ok()?;
    let severity = parts[start + 2].parse::<u8>().ok()?;
    if line_num == 0 || column == 0 || !(1..=5).contains(&severity) {
        return None;
    }

    let tail = parts[start + 3..].join(":");
    let boundary = find_policy_message_boundary(&tail)?;

    let policy = tail[..boundary].to_string();
    let message = tail[boundary + 1..].to_string();

    if policy.is_empty() || message.is_empty() {
        return None;
    }

    Some(ParsedCriticLine { file, line: line_num, column, severity, policy, message })
}

#[cfg(test)]
mod tests {
    use super::{parse_perlcritic_line, parse_perlcritic_output};

    #[test]
    fn parse_perlcritic_line_rejects_zero_line_and_column() {
        let zero_line = "lib/Foo.pm:0:2:3:TestingAndDebugging::RequireUseStrict:msg";
        assert!(parse_perlcritic_line(zero_line).is_none());

        let zero_col = "lib/Foo.pm:1:0:3:TestingAndDebugging::RequireUseStrict:msg";
        assert!(parse_perlcritic_line(zero_col).is_none());
    }

    #[test]
    fn parse_perlcritic_line_rejects_out_of_range_severity() {
        let too_low = "lib/Foo.pm:1:1:0:TestingAndDebugging::RequireUseStrict:msg";
        assert!(parse_perlcritic_line(too_low).is_none());

        let too_high = "lib/Foo.pm:1:1:7:TestingAndDebugging::RequireUseStrict:msg";
        assert!(parse_perlcritic_line(too_high).is_none());
    }

    #[test]
    fn parse_perlcritic_line_trims_crlf_line_endings() {
        let line = "lib/Foo.pm:1:1:5:TestingAndDebugging::RequireUseStrict:msg\r";
        let parsed = parse_perlcritic_line(line).expect("valid perlcritic line");
        assert_eq!(parsed.message, "msg");
    }

    #[test]
    fn parse_perlcritic_line_supports_windows_drive_paths() {
        let line = "C:\\project\\lib\\Foo.pm:10:4:2:TestingAndDebugging::RequireUseStrict:msg";
        let parsed = parse_perlcritic_line(line).expect("valid windows path");
        assert_eq!(parsed.file, "C:\\project\\lib\\Foo.pm");
        assert_eq!(parsed.line, 10);
        assert_eq!(parsed.column, 4);
    }

    #[test]
    fn parse_perlcritic_output_skips_invalid_lines() {
        let output = [
            "lib/Foo.pm:1:1:5:TestingAndDebugging::RequireUseStrict:msg",
            "lib/Foo.pm:0:1:5:TestingAndDebugging::RequireUseStrict:invalid",
            "",
        ]
        .join("\n");

        let parsed = parse_perlcritic_output(&output);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].line, 1);
    }
}

fn find_policy_message_boundary(tail: &str) -> Option<usize> {
    let bytes = tail.as_bytes();
    for (idx, byte) in bytes.iter().enumerate() {
        if *byte != b':' {
            continue;
        }

        let prev_is_colon = idx > 0 && bytes[idx - 1] == b':';
        let next_is_colon = idx + 1 < bytes.len() && bytes[idx + 1] == b':';
        if prev_is_colon || next_is_colon {
            continue;
        }

        let policy_candidate = &tail[..idx];
        if is_valid_policy(policy_candidate) {
            return Some(idx);
        }
    }

    None
}

fn is_valid_policy(policy: &str) -> bool {
    if policy.is_empty() {
        return false;
    }

    for segment in policy.split("::") {
        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !(first.is_ascii_alphabetic() || first == '_') {
            return false;
        }
        if chars.any(|c| !(c.is_ascii_alphanumeric() || c == '_')) {
            return false;
        }
    }

    true
}
