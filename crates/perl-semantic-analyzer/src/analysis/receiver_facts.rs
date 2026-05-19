//! Receiver facts for trust-bounded method completion.
//!
//! This module classifies method-call receivers such as `$self->method`,
//! `Class->new`, `$hash{slot}->method`, and `$array[0]->method` without
//! changing completion behavior.  It converts existing rich [`TypeFact`] values
//! into a receiver-shaped fact that completion can later use for source-backed
//! ranking receipts.

use super::type_facts::{DynamicBoundary, ShapeFact, TypeEvidence, TypeFact};
use super::type_inference::{PerlType, TypeEnvironment};
use crate::ast::{Node, NodeKind};
use perl_semantic_facts::Confidence;

/// Context used while extracting a receiver fact.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct ReceiverFactContext<'a> {
    /// Rich type environment available at the call site.
    pub type_environment: Option<&'a TypeEnvironment>,
    /// Source text for distinguishing syntax that the AST intentionally erases,
    /// such as `$hash{key}` versus `$hashref->{key}`.
    pub source: Option<&'a str>,
}

impl<'a> ReceiverFactContext<'a> {
    /// Creates a receiver-fact context from an optional type environment.
    pub fn new(type_environment: Option<&'a TypeEnvironment>) -> Self {
        Self { type_environment, source: None }
    }

    /// Adds source text to the receiver-fact context.
    pub fn with_source(mut self, source: &'a str) -> Self {
        self.source = Some(source);
        self
    }
}

/// Receiver shape recognized for a method call.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReceiverKind {
    /// A `$self` or `$this` receiver.
    SelfReceiver,
    /// A scalar object receiver such as `$object`.
    ObjectVariable,
    /// A static package receiver such as `Class->new`.
    StaticPackage,
    /// A hash slot receiver such as `$hash{key}`.
    HashSlot,
    /// A hash-reference slot receiver such as `$hashref->{key}`.
    HashRefSlot,
    /// An array index receiver such as `$array[0]`.
    ArrayIndex,
    /// A receiver with a runtime-computed slot key.
    DynamicKey,
    /// A receiver that cannot be classified statically.
    Unknown,
}

/// Freshness of the evidence used for a receiver fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReceiverFactFreshness {
    /// The receiver fact came from the current AST or supplied type environment.
    Fresh,
    /// No fresh source-backed fact was available.
    Unknown,
}

/// Trust-bounded evidence about a method-call receiver.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ReceiverFact {
    /// Classified receiver kind.
    pub kind: ReceiverKind,
    /// Inferred package for method ranking, when available.
    pub package: Option<String>,
    /// Structural shape fact associated with the receiver, when available.
    pub shape: Option<ShapeFact>,
    /// Confidence assigned to this receiver fact.
    pub confidence: Confidence,
    /// Evidence used to classify the receiver.
    pub evidence: Vec<TypeEvidence>,
    /// Freshness of the receiver evidence.
    pub freshness: ReceiverFactFreshness,
    /// Dynamic boundary that prevents precise method completion, when present.
    pub dynamic_boundary: Option<DynamicBoundary>,
    /// Source-backed byte range for the receiver expression.
    pub source_range: Option<(usize, usize)>,
}

impl ReceiverFact {
    fn unknown(receiver: &Node, reason: impl Into<String>) -> Self {
        Self {
            kind: ReceiverKind::Unknown,
            package: None,
            shape: None,
            confidence: Confidence::Low,
            evidence: vec![TypeEvidence::Heuristic { reason: reason.into() }],
            freshness: ReceiverFactFreshness::Unknown,
            dynamic_boundary: Some(DynamicBoundary::UnknownReceiver),
            source_range: Some((receiver.location.start, receiver.location.end)),
        }
    }

    fn dynamic_key(receiver: &Node, evidence: TypeEvidence) -> Self {
        Self {
            kind: ReceiverKind::DynamicKey,
            package: None,
            shape: None,
            confidence: Confidence::Low,
            evidence: vec![evidence],
            freshness: ReceiverFactFreshness::Unknown,
            dynamic_boundary: Some(DynamicBoundary::DynamicHashKey),
            source_range: Some((receiver.location.start, receiver.location.end)),
        }
    }

