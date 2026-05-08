//! AST-to-HIR lowering.

use crate::{Node, NodeKind, SourceLocation};

use super::model::{
    AstAnchor, BarewordExpr, Binding, BindingReference, BlockShell, CallExpr, CallForm,
    DynamicBoundary, DynamicBoundaryKind, HirBindingId, HirFile, HirId, HirItem, HirKind,
    HirScopeId, IndirectCallExpr, LiteralExpr, LiteralKind, MethodCallExpr, MethodDecl,
    PackageDecl, RecoveryConfidence, RequireDecl, ScopeFrame, ScopeGraph, ScopeKind, StorageClass,
    SubDecl, UseDecl, VariableBinding, VariableDecl,
};

/// Lower a parser AST into first-slice HIR items.
///
/// This is intentionally conservative: it emits only package, subroutine,
/// method, use, require, variable-declaration, and expression-shell items. It
/// records a local scope graph, but it does not perform stash, import, or
/// provider behavior changes.
pub fn lower_ast(ast: &Node) -> HirFile {
    let mut lowerer = Lowerer::new(ast.location);
    lowerer.visit(ast, RecoveryConfidence::Parsed);
    lowerer.finish()
}

struct Lowerer {
    items: Vec<HirItem>,
    next_id: u32,
    package_context: Option<String>,
    scope_graph: ScopeGraph,
    scope_stack: Vec<HirScopeId>,
}

impl Lowerer {
    fn new(file_range: SourceLocation) -> Self {
        let mut scope_graph = ScopeGraph::default();
        let file_scope = HirScopeId::from_index(0);
        scope_graph.scopes.push(ScopeFrame {
            id: file_scope,
            parent: None,
            kind: ScopeKind::File,
            range: file_range,
            package_context: None,
        });

        Self {
            items: Vec::new(),
            next_id: 0,
            package_context: None,
            scope_graph,
            scope_stack: vec![file_scope],
        }
    }

    fn finish(self) -> HirFile {
        HirFile { items: self.items, scope_graph: self.scope_graph }
    }

