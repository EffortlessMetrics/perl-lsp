//! Parse Perl::Critic verbose output into structured records.
//!
//! Expected line format:
//! `file:line:column:severity:policy:message`

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

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
    if line.trim().is_empty() {
        return None;
    }

    let parts: Vec<&str> = line.split(':').collect();

    let mut numeric_idx = None;
    let max_start = parts.len().saturating_sub(4);
    for idx in 1..=max_start {
        if parts.get(idx).and_then(|v| v.parse::<u32>().ok()).is_some()
            && parts
                .get(idx + 1)
                .and_then(|v| v.parse::<u32>().ok())
                .is_some()
            && parts
                .get(idx + 2)
                .and_then(|v| v.parse::<u8>().ok())
                .is_some()
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

    let tail = parts[start + 3..].join(":");
    let boundary = find_policy_message_boundary(&tail)?;

    let policy = tail[..boundary].to_string();
    let message = tail[boundary + 1..].to_string();

    if policy.is_empty() || message.is_empty() {
        return None;
    }

    Some(ParsedCriticLine {
        file,
        line: line_num,
        column,
        severity,
        policy,
        message,
    })
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
