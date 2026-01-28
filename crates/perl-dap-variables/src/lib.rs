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
pub use renderer::{PerlVariableRenderer, RenderedVariable, VariableRenderer};

use serde::{Deserialize, Serialize};

/// Represents a Perl value in the debugger context.
///
/// This enum models the different types of values that can be inspected
/// during a Perl debugging session.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum PerlValue {
    /// Undefined value (Perl's `undef`)
    #[default]
    Undef,

    /// Scalar value (string representation)
    Scalar(String),

    /// Numeric scalar value
    Number(f64),

    /// Integer scalar value
    Integer(i64),

    /// Array value with elements
    Array(Vec<PerlValue>),

    /// Hash value with key-value pairs
    Hash(Vec<(String, PerlValue)>),

    /// Reference to another value
    Reference(Box<PerlValue>),

    /// Blessed reference (object)
    Object {
        /// The package/class name
        class: String,
        /// The underlying value
        value: Box<PerlValue>,
    },

    /// Code reference (subroutine)
    Code {
        /// Optional name if it's a named subroutine
        name: Option<String>,
    },

    /// Glob (typeglob)
    Glob(String),

    /// Regular expression (compiled pattern)
    Regex(String),

    /// Tied variable (magic)
    Tied {
        /// The tie class
        class: String,
        /// The underlying value if available
        value: Option<Box<PerlValue>>,
    },

    /// Truncated value (for large data structures)
    Truncated {
        /// Brief description of the truncated value
        summary: String,
        /// Total count of elements if applicable
        total_count: Option<usize>,
    },

    /// Error during value inspection
    Error(String),
}

impl PerlValue {
    /// Returns true if this value can be expanded (has children).
    ///
    /// Arrays, hashes, references, and objects can all be expanded
    /// to reveal their contents.
    #[must_use]
    pub fn is_expandable(&self) -> bool {
        matches!(
            self,
            PerlValue::Array(_)
                | PerlValue::Hash(_)
                | PerlValue::Reference(_)
                | PerlValue::Object { .. }
                | PerlValue::Tied { .. }
        )
    }

    /// Returns the type name for this value.
    ///
    /// This is used for display in the DAP variables view.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            PerlValue::Undef => "undef",
            PerlValue::Scalar(_) => "SCALAR",
            PerlValue::Number(_) => "SCALAR",
            PerlValue::Integer(_) => "SCALAR",
            PerlValue::Array(_) => "ARRAY",
            PerlValue::Hash(_) => "HASH",
            PerlValue::Reference(_) => "REF",
            PerlValue::Object { .. } => "OBJECT",
            PerlValue::Code { .. } => "CODE",
            PerlValue::Glob(_) => "GLOB",
            PerlValue::Regex(_) => "Regexp",
            PerlValue::Tied { .. } => "TIED",
            PerlValue::Truncated { .. } => "...",
            PerlValue::Error(_) => "ERROR",
        }
    }

    /// Returns the number of child elements if applicable.
    ///
    /// For arrays, returns the element count.
    /// For hashes, returns the key count.
    /// For other types, returns None.
    #[must_use]
    pub fn child_count(&self) -> Option<usize> {
        match self {
            PerlValue::Array(elements) => Some(elements.len()),
            PerlValue::Hash(pairs) => Some(pairs.len()),
            PerlValue::Truncated { total_count, .. } => *total_count,
            _ => None,
        }
    }

    /// Creates a scalar value from a string.
    #[must_use]
    pub fn scalar(s: impl Into<String>) -> Self {
        PerlValue::Scalar(s.into())
    }

    /// Creates an array value from elements.
    #[must_use]
    pub fn array(elements: Vec<PerlValue>) -> Self {
        PerlValue::Array(elements)
    }

    /// Creates a hash value from key-value pairs.
    #[must_use]
    pub fn hash(pairs: Vec<(String, PerlValue)>) -> Self {
        PerlValue::Hash(pairs)
    }

    /// Creates a reference to another value.
    #[must_use]
    pub fn reference(value: PerlValue) -> Self {
        PerlValue::Reference(Box::new(value))
    }

    /// Creates an object (blessed reference).
    #[must_use]
    pub fn object(class: impl Into<String>, value: PerlValue) -> Self {
        PerlValue::Object { class: class.into(), value: Box::new(value) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perl_value_is_expandable() {
        assert!(!PerlValue::Undef.is_expandable());
        assert!(!PerlValue::Scalar("test".to_string()).is_expandable());
        assert!(PerlValue::Array(vec![]).is_expandable());
        assert!(PerlValue::Hash(vec![]).is_expandable());
        assert!(PerlValue::Reference(Box::new(PerlValue::Undef)).is_expandable());
    }

    #[test]
    fn test_perl_value_type_name() {
        assert_eq!(PerlValue::Undef.type_name(), "undef");
        assert_eq!(PerlValue::Scalar("test".to_string()).type_name(), "SCALAR");
        assert_eq!(PerlValue::Array(vec![]).type_name(), "ARRAY");
        assert_eq!(PerlValue::Hash(vec![]).type_name(), "HASH");
    }

    #[test]
    fn test_perl_value_child_count() {
        assert_eq!(PerlValue::Undef.child_count(), None);
        assert_eq!(
            PerlValue::Array(vec![PerlValue::Undef, PerlValue::Undef]).child_count(),
            Some(2)
        );
        assert_eq!(
            PerlValue::Hash(vec![("key".to_string(), PerlValue::Undef)]).child_count(),
            Some(1)
        );
    }

    #[test]
    fn test_perl_value_constructors() {
        let scalar = PerlValue::scalar("hello");
        assert!(matches!(scalar, PerlValue::Scalar(s) if s == "hello"));

        let array = PerlValue::array(vec![PerlValue::Integer(1), PerlValue::Integer(2)]);
        assert!(matches!(array, PerlValue::Array(a) if a.len() == 2));

        let hash = PerlValue::hash(vec![("key".to_string(), PerlValue::scalar("value"))]);
        assert!(matches!(hash, PerlValue::Hash(h) if h.len() == 1));

        let reference = PerlValue::reference(PerlValue::Integer(42));
        assert!(matches!(reference, PerlValue::Reference(_)));

        let object = PerlValue::object("MyClass", PerlValue::Hash(vec![]));
        assert!(matches!(object, PerlValue::Object { class, .. } if class == "MyClass"));
    }
}