    fn from_type_fact(kind: ReceiverKind, fact: TypeFact, receiver: &Node) -> Self {
        let package = package_from_type_fact(&fact);
        Self {
            kind,
            package,
            shape: fact.shape,
            confidence: fact.confidence,
            evidence: fact.evidence,
            freshness: ReceiverFactFreshness::Fresh,
            dynamic_boundary: fact.dynamic_boundary,
            source_range: Some((receiver.location.start, receiver.location.end)),
        }
    }
}

/// Extracts a receiver fact from a method-call node.
pub fn receiver_fact_for_method_call(
    call: &Node,
    context: ReceiverFactContext<'_>,
) -> ReceiverFact {
    let NodeKind::MethodCall { object, method, .. } = &call.kind else {
        return ReceiverFact::unknown(call, "node is not a method call");
    };

    infer_receiver_fact(object, Some(method.as_str()), context)
}

/// Extracts a receiver fact from an expression used as a method-call receiver.
pub fn infer_receiver_fact(
    receiver: &Node,
    method_name: Option<&str>,
    context: ReceiverFactContext<'_>,
) -> ReceiverFact {
    match &receiver.kind {
        NodeKind::Variable { sigil, name } if sigil == "$" => {
            variable_receiver_fact(receiver, name, context)
        }
        NodeKind::VariableWithAttributes { variable, .. } => {
            infer_receiver_fact(variable, method_name, context)
        }
        NodeKind::Identifier { name } => static_package_receiver(receiver, name, method_name),
        NodeKind::String { value, .. } => {
            let normalized = normalize_package_string(value);
            match normalized {
                Some(package) => static_package_receiver(receiver, &package, method_name),
                None => ReceiverFact::unknown(receiver, "empty package string receiver"),
            }
        }
        NodeKind::Binary { op, left, right } if op == "{}" || op == "->{}" => {
            hash_receiver_fact(receiver, left, right, context)
        }
        NodeKind::Binary { op, left, right } if op == "[]" || op == "->[]" => {
            array_receiver_fact(receiver, left, right, context)
        }
        NodeKind::MethodCall { .. } => ReceiverFact::unknown(
            receiver,
            "receiver is itself a method call and requires completion-chain evidence",
        ),
        _ => ReceiverFact::unknown(receiver, "receiver expression has no source-backed fact"),
    }
}

fn variable_receiver_fact(
    receiver: &Node,
    name: &str,
    context: ReceiverFactContext<'_>,
) -> ReceiverFact {
    let kind = if is_self_like_name(name) {
        ReceiverKind::SelfReceiver
    } else {
        ReceiverKind::ObjectVariable
    };

    if let Some(fact) = context.type_environment.and_then(|env| env.get_fact_at(name)) {
        return ReceiverFact::from_type_fact(kind, fact, receiver);
    }

    if is_self_like_name(name) {
        return ReceiverFact {
            kind,
            package: None,
            shape: None,
            confidence: Confidence::Medium,
            evidence: vec![TypeEvidence::Heuristic {
                reason: "self-like receiver without package fact".to_string(),
            }],
            freshness: ReceiverFactFreshness::Unknown,
            dynamic_boundary: None,
            source_range: Some((receiver.location.start, receiver.location.end)),
        };
    }

    ReceiverFact::unknown(receiver, "object variable has no type fact")
}

fn static_package_receiver(
    receiver: &Node,
    package: &str,
    method_name: Option<&str>,
) -> ReceiverFact {
    let evidence = if method_name == Some("new") {
        TypeEvidence::ConstructorCall { package: package.to_string() }
    } else {
        TypeEvidence::Heuristic { reason: "static package receiver".to_string() }
    };

    ReceiverFact {
        kind: ReceiverKind::StaticPackage,
        package: Some(package.to_string()),
        shape: None,
        confidence: Confidence::High,
        evidence: vec![evidence],
        freshness: ReceiverFactFreshness::Fresh,
        dynamic_boundary: None,
        source_range: Some((receiver.location.start, receiver.location.end)),
    }
}

