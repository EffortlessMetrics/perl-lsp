//! Rich expression type facts layered on top of [`PerlType`].
//!
//! These facts keep the existing coarse type model stable while preserving the
//! evidence, confidence, dynamic-boundary, and container-shape information needed
//! by receiver-aware features such as method completion.

use crate::analysis::type_inference::{PerlType, ScalarType};
use perl_semantic_facts::Confidence;
use std::collections::BTreeMap;

/// A typed fact inferred for an expression or variable.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TypeFact {
    /// The coarse type used by existing public APIs.
    pub ty: PerlType,
    /// Confidence in the static inference.
    pub confidence: Confidence,
    /// Evidence explaining why this fact was produced.
    pub evidence: Vec<TypeEvidence>,
    /// Dynamic boundary that prevented precise inference, if any.
    pub dynamic_boundary: Option<DynamicBoundary>,
    /// Optional structural shape for containers and objects.
    pub shape: Option<ShapeFact>,
}

/// Structural information attached to a type fact.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ShapeFact {
    /// Statically-known hash slots.
    Hash(HashShape),
    /// Statically-known array indexes or element type.
    Array(ArrayShape),
    /// Statically-known object fields.
    Object(ObjectShape),
}

/// Statically-known hash shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct HashShape {
    /// Per-key slot facts for static keys.
    pub slots: BTreeMap<String, TypeFact>,
    /// Fallback value fact when a static key is not present.
    pub fallback_value: Option<Box<TypeFact>>,
}

/// Statically-known array shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ArrayShape {
    /// Per-index facts for static indexes.
    pub indexed: BTreeMap<usize, TypeFact>,
    /// Fallback element fact when a static index is not present.
    pub element: Option<Box<TypeFact>>,
}

/// Statically-known object shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ObjectShape {
    /// Package represented by this object shape.
    pub package: String,
    /// Per-field facts for statically-known fields.
    pub fields: BTreeMap<String, TypeFact>,
}

/// Evidence carried by a type fact.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TypeEvidence {
    /// Literal syntax produced the fact.
    Literal,
    /// A variable initializer produced the fact.
    VariableInitializer {
        /// Variable name.
        name: String,
    },
    /// An assignment produced the fact.
    Assignment {
        /// Assigned variable name.
        name: String,
    },
    /// A hash slot produced the fact.
    HashSlot {
        /// Hash variable or literal marker.
        hash: String,
        /// Static hash key.
        key: String,
    },
    /// A hash-reference slot produced the fact.
    HashRefSlot {
        /// Base receiver expression label.
        base: String,
        /// Static hash key.
        key: String,
    },
    /// A constructor call produced the fact.
    ConstructorCall {
        /// Constructed package name.
        package: String,
    },
    /// A bless literal produced the fact.
    BlessLiteral {
        /// Blessed package name.
        package: String,
    },
    /// Moose/Moo `isa` metadata produced the fact.
    MooseIsa {
        /// Attribute name.
        attr: String,
        /// Moose/Moo isa string.
        isa: String,
    },
    /// Object::Pad field metadata produced the fact.
    ObjectPadField {
        /// Field name.
        field: String,
    },
    /// Workspace symbols produced the fact.
    WorkspaceSymbol {
        /// Workspace package symbol name.
        package: String,
    },
    /// A heuristic produced the fact.
    Heuristic {
        /// Heuristic explanation.
        reason: String,
    },
}

/// Dynamic boundary that prevented precise inference.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DynamicBoundary {
    /// Hash key expression is not statically known.
    DynamicHashKey,
    /// Bless class expression is not statically known.
    DynamicBlessClass,
    /// Method name expression is not statically known.
    DynamicMethodName,
    /// Runtime import prevents exact static knowledge.
    RuntimeImport,
    /// Receiver expression could not be identified.
    UnknownReceiver,
}

impl TypeFact {
    /// Return the coarse type used by existing type APIs.
    pub fn erased_type(&self) -> PerlType {
        self.ty.clone()
    }

    /// Build a fact with high confidence and no shape.
    pub fn high(ty: PerlType, evidence: Vec<TypeEvidence>) -> Self {
        Self { ty, confidence: Confidence::High, evidence, dynamic_boundary: None, shape: None }
    }

    /// Build a low-confidence `Any` fact.
    pub fn unknown() -> Self {
        Self {
            ty: PerlType::Any,
            confidence: Confidence::Low,
            evidence: vec![TypeEvidence::Heuristic { reason: "unknown expression".to_string() }],
            dynamic_boundary: None,
            shape: None,
        }
    }

    /// Build a low-confidence fact for a dynamic boundary.
    pub fn dynamic(boundary: DynamicBoundary) -> Self {
        Self {
            ty: PerlType::Any,
            confidence: Confidence::Low,
            evidence: vec![TypeEvidence::Heuristic { reason: "dynamic boundary".to_string() }],
            dynamic_boundary: Some(boundary),
            shape: None,
        }
    }

    /// Build an unknown hash fact.
    pub fn unknown_hash() -> Self {
        Self {
            ty: PerlType::Hash {
                key: Box::new(PerlType::Scalar(ScalarType::String)),
                value: Box::new(PerlType::Any),
            },
            confidence: Confidence::Low,
            evidence: vec![TypeEvidence::Heuristic { reason: "unknown hash".to_string() }],
            dynamic_boundary: None,
            shape: Some(ShapeFact::Hash(HashShape {
                slots: BTreeMap::new(),
                fallback_value: None,
            })),
        }
    }
}
