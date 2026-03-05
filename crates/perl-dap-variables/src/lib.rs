//! Variable rendering for Perl DAP.
//!
//! This crate now focuses on rendering concerns and re-exports shared parsing
//! and type crates for compatibility.

mod renderer;

pub use perl_dap_variable_parser::{VariableParseError, VariableParser};
pub use perl_dap_variable_types::PerlValue;
pub use renderer::{PerlVariableRenderer, RenderedVariable, VariableRenderer};
