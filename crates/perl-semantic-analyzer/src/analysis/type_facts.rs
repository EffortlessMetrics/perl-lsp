//! Rich expression facts layered on top of coarse Perl types.

use super::type_inference::{PerlType, ScalarType};
use perl_semantic_facts::Confidence;
use std::collections::BTreeMap;

/// A coarse Perl type enriched with confidence, evidence, and optional shape data.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TypeFact {
    /// The erased, compatibility-preserving Perl type.
    pub ty: PerlType,
    /// Confidence in the fact.
    pub confidence: Confidence,
    /// Static evidence that produced this fact.
    pub evidence: Vec<TypeEvidence>,
    /// Dynamic boundary that prevented precise inference, when present.
    pub dynamic_boundary: Option<DynamicBoundary>,
    /// Optional structural information for aggregate and object values.
    pub shape: Option<ShapeFact>,
}

impl TypeFact {
    /// Returns the coarse Perl type for compatibility with existing APIs.
    pub fn erased_type(&self) -> PerlType {
        self.ty.clone()
    }

    /// Creates a high-confidence literal fact.
    pub fn literal(ty: PerlType) -> Self {
        Self {
            ty,
            confidence: Confidence::High,
            evidence: vec![TypeEvidence::Literal],
            dynamic_boundary: None,
            shape: None,
        }
    }

    /// Creates an unknown fact.
    pub fn unknown() -> Self {
        Self::any_low_confidence(TypeEvidence::Heuristic { reason: "unknown type".to_string() })
    }

    /// Creates a low-confidence `Any` fact with the supplied evidence.
    pub fn any_low_confidence(evidence: TypeEvidence) -> Self {
        Self {
            ty: PerlType::Any,
            confidence: Confidence::Low,
            evidence: vec![evidence],
            dynamic_boundary: None,
            shape: None,
        }
    }

    /// Creates an unknown fact that records the dynamic boundary encountered.
    pub fn dynamic(boundary: DynamicBoundary) -> Self {
        Self {
            ty: PerlType::Any,
            confidence: Confidence::Low,
            evidence: vec![TypeEvidence::Heuristic { reason: "dynamic boundary".to_string() }],
            dynamic_boundary: Some(boundary),
            shape: None,
        }
    }

    /// Creates an unknown hash fact for declarations without initializers.
    pub fn unknown_hash() -> Self {
        Self {
            ty: PerlType::Hash {
                key: Box::new(PerlType::Scalar(ScalarType::String)),
                value: Box::new(PerlType::Any),
            },
            confidence: Confidence::Low,
            evidence: vec![TypeEvidence::Heuristic { reason: "uninitialized hash".to_string() }],
            dynamic_boundary: None,
            shape: Some(ShapeFact::Hash(HashShape {
                slots: BTreeMap::new(),
                fallback_value: None,
            })),
        }
    }
}

/// Optional structural shape attached to a type fact.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ShapeFact {
    /// Hash shape with known static slots.
    Hash(HashShape),
    /// Array shape with known indexes and/or homogeneous element type.
    Array(ArrayShape),
    /// Object shape with known fields.
    Object(ObjectShape),
}

/// Static hash shape facts.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct HashShape {
    /// Known static slots by key.
    pub slots: BTreeMap<String, TypeFact>,
    /// Fallback value type for unknown static slots.
    pub fallback_value: Option<Box<TypeFact>>,
}

/// Static array shape facts.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ArrayShape {
    /// Known indexed elements.
    pub indexed: BTreeMap<usize, TypeFact>,
    /// Homogeneous element fallback.
    pub element: Option<Box<TypeFact>>,
}

/// Static object shape facts.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ObjectShape {
    /// Package/class name for the object.
    pub package: String,
    /// Known object fields by field name.
    pub fields: BTreeMap<String, TypeFact>,
}

/// Evidence that contributed to a type fact.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TypeEvidence {
    /// Literal syntax.
    Literal,
    /// Variable initializer evidence.
    VariableInitializer {
        /// Variable name initialized.
        name: String,
    },
    /// Assignment evidence.
    Assignment {
        /// Variable name assigned.
        name: String,
    },
    /// Plain hash slot evidence.
    HashSlot {
        /// Hash variable name.
        hash: String,
        /// Static hash key.
        key: String,
    },
    /// Hash-reference slot evidence.
    HashRefSlot {
        /// Base receiver expression label.
        base: String,
        /// Static hash key.
        key: String,
    },
    /// Constructor call evidence.
    ConstructorCall {
        /// Constructed package name.
        package: String,
    },
    /// Bless literal evidence.
    BlessLiteral {
        /// Blessed package name.
        package: String,
    },
    /// Moose/Moo `isa` metadata evidence.
    MooseIsa {
        /// Attribute name.
        attr: String,
        /// Declared `isa` type.
        isa: String,
    },
    /// Object::Pad field evidence.
    ObjectPadField {
        /// Field name.
        field: String,
    },
    /// Workspace symbol evidence.
    WorkspaceSymbol {
        /// Resolved package name.
        package: String,
    },
    /// Heuristic evidence with a reason.
    Heuristic {
        /// Explanation for the heuristic.
        reason: String,
    },
}

/// Dynamic boundary that prevents precise static inference.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DynamicBoundary {
    /// Hash key is not statically known.
    DynamicHashKey,
    /// Bless class is not statically known.
    DynamicBlessClass,
    /// Method name is not statically known.
    DynamicMethodName,
    /// Import is resolved at runtime.
    RuntimeImport,
    /// Receiver expression is unknown.
    UnknownReceiver,
}
