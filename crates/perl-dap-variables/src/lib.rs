//! Variable rendering for Perl DAP
//!
//! This crate provides types and utilities for rendering Perl variables
//! in the Debug Adapter Protocol (DAP) format, enabling debugging support
//! in VSCode and other DAP-compatible editors.
//!
//! # Overview
//!
//! The crate provides:
//!
//! - [`PerlValue`] - Represents Perl values (scalars, arrays, hashes, references)
//! - [`RenderedVariable`] - DAP-compatible variable representation with lazy expansion
//! - [`VariableRenderer`] - Trait for custom variable rendering strategies
//! - [`PerlVariableRenderer`] - Default implementation for Perl variables
//!
//! # Example
//!
//! ```rust
//! use perl_dap_variables::{PerlValue, RenderedVariable, PerlVariableRenderer, VariableRenderer};
//!
//! let renderer = PerlVariableRenderer::new();
//! let value = PerlValue::Scalar("hello".to_string());
//! let rendered = renderer.render("$greeting", &value);
//!
//! assert_eq!(rendered.name, "$greeting");
//! assert_eq!(rendered.value, "\"hello\"");
//! ```

mod parser;
mod renderer;

pub use parser::{VariableParseError, VariableParser};
pub use perl_dap_value::PerlValue;
pub use renderer::{PerlVariableRenderer, RenderedVariable, VariableRenderer};
