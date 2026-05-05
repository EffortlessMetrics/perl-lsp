//! Workspace-side generated member extraction for Moo/Moose/Mouse `has`.
//!
//! Recognizes statically visible framework attribute declarations and emits
//! [`EntityFact`] entries with `kind = EntityKind::GeneratedMember` anchored to
//! the declared attribute name.
//!
//! # Scope
//!
//! Only package-level `has` calls after an order-visible `use Moo`, `use Moose`,
//! or `use Mouse` are recognized. Plain packages without a supported framework
//! import are skipped.
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
use std::collections::BTreeMap;

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
        NodeKind::Use { module, .. } if is_accessor_framework_module(module) => {
            ctx.accessor_framework_active = true;
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
    for statement in statements {
        walk(statement, file_id, ctx, out);
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
    matches!(module, "Moo" | "Moo::Role" | "Moose" | "Moose::Role" | "Mouse" | "Mouse::Role")
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