fn hash_receiver_fact(
    receiver: &Node,
    left: &Node,
    right: &Node,
    context: ReceiverFactContext<'_>,
) -> ReceiverFact {
    let Some(key) = static_slot_key(right) else {
        return ReceiverFact::dynamic_key(
            receiver,
            TypeEvidence::Heuristic { reason: "hash receiver key is dynamic".to_string() },
        );
    };

    let base = receiver_base_label(left);
    let kind = if matches!(&receiver.kind, NodeKind::Binary { op, .. } if op == "->{}")
        || receiver_text(receiver, context.source).is_some_and(|text| text.contains("->{"))
    {
        ReceiverKind::HashRefSlot
    } else {
        ReceiverKind::HashSlot
    };
    let evidence = match kind {
        ReceiverKind::HashRefSlot => TypeEvidence::HashRefSlot { base: base.clone(), key },
        _ => TypeEvidence::HashSlot { hash: base.clone(), key },
    };

    let Some(container_fact) = receiver_container_fact(left, context) else {
        return ReceiverFact {
            kind,
            package: None,
            shape: None,
            confidence: Confidence::Low,
            evidence: vec![evidence],
            freshness: ReceiverFactFreshness::Unknown,
            dynamic_boundary: None,
            source_range: Some((receiver.location.start, receiver.location.end)),
        };
    };

    if let Some(slot_fact) = hash_slot_type_fact(&container_fact, &evidence) {
        return ReceiverFact::from_type_fact(
            kind,
            with_extra_evidence(slot_fact, evidence),
            receiver,
        );
    }

    ReceiverFact {
        kind,
        package: None,
        shape: container_fact.shape,
        confidence: Confidence::Low,
        evidence: vec![evidence],
        freshness: ReceiverFactFreshness::Fresh,
        dynamic_boundary: container_fact.dynamic_boundary,
        source_range: Some((receiver.location.start, receiver.location.end)),
    }
}

fn array_receiver_fact(
    receiver: &Node,
    left: &Node,
    right: &Node,
    context: ReceiverFactContext<'_>,
) -> ReceiverFact {
    let evidence = TypeEvidence::Heuristic { reason: "array index receiver".to_string() };
    let Some(container_fact) = receiver_container_fact(left, context) else {
        return ReceiverFact {
            kind: ReceiverKind::ArrayIndex,
            package: None,
            shape: None,
            confidence: Confidence::Low,
            evidence: vec![evidence],
            freshness: ReceiverFactFreshness::Unknown,
            dynamic_boundary: None,
            source_range: Some((receiver.location.start, receiver.location.end)),
        };
    };

    let Some(index) = static_array_index(right) else {
        return ReceiverFact {
            kind: ReceiverKind::ArrayIndex,
            package: None,
            shape: container_fact.shape,
            confidence: Confidence::Low,
            evidence: vec![evidence],
            freshness: ReceiverFactFreshness::Unknown,
            dynamic_boundary: Some(DynamicBoundary::UnknownReceiver),
            source_range: Some((receiver.location.start, receiver.location.end)),
        };
    };

    if let Some(index_fact) = array_index_type_fact(&container_fact, index) {
        return ReceiverFact::from_type_fact(
            ReceiverKind::ArrayIndex,
            with_extra_evidence(index_fact, evidence),
            receiver,
        );
    }

    ReceiverFact {
        kind: ReceiverKind::ArrayIndex,
        package: None,
        shape: container_fact.shape,
        confidence: Confidence::Low,
        evidence: vec![evidence],
        freshness: ReceiverFactFreshness::Fresh,
        dynamic_boundary: container_fact.dynamic_boundary,
        source_range: Some((receiver.location.start, receiver.location.end)),
    }
}

fn receiver_container_fact(left: &Node, context: ReceiverFactContext<'_>) -> Option<TypeFact> {
    let (_, name) = variable_identity(left)?;
    context.type_environment.and_then(|env| env.get_fact_at(name))
}

