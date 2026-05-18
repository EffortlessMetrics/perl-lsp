//! Receiver facts for method-call completion and navigation.
//!
//! A receiver fact is the semantic-layer contract between expression inference
//! and consumers that need to resolve a method receiver to a package.

use crate::analysis::type_facts::TypeFact;
use crate::analysis::type_inference::{PerlType, TypeEnvironment, TypeInferenceEngine};
use crate::ast::{Node, NodeKind};

/// Inferred fact for a method-call receiver.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ReceiverFact {
    /// Syntactic receiver expression classification.
    pub receiver: ReceiverExpr,
    /// Rich type fact inferred for the receiver expression.
    pub fact: TypeFact,
    /// Resolved package when the receiver is statically known as an object.
    pub package: Option<String>,
}

/// Syntactic classification of a method receiver expression.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ReceiverExpr {
    /// Static package receiver such as `MyApp::Service`.
    StaticPackage(String),
    /// Variable receiver such as `$service`.
    Variable(String),
    /// Plain hash slot receiver such as `$services{db}`.
    HashSlot {
        /// Hash variable name.
        base: String,
        /// Static hash key.
        key: String,
    },
    /// Hash-reference slot receiver such as `$services->{db}`.
    HashRefSlot {
        /// Base receiver expression.
        base: Box<ReceiverExpr>,
        /// Static hash key.
        key: String,
    },
    /// Method-call receiver such as `$self->db`.
    MethodCall {
        /// Base receiver expression.
        receiver: Box<ReceiverExpr>,
        /// Method name.
        method: String,
    },
    /// Unknown receiver syntax.
    Unknown,
}

impl TypeInferenceEngine {
    /// Infer a receiver fact for a method-call object expression.
    pub fn receiver_fact_for_method_call(
        &mut self,
        object: &Node,
        env: &mut TypeEnvironment,
    ) -> ReceiverFact {
        let fact = self.infer_expr_fact(object, env);
        let package = match &fact.ty {
            PerlType::Object(package) => Some(package.clone()),
            PerlType::Reference(inner) => match inner.as_ref() {
                PerlType::Object(package) => Some(package.clone()),
                _ => None,
            },
            _ => None,
        };

        ReceiverFact { receiver: receiver_expr_from_node(object), fact, package }
    }
}

fn receiver_expr_from_node(node: &Node) -> ReceiverExpr {
    match &node.kind {
        NodeKind::Identifier { name } => ReceiverExpr::StaticPackage(name.clone()),
        NodeKind::Variable { name, .. } => ReceiverExpr::Variable(name.clone()),
        NodeKind::Binary { op, left, right } if op == "{}" => {
            let Some(key) = static_key(right) else {
                return ReceiverExpr::Unknown;
            };
            let NodeKind::Variable { name, .. } = &left.kind else {
                return ReceiverExpr::Unknown;
            };
            ReceiverExpr::HashSlot { base: name.clone(), key }
        }
        NodeKind::Binary { op, left, right } if op == "->{}" => {
            let Some(key) = static_key(right) else {
                return ReceiverExpr::Unknown;
            };
            ReceiverExpr::HashRefSlot { base: Box::new(receiver_expr_from_node(left)), key }
        }
        NodeKind::MethodCall { object, method, .. } => ReceiverExpr::MethodCall {
            receiver: Box::new(receiver_expr_from_node(object)),
            method: method.clone(),
        },
        _ => ReceiverExpr::Unknown,
    }
}

fn static_key(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::Identifier { name } => Some(name.clone()),
        NodeKind::String { value, .. } | NodeKind::Number { value } => Some(value.clone()),
        _ => None,
    }
}
