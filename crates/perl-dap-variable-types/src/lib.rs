//! Core value types for Perl DAP variable parsing and rendering.

use serde::{Deserialize, Serialize};

/// Represents a Perl value in the debugger context.
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
        /// The package/class name.
        class: String,
        /// The underlying value.
        value: Box<PerlValue>,
    },
    /// Code reference (subroutine)
    Code {
        /// Optional name if it's a named subroutine.
        name: Option<String>,
    },
    /// Glob (typeglob)
    Glob(String),
    /// Regular expression (compiled pattern)
    Regex(String),
    /// Tied variable (magic)
    Tied {
        /// The tie class.
        class: String,
        /// The underlying value if available.
        value: Option<Box<PerlValue>>,
    },
    /// Truncated value (for large data structures)
    Truncated {
        /// Brief description of the truncated value.
        summary: String,
        /// Total count of elements if applicable.
        total_count: Option<usize>,
    },
    /// Error during value inspection
    Error(String),
}

impl PerlValue {
    /// Returns true if this value can be expanded (has children).
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
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            PerlValue::Undef => "undef",
            PerlValue::Scalar(_) | PerlValue::Number(_) | PerlValue::Integer(_) => "SCALAR",
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
        Self::Scalar(s.into())
    }

    /// Creates an array value from elements.
    #[must_use]
    pub fn array(elements: Vec<PerlValue>) -> Self {
        Self::Array(elements)
    }

    /// Creates a hash value from key-value pairs.
    #[must_use]
    pub fn hash(pairs: Vec<(String, PerlValue)>) -> Self {
        Self::Hash(pairs)
    }

    /// Creates a reference to another value.
    #[must_use]
    pub fn reference(value: PerlValue) -> Self {
        Self::Reference(Box::new(value))
    }

    /// Creates an object (blessed reference).
    #[must_use]
    pub fn object(class: impl Into<String>, value: PerlValue) -> Self {
        Self::Object { class: class.into(), value: Box::new(value) }
    }
}

#[cfg(test)]
mod tests {
    use super::PerlValue;

    #[test]
    fn perl_value_helpers_work() {
        assert!(!PerlValue::Undef.is_expandable());
        assert_eq!(PerlValue::Integer(1).type_name(), "SCALAR");
        assert_eq!(PerlValue::array(vec![PerlValue::Undef]).child_count(), Some(1));
    }
}
