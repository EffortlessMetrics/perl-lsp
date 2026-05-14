//! Workspace-side generated member extraction for Moo/Moose/Mouse `has`
//! and Class::Tiny accessors.
//!
//! Recognizes statically visible framework attribute declarations and emits
//! [`EntityFact`] entries with `kind = EntityKind::GeneratedMember` anchored to
//! the declared attribute name.
//!
//! # Scope
//!
//! Only package-level `has` calls after an order-visible `use Moo`, `use Moose`,
//! `use Mouse`, `use Class::Tiny`, or `use Class::Tiny::RW` are recognized.
//! Class::Tiny import arguments and the optional following default hash are also
//! recognized. Plain packages without a supported framework import are skipped.
//!
//! # Placement note — circular dependency debt
//!
//! This extractor lives in `perl-workspace` rather than
//! `perl-semantic-analyzer` because `perl-semantic-analyzer` currently depends
//! on `perl-workspace`. Moving this producer to the semantic analyzer today
//! would create a crate cycle. Keep this narrow until the semantic producer
//! boundary is inverted.

use crate::{Node, NodeKind};
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EntityFact, EntityId, EntityKind, FileId, Provenance,
};
use std::collections::{BTreeMap, BTreeSet};

/// Generated member entity plus the source anchor that proves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratedMemberFact {
    pub(crate) entity: EntityFact,
    pub(crate) anchor: AnchorFact,
}

#[derive(Debug, Clone, Default)]
struct WalkCtx {
    current_package: Option<String>,
    accessor_framework_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NameCandidate {
    name: String,
    span_start: usize,
    span_end: usize,
}

/// Extract generated member facts from package-level framework declarations.
pub(crate) fn extract_generated_member_facts(
    ast: &Node,
    file_id: FileId,
) -> Vec<GeneratedMemberFact> {
    let mut out = Vec::new();
    let mut ctx = WalkCtx::default();
    walk(ast, file_id, &mut ctx, &mut out);
    out
}

fn walk(node: &Node, file_id: FileId, ctx: &mut WalkCtx, out: &mut Vec<GeneratedMemberFact>) {
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            walk_statements(statements, file_id, ctx, out);
        }
        NodeKind::Package { name, block, .. } => {
            if let Some(block) = block {
                let saved = ctx.clone();
                ctx.current_package = Some(name.clone());
                ctx.accessor_framework_active = false;
                walk(block, file_id, ctx, out);
                *ctx = saved;
            } else {
                ctx.current_package = Some(name.clone());
                ctx.accessor_framework_active = false;
            }
        }
        NodeKind::Use { module, args, .. } if is_accessor_framework_module(module) => {
            ctx.accessor_framework_active = true;
            if is_class_tiny_module(module) {
                emit_class_tiny_use_members(args, node, file_id, ctx, out);
            }
        }
        NodeKind::No { module, .. } if is_accessor_framework_module(module) => {
            ctx.accessor_framework_active = false;
        }
        NodeKind::ExpressionStatement { expression } => {
            if ctx.accessor_framework_active {
                extract_has_call(expression, file_id, ctx, out);
            }
        }
        NodeKind::Subroutine { .. } | NodeKind::Method { .. } => {}
        _ => {
            for child in node.children() {
                walk(child, file_id, ctx, out);
            }
        }
    }
}

fn walk_statements(
    statements: &[Node],
    file_id: FileId,
    ctx: &mut WalkCtx,
    out: &mut Vec<GeneratedMemberFact>,
) {
    let mut idx = 0;
    while idx < statements.len() {
        if let NodeKind::Use { module, args, .. } = &statements[idx].kind
            && is_class_tiny_module(module)
        {
            ctx.accessor_framework_active = true;
            let emitted_names =
                emit_class_tiny_use_members(args, &statements[idx], file_id, ctx, out);
            let consumed_default_hash = emit_class_tiny_default_hash_members(
                statements.get(idx + 1),
                file_id,
                ctx,
                out,
                &emitted_names,
            );
            idx += 1 + usize::from(consumed_default_hash);
            continue;
        }

        walk(&statements[idx], file_id, ctx, out);
        idx += 1;
    }
}

fn extract_has_call(
    expression: &Node,
    file_id: FileId,
    ctx: &WalkCtx,
    out: &mut Vec<GeneratedMemberFact>,
) {
    let NodeKind::FunctionCall { name, args } = &expression.kind else {
        return;
    };
    if name != "has" || args.is_empty() {
        return;
    }

    let options_idx = args.iter().rposition(|arg| matches!(arg.kind, NodeKind::HashLiteral { .. }));
    let Some(options_idx) = options_idx else {
        emit_members_for_names(args, &[], file_id, ctx, out);
        return;
    };

    let NodeKind::HashLiteral { pairs } = &args[options_idx].kind else {
        return;
    };
    emit_members_for_names(&args[..options_idx], pairs, file_id, ctx, out);
}