fn hash_slot_type_fact(container_fact: &TypeFact, evidence: &TypeEvidence) -> Option<TypeFact> {
    let key = match evidence {
        TypeEvidence::HashSlot { key, .. } | TypeEvidence::HashRefSlot { key, .. } => key,
        _ => return None,
    };
    match &container_fact.shape {
        Some(ShapeFact::Hash(shape)) => shape
            .slots
            .get(key)
            .cloned()
            .or_else(|| shape.fallback_value.as_ref().map(|fact| fact.as_ref().clone())),
        _ => None,
    }
}

fn array_index_type_fact(container_fact: &TypeFact, index: usize) -> Option<TypeFact> {
    match &container_fact.shape {
        Some(ShapeFact::Array(shape)) => shape
            .indexed
            .get(&index)
            .cloned()
            .or_else(|| shape.element.as_ref().map(|fact| fact.as_ref().clone())),
        _ => None,
    }
}

fn with_extra_evidence(mut fact: TypeFact, evidence: TypeEvidence) -> TypeFact {
    fact.evidence.push(evidence);
    fact
}

fn variable_identity(node: &Node) -> Option<(&str, &str)> {
    match &node.kind {
        NodeKind::Variable { sigil, name } => Some((sigil.as_str(), name.as_str())),
        NodeKind::VariableWithAttributes { variable, .. } => variable_identity(variable),
        _ => None,
    }
}

fn receiver_base_label(node: &Node) -> String {
    match variable_identity(node) {
        Some((sigil, name)) => format!("{sigil}{name}"),
        None => node.kind.kind_name().to_string(),
    }
}

fn static_slot_key(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::String { value, .. } => Some(normalize_literal(value)),
        NodeKind::Identifier { name } => Some(name.clone()),
        NodeKind::Number { value } => Some(value.clone()),
        _ => None,
    }
}

fn static_array_index(node: &Node) -> Option<usize> {
    match &node.kind {
        NodeKind::Number { value } => value.parse().ok(),
        _ => None,
    }
}

fn receiver_text<'a>(receiver: &Node, source: Option<&'a str>) -> Option<&'a str> {
    source?.get(receiver.location.start..receiver.location.end)
}

fn package_from_type_fact(fact: &TypeFact) -> Option<String> {
    package_from_type(&fact.ty).or_else(|| match &fact.shape {
        Some(ShapeFact::Object(shape)) => Some(shape.package.clone()),
        _ => None,
    })
}

fn package_from_type(ty: &PerlType) -> Option<String> {
    match ty {
        PerlType::Object(package) => Some(package.clone()),
        PerlType::Reference(inner) => package_from_type(inner),
        PerlType::Union(types) => types.iter().find_map(package_from_type),
        _ => None,
    }
}

fn normalize_package_string(value: &str) -> Option<String> {
    let normalized = normalize_literal(value);
    if normalized.is_empty() { None } else { Some(normalized) }
}

fn normalize_literal(value: &str) -> String {
    value.trim().trim_matches('\'').trim_matches('"').trim().to_string()
}

