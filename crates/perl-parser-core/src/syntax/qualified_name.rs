//! Focused helpers for Perl qualified-name parsing and validation (previously `perl-qualified-name`).
//!
//! Splits canonical Perl package-qualified names (`Foo::Bar`) and validates
//! each identifier segment with Unicode-safe rules.

use core::fmt;

/// A validated parse failure for Perl-qualified names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualifiedNameError {
    /// The name is empty and cannot be interpreted.
    EmptyName,
    /// The name begins with a Perl sigil and is therefore not a package-qualified name.
    LeadingSigil(char),
    /// A `::` separator produced an empty segment.
    EmptySegment {
        /// Zero-based segment index where an empty segment was encountered.
        index: usize,
    },
    /// A segment did not satisfy the Perl identifier rules.
    InvalidSegment {
        /// Zero-based segment index that failed identifier validation.
        index: usize,
    },
}

impl fmt::Display for QualifiedNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "name is empty"),
            Self::LeadingSigil(sigil) => {
                write!(f, "qualified name cannot start with sigil '{sigil}'")
            }
            Self::EmptySegment { index } => {
                write!(f, "segment {index} is empty (leading/trailing/double separator)")
            }
            Self::InvalidSegment { index } => {
                write!(f, "segment {index} is not a valid identifier")
            }
        }
    }
}

impl std::error::Error for QualifiedNameError {}

/// Split a potentially qualified Perl name into `(package, bare_name)`.
///
/// `Foo::Bar` => `(Some("Foo"), "Bar")`
/// `process` => `(None, "process")`
#[must_use]
pub fn split_qualified_name(name: &str) -> (Option<&str>, &str) {
    if let Some(idx) = name.rfind("::") {
        (Some(&name[..idx]), &name[idx + 2..])
    } else {
        (None, name)
    }
}

/// Extract the parent container/package from a qualified Perl symbol.
///
/// `Foo::Bar::baz` => `Some("Foo::Bar")`
/// `process` => `None`
#[must_use]
pub fn container_name(name: &str) -> Option<&str> {
    split_qualified_name(name).0
}

/// Validate a full Perl qualified name with package separators.
///
/// - Rejects empty input.
/// - Rejects leading sigils (`$`, `@`, `%`, `&`, `*`).
/// - Requires each segment to be a valid Perl identifier.
/// - Rejects empty segments from trailing/double/leading separators.
pub fn validate_perl_qualified_name(name: &str) -> Result<(), QualifiedNameError> {
    if name.is_empty() {
        return Err(QualifiedNameError::EmptyName);
    }

    if name.starts_with(['$', '@', '%', '&', '*']) {
        let sigil = name.chars().next().unwrap_or_default();
        return Err(QualifiedNameError::LeadingSigil(sigil));
    }

    for (idx, part) in name.split("::").enumerate() {
        if part.is_empty() {
            return Err(QualifiedNameError::EmptySegment { index: idx });
        }

        if !is_valid_identifier_part(part) {
            return Err(QualifiedNameError::InvalidSegment { index: idx });
        }
    }

    Ok(())
}

/// Check whether a string is a valid Perl identifier component.
#[must_use]
pub fn is_valid_identifier_part(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' => chars.all(|c| c.is_alphanumeric() || c == '_'),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        container_name, is_valid_identifier_part, split_qualified_name,
        validate_perl_qualified_name,
    };

    #[test]
    fn splits_qualified_and_unqualified_names() {
        assert_eq!(split_qualified_name("Foo::Bar"), (Some("Foo"), "Bar"));
        assert_eq!(split_qualified_name("process"), (None, "process"));
    }

    #[test]
    fn extracts_container_name() {
        assert_eq!(container_name("Foo::Bar::baz"), Some("Foo::Bar"));
        assert_eq!(container_name("MyClass::new"), Some("MyClass"));
        assert_eq!(container_name("toplevel"), None);
        assert_eq!(container_name(""), None);
        assert_eq!(container_name("Package::"), Some("Package"));
    }

    #[test]
    fn validates_simple_qualified_names() {
        assert!(validate_perl_qualified_name("Package").is_ok());
        assert!(validate_perl_qualified_name("Package::Sub").is_ok());
        assert!(validate_perl_qualified_name("Müller::Util").is_ok());
        assert!(validate_perl_qualified_name("日本::パッケージ").is_ok());
    }

    #[test]
    fn rejects_invalid_qualified_names() {
        assert!(validate_perl_qualified_name("").is_err());
        assert!(validate_perl_qualified_name("Package::").is_err());
        assert!(validate_perl_qualified_name("Foo::::Bar").is_err());
        assert!(validate_perl_qualified_name("$foo").is_err());
    }

    #[test]
    fn validates_identifier_part_syntax() {
        assert!(is_valid_identifier_part("MyPkg"));
        assert!(is_valid_identifier_part("π"));
        assert!(!is_valid_identifier_part("1abc"));
        assert!(!is_valid_identifier_part(""));
    }
}