fn emit_members_for_names(
    name_nodes: &[Node],
    option_pairs: &[(Node, Node)],
    file_id: FileId,
    ctx: &WalkCtx,
    out: &mut Vec<GeneratedMemberFact>,
) {
    let package = ctx.current_package.as_deref().unwrap_or("main");
    let options = extract_hash_options(option_pairs);

    for raw_name in name_nodes.iter().flat_map(collect_name_candidates) {
        let Some(attribute_name) = normalize_attribute_name(&raw_name.name) else {
            continue;
        };
        let primary_name = options
            .get("accessor")
            .or_else(|| options.get("reader"))
            .cloned()
            .unwrap_or_else(|| attribute_name.clone());

        if options.get("is").is_none_or(|mode| mode != "bare") {
            push_member(package, &primary_name, &raw_name, file_id, out);
        }

        if let Some(writer) = options.get("writer") {
            push_member(package, writer, &raw_name, file_id, out);
        }
        if let Some(predicate) =
            option_method_name(options.get("predicate"), "has", &attribute_name)
        {
            push_member(package, &predicate, &raw_name, file_id, out);
        }
        if let Some(clearer) = option_method_name(options.get("clearer"), "clear", &attribute_name)
        {
            push_member(package, &clearer, &raw_name, file_id, out);
        }
        if let Some(builder) = option_method_name(options.get("builder"), "_build", &attribute_name)
        {
            push_member(package, &builder, &raw_name, file_id, out);
        }
    }
}

fn emit_class_tiny_use_members(
    args: &[String],
    source: &Node,
    file_id: FileId,
    ctx: &WalkCtx,
    out: &mut Vec<GeneratedMemberFact>,
) -> BTreeSet<String> {
    let package = ctx.current_package.as_deref().unwrap_or("main");
    let mut emitted = BTreeSet::new();
    for attribute_name in class_tiny_attribute_names_from_use_args(args) {
        emitted.insert(attribute_name.clone());
        let candidate = NameCandidate {
            name: attribute_name.clone(),
            span_start: source.location.start,
            span_end: source.location.end,
        };
        push_member(package, &attribute_name, &candidate, file_id, out);
    }
    emitted
}

fn emit_class_tiny_default_hash_members(
    statement: Option<&Node>,
    file_id: FileId,
    ctx: &WalkCtx,
    out: &mut Vec<GeneratedMemberFact>,
    already_emitted: &BTreeSet<String>,
) -> bool {
    let Some(statement) = statement else { return false };
    let Some(pairs) = class_tiny_default_hash_pairs(statement) else {
        return false;
    };

    let package = ctx.current_package.as_deref().unwrap_or("main");
    let mut seen = already_emitted.clone();
    for (key_node, _) in pairs {
        for raw_name in collect_name_candidates(key_node) {
            let Some(attribute_name) = normalize_class_tiny_attribute_name(&raw_name.name) else {
                continue;
            };
            if !seen.insert(attribute_name.clone()) {
                continue;
            }
            let candidate = NameCandidate { name: attribute_name.clone(), ..raw_name };
            push_member(package, &attribute_name, &candidate, file_id, out);
        }
    }

    true
}

fn push_member(
    package: &str,
    member_name: &str,
    source_name: &NameCandidate,
    file_id: FileId,
    out: &mut Vec<GeneratedMemberFact>,
) {
    if member_name.is_empty() {
        return;
    }
    let canonical_name = format!("{package}::{member_name}");
    if out.iter().any(|fact| {
        fact.entity.canonical_name == canonical_name
            && fact.anchor.span_start_byte as usize == source_name.span_start
            && fact.anchor.span_end_byte as usize == source_name.span_end
    }) {
        return;
    }

    let entity_id = EntityId(stable_id(
        "generated-member-entity",
        file_id,
        source_name.span_start,
        package,
        member_name,
    ));
    let anchor_id = AnchorId(stable_id(
        "generated-member-anchor",
        file_id,
        source_name.span_start,
        package,
        member_name,
    ));
    let anchor = AnchorFact {
        id: anchor_id,
        file_id,
        span_start_byte: source_name.span_start as u32,
        span_end_byte: source_name.span_end as u32,
        scope_id: None,
        provenance: Provenance::FrameworkSynthesis,
        confidence: Confidence::Medium,
    };
    let entity = EntityFact {
        id: entity_id,
        kind: EntityKind::GeneratedMember,
        canonical_name,
        anchor_id: Some(anchor_id),
        scope_id: None,
        provenance: Provenance::FrameworkSynthesis,
        confidence: Confidence::Medium,
    };
    out.push(GeneratedMemberFact { entity, anchor });
}