    fn visit(&mut self, node: &Node, confidence: RecoveryConfidence) {
        match &node.kind {
            NodeKind::Program { statements } => {
                for statement in statements {
                    self.visit(statement, confidence);
                }
            }
            NodeKind::Block { statements } => {
                let scope_id =
                    self.enter_scope(ScopeKind::Block, node.location, self.package_context.clone());
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::BlockShell(BlockShell { statement_count: statements.len() }),
                    self.package_context.clone(),
                    Some(scope_id),
                );
                for statement in statements {
                    self.visit(statement, confidence);
                }
                self.exit_scope();
            }
            NodeKind::Package { name, name_span, block } => {
                let package_scope =
                    self.enter_scope(ScopeKind::Package, node.location, Some(name.clone()));
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
                    Some(package_scope),
                );

                if let Some(block) = block {
                    let previous_package = self.package_context.replace(name.clone());
                    self.visit(block, confidence);
                    self.package_context = previous_package;
                    self.exit_scope();
                } else {
                    self.package_context = Some(name.clone());
                }
            }
            NodeKind::Subroutine { name, name_span, prototype, signature, attributes, body } => {
                let sub_scope = self.enter_scope(
                    ScopeKind::Subroutine,
                    node.location,
                    self.package_context.clone(),
                );
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
                    Some(sub_scope),
                );
                if let Some(prototype) = prototype {
                    self.visit(prototype, confidence);
                }
                let has_signature_scope = if let Some(signature) = signature {
                    let signature_scope = self.enter_scope(
                        ScopeKind::Signature,
                        signature.location,
                        self.package_context.clone(),
                    );
                    self.record_signature_bindings(signature, signature_scope);
                    true
                } else {
                    false
                };
                self.visit(body, confidence);
                if has_signature_scope {
                    self.exit_scope();
                }
                self.exit_scope();
            }
            NodeKind::Method { name, signature, attributes, body } => {
                let method_scope = self.enter_scope(
                    ScopeKind::Method,
                    node.location,
                    self.package_context.clone(),
                );
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
                    Some(method_scope),
                );
                let has_signature_scope = if let Some(signature) = signature {
                    let signature_scope = self.enter_scope(
                        ScopeKind::Signature,
                        signature.location,
                        self.package_context.clone(),
                    );
                    self.record_signature_bindings(signature, signature_scope);
                    true
                } else {
                    false
                };
                self.visit(body, confidence);
                if has_signature_scope {
                    self.exit_scope();
                }
                self.exit_scope();
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
                    Some(self.current_scope()),
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
                    Some(self.current_scope()),
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
                        Some(self.current_scope()),
                    );
                }
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::CallExpr(CallExpr { name: name.clone(), arg_count, form }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
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
                    Some(self.current_scope()),
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
                    Some(self.current_scope()),
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
                    Some(self.current_scope()),
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
                    Some(self.current_scope()),
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
                    Some(self.current_scope()),
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
                    Some(self.current_scope()),
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
                    Some(self.current_scope()),
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
                    Some(self.current_scope()),
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
                        Some(self.current_scope()),
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
                        Some(self.current_scope()),
                    );
                }
                self.visit_children(node, confidence);
            }
            NodeKind::VariableDeclaration { declarator, variable, attributes, initializer } => {
                let (variables, has_embedded_initializer) = variable_decl_bindings(variable);
                let item_id = self.push_item(
                    node,
                    variables.first().map(|binding| binding.range),
                    confidence,
                    HirKind::VariableDecl(VariableDecl {
                        declarator: declarator.clone(),
                        variables: variables.clone(),
                        attribute_count: attributes.len(),
                        has_initializer: initializer.is_some() || has_embedded_initializer,
                        is_list: false,
                    }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
                self.record_declaration_bindings(declarator, &variables, item_id);
                if let Some(initializer) = initializer {
                    self.visit(initializer, confidence);
                } else if has_embedded_initializer {
                    self.visit_declaration_variable_payload(variable, confidence);
                }
            }
            NodeKind::VariableListDeclaration {
                declarator,
                variables,
                attributes,
                initializer,
            } => {
                let bindings = variables.iter().filter_map(variable_binding).collect::<Vec<_>>();
                let item_id = self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::VariableDecl(VariableDecl {
                        declarator: declarator.clone(),
                        variables: bindings.clone(),
                        attribute_count: attributes.len(),
                        has_initializer: initializer.is_some(),
                        is_list: true,
                    }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
                self.record_declaration_bindings(declarator, &bindings, item_id);
                self.visit_declaration_list_entries(variables, confidence);
                if let Some(initializer) = initializer {
                    self.visit(initializer, confidence);
                }
            }
            NodeKind::Variable { sigil, name } => {
                self.record_reference(sigil, name, node.location);
            }
            NodeKind::PhaseBlock { phase: _, block, .. } => {
                self.enter_scope(
                    ScopeKind::PhaseBlock,
                    node.location,
                    self.package_context.clone(),
                );
                self.visit(block, confidence);
                self.exit_scope();
            }
            NodeKind::Format { .. } => {
                self.enter_scope(ScopeKind::Format, node.location, self.package_context.clone());
                self.exit_scope();
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

    fn current_scope(&self) -> HirScopeId {
        self.scope_stack.last().copied().unwrap_or_else(|| HirScopeId::from_index(0))
    }

    fn enter_scope(
        &mut self,
        kind: ScopeKind,
        range: SourceLocation,
        package_context: Option<String>,
    ) -> HirScopeId {
        let id = HirScopeId::from_index(self.scope_graph.scopes.len() as u32);
        let parent = Some(self.current_scope());
        self.scope_graph.scopes.push(ScopeFrame { id, parent, kind, range, package_context });
        self.scope_stack.push(id);
        id
    }

    fn exit_scope(&mut self) {
        if self.scope_stack.len() > 1 {
            self.scope_stack.pop();
        }
    }

    fn push_item(
        &mut self,
        node: &Node,
        name_range: Option<SourceLocation>,
        recovery_confidence: RecoveryConfidence,
        kind: HirKind,
        package_context: Option<String>,
        scope_context: Option<HirScopeId>,
    ) -> HirId {
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
            scope_context,
        });
        id
    }

    fn record_declaration_bindings(
        &mut self,
        declarator: &str,
        variables: &[VariableBinding],
        declaration_item: HirId,
    ) {
        let storage = storage_class_for_declarator(declarator);
        for variable in variables {
            self.record_binding(
                variable.sigil.clone(),
                variable.name.clone(),
                variable.range,
                storage,
                self.current_scope(),
                Some(declaration_item),
            );
        }
    }

    fn record_signature_bindings(&mut self, signature: &Node, scope_id: HirScopeId) {
        if let NodeKind::Signature { parameters } = &signature.kind {
            for parameter in parameters {
                self.record_signature_parameter(parameter, scope_id);
            }
        }
    }

    fn record_signature_parameter(&mut self, parameter: &Node, scope_id: HirScopeId) {
        match &parameter.kind {
            NodeKind::MandatoryParameter { variable }
            | NodeKind::SlurpyParameter { variable }
            | NodeKind::NamedParameter { variable } => {
                if let Some(binding) = variable_binding(variable) {
                    self.record_binding(
                        binding.sigil,
                        binding.name,
                        binding.range,
                        StorageClass::Parameter,
                        scope_id,
                        None,
                    );
                }
            }
            NodeKind::OptionalParameter { variable, default_value } => {
                if let Some(binding) = variable_binding(variable) {
                    self.record_binding(
                        binding.sigil,
                        binding.name,
                        binding.range,
                        StorageClass::Parameter,
                        scope_id,
                        None,
                    );
                }
                self.visit(default_value, RecoveryConfidence::Parsed);
            }
            _ => {}
        }
    }

    fn record_binding(
        &mut self,
        sigil: String,
        name: String,
        range: SourceLocation,
        storage: StorageClass,
        scope_id: HirScopeId,
        declaration_item: Option<HirId>,
    ) -> HirBindingId {
        let shadows = self.resolve_visible_binding(scope_id, &sigil, &name);
        let id = HirBindingId::from_index(self.scope_graph.bindings.len() as u32);
        self.scope_graph.bindings.push(Binding {
            id,
            scope_id,
            sigil,
            name,
            range,
            storage,
            package_context: self.package_context.clone(),
            declaration_item,
            shadows,
        });
        id
    }

    fn record_reference(&mut self, sigil: &str, name: &str, range: SourceLocation) {
        let scope_id = self.current_scope();
        let resolved_binding = self.resolve_visible_binding(scope_id, sigil, name);
        self.scope_graph.references.push(BindingReference {
            scope_id,
            sigil: sigil.to_string(),
            name: name.to_string(),
            range,
            resolved_binding,
        });
    }

    fn resolve_visible_binding(
        &self,
        scope_id: HirScopeId,
        sigil: &str,
        name: &str,
    ) -> Option<HirBindingId> {
        let mut cursor = Some(scope_id);
        while let Some(current_scope) = cursor {
            for binding in self.scope_graph.bindings.iter().rev() {
                if binding.scope_id == current_scope
                    && binding.sigil == sigil
                    && binding.name == name
                {
                    return Some(binding.id);
                }
            }
            cursor = self
                .scope_graph
                .scopes
                .get(current_scope.index() as usize)
                .and_then(|scope| scope.parent);
        }
        None
    }

    fn visit_declaration_variable_payload(
        &mut self,
        variable: &Node,
        confidence: RecoveryConfidence,
    ) {
        match &variable.kind {
            NodeKind::Assignment { rhs, .. } => self.visit(rhs, confidence),
            NodeKind::VariableWithAttributes { variable, .. } => {
                self.visit_declaration_variable_payload(variable, confidence);
            }
            _ => {}
        }
    }

    fn visit_declaration_list_entries(
        &mut self,
        variables: &[Node],
        confidence: RecoveryConfidence,
    ) {
        for variable in variables {
            if !is_declaration_binding_node(variable) {
                self.visit(variable, confidence);
            }
        }
    }
}

fn storage_class_for_declarator(declarator: &str) -> StorageClass {
    match declarator {
        "my" => StorageClass::LexicalMy,
        "our" => StorageClass::PackageOur,
        "state" => StorageClass::LexicalState,
        "local" => StorageClass::LocalizedPackage,
        _ => StorageClass::PackageGlobal,
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

fn is_declaration_binding_node(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Variable { .. } | NodeKind::Typeglob { .. } => true,
        NodeKind::VariableWithAttributes { variable, .. } => is_declaration_binding_node(variable),
        _ => false,
    }
}