fn is_self_like_name(name: &str) -> bool {
    matches!(name, "self" | "this")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    use std::collections::BTreeMap;

    fn parse_ast(code: &str) -> Result<Node, String> {
        let mut parser = Parser::new(code);
        parser.parse().map_err(|err| format!("parse failed: {err:?}"))
    }

    fn method_call_named<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
        if let NodeKind::MethodCall { method, .. } = &node.kind {
            if method == name {
                return Some(node);
            }
        }

        match &node.kind {
            NodeKind::Program { statements } => {
                statements.iter().find_map(|child| method_call_named(child, name))
            }
            NodeKind::ExpressionStatement { expression } => method_call_named(expression, name),
            NodeKind::VariableDeclaration { initializer, .. } => {
                initializer.as_deref().and_then(|child| method_call_named(child, name))
            }
            NodeKind::Assignment { lhs, rhs, .. } => {
                method_call_named(lhs, name).or_else(|| method_call_named(rhs, name))
            }
            NodeKind::MethodCall { object, args, .. } => method_call_named(object, name)
                .or_else(|| args.iter().find_map(|child| method_call_named(child, name))),
            NodeKind::Binary { left, right, .. } => {
                method_call_named(left, name).or_else(|| method_call_named(right, name))
            }
            _ => None,
        }
    }

    fn object_fact(package: &str, confidence: Confidence) -> TypeFact {
        TypeFact {
            ty: PerlType::Object(package.to_string()),
            confidence,
            evidence: vec![TypeEvidence::WorkspaceSymbol { package: package.to_string() }],
            dynamic_boundary: None,
            shape: Some(ShapeFact::Object(super::super::type_facts::ObjectShape::new(
                package.to_string(),
                BTreeMap::new(),
            ))),
        }
    }

    fn hash_shape_fact(slot: &str, package: &str) -> TypeFact {
        let mut slots = BTreeMap::new();
        slots.insert(slot.to_string(), object_fact(package, Confidence::High));
        TypeFact {
            ty: PerlType::Hash { key: Box::new(PerlType::Any), value: Box::new(PerlType::Any) },
            confidence: Confidence::High,
            evidence: vec![TypeEvidence::Literal],
            dynamic_boundary: None,
            shape: Some(ShapeFact::Hash(super::super::type_facts::HashShape::new(slots, None))),
        }
    }

    fn array_shape_fact(index: usize, package: &str) -> TypeFact {
        let mut indexed = BTreeMap::new();
        indexed.insert(index, object_fact(package, Confidence::High));
        TypeFact {
            ty: PerlType::Array(Box::new(PerlType::Any)),
            confidence: Confidence::High,
            evidence: vec![TypeEvidence::Literal],
            dynamic_boundary: None,
            shape: Some(ShapeFact::Array(super::super::type_facts::ArrayShape::new(indexed, None))),
        }
    }

    fn receiver_fact_for(
        code: &str,
        method: &str,
        env: &TypeEnvironment,
    ) -> Result<ReceiverFact, String> {
        let ast = parse_ast(code)?;
        let call = method_call_named(&ast, method).ok_or("expected method call")?;
        Ok(receiver_fact_for_method_call(
            call,
            ReceiverFactContext::new(Some(env)).with_source(code),
        ))
    }

    #[test]
    fn static_constructor_receiver_records_package() -> Result<(), String> {
        let env = TypeEnvironment::new();
        let fact = receiver_fact_for("Foo::Bar->new();", "new", &env)?;

        assert_eq!(fact.kind, ReceiverKind::StaticPackage);
        assert_eq!(fact.package.as_deref(), Some("Foo::Bar"));
        assert_eq!(fact.confidence, Confidence::High);
        assert_eq!(fact.freshness, ReceiverFactFreshness::Fresh);
        assert!(matches!(
            fact.evidence.first(),
            Some(TypeEvidence::ConstructorCall { package }) if package == "Foo::Bar"
        ));
        Ok(())
    }

    #[test]
    fn self_receiver_uses_type_environment_fact() -> Result<(), String> {
        let mut env = TypeEnvironment::new();
        env.set_variable_fact("self".to_string(), object_fact("My::Controller", Confidence::High));

        let fact = receiver_fact_for("$self->render();", "render", &env)?;

        assert_eq!(fact.kind, ReceiverKind::SelfReceiver);
        assert_eq!(fact.package.as_deref(), Some("My::Controller"));
        assert_eq!(fact.confidence, Confidence::High);
        assert!(matches!(fact.shape, Some(ShapeFact::Object(_))));
        assert_eq!(fact.dynamic_boundary, None);
        Ok(())
    }

    #[test]
    fn object_receiver_uses_type_environment_fact() -> Result<(), String> {
        let mut env = TypeEnvironment::new();
        env.set_variable_fact("object".to_string(), object_fact("My::Service", Confidence::High));

        let fact = receiver_fact_for("$object->run();", "run", &env)?;

        assert_eq!(fact.kind, ReceiverKind::ObjectVariable);
        assert_eq!(fact.package.as_deref(), Some("My::Service"));
        assert_eq!(fact.freshness, ReceiverFactFreshness::Fresh);
        Ok(())
    }

    #[test]
    fn hash_slot_receiver_uses_known_slot_fact() -> Result<(), String> {
        let mut env = TypeEnvironment::new();
        env.set_variable_fact("services".to_string(), hash_shape_fact("mailer", "My::Mailer"));

        let fact = receiver_fact_for("$services{mailer}->send();", "send", &env)?;

        assert_eq!(fact.kind, ReceiverKind::HashSlot);
        assert_eq!(fact.package.as_deref(), Some("My::Mailer"));
        assert!(fact.evidence.iter().any(|evidence| {
            matches!(evidence, TypeEvidence::HashSlot { hash, key } if hash == "$services" && key == "mailer")
        }));
        Ok(())
    }

    #[test]
    fn hashref_slot_receiver_preserves_hashref_kind() -> Result<(), String> {
        let mut env = TypeEnvironment::new();
        env.set_variable_fact("services".to_string(), hash_shape_fact("mailer", "My::Mailer"));

        let fact = receiver_fact_for("$services->{mailer}->send();", "send", &env)?;

        assert_eq!(fact.kind, ReceiverKind::HashRefSlot);
        assert_eq!(fact.package.as_deref(), Some("My::Mailer"));
        assert!(fact.evidence.iter().any(|evidence| {
            matches!(evidence, TypeEvidence::HashRefSlot { base, key } if base == "$services" && key == "mailer")
        }));
        Ok(())
    }

    #[test]
    fn dynamic_hash_key_marks_dynamic_boundary() -> Result<(), String> {
        let mut env = TypeEnvironment::new();
        env.set_variable_fact("services".to_string(), hash_shape_fact("mailer", "My::Mailer"));

        let fact = receiver_fact_for("$services{$name}->send();", "send", &env)?;

        assert_eq!(fact.kind, ReceiverKind::DynamicKey);
        assert_eq!(fact.confidence, Confidence::Low);
        assert_eq!(fact.dynamic_boundary, Some(DynamicBoundary::DynamicHashKey));
        assert_eq!(fact.package, None);
        Ok(())
    }

    #[test]
    fn array_index_receiver_uses_known_index_fact() -> Result<(), String> {
        let mut env = TypeEnvironment::new();
        env.set_variable_fact("items".to_string(), array_shape_fact(0, "My::Item"));

        let fact = receiver_fact_for("$items[0]->render();", "render", &env)?;

        assert_eq!(fact.kind, ReceiverKind::ArrayIndex);
        assert_eq!(fact.package.as_deref(), Some("My::Item"));
        assert_eq!(fact.confidence, Confidence::High);
        Ok(())
    }

    #[test]
    fn dynamic_array_index_stays_low_confidence() -> Result<(), String> {
        let mut env = TypeEnvironment::new();
        env.set_variable_fact("items".to_string(), array_shape_fact(0, "My::Item"));

        let fact = receiver_fact_for("$items[$i]->render();", "render", &env)?;

        assert_eq!(fact.kind, ReceiverKind::ArrayIndex);
        assert_eq!(fact.package, None);
        assert_eq!(fact.confidence, Confidence::Low);
        assert_eq!(fact.dynamic_boundary, Some(DynamicBoundary::UnknownReceiver));
        Ok(())
    }

    #[test]
    fn unknown_receiver_stays_low_confidence() -> Result<(), String> {
        let env = TypeEnvironment::new();

        let fact = receiver_fact_for("$unknown->run();", "run", &env)?;

        assert_eq!(fact.kind, ReceiverKind::Unknown);
        assert_eq!(fact.confidence, Confidence::Low);
        assert_eq!(fact.dynamic_boundary, Some(DynamicBoundary::UnknownReceiver));
        assert_eq!(fact.package, None);
        Ok(())
    }
}
