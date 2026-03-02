//! Sort-and-dedup helpers shared across Perl LSP crates.
//!
//! This microcrate keeps de-duplication policy in one place while allowing each
//! caller to define domain-specific sort and equality behavior.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::cmp::Ordering;

/// Sort items with `sort_by`, then remove duplicates based on `is_equal`.
///
/// Callers define what ordering and equality mean for their domain type.
pub fn sort_and_dedup_by<T, FSort, FEqual>(
    items: &mut Vec<T>,
    mut sort_by: FSort,
    mut is_equal: FEqual,
) where
    FSort: FnMut(&T, &T) -> Ordering,
    FEqual: FnMut(&T, &T) -> bool,
{
    items.sort_by(|left, right| sort_by(left, right));
    items.dedup_by(|left, right| is_equal(left, right));
}

#[cfg(test)]
mod tests {
    use super::sort_and_dedup_by;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestDiagnostic {
        range: (usize, usize),
        severity: u8,
        code: Option<String>,
        message: String,
    }

    #[test]
    fn removes_exact_duplicates_after_sorting() {
        let mut diagnostics = vec![
            TestDiagnostic {
                range: (10, 11),
                severity: 1,
                code: Some("parse-error".to_string()),
                message: "Issue A".to_string(),
            },
            TestDiagnostic {
                range: (3, 4),
                severity: 1,
                code: Some("parse-error".to_string()),
                message: "Issue B".to_string(),
            },
            TestDiagnostic {
                range: (10, 11),
                severity: 1,
                code: Some("parse-error".to_string()),
                message: "Issue A".to_string(),
            },
        ];

        sort_and_dedup_by(
            &mut diagnostics,
            |a, b| {
                a.range
                    .0
                    .cmp(&b.range.0)
                    .then(a.range.1.cmp(&b.range.1))
                    .then(a.severity.cmp(&b.severity))
                    .then(a.code.cmp(&b.code))
                    .then(a.message.cmp(&b.message))
            },
            |a, b| {
                a.range == b.range
                    && a.severity == b.severity
                    && a.code == b.code
                    && a.message == b.message
            },
        );

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].range, (3, 4));
        assert_eq!(diagnostics[1].range, (10, 11));
    }
}
