//! AST-to-HIR lowering.

use crate::{Node, NodeKind, SourceLocation};

use super::model::{
    AstAnchor, BarewordExpr, BlockShell, CallExpr, CallForm, DynamicBoundary, DynamicBoundaryKind,
    HirFile, HirId, HirItem, HirKind, IndirectCallExpr, LiteralExpr, LiteralKind, MethodCallExpr,
    MethodDecl, PackageDecl, RecoveryConfidence, RequireDecl, SubDecl, UseDecl, VariableBinding,
    VariableDecl,
};

/// Lower a parser AST into first-slice HIR items.
///
/// This is intentionally conservative: it emits only package, subroutine,
/// method, use, require, variable-declaration, and expression-shell items, and it does not
/// perform scope, stash, import, or provider behavior changes.
pub fn lower_ast(ast: &Node) -> HirFile {
    let mut lowerer = Lowerer::default();
    lowerer.visit(ast, RecoveryConfidence::Parsed);
    lowerer.finish()
}

#[derive(Default)]
struct Lowerer {
    items: Vec<HirItem>,
    next_id: u32,
    package_context: Option<String>,
}

impl Lowerer {
    fn finish(self) -> HirFile {
        HirFile { items: self.items }
    }

    fn visit(&mut self, node: &Node, confidence: RecoveryConfidence) {
        match &node.kind {
            NodeKind::Program { statements } => {
                for statement in statements {
                    self.visit(statement, confidence);
                }
            }
            NodeKind::Block { statements } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::BlockShell(BlockShell { statement_count: statements.len() }),
                    self.package_context.clone(),
                );
                for statement in statements {
                    self.visit(statement, confidence);
                }
            }
            NodeKind::Package { name, name_span, block } => {
                self.push_item(
                    node,
                    Some(*name_span),
                    confidence,
                    HirKind::PackageDecl(PackageDecl {
                        name: name.clone(),
                        name_range: *name_span,
                        has_block: block.is_some(),
                    }),
                    Some(name.clone()),
                );

                if let Some(block) = block {
                    let previous_package = self.package_context.replace(name.clone());
                    self.visit(block, confidence);
                    self.package_context = previous_package;
                } else {
                    self.package_context = Some(name.clone());
                }
            }
            NodeKind::Subroutine { name, name_span, prototype, signature, attributes, .. } => {
                self.push_item(
                    node,
                    *name_span,
                    confidence,
                    HirKind::SubDecl(SubDecl {
                        name: name.clone(),
                        name_range: *name_span,
                        has_prototype: prototype.is_some(),
                        has_signature: signature.is_some(),
                        attribute_count: attributes.len(),
                    }),
                    self.package_context.clone(),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::Method { name, signature, attributes, .. } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::MethodDecl(MethodDecl {
                        name: name.clone(),
                        has_signature: signature.is_some(),
                        attribute_count: attributes.len(),
                    }),
                    self.package_context.clone(),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::Use { module, args, has_filter_risk } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::UseDecl(UseDecl {
                        module: module.clone(),
                        args: args.clone(),
                        has_filter_risk: *has_filter_risk,
                    }),
                    self.package_context.clone(),
                );
            }
            NodeKind::FunctionCall { name, args } if name == "require" => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::RequireDecl(RequireDecl {
                        target: require_target(args.first()),
                        arg_count: args.len(),
                    }),
                    self.package_context.clone(),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::FunctionCall { name, args } => {
                let form = if name == "->()" { CallForm::Coderef } else { CallForm::NamedFunction };
                let arg_count = match form {
                    CallForm::NamedFunction => args.len(),
                    // The parser stores the dynamic callee as args[0] for coderef invocation.
                    CallForm::Coderef => args.len().saturating_sub(1),
                };
                if name == "->()" {
                    self.push_item(
                        node,
                        None,
                        confidence,
                        HirKind::DynamicBoundary(DynamicBoundary {
                            kind: DynamicBoundaryKind::CoderefCall,
                            reason: "coderef or dynamic callee invoked through ->()".to_string(),
                        }),
                        self.package_context.clone(),
                    );
                }
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::CallExpr(CallExpr { name: name.clone(), arg_count, form }),
                    self.package_context.clone(),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::MethodCall { object, method, args } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::MethodCallExpr(MethodCallExpr {
                        method: method.clone(),
                        arg_count: args.len(),
                        object_kind: object.kind.kind_name(),
                    }),
                    self.package_context.clone(),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::IndirectCall { method, object, args } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::IndirectCallExpr(IndirectCallExpr {
                        method: method.clone(),
                        arg_count: args.len(),
                        object_kind: object.kind.kind_name(),
                    }),
                    self.package_context.clone(),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::Identifier { name } => {
                self.push_item(
                    node,
                    Some(node.location),
                    confidence,
                    HirKind::BarewordExpr(BarewordExpr { name: name.clone() }),
                    self.package_context.clone(),
                );
            }
            NodeKind::Number { value } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::LiteralExpr(LiteralExpr {
                        kind: LiteralKind::Number,
                        value: Some(value.clone()),
                        interpolated: None,
                        element_count: None,
                        pair_count: None,
                    }),
                    self.package_context.clone(),
                );
            }
            NodeKind::String { value, interpolated } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::LiteralExpr(LiteralExpr {
                        kind: LiteralKind::String,
                        value: Some(value.clone()),
                        interpolated: Some(*interpolated),
                        element_count: None,
                        pair_count: None,
                    }),
                    self.package_context.clone(),
                );
            }
            NodeKind::Undef => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::LiteralExpr(LiteralExpr {
                        kind: LiteralKind::Undef,
                        value: None,
                        interpolated: None,
                        element_count: None,
                        pair_count: None,
                    }),
                    self.package_context.clone(),
                );
            }
            NodeKind::ArrayLiteral { elements } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::LiteralExpr(LiteralExpr {
                        kind: LiteralKind::Array,
                        value: None,
                        interpolated: None,
                        element_count: Some(elements.len()),
                        pair_count: None,
                    }),
                    self.package_context.clone(),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::HashLiteral { pairs } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::LiteralExpr(LiteralExpr {
                        kind: LiteralKind::Hash,
                        value: None,
                        interpolated: None,
                        element_count: None,
                        pair_count: Some(pairs.len()),
                    }),
                    self.package_context.clone(),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::Eval { block } => {
                if !matches!(block.kind, NodeKind::Block { .. }) {
                    self.push_item(
                        node,
                        None,
                        confidence,
                        HirKind::DynamicBoundary(DynamicBoundary {
                            kind: DynamicBoundaryKind::EvalExpression,
                            reason: "eval body is an expression rather than a parsed block"
                                .to_string(),
                        }),
                        self.package_context.clone(),
                    );
                }
                self.visit_children(node, confidence);
            }
            NodeKind::Do { block } => {
                if !matches!(block.kind, NodeKind::Block { .. }) {
                    self.push_item(
                        node,
                        None,
                        confidence,
                        HirKind::DynamicBoundary(DynamicBoundary {
                            kind: DynamicBoundaryKind::DoExpression,
                            reason: "do body is an expression rather than a parsed block"
                                .to_string(),
                        }),
                        self.package_context.clone(),
                    );
                }
                self.visit_children(node, confidence);
            }
            NodeKind::VariableDeclaration { declarator, variable, attributes, initializer } => {
                let (variables, has_embedded_initializer) = variable_decl_bindings(variable);
                self.push_item(
                    node,
                    variables.first().map(|binding| binding.range),
                    confidence,
                    HirKind::VariableDecl(VariableDecl {
                        declarator: declarator.clone(),
                        variables,
                        attribute_count: attributes.len(),
                        has_initializer: initializer.is_some() || has_embedded_initializer,
                        is_list: false,
                    }),
                    self.package_context.clone(),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::VariableListDeclaration {
                declarator,
                variables,
                attributes,
                initializer,
            } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::VariableDecl(VariableDecl {
                        declarator: declarator.clone(),
                        variables: variables.iter().filter_map(variable_binding).collect(),
                        attribute_count: attributes.len(),
                        has_initializer: initializer.is_some(),
                        is_list: true,
                    }),
                    self.package_context.clone(),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::Error { partial: Some(partial), .. } => {
                self.visit(partial, RecoveryConfidence::Recovered);
            }
            NodeKind::Error { partial: None, .. }
            | NodeKind::MissingExpression
            | NodeKind::MissingStatement
            | NodeKind::MissingIdentifier
            | NodeKind::MissingBlock
            | NodeKind::UnknownRest => {}
            _ => self.visit_children(node, confidence),
        }
    }

    fn visit_children(&mut self, node: &Node, confidence: RecoveryConfidence) {
        node.for_each_child(|child| self.visit(child, confidence));
    }

    fn push_item(
        &mut self,
        node: &Node,
        name_range: Option<SourceLocation>,
        recovery_confidence: RecoveryConfidence,
        kind: HirKind,
        package_context: Option<String>,
    ) {
        let id = HirId::from_index(self.next_id);
        self.next_id += 1;
        self.items.push(HirItem {
            id,
            kind,
            range: node.location,
            anchor: AstAnchor {
                node_kind: node.kind.kind_name(),
                range: node.location,
                name_range,
            },
            recovery_confidence,
            package_context,
            scope_context: None,
        });
    }
}

fn variable_decl_bindings(node: &Node) -> (Vec<VariableBinding>, bool) {
    match &node.kind {
        NodeKind::Assignment { lhs, .. } => (variable_binding(lhs).into_iter().collect(), true),
        NodeKind::VariableWithAttributes { variable, .. } => variable_decl_bindings(variable),
        _ => (variable_binding(node).into_iter().collect(), false),
    }
}

fn require_target(argument: Option<&Node>) -> Option<String> {
    match argument.map(|node| &node.kind) {
        Some(NodeKind::Identifier { name })
        | Some(NodeKind::String { value: name, .. })
        | Some(NodeKind::Typeglob { name }) => Some(name.clone()),
        _ => None,
    }
}

fn variable_binding(node: &Node) -> Option<VariableBinding> {
    match &node.kind {
        NodeKind::Variable { sigil, name } => {
            Some(VariableBinding { sigil: sigil.clone(), name: name.clone(), range: node.location })
        }
        NodeKind::VariableWithAttributes { variable, .. } => variable_binding(variable),
        NodeKind::Typeglob { name } => Some(VariableBinding {
            sigil: "*".to_string(),
            name: name.clone(),
            range: node.location,
        }),
        _ => None,
    }
}
