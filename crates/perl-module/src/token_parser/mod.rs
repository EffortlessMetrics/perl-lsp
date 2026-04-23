//! Single-line Perl module token parsing.
//!
//! Parse Perl package-style tokens from byte offsets in a source line.
//! Supports canonical (`::`) and legacy (`'`) package separators.

pub use crate::token_core::{ModuleTokenSpan, parse_module_token};
