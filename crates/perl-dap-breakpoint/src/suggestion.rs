//! Nearest valid line suggestion
//!
//! This module provides functionality to find the nearest valid line
//! when a breakpoint is placed on an invalid location (comment, blank, etc.)

use crate::validator::{AstBreakpointValidator, BreakpointValidator};

/// Direction to search for a valid line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    /// Search forward (towards end of file)
    Forward,
    /// Search backward (towards start of file)
    Backward,
    /// Search in both directions, returning the nearest
    Both,
}

/// Find the nearest valid line to the given line number
///
/// # Arguments
///
/// * `validator` - The breakpoint validator to use
/// * `line` - The 1-based line number to start from
/// * `direction` - The direction to search
/// * `max_distance` - Maximum number of lines to search (None for unlimited)
///
/// # Returns
///
/// The nearest valid line number, or None if no valid line is found within the search range.
pub fn find_nearest_valid_line(
    validator: &AstBreakpointValidator,
    line: i64,
    direction: SearchDirection,
    max_distance: Option<usize>,
) -> Option<i64> {
    let max_dist = max_distance.unwrap_or(usize::MAX);

    match direction {
        SearchDirection::Forward => find_forward(validator, line, max_dist),
        SearchDirection::Backward => find_backward(validator, line, max_dist),
        SearchDirection::Both => {
            let forward = find_forward(validator, line, max_dist);
            let backward = find_backward(validator, line, max_dist);

            match (forward, backward) {
                (Some(f), Some(b)) => {
                    let f_dist = (f - line).unsigned_abs();
                    let b_dist = (line - b).unsigned_abs();
                    if f_dist <= b_dist { Some(f) } else { Some(b) }
                }
                (Some(f), None) => Some(f),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            }
        }
    }
}

fn find_forward(
    validator: &AstBreakpointValidator,
    start_line: i64,
    max_distance: usize,
) -> Option<i64> {
    for offset in 1..=max_distance {
        let line = start_line + offset as i64;
        if validator.is_executable_line(line) {
            return Some(line);
        }
        // Stop if we've gone past end of file
        let result = validator.validate(line);
        if result.reason == Some(crate::validator::ValidationReason::LineOutOfRange) {
            break;
        }
    }
    None
}

fn find_backward(
    validator: &AstBreakpointValidator,
    start_line: i64,
    max_distance: usize,
) -> Option<i64> {
    for offset in 1..=max_distance {
        let line = start_line - offset as i64;
        if line < 1 {
            break;
        }
        if validator.is_executable_line(line) {
            return Some(line);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must;

    #[test]
    fn test_find_nearest_forward() {
        let source = "# comment\n# comment\nmy $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = find_nearest_valid_line(&validator, 1, SearchDirection::Forward, None);
        assert_eq!(result, Some(3));
    }

    #[test]
    fn test_find_nearest_backward() {
        let source = "my $x = 1;\n# comment\n# comment\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = find_nearest_valid_line(&validator, 3, SearchDirection::Backward, None);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_find_nearest_both_prefers_closer() {
        let source = "my $x = 1;\n# comment\n# comment\n# comment\nmy $y = 2;\n";
        let validator = must(AstBreakpointValidator::new(source));

        // From line 2 (comment), line 1 is closer than line 5
        let result = find_nearest_valid_line(&validator, 2, SearchDirection::Both, None);
        assert_eq!(result, Some(1));

        // From line 4 (comment), line 5 is closer than line 1
        let result = find_nearest_valid_line(&validator, 4, SearchDirection::Both, None);
        assert_eq!(result, Some(5));
    }

    #[test]
    fn test_find_nearest_with_max_distance() {
        let source = "# comment\n# comment\n# comment\nmy $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        // With max distance 2, can't reach line 4
        let result = find_nearest_valid_line(&validator, 1, SearchDirection::Forward, Some(2));
        assert_eq!(result, None);

        // With max distance 3, can reach line 4
        let result = find_nearest_valid_line(&validator, 1, SearchDirection::Forward, Some(3));
        assert_eq!(result, Some(4));
    }

    #[test]
    fn test_find_nearest_no_valid_lines() {
        let source = "# all comments\n# more comments\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = find_nearest_valid_line(&validator, 1, SearchDirection::Both, None);
        assert_eq!(result, None);
    }
}
