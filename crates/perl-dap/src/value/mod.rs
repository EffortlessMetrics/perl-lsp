//! Shared Perl value model for DAP parser and renderer crates.

#![deny(unsafe_code)]

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
    #[must_use]
    pub fn is_expandable(&self) -> bool {
        matches!(
            self,
            Self::Array(_)
                | Self::Hash(_)
                | Self::Reference(_)
                | Self::Object { .. }
                | Self::Tied { .. }
        )
    }

    /// Returns the type name for this value.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Undef => "undef",
            Self::Scalar(_) | Self::Number(_) | Self::Integer(_) => "SCALAR",
            Self::Array(_) => "ARRAY",
            Self::Hash(_) => "HASH",
            Self::Reference(_) => "REF",
            Self::Object { .. } => "OBJECT",
            Self::Code { .. } => "CODE",
            Self::Glob(_) => "GLOB",
            Self::Regex(_) => "Regexp",
            Self::Tied { .. } => "TIED",
            Self::Truncated { .. } => "...",
            Self::Error(_) => "ERROR",
        }
    }

    /// Returns the number of child elements if applicable.
    #[must_use]
    pub fn child_count(&self) -> Option<usize> {
        match self {
            Self::Array(elements) => Some(elements.len()),
            Self::Hash(pairs) => Some(pairs.len()),
            Self::Truncated { total_count, .. } => *total_count,
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
        Self::Object {
            class: class.into(),
            value: Box::new(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PerlValue;

    #[test]
    fn perl_value_is_expandable() {
        assert!(!PerlValue::Undef.is_expandable());
        assert!(!PerlValue::Scalar("test".to_string()).is_expandable());
        assert!(PerlValue::Array(vec![]).is_expandable());
        assert!(PerlValue::Hash(vec![]).is_expandable());
        assert!(PerlValue::Reference(Box::new(PerlValue::Undef)).is_expandable());
    }

    #[test]
    fn perl_value_type_name() {
        assert_eq!(PerlValue::Undef.type_name(), "undef");
        assert_eq!(PerlValue::Scalar("test".to_string()).type_name(), "SCALAR");
        assert_eq!(PerlValue::Array(vec![]).type_name(), "ARRAY");
        assert_eq!(PerlValue::Hash(vec![]).type_name(), "HASH");
    }

    #[test]
    fn perl_value_child_count() {
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
    fn perl_value_constructors() {
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

    #[test]
    fn is_expandable_all_variants() {
        // Non-expandable
        assert!(!PerlValue::Undef.is_expandable());
        assert!(!PerlValue::Scalar("test".into()).is_expandable());
        assert!(!PerlValue::Number(3.125).is_expandable());
        assert!(!PerlValue::Integer(42).is_expandable());
        assert!(!PerlValue::Code { name: None }.is_expandable());
        assert!(
            !PerlValue::Code {
                name: Some("foo".into())
            }
            .is_expandable()
        );
        assert!(!PerlValue::Glob("*main::STDOUT".into()).is_expandable());
        assert!(!PerlValue::Regex("^foo$".into()).is_expandable());
        assert!(
            !PerlValue::Truncated {
                summary: "...".into(),
                total_count: Some(100)
            }
            .is_expandable()
        );
        assert!(!PerlValue::Error("oops".into()).is_expandable());

        // Expandable
        assert!(PerlValue::Array(vec![]).is_expandable());
        assert!(PerlValue::Hash(vec![]).is_expandable());
        assert!(PerlValue::Reference(Box::new(PerlValue::Undef)).is_expandable());
        assert!(
            PerlValue::Object {
                class: "Foo".into(),
                value: Box::new(PerlValue::Hash(vec![]))
            }
            .is_expandable()
        );
        assert!(
            PerlValue::Tied {
                class: "Tie::Hash".into(),
                value: None
            }
            .is_expandable()
        );
    }

    #[test]
    fn type_name_all_variants() {
        assert_eq!(PerlValue::Undef.type_name(), "undef");
        assert_eq!(PerlValue::Scalar("s".into()).type_name(), "SCALAR");
        assert_eq!(PerlValue::Number(1.0).type_name(), "SCALAR");
        assert_eq!(PerlValue::Integer(1).type_name(), "SCALAR");
        assert_eq!(PerlValue::Array(vec![]).type_name(), "ARRAY");
        assert_eq!(PerlValue::Hash(vec![]).type_name(), "HASH");
        assert_eq!(
            PerlValue::Reference(Box::new(PerlValue::Undef)).type_name(),
            "REF"
        );
        assert_eq!(
            PerlValue::Object {
                class: "Foo".into(),
                value: Box::new(PerlValue::Undef)
            }
            .type_name(),
            "OBJECT"
        );
        assert_eq!(PerlValue::Code { name: None }.type_name(), "CODE");
        assert_eq!(PerlValue::Glob("g".into()).type_name(), "GLOB");
        assert_eq!(PerlValue::Regex("r".into()).type_name(), "Regexp");
        assert_eq!(
            PerlValue::Tied {
                class: "T".into(),
                value: None
            }
            .type_name(),
            "TIED"
        );
        assert_eq!(
            PerlValue::Truncated {
                summary: "s".into(),
                total_count: None
            }
            .type_name(),
            "..."
        );
        assert_eq!(PerlValue::Error("e".into()).type_name(), "ERROR");
    }

    #[test]
    fn child_count_all_variants() {
        assert_eq!(PerlValue::Undef.child_count(), None);
        assert_eq!(PerlValue::Scalar("s".into()).child_count(), None);
        assert_eq!(PerlValue::Number(1.0).child_count(), None);
        assert_eq!(PerlValue::Integer(1).child_count(), None);
        assert_eq!(
            PerlValue::Array(vec![
                PerlValue::Integer(1),
                PerlValue::Integer(2),
                PerlValue::Integer(3)
            ])
            .child_count(),
            Some(3)
        );
        assert_eq!(PerlValue::Array(vec![]).child_count(), Some(0));
        assert_eq!(
            PerlValue::Hash(vec![
                ("a".into(), PerlValue::Undef),
                ("b".into(), PerlValue::Undef)
            ])
            .child_count(),
            Some(2)
        );
        assert_eq!(PerlValue::Hash(vec![]).child_count(), Some(0));
        assert_eq!(
            PerlValue::Reference(Box::new(PerlValue::Undef)).child_count(),
            None
        );
        assert_eq!(PerlValue::Code { name: None }.child_count(), None);
        assert_eq!(
            PerlValue::Truncated {
                summary: "big".into(),
                total_count: Some(500)
            }
            .child_count(),
            Some(500)
        );
        assert_eq!(
            PerlValue::Truncated {
                summary: "big".into(),
                total_count: None
            }
            .child_count(),
            None
        );
    }

    #[test]
    fn default_is_undef() {
        assert_eq!(PerlValue::default(), PerlValue::Undef);
    }

    #[test]
    fn nested_value_structure() {
        let nested = PerlValue::object(
            "My::Class",
            PerlValue::hash(vec![
                ("name".into(), PerlValue::scalar("Alice")),
                (
                    "scores".into(),
                    PerlValue::array(vec![PerlValue::Integer(95), PerlValue::Integer(87)]),
                ),
            ]),
        );
        assert!(nested.is_expandable());
        assert_eq!(nested.type_name(), "OBJECT");
        assert_eq!(nested.child_count(), None); // Objects don't have direct child_count
    }

    #[test]
    fn clone_and_equality() {
        let original = PerlValue::array(vec![PerlValue::scalar("hello"), PerlValue::Integer(42)]);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}