fn collect_name_candidates(node: &Node) -> Vec<NameCandidate> {
    match &node.kind {
        NodeKind::String { value, .. } | NodeKind::Identifier { name: value } => {
            expand_symbol_list(value)
                .into_iter()
                .map(|name| NameCandidate {
                    name,
                    span_start: node.location.start,
                    span_end: node.location.end,
                })
                .collect()
        }
        NodeKind::ArrayLiteral { elements } => {
            elements.iter().flat_map(collect_name_candidates).collect()
        }
        NodeKind::Binary { op, left, right } if op == "," => {
            let mut names = collect_name_candidates(left);
            names.extend(collect_name_candidates(right));
            names
        }
        _ => Vec::new(),
    }
}

fn class_tiny_default_hash_pairs(statement: &Node) -> Option<&[(Node, Node)]> {
    let expression = match &statement.kind {
        NodeKind::ExpressionStatement { expression } => expression.as_ref(),
        NodeKind::Block { statements } if statements.len() == 1 => {
            let NodeKind::ExpressionStatement { expression } = &statements.first()?.kind else {
                return None;
            };
            expression.as_ref()
        }
        _ => return None,
    };

    let NodeKind::HashLiteral { pairs } = &expression.kind else {
        return None;
    };
    Some(pairs.as_slice())
}

fn extract_hash_options(pairs: &[(Node, Node)]) -> BTreeMap<String, String> {
    let mut options = BTreeMap::new();
    for (key_node, value_node) in pairs {
        let Some(key_name) = collect_name_candidates(key_node).into_iter().next() else {
            continue;
        };
        options.insert(key_name.name, value_summary(value_node));
    }
    options
}

fn value_summary(node: &Node) -> String {
    match &node.kind {
        NodeKind::String { value, .. } => {
            normalize_symbol_name(value).unwrap_or_else(|| value.clone())
        }
        NodeKind::Identifier { name } => name.clone(),
        NodeKind::Number { value } => value.clone(),
        _ => "expr".to_string(),
    }
}

fn option_method_name(
    value: Option<&String>,
    default_prefix: &str,
    attribute: &str,
) -> Option<String> {
    let value = value?;
    if value == "1" || value == "true" {
        return Some(format!("{default_prefix}_{attribute}"));
    }
    Some(value.clone())
}

fn normalize_symbol_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches('\'').trim_matches('"').trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

fn normalize_attribute_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let without_override_prefix = trimmed.strip_prefix('+').unwrap_or(trimmed);
    normalize_symbol_name(without_override_prefix)
}

fn normalize_class_tiny_attribute_name(raw: &str) -> Option<String> {
    let name = normalize_attribute_name(raw)?;
    if is_class_tiny_attribute_name(&name) { Some(name) } else { None }
}

fn class_tiny_attribute_names_from_use_args(args: &[String]) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    let mut idx = 0;

    while idx < args.len() {
        let token = args[idx].trim();
        match token {
            "" | "," | "=>" | "}" => {
                idx += 1;
            }
            "+" if args.get(idx + 1).map(String::as_str) == Some("{") => {
                idx = collect_class_tiny_hash_keys(args, idx + 1, &mut names, &mut seen);
            }
            "+{" | "{" => {
                idx = collect_class_tiny_hash_keys(args, idx, &mut names, &mut seen);
            }
            _ => {
                for raw_name in expand_symbol_list(token) {
                    push_class_tiny_attribute_name(&raw_name, &mut names, &mut seen);
                }
                idx += 1;
            }
        }
    }

    names
}

fn collect_class_tiny_hash_keys(
    args: &[String],
    start_idx: usize,
    names: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
) -> usize {
    let mut idx = start_idx;
    let mut depth = 0usize;

    while idx < args.len() {
        let token = args[idx].trim();
        match token {
            "+{" | "{" => {
                depth = depth.saturating_add(1);
                idx += 1;
            }
            "}" => {
                depth = depth.saturating_sub(1);
                idx += 1;
                if depth == 0 {
                    break;
                }
            }
            _ if depth == 1 && args.get(idx + 1).map(String::as_str) == Some("=>") => {
                push_class_tiny_attribute_name(token, names, seen);
                idx += 2;
            }
            _ => {
                idx += 1;
            }
        }
    }

    idx
}

fn push_class_tiny_attribute_name(
    raw_name: &str,
    names: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
) {
    let Some(name) = normalize_class_tiny_attribute_name(raw_name) else { return };
    if seen.insert(name.clone()) {
        names.push(name);
    }
}

