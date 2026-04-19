//! Boundary-aware standalone Perl module-token scanning.
//!
//! Find standalone occurrences of a module token on a single source line
//! while respecting canonical (`::`) and legacy (`'`) package separator
//! boundaries.

use crate::token_core::has_standalone_module_token_boundaries;

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
#[must_use]
pub fn find_standalone_module_token_ranges<'a>(
    line: &'a str,
    module_name: &'a str,
) -> ModuleTokenRangeIter<'a> {
    ModuleTokenRangeIter {
        line,
        module_name,
        search_start: 0,
        done: false,
    }
}

/// Return `true` when `line` contains `module_name` as a standalone module token.
#[must_use]
pub fn contains_standalone_module_token(line: &str, module_name: &str) -> bool {
    find_standalone_module_token_ranges(line, module_name)
        .next()
        .is_some()
}
