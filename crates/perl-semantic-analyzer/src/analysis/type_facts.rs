//! Rich type facts used by expression and receiver inference.
//!
//! [`PerlType`] remains the erased compatibility type for existing APIs.  This
//! module adds confidence, evidence, dynamic-boundary, and shape metadata that
//! can be consumed by completion and other semantic features without replacing
//! the coarse type model.

use super::type_inference::{PerlType, ScalarType};
use perl_semantic_facts::Confidence;
use std::collections::BTreeMap;

/// A type with confidence, evidence, dynamic-boundary, and shape metadata.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TypeFact {
    /// The erased coarse Perl type represented by this fact.
    pub ty: PerlType,
    /// Confidence assigned to this fact.
    pub confidence: Confidence,
    /// Evidence that produced or refined this fact.
    pub evidence: Vec<TypeEvidence>,
    /// Dynamic boundary that prevented precise inference, when applicable.
    pub dynamic_boundary: Option<DynamicBoundary>,
    /// Optional structural shape information for aggregate or object values.
    pub shape: Option<ShapeFact>,
}

impl TypeFact {
    /// Creates a type fact with the supplied coarse type and confidence.
    pub fn new(ty: PerlType, confidence: Confidence) -> Self {
        Self { ty, confidence, evidence: Vec::new(), dynamic_boundary: None, shape: None }
    }

    /// Creates a high-confidence fact for an existing erased type.
    pub fn from_type(ty: PerlType) -> Self {
        Self::new(ty, Confidence::High)
    }

    /// Creates a low-confidence unknown fact.
    pub fn unknown() -> Self {
        Self::new(PerlType::Any, Confidence::Low)
    }

    /// Creates a low-confidence unknown fact with a dynamic-boundary marker.
    pub fn dynamic(boundary: DynamicBoundary) -> Self {
        Self {
            ty: PerlType::Any,
            confidence: Confidence::Low,
            evidence: Vec::new(),
            dynamic_boundary: Some(boundary),
            shape: None,
        }
    }

    /// Creates a low-confidence unknown fact with heuristic evidence.
    pub fn any_low_confidence(reason: impl Into<String>) -> Self {
        Self {
            evidence: vec![TypeEvidence::Heuristic { reason: reason.into() }],
            ..Self::unknown()
        }
    }

    /// Creates an unknown hash fact suitable for uninitialized hash variables.
    pub fn unknown_hash() -> Self {
        Self::new(
            PerlType::Hash {
                key: Box::new(PerlType::Scalar(ScalarType::String)),
                value: Box::new(PerlType::Any),
            },
            Confidence::Low,
        )
    }

    /// Returns the coarse compatibility type for existing APIs.
    pub fn erased_type(&self) -> PerlType {
        self.ty.clone()
    }
}

/// Structural shape metadata attached to a type fact.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ShapeFact {
    /// Hash shape with known slots and optional fallback value.
    Hash(HashShape),
    /// Array shape with known indexes and optional fallback element value.
    Array(ArrayShape),
    /// Object shape with known fields.
    Object(ObjectShape),
}

/// Shape information for a Perl hash or hash-like literal.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct HashShape {
    /// Known statically-addressable hash slots.
    pub slots: BTreeMap<String, TypeFact>,
    /// Fallback fact for unknown keys when the hash value type is known.
    pub fallback_value: Option<Box<TypeFact>>,
}

impl HashShape {
    /// Creates a hash shape from known slots and an optional fallback value.
    pub fn new(slots: BTreeMap<String, TypeFact>, fallback_value: Option<TypeFact>) -> Self {
        Self { slots, fallback_value: fallback_value.map(Box::new) }
    }
}

/// Shape information for a Perl array or array-like literal.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ArrayShape {
    /// Known statically-addressable array indexes.
    pub indexed: BTreeMap<usize, TypeFact>,
    /// Fallback fact for unknown indexes when the array element type is known.
    pub element: Option<Box<TypeFact>>,
}

impl ArrayShape {
    /// Creates an array shape from known indexes and an optional element fallback.
    pub fn new(indexed: BTreeMap<usize, TypeFact>, element: Option<TypeFact>) -> Self {
        Self { indexed, element: element.map(Box::new) }
    }
}

/// Shape information for an object with field facts.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ObjectShape {
    /// Package represented by the object shape.
    pub package: String,
    /// Known fields on the object.
    pub fields: BTreeMap<String, TypeFact>,
}

impl ObjectShape {
    /// Creates an object shape for a package from known field facts.
    pub fn new(package: String, fields: BTreeMap<String, TypeFact>) -> Self {
        Self { package, fields }
    }
}

/// Evidence explaining how a type fact was produced.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TypeEvidence {
    /// Literal syntax provided the fact.
    Literal,
    /// Variable initializer provided the fact.
    VariableInitializer {
        /// Variable name without sigil.
        name: String,
    },
    /// Assignment provided the fact.
    Assignment {
        /// Variable name without sigil.
        name: String,
    },
    /// Plain hash slot evidence, such as `$hash{key}`.
    HashSlot {
        /// Hash variable or literal source.
        hash: String,
        /// Static hash key.
        key: String,
    },
    /// Hash-reference slot evidence, such as `$hashref->{key}`.
    HashRefSlot {
        /// Base receiver expression label.
        base: String,
        /// Static hash key.
        key: String,
    },
    /// Constructor call evidence, such as `Package->new`.
    ConstructorCall {
        /// Constructed package.
        package: String,
    },
    /// Bless literal evidence.
    BlessLiteral {
        /// Blessed package.
        package: String,
    },
    /// Accessor return evidence, such as `$self->db` returning the `db` field.
    AccessorReturn {
        /// Accessor method name.
        method: String,
        /// Backing field or attribute name.
        field: String,
    },
    /// Method-return evidence, such as `$self->db` returning a constructed package.
    MethodReturn {
        /// Method name.
        method: String,
        /// Returned package shape.
        package: String,
    },
    /// Moose/Moo `isa` evidence for an attribute.
    MooseIsa {
        /// Attribute name.
        attr: String,
        /// `isa` type string.
        isa: String,
    },
    /// Object::Pad field evidence.
    ObjectPadField {
        /// Field name.
        field: String,
    },
    /// Workspace symbol evidence.
    WorkspaceSymbol {
        /// Package resolved from workspace symbols.
        package: String,
    },
    /// Heuristic fallback evidence.
    Heuristic {
        /// Explanation for the heuristic.
        reason: String,
    },
}

/// Dynamic boundary that prevented precise static inference.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DynamicBoundary {
    /// Hash key is computed at runtime.
    DynamicHashKey,
    /// Bless target package is computed at runtime.
    DynamicBlessClass,
    /// Method name is computed at runtime.
    DynamicMethodName,
    /// Import behavior is computed at runtime.
    RuntimeImport,
    /// Receiver expression is unknown.
    UnknownReceiver,
}