fn is_class_tiny_attribute_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else { return false };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn expand_symbol_list(raw: &str) -> Vec<String> {
    let raw = raw.trim();

    if raw.starts_with("qw(") && raw.ends_with(')') {
        return raw[3..raw.len() - 1]
            .split_whitespace()
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect();
    }

    if raw.starts_with("qw") && raw.len() > 2 {
        let open = raw.chars().nth(2).unwrap_or(' ');
        let close = match open {
            '(' => ')',
            '{' => '}',
            '[' => ']',
            '<' => '>',
            c => c,
        };
        if let (Some(start), Some(end)) = (raw.find(open), raw.rfind(close))
            && start < end
        {
            return raw[start + 1..end]
                .split_whitespace()
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect();
        }
    }

    normalize_symbol_name(raw).into_iter().collect()
}

fn is_accessor_framework_module(module: &str) -> bool {
    matches!(
        module,
        "Moo"
            | "Moo::Role"
            | "Moose"
            | "Moose::Role"
            | "Mouse"
            | "Mouse::Role"
            | "Class::Tiny"
            | "Class::Tiny::RW"
    )
}

fn is_class_tiny_module(module: &str) -> bool {
    matches!(module, "Class::Tiny" | "Class::Tiny::RW")
}

fn stable_id(label: &str, file_id: FileId, anchor_start: usize, package: &str, name: &str) -> u64 {
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;

    let mut hash = FNV_OFFSET;
    for byte in label
        .as_bytes()
        .iter()
        .chain(file_id.0.to_le_bytes().iter())
        .chain((anchor_start as u64).to_le_bytes().iter())
        .chain(package.as_bytes())
        .chain(name.as_bytes())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    fn extract_from_source(source: &str) -> Vec<GeneratedMemberFact> {
        let mut parser = Parser::new(source);
        let output = parser.parse_with_recovery();
        extract_generated_member_facts(&output.ast, FileId(1))
    }

    fn canonical_names(facts: &[GeneratedMemberFact]) -> Vec<&str> {
        let mut names: Vec<_> =
            facts.iter().map(|fact| fact.entity.canonical_name.as_str()).collect();
        names.sort_unstable();
        names
    }

    fn generated_fact<'a>(
        facts: &'a [GeneratedMemberFact],
        canonical_name: &str,
    ) -> Result<&'a GeneratedMemberFact, Box<dyn std::error::Error>> {
        facts
            .iter()
            .find(|fact| fact.entity.canonical_name == canonical_name)
            .ok_or_else(|| format!("missing generated member fact for {canonical_name}").into())
    }

    #[test]
    fn class_tiny_use_args_emit_generated_member_facts() -> Result<(), Box<dyn std::error::Error>> {
        let facts = extract_from_source(
            r#"
package User;
use Class::Tiny qw(name email);
1;
"#,
        );

        let names = canonical_names(&facts);
        assert!(names.contains(&"User::name"));
        assert!(names.contains(&"User::email"));

        let fact = generated_fact(&facts, "User::name")?;
        assert_eq!(fact.entity.kind, EntityKind::GeneratedMember);
        assert_eq!(fact.entity.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(fact.entity.confidence, Confidence::Medium);
        assert_eq!(fact.anchor.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(fact.anchor.confidence, Confidence::Medium);
        Ok(())
    }

    #[test]
    fn class_tiny_default_hash_keys_emit_generated_member_facts()
    -> Result<(), Box<dyn std::error::Error>> {
        let facts = extract_from_source(
            r#"
package User;
use Class::Tiny qw(name), {
  email => sub { $_[0]->_build_email },
  status => 'active',
};
1;
"#,
        );

        let names = canonical_names(&facts);
        assert!(names.contains(&"User::name"));
        assert!(names.contains(&"User::email"));
        assert!(names.contains(&"User::status"));
        assert!(!names.contains(&"User::active"));
        assert!(!names.contains(&"User::_build_email"));
        assert_eq!(names.iter().filter(|name| **name == "User::name").count(), 1);
        Ok(())
    }

    #[test]
    fn class_tiny_rw_use_args_emit_generated_member_facts() -> Result<(), Box<dyn std::error::Error>>
    {
        let facts = extract_from_source(
            r#"
package Rectangle;
use Class::Tiny::RW qw(width height);
1;
"#,
        );

        let names = canonical_names(&facts);
        assert!(names.contains(&"Rectangle::width"));
        assert!(names.contains(&"Rectangle::height"));
        Ok(())
    }

    #[test]
    fn class_tiny_has_declaration_emits_generated_member_facts()
    -> Result<(), Box<dyn std::error::Error>> {
        let facts = extract_from_source(
            r#"
package Shape;
use Class::Tiny;
has 'color';
1;
"#,
        );

        let names = canonical_names(&facts);
        assert!(names.contains(&"Shape::color"));
        Ok(())
    }
}
