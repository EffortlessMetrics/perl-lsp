//! Boundary-aware standalone Perl module-token scanning.
//!
//! This crate has one responsibility: find standalone occurrences of a module
//! token on a single source line while respecting canonical (`::`) and legacy
//! (`'`) package separator boundaries.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_module_token_core::has_standalone_module_token_boundaries;

/// Byte range for a standalone module-token match in a source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleTokenRange {
    /// Inclusive byte start offset in the scanned line.
    pub start: usize,
    /// Exclusive byte end offset in the scanned line.
    pub end: usize,
}

/// Iterator over standalone `module_name` matches in `line`.
#[derive(Debug, Clone)]
pub struct ModuleTokenRangeIter<'a> {
    line: &'a str,
    module_name: &'a str,
    search_start: usize,
    done: bool,
}

impl<'a> Iterator for ModuleTokenRangeIter<'a> {
    type Item = ModuleTokenRange;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.module_name.is_empty() || self.line.is_empty() {
            self.done = true;
            return None;
        }

        while self.search_start <= self.line.len() {
            let Some(rel_pos) = self.line[self.search_start..].find(self.module_name) else {
                break;
            };

            let start = self.search_start + rel_pos;
            let end = start + self.module_name.len();
            self.search_start = end;

            if has_standalone_module_token_boundaries(self.line, start, end) {
                return Some(ModuleTokenRange { start, end });
            }
        }

        self.done = true;
        None
    }
}

/// Return an iterator of standalone `module_name` matches in `line`.
///
/// Matches are returned in byte-order and are non-overlapping.
#[must_use]
pub fn find_standalone_module_token_ranges<'a>(
    line: &'a str,
    module_name: &'a str,
) -> ModuleTokenRangeIter<'a> {
    ModuleTokenRangeIter { line, module_name, search_start: 0, done: false }
}

/// Return `true` when `line` contains `module_name` as a standalone module
/// token.
#[must_use]
pub fn contains_standalone_module_token(line: &str, module_name: &str) -> bool {
    find_standalone_module_token_ranges(line, module_name).next().is_some()
}

#[cfg(test)]
mod tests {
    use super::{contains_standalone_module_token, find_standalone_module_token_ranges};

    #[test]
    fn finds_standalone_canonical_module_token() {
        let ranges =
            find_standalone_module_token_ranges("use Foo::Bar;", "Foo::Bar").collect::<Vec<_>>();

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, 4);
        assert_eq!(ranges[0].end, 12);
        assert!(contains_standalone_module_token("use Foo::Bar;", "Foo::Bar"));
    }

    #[test]
    fn rejects_partial_module_name_matches() {
        let ranges = find_standalone_module_token_ranges("use Foo::Barista;", "Foo::Bar")
            .collect::<Vec<_>>();
        assert!(ranges.is_empty());
        assert!(!contains_standalone_module_token("use Foo::Barista;", "Foo::Bar"));
    }

    #[test]
    fn treats_legacy_separator_as_module_boundary() {
        let ranges =
            find_standalone_module_token_ranges("use Foo'Bar'Baz;", "Foo'Bar").collect::<Vec<_>>();
        assert!(ranges.is_empty());
        assert!(!contains_standalone_module_token("use Foo'Bar'Baz;", "Foo'Bar"));
    }

    #[test]
    fn empty_inputs_never_match() {
        assert!(!contains_standalone_module_token("", "Foo::Bar"));
        assert!(!contains_standalone_module_token("use Foo::Bar;", ""));
        assert!(
            find_standalone_module_token_ranges("use Foo::Bar;", "").collect::<Vec<_>>().is_empty()
        );
    }
}
