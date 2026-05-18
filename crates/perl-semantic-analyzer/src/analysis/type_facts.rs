//! Rich type facts layered on top of coarse Perl types.
//!
//! This module preserves the existing [`PerlType`] API as the erased type
//! representation while carrying confidence, evidence, dynamic-boundary, and
//! shape details for expression-level inference.

use crate::analysis::type_inference::{PerlType, ScalarType};
use perl_semantic_facts::Confidence;
use std::collections::BTreeMap;

/// Rich type information inferred for an expression or variable.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeFact {
    /// Existing coarse Perl type, kept for compatibility with current APIs.
    pub ty: PerlType,
    /// Confidence in this fact.
    pub confidence: Confidence,
    /// Evidence that produced this fact.
    pub evidence: Vec<TypeEvidence>,
    /// Dynamic boundary that prevented precise static inference.
    pub dynamic_boundary: Option<DynamicBoundary>,
    /// Optional structural shape associated with the value.
    pub shape: Option<ShapeFact>,
}

/// Structural shape information for aggregate and object values.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeFact {
    /// Hash slots and fallback value type.
    Hash(HashShape),
    /// Array indexes and fallback element type.
    Array(ArrayShape),
    /// Object fields inferred from constructors or class metadata.
    Object(ObjectShape),
}

/// Known slot facts for a hash value.
#[derive(Debug, Clone, PartialEq)]
pub struct HashShape {
    /// Facts for statically known hash slots.
    pub slots: BTreeMap<String, TypeFact>,
    /// Fallback fact for unknown/static-but-unlisted keys.
    pub fallback_value: Option<Box<TypeFact>>,
}

/// Known index facts for an array value.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayShape {
    /// Facts for statically known array indexes.
    pub indexed: BTreeMap<usize, TypeFact>,
    /// Fallback fact for unknown indexes.
    pub element: Option<Box<TypeFact>>,
}

/// Known field facts for an object value.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectShape {
    /// Package this object shape belongs to.
    pub package: String,
    /// Facts for statically known fields.
    pub fields: BTreeMap<String, TypeFact>,
}

/// Evidence source for a type fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeEvidence {
    /// Literal expression evidence.
    Literal,
    /// Variable initializer evidence.
    VariableInitializer {
        /// Variable name initialized by the evidence.
        name: String,
    },
    /// Assignment evidence.
    Assignment {
        /// Variable or slot name assigned by the evidence.
        name: String,
    },
    /// Plain hash slot evidence, such as `$hash{key}`.
    HashSlot {
        /// Hash variable name.
        hash: String,
        /// Statically known hash key.
        key: String,
    },
    /// Hash-reference slot evidence, such as `$hashref->{key}`.
    HashRefSlot {
        /// Hash-reference base expression label.
        base: String,
        /// Statically known hash key.
        key: String,
    },
    /// Constructor call evidence, such as `Package->new`.
    ConstructorCall {
        /// Constructed package name.
        package: String,
    },
    /// Literal bless evidence.
    BlessLiteral {
        /// Blessed package name.
        package: String,
    },
    /// Moose/Moo `isa` evidence for an attribute.
    MooseIsa {
        /// Moose/Moo attribute name.
        attr: String,
        /// Declared `isa` type string.
        isa: String,
    },
    /// Object::Pad field evidence.
    ObjectPadField {
        /// Object::Pad field name.
        field: String,
    },
    /// Workspace symbol evidence.
    WorkspaceSymbol {
        /// Package name from the workspace symbol index.
        package: String,
    },
    /// Heuristic evidence.
    Heuristic {
        /// Human-readable heuristic reason.
        reason: String,
    },
}

/// Dynamic boundary that prevented exact static inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicBoundary {
    /// Hash key is not statically known.
    DynamicHashKey,
    /// Bless target package is dynamic.
    DynamicBlessClass,
    /// Method name is dynamic.
    DynamicMethodName,
    /// Runtime import prevents static symbol knowledge.
    RuntimeImport,
    /// Receiver expression could not be inferred.
    UnknownReceiver,
}

impl TypeFact {
    /// Returns the coarse type that existing APIs expose.
    pub fn erased_type(&self) -> PerlType {
        self.ty.clone()
    }

    /// Builds a fact from an existing coarse type with low-confidence heuristic evidence.
    pub fn from_erased_type(ty: PerlType) -> Self {
        Self {
            ty,
            confidence: Confidence::Low,
            evidence: vec![TypeEvidence::Heuristic { reason: "erased type".to_string() }],
            dynamic_boundary: None,
            shape: None,
        }
    }

    /// Builds an unknown low-confidence fact.
    pub fn unknown() -> Self {
        Self {
            ty: PerlType::Any,
            confidence: Confidence::Low,
            evidence: Vec::new(),
            dynamic_boundary: None,
            shape: None,
        }
    }

    /// Builds an unknown fact with a dynamic boundary.
    pub fn dynamic(boundary: DynamicBoundary) -> Self {
        Self {
            ty: PerlType::Any,
            confidence: Confidence::Low,
            evidence: Vec::new(),
            dynamic_boundary: Some(boundary),
            shape: None,
        }
    }

    /// Builds an unknown hash fact for declarations without initializers.
    pub fn unknown_hash() -> Self {
        Self {
            ty: PerlType::Hash {
                key: Box::new(PerlType::Scalar(ScalarType::String)),
                value: Box::new(PerlType::Any),
            },
            confidence: Confidence::Low,
            evidence: Vec::new(),
            dynamic_boundary: None,
            shape: Some(ShapeFact::Hash(HashShape {
                slots: BTreeMap::new(),
                fallback_value: Some(Box::new(Self::unknown())),
            })),
        }
    }
}
