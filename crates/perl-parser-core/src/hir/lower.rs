//! AST-to-HIR lowering.

use crate::{Node, NodeKind, SourceLocation};
use perl_pragma::{CompileTimePragmaEnvironment, PragmaSnapshot};
use perl_semantic_facts::AnchorId;

use super::model::{
    AstAnchor, BarewordExpr, BarewordFact, BarewordRole, BarewordTable, Binding, BindingReference,
    BlockShell, CallExpr, CallForm, CompileConfidence, CompileDirective, CompileDirectiveAction,
    CompileDirectiveKind, CompileEnvironment, CompileEnvironmentBoundary,
    CompileEnvironmentBoundaryKind, CompilePhase, CompilePhaseBlock, CompileProvenance,
    DynamicBoundary, DynamicBoundaryKind, ExportDeclaration, ExportDeclarationKind, GlobSlot,
    GlobSlotKind, GlobSlotSource, HirBindingId, HirFile, HirId, HirItem, HirKind, HirScopeId,
    IncRootAction, IncRootFact, IncRootKind, IndirectCallExpr, InheritanceSource, LiteralExpr,
    LiteralKind, MethodCallExpr, MethodDecl, ModuleRequest, ModuleRequestKind,
    ModuleResolutionStatus, PackageDecl, PackageInheritanceEdge, PackageStash, PragmaArgumentKind,
    PragmaEffect, PragmaStateFact, PrototypeFact, PrototypeTable, RecoveryConfidence, RequireDecl,
    ScopeFrame, ScopeGraph, ScopeKind, StashConfidence, StashDynamicBoundary,
    StashDynamicBoundaryKind, StashGraph, StashProvenance, StorageClass, SubDecl, UseDecl,
    VariableBinding, VariableDecl,
};

/// Lower a parser AST into first-slice HIR items.
///
/// This is intentionally conservative: it emits only package, subroutine,
/// method, use, require, variable-declaration, and expression-shell items. It
/// records local scope, stash, and compile-environment side graphs, but it does
/// not change provider behavior.
pub fn lower_ast(ast: &Node) -> HirFile {
    let pragma_environment = CompileTimePragmaEnvironment::build(ast);
    let mut lowerer = Lowerer::new(ast.location, pragma_environment);
    lowerer.visit(ast, RecoveryConfidence::Parsed);
    lowerer.record_pragma_state_facts();
    lowerer.finish()
}

struct Lowerer {
    items: Vec<HirItem>,
    next_id: u32,
    package_context: Option<String>,
    scope_graph: ScopeGraph,
    stash_graph: StashGraph,
    compile_environment: CompileEnvironment,
    prototype_table: PrototypeTable,
    bareword_table: BarewordTable,
    bareword_context: BarewordContext,
    pragma_environment: CompileTimePragmaEnvironment,
    scope_stack: Vec<HirScopeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarewordContext {
    Expression,
    ModuleRequest,
    MethodReceiver,
    IndirectObject,
    HashKey,
}

impl Lowerer {
    fn new(file_range: SourceLocation, pragma_environment: CompileTimePragmaEnvironment) -> Self {
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
            stash_graph: StashGraph::default(),
            compile_environment: CompileEnvironment::default(),
            prototype_table: PrototypeTable::default(),
            bareword_table: BarewordTable::default(),
            bareword_context: BarewordContext::Expression,
            pragma_environment,
            scope_stack: vec![file_scope],
        }
    }

    fn finish(self) -> HirFile {
        HirFile {
            items: self.items,
            scope_graph: self.scope_graph,
            stash_graph: self.stash_graph,
            compile_environment: self.compile_environment,
            prototype_table: self.prototype_table,
            bareword_table: self.bareword_table,
        }
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
                let item_id = self.push_item(
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
                self.record_package_declaration(name.clone(), node.location, item_id);

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
                let item_id = self.push_item(
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
                if let Some(name) = name {
                    if let Some(prototype) = prototype {
                        if let NodeKind::Prototype { content } = &prototype.kind {
                            self.prototype_table.facts.push(PrototypeFact {
                                sub_name: name.clone(),
                                package_context: self.package_context.clone(),
                                content: content.clone(),
                                range: prototype.location,
                                declaration_range: node.location,
                                declaration_item: item_id,
                                scope_id: Some(sub_scope),
                                anchor_id: AnchorId(prototype.location.start as u64),
                                provenance: CompileProvenance::ExactAst,
                                confidence: CompileConfidence::High,
                            });
                        }
                    }
                    let source = if has_empty_prototype(prototype.as_deref()) {
                        GlobSlotSource::ConstantDeclaration
                    } else {
                        GlobSlotSource::SubDeclaration
                    };
                    self.record_code_slot(
                        name,
                        (*name_span).unwrap_or(node.location),
                        item_id,
                        source,
                    );
                    if name == "AUTOLOAD" {
                        let boundary_item = self.push_item(
                            node,
                            *name_span,
                            confidence,
                            HirKind::DynamicBoundary(DynamicBoundary {
                                kind: DynamicBoundaryKind::Autoload,
                                reason: "AUTOLOAD declaration makes method dispatch dynamic"
                                    .to_string(),
                            }),
                            self.package_context.clone(),
                            Some(sub_scope),
                        );
                        self.record_dynamic_stash_boundary(
                            Some(self.current_package_name()),
                            Some(name.clone()),
                            node.location,
                            Some(boundary_item),
                            StashDynamicBoundaryKind::Autoload,
                            "AUTOLOAD declaration makes method dispatch dynamic",
                        );
                    }
                }
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
                let item_id = self.push_item(
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
                self.record_slot(
                    self.current_package_name(),
                    stash_slot(
                        name.clone(),
                        GlobSlotKind::Code,
                        node.location,
                        Some(item_id),
                        GlobSlotSource::MethodDeclaration,
                        None,
                        StashProvenance::ExactAst,
                        StashConfidence::High,
                    ),
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
                let item_id = self.push_item(
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
                self.record_compile_directive(
                    CompileDirectiveAction::Use,
                    Some(module.clone()),
                    args.clone(),
                    node.location,
                    Some(item_id),
                    compile_directive_kind(module),
                );
                self.record_use_compile_effects(module, args, node.location, Some(item_id));
                self.record_use_stash_effects(module, args, node.location, item_id);
            }
            NodeKind::No { module, args, has_filter_risk: _ } => {
                self.record_compile_directive(
                    CompileDirectiveAction::No,
                    Some(module.clone()),
                    args.clone(),
                    node.location,
                    None,
                    compile_directive_kind(module),
                );
                self.record_no_compile_effects(module, args, node.location);
            }
            NodeKind::FunctionCall { name, args } if name == "require" => {
                let target = require_target(args.first());
                let item_id = self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::RequireDecl(RequireDecl {
                        target: target.clone(),
                        arg_count: args.len(),
                    }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
                self.record_compile_directive(
                    CompileDirectiveAction::Require,
                    target.clone(),
                    Vec::new(),
                    node.location,
                    Some(item_id),
                    if target.is_some() {
                        CompileDirectiveKind::Module
                    } else {
                        CompileDirectiveKind::Dynamic
                    },
                );
                self.record_require_compile_effect(target, node.location, Some(item_id));
                self.visit_require_args(args, confidence);
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
                self.visit_identifier_with_bareword_context(
                    object,
                    confidence,
                    BarewordContext::MethodReceiver,
                );
                for arg in args {
                    self.visit(arg, confidence);
                }
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
                self.visit_identifier_with_bareword_context(
                    object,
                    confidence,
                    BarewordContext::IndirectObject,
                );
                for arg in args {
                    self.visit(arg, confidence);
                }
            }
            NodeKind::Identifier { name } => {
                let item_id = self.push_item(
                    node,
                    Some(node.location),
                    confidence,
                    HirKind::BarewordExpr(BarewordExpr { name: name.clone() }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
                self.bareword_table.facts.push(BarewordFact {
                    name: name.clone(),
                    role: self.classify_bareword(name),
                    package_context: self.package_context.clone(),
                    range: node.location,
                    source_item: item_id,
                    scope_id: Some(self.current_scope()),
                    anchor_id: AnchorId(node.location.start as u64),
                    provenance: CompileProvenance::ExactAst,
                    confidence: CompileConfidence::High,
                });
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
                for element in elements {
                    self.visit(element, confidence);
                }
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
                for (key, value) in pairs {
                    self.visit_identifier_with_bareword_context(
                        key,
                        confidence,
                        BarewordContext::HashKey,
                    );
                    self.visit(value, confidence);
                }
            }
            NodeKind::Assignment { lhs, rhs, op } => {
                if op == "=" {
                    self.record_assignment_stash_effect(lhs, rhs, node.location, confidence);
                }
                self.visit_children(node, confidence);
            }
            NodeKind::Unary { op, operand } => {
                if is_symbolic_reference_deref_op(op)
                    && !self.strict_refs_enabled_at(node.location.start)
                {
                    let reason = "symbolic reference dereference is not statically known";
                    let item_id = self.push_item(
                        node,
                        None,
                        confidence,
                        HirKind::DynamicBoundary(DynamicBoundary {
                            kind: DynamicBoundaryKind::SymbolicReferenceDeref,
                            reason: reason.to_string(),
                        }),
                        self.package_context.clone(),
                        Some(self.current_scope()),
                    );
                    self.record_compile_environment_boundary(
                        CompileEnvironmentBoundaryKind::SymbolicReferenceDeref,
                        node.location,
                        Some(item_id),
                        reason,
                    );
                }
                self.visit(operand, confidence);
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
                self.record_variable_stash_effects(
                    declarator,
                    &variables,
                    initializer.as_deref(),
                    item_id,
                );
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
            NodeKind::PhaseBlock { phase, block, .. } => {
                self.enter_scope(
                    ScopeKind::PhaseBlock,
                    node.location,
                    self.package_context.clone(),
                );
                self.record_phase_block(phase, node.location);
                self.visit(block, confidence);
                self.exit_scope();
            }
            NodeKind::Format { name, .. } => {
                self.record_slot(
                    self.current_package_name(),
                    stash_slot(
                        name.clone(),
                        GlobSlotKind::Format,
                        node.location,
                        None,
                        GlobSlotSource::FormatDeclaration,
                        None,
                        StashProvenance::ExactAst,
                        StashConfidence::High,
                    ),
                );
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

    fn visit_identifier_with_bareword_context(
        &mut self,
        node: &Node,
        confidence: RecoveryConfidence,
        context: BarewordContext,
    ) {
        if !matches!(node.kind, NodeKind::Identifier { .. }) {
            self.visit(node, confidence);
            return;
        }

        let previous = self.bareword_context;
        self.bareword_context = context;
        self.visit(node, confidence);
        self.bareword_context = previous;
    }

    fn visit_require_args(&mut self, args: &[Node], confidence: RecoveryConfidence) {
        let Some((first, rest)) = args.split_first() else {
            return;
        };

        self.visit_identifier_with_bareword_context(
            first,
            confidence,
            BarewordContext::ModuleRequest,
        );
        for arg in rest {
            self.visit(arg, confidence);
        }
    }

    fn classify_bareword(&self, name: &str) -> BarewordRole {
        match self.bareword_context {
            BarewordContext::ModuleRequest => BarewordRole::ModuleRequest,
            BarewordContext::MethodReceiver => BarewordRole::MethodReceiver,
            BarewordContext::IndirectObject => BarewordRole::IndirectObject,
            BarewordContext::HashKey => BarewordRole::HashKey,
            BarewordContext::Expression if name.contains("::") => BarewordRole::QualifiedName,
            BarewordContext::Expression => BarewordRole::Expression,
        }
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

    fn record_package_declaration(
        &mut self,
        package: String,
        range: SourceLocation,
        item_id: HirId,
    ) {
        let index = self.ensure_package(package.clone(), range, Some(item_id));
        let stash = &mut self.stash_graph.packages[index];
        if stash.declaration_item.is_none() {
            stash.declaration_item = Some(item_id);
            stash.range = range;
        }
    }

    fn ensure_package(
        &mut self,
        package: String,
        range: SourceLocation,
        declaration_item: Option<HirId>,
    ) -> usize {
        if let Some(index) =
            self.stash_graph.packages.iter().position(|stash| stash.package == package)
        {
            return index;
        }
        let index = self.stash_graph.packages.len();
        self.stash_graph.packages.push(PackageStash {
            package,
            range,
            declaration_item,
            slots: Vec::new(),
            provenance: StashProvenance::ExactAst,
            confidence: StashConfidence::High,
        });
        index
    }

    fn current_package_name(&self) -> String {
        self.package_context.clone().unwrap_or_else(|| "main".to_string())
    }

    fn record_code_slot(
        &mut self,
        name: &str,
        range: SourceLocation,
        item_id: HirId,
        source: GlobSlotSource,
    ) {
        let (package, symbol) = package_and_symbol(name, self.package_context.as_deref());
        self.record_slot(
            package,
            stash_slot(
                symbol,
                GlobSlotKind::Code,
                range,
                Some(item_id),
                source,
                None,
                StashProvenance::ExactAst,
                StashConfidence::High,
            ),
        );
    }

    fn record_slot(&mut self, package: String, slot: GlobSlot) {
        let package_index = self.ensure_package(package, slot.range, None);
        self.stash_graph.packages[package_index].slots.push(slot);
    }

    fn record_variable_stash_effects(
        &mut self,
        declarator: &str,
        variables: &[VariableBinding],
        initializer: Option<&Node>,
        item_id: HirId,
    ) {
        if declarator != "our" {
            return;
        }
        for variable in variables {
            if let Some(kind) = slot_kind_for_sigil(&variable.sigil) {
                let (package, symbol) =
                    package_and_symbol(&variable.name, self.package_context.as_deref());
                self.record_slot(
                    package.clone(),
                    stash_slot(
                        symbol.clone(),
                        kind,
                        variable.range,
                        Some(item_id),
                        GlobSlotSource::OurDeclaration,
                        None,
                        StashProvenance::ExactAst,
                        StashConfidence::High,
                    ),
                );
                if variable.sigil == "@" && symbol == "ISA" {
                    self.record_isa_edges_from_node(
                        initializer,
                        package.clone(),
                        variable.range,
                        Some(item_id),
                        InheritanceSource::IsaAssignment,
                    );
                }
                self.record_export_declarations_from_node(
                    package,
                    &symbol,
                    &variable.sigil,
                    initializer,
                    variable.range,
                    Some(item_id),
                );
            }
        }
    }

    fn record_compile_directive(
        &mut self,
        action: CompileDirectiveAction,
        module: Option<String>,
        args: Vec<String>,
        range: SourceLocation,
        item_id: Option<HirId>,
        kind: CompileDirectiveKind,
    ) {
        self.compile_environment.directives.push(CompileDirective {
            action,
            module,
            args,
            range,
            item_id,
            scope_id: Some(self.current_scope()),
            package_context: self.package_context.clone(),
            kind,
            provenance: CompileProvenance::ExactAst,
            confidence: CompileConfidence::High,
        });
    }

    fn record_use_compile_effects(
        &mut self,
        module: &str,
        args: &[String],
        range: SourceLocation,
        item_id: Option<HirId>,
    ) {
        match module {
            "strict" | "warnings" | "feature" => {
                self.record_pragma_effect(module, true, args, range, item_id);
            }
            "lib" => {
                self.record_inc_root_effects(args, IncRootAction::Add, range, item_id);
            }
            "parent" => {
                self.record_module_request(
                    Some(module.to_string()),
                    ModuleRequestKind::Use,
                    range,
                    item_id,
                );
                for parent in static_package_args(args) {
                    self.record_module_request(
                        Some(parent),
                        ModuleRequestKind::Parent,
                        range,
                        item_id,
                    );
                }
            }
            "base" => {
                self.record_module_request(
                    Some(module.to_string()),
                    ModuleRequestKind::Use,
                    range,
                    item_id,
                );
                for parent in static_package_args(args) {
                    self.record_module_request(
                        Some(parent),
                        ModuleRequestKind::Base,
                        range,
                        item_id,
                    );
                }
            }
            _ => {
                self.record_module_request(
                    Some(module.to_string()),
                    ModuleRequestKind::Use,
                    range,
                    item_id,
                );
            }
        }
    }

    fn record_no_compile_effects(&mut self, module: &str, args: &[String], range: SourceLocation) {
        match module {
            "strict" | "warnings" | "feature" => {
                self.record_pragma_effect(module, false, args, range, None);
            }
            "lib" => {
                self.record_inc_root_effects(args, IncRootAction::Remove, range, None);
            }
            _ => {}
        }
    }

    fn record_require_compile_effect(
        &mut self,
        target: Option<String>,
        range: SourceLocation,
        item_id: Option<HirId>,
    ) {
        let status = if target.is_some() {
            ModuleResolutionStatus::Deferred
        } else {
            ModuleResolutionStatus::Dynamic
        };
        self.compile_environment.module_requests.push(ModuleRequest {
            target: target.clone(),
            kind: ModuleRequestKind::Require,
            range,
            directive_item: item_id,
            scope_id: Some(self.current_scope()),
            package_context: self.package_context.clone(),
            resolution: status,
            provenance: CompileProvenance::ExactAst,
            confidence: if target.is_some() {
                CompileConfidence::High
            } else {
                CompileConfidence::Low
            },
        });
        if target.is_none() {
            self.record_compile_environment_boundary(
                CompileEnvironmentBoundaryKind::DynamicRequire,
                range,
                item_id,
                "require target is not statically known",
            );
        }
    }

    fn record_pragma_effect(
        &mut self,
        pragma: &str,
        enabled: bool,
        args: &[String],
        range: SourceLocation,
        item_id: Option<HirId>,
    ) {
        let Some((argument_kind, args)) = static_pragma_args(pragma, args) else {
            self.record_compile_environment_boundary(
                CompileEnvironmentBoundaryKind::DynamicPragmaArgs,
                range,
                item_id,
                "pragma arguments are not statically known",
            );
            return;
        };

        self.compile_environment.pragma_effects.push(PragmaEffect {
            pragma: pragma.to_string(),
            enabled,
            args,
            argument_kind,
            range,
            directive_item: item_id,
            scope_id: Some(self.current_scope()),
            package_context: self.package_context.clone(),
            provenance: CompileProvenance::ExactAst,
            confidence: CompileConfidence::High,
        });
    }

    fn record_pragma_state_facts(&mut self) {
        let entries = self.pragma_environment.map().entries().to_vec();
        for entry in entries {
            let range = SourceLocation::new(entry.range.start, entry.range.end);
            if self.is_dynamic_pragma_offset(range.start) {
                continue;
            }

            let (directive_item, scope_id, package_context) =
                self.compile_environment_metadata_at(range.start);
            self.compile_environment.pragma_state_facts.push(pragma_state_fact(
                range,
                &entry.snapshot,
                directive_item,
                scope_id,
                package_context,
            ));
        }
    }

    fn strict_refs_enabled_at(&self, offset: usize) -> bool {
        self.pragma_environment.snapshot_at(offset).state().strict_refs
    }

    fn is_dynamic_pragma_offset(&self, offset: usize) -> bool {
        self.compile_environment.dynamic_boundaries.iter().any(|boundary| {
            boundary.kind == CompileEnvironmentBoundaryKind::DynamicPragmaArgs
                && boundary.range.start == offset
        })
    }

    fn compile_environment_metadata_at(
        &self,
        offset: usize,
    ) -> (Option<HirId>, Option<HirScopeId>, Option<String>) {
        if let Some(effect) = self
            .compile_environment
            .pragma_effects
            .iter()
            .find(|effect| effect.range.start == offset)
        {
            return (effect.directive_item, effect.scope_id, effect.package_context.clone());
        }

        if let Some(directive) = self
            .compile_environment
            .directives
            .iter()
            .find(|directive| directive.range.start == offset)
        {
            return (directive.item_id, directive.scope_id, directive.package_context.clone());
        }

        if let Some(scope) = self.innermost_scope_at(offset) {
            return (None, Some(scope.id), scope.package_context.clone());
        }

        (None, None, self.package_context.clone())
    }

    fn innermost_scope_at(&self, offset: usize) -> Option<&ScopeFrame> {
        self.scope_graph
            .scopes
            .iter()
            .filter(|scope| scope.range.start <= offset && offset <= scope.range.end)
            .max_by_key(|scope| (scope.range.start, scope.id.index()))
    }

    fn record_inc_root_effects(
        &mut self,
        args: &[String],
        action: IncRootAction,
        range: SourceLocation,
        item_id: Option<HirId>,
    ) {
        let paths = static_path_args(args);
        if paths.is_empty() {
            self.record_compile_environment_boundary(
                CompileEnvironmentBoundaryKind::DynamicIncRoot,
                range,
                item_id,
                "include root arguments are not statically known",
            );
            return;
        }

        for path in paths {
            self.compile_environment.inc_roots.push(IncRootFact {
                path,
                action,
                kind: IncRootKind::UseLib,
                range,
                directive_item: item_id,
                scope_id: Some(self.current_scope()),
                package_context: self.package_context.clone(),
                provenance: CompileProvenance::ExactAst,
                confidence: CompileConfidence::High,
            });
        }
    }

    fn record_module_request(
        &mut self,
        target: Option<String>,
        kind: ModuleRequestKind,
        range: SourceLocation,
        item_id: Option<HirId>,
    ) {
        self.compile_environment.module_requests.push(ModuleRequest {
            target,
            kind,
            range,
            directive_item: item_id,
            scope_id: Some(self.current_scope()),
            package_context: self.package_context.clone(),
            resolution: ModuleResolutionStatus::Deferred,
            provenance: CompileProvenance::ExactAst,
            confidence: CompileConfidence::High,
        });
    }

    fn record_phase_block(&mut self, phase: &str, range: SourceLocation) {
        self.compile_environment.phase_blocks.push(CompilePhaseBlock {
            phase: compile_phase(phase),
            range,
            scope_id: Some(self.current_scope()),
            package_context: self.package_context.clone(),
            provenance: CompileProvenance::ExactAst,
            confidence: CompileConfidence::High,
        });
        self.record_compile_environment_boundary(
            CompileEnvironmentBoundaryKind::PhaseBlockExecution,
            range,
            None,
            "phase block compile-time execution is recorded but not evaluated",
        );
    }

    fn record_compile_environment_boundary(
        &mut self,
        kind: CompileEnvironmentBoundaryKind,
        range: SourceLocation,
        item_id: Option<HirId>,
        reason: &str,
    ) {
        self.compile_environment.dynamic_boundaries.push(CompileEnvironmentBoundary {
            kind,
            range,
            boundary_item: item_id,
            scope_id: Some(self.current_scope()),
            package_context: self.package_context.clone(),
            reason: reason.to_string(),
            provenance: CompileProvenance::DynamicBoundary,
            confidence: CompileConfidence::Low,
        });
    }

    fn record_use_stash_effects(
        &mut self,
        module: &str,
        args: &[String],
        range: SourceLocation,
        item_id: HirId,
    ) {
        match module {
            "parent" => {
                for parent in static_package_args(args) {
                    self.record_inheritance_edge(
                        self.current_package_name(),
                        parent,
                        range,
                        Some(item_id),
                        InheritanceSource::UseParent,
                        StashProvenance::DesugaredAst,
                    );
                }
            }
            "base" => {
                for parent in static_package_args(args) {
                    self.record_inheritance_edge(
                        self.current_package_name(),
                        parent,
                        range,
                        Some(item_id),
                        InheritanceSource::UseBase,
                        StashProvenance::DesugaredAst,
                    );
                }
            }
            "constant" => {
                for constant in constant_names_from_use_args(args) {
                    self.record_slot(
                        self.current_package_name(),
                        stash_slot(
                            constant,
                            GlobSlotKind::Code,
                            range,
                            Some(item_id),
                            GlobSlotSource::ConstantDeclaration,
                            None,
                            StashProvenance::DesugaredAst,
                            StashConfidence::High,
                        ),
                    );
                }
            }
            _ => {}
        }
    }

    fn record_assignment_stash_effect(
        &mut self,
        lhs: &Node,
        rhs: &Node,
        range: SourceLocation,
        confidence: RecoveryConfidence,
    ) {
        match &lhs.kind {
            NodeKind::Typeglob { name } => {
                let (package, symbol) = package_and_symbol(name, self.package_context.as_deref());
                if let Some(alias_target) = static_code_alias_target(rhs) {
                    self.record_slot(
                        package,
                        stash_slot(
                            symbol,
                            GlobSlotKind::Code,
                            lhs.location,
                            None,
                            GlobSlotSource::TypeglobAlias,
                            Some(alias_target),
                            StashProvenance::ExactAst,
                            StashConfidence::Medium,
                        ),
                    );
                } else {
                    let boundary_item = self.push_item(
                        lhs,
                        Some(lhs.location),
                        confidence,
                        HirKind::DynamicBoundary(DynamicBoundary {
                            kind: DynamicBoundaryKind::DynamicStashMutation,
                            reason: "typeglob assignment has a non-static RHS".to_string(),
                        }),
                        self.package_context.clone(),
                        Some(self.current_scope()),
                    );
                    self.record_dynamic_stash_boundary(
                        Some(package),
                        Some(symbol),
                        range,
                        Some(boundary_item),
                        StashDynamicBoundaryKind::DynamicStashMutation,
                        "typeglob assignment has a non-static RHS",
                    );
                }
            }
            NodeKind::Variable { sigil, name } if sigil == "@" => {
                let (package, symbol) = package_and_symbol(name, self.package_context.as_deref());
                if symbol == "ISA" {
                    self.record_slot(
                        package.clone(),
                        stash_slot(
                            symbol.clone(),
                            GlobSlotKind::Array,
                            lhs.location,
                            None,
                            GlobSlotSource::PackageAssignment,
                            None,
                            StashProvenance::ExactAst,
                            StashConfidence::High,
                        ),
                    );
                    self.record_isa_edges_from_node(
                        Some(rhs),
                        package.clone(),
                        range,
                        None,
                        InheritanceSource::IsaAssignment,
                    );
                }
                if matches!(symbol.as_str(), "EXPORT" | "EXPORT_OK") {
                    self.record_slot(
                        package.clone(),
                        stash_slot(
                            symbol.clone(),
                            GlobSlotKind::Array,
                            lhs.location,
                            None,
                            GlobSlotSource::PackageAssignment,
                            None,
                            StashProvenance::ExactAst,
                            StashConfidence::High,
                        ),
                    );
                    self.record_export_declarations_from_node(
                        package,
                        &symbol,
                        sigil,
                        Some(rhs),
                        range,
                        None,
                    );
                }
            }
            NodeKind::Variable { sigil, name } if sigil == "%" => {
                let (package, symbol) = package_and_symbol(name, self.package_context.as_deref());
                if symbol == "EXPORT_TAGS" {
                    self.record_slot(
                        package.clone(),
                        stash_slot(
                            symbol.clone(),
                            GlobSlotKind::Hash,
                            lhs.location,
                            None,
                            GlobSlotSource::PackageAssignment,
                            None,
                            StashProvenance::ExactAst,
                            StashConfidence::High,
                        ),
                    );
                    self.record_export_declarations_from_node(
                        package,
                        &symbol,
                        sigil,
                        Some(rhs),
                        range,
                        None,
                    );
                }
            }
            _ => {}
        }
    }

    fn record_export_declarations_from_node(
        &mut self,
        package: String,
        symbol: &str,
        sigil: &str,
        initializer: Option<&Node>,
        range: SourceLocation,
        item_id: Option<HirId>,
    ) {
        let Some(initializer) = initializer else {
            return;
        };

        match (sigil, symbol) {
            ("@", "EXPORT") => {
                self.record_export_symbol_declaration(
                    package,
                    ExportDeclarationKind::Default,
                    initializer,
                    range,
                    item_id,
                );
            }
            ("@", "EXPORT_OK") => {
                self.record_export_symbol_declaration(
                    package,
                    ExportDeclarationKind::Optional,
                    initializer,
                    range,
                    item_id,
                );
            }
            ("%", "EXPORT_TAGS") => {
                self.record_export_tag_declarations(package, initializer, range, item_id);
            }
            _ => {}
        }
    }

    fn record_export_symbol_declaration(
        &mut self,
        package: String,
        kind: ExportDeclarationKind,
        initializer: &Node,
        range: SourceLocation,
        item_id: Option<HirId>,
    ) {
        if let Some(symbols) = static_export_symbols_from_node(initializer) {
            self.stash_graph.export_declarations.push(ExportDeclaration {
                package,
                kind,
                tag_name: None,
                symbols,
                range,
                declaration_item: item_id,
                provenance: StashProvenance::ExactAst,
                confidence: StashConfidence::High,
            });
        } else {
            self.record_dynamic_stash_boundary(
                Some(package),
                Some(export_declaration_symbol(kind).to_string()),
                range,
                item_id,
                StashDynamicBoundaryKind::DynamicExportDeclaration,
                "export declaration has non-static members",
            );
        }
    }

    fn record_export_tag_declarations(
        &mut self,
        package: String,
        initializer: &Node,
        range: SourceLocation,
        item_id: Option<HirId>,
    ) {
        if let Some(tags) = static_export_tags_from_node(initializer) {
            for (tag_name, symbols) in tags {
                self.stash_graph.export_declarations.push(ExportDeclaration {
                    package: package.clone(),
                    kind: ExportDeclarationKind::Tag,
                    tag_name: Some(tag_name),
                    symbols,
                    range,
                    declaration_item: item_id,
                    provenance: StashProvenance::ExactAst,
                    confidence: StashConfidence::High,
                });
            }
        } else {
            self.record_dynamic_stash_boundary(
                Some(package),
                Some("EXPORT_TAGS".to_string()),
                range,
                item_id,
                StashDynamicBoundaryKind::DynamicExportDeclaration,
                "export tag declaration has non-static members",
            );
        }
    }

    fn record_isa_edges_from_node(
        &mut self,
        node: Option<&Node>,
        package: String,
        range: SourceLocation,
        item_id: Option<HirId>,
        source: InheritanceSource,
    ) {
        if let Some(node) = node {
            for parent in static_package_names_from_node(node) {
                self.record_inheritance_edge(
                    package.clone(),
                    parent,
                    range,
                    item_id,
                    source,
                    StashProvenance::ExactAst,
                );
            }
        }
    }

    fn record_inheritance_edge(
        &mut self,
        from_package: String,
        to_package: String,
        range: SourceLocation,
        declaration_item: Option<HirId>,
        source: InheritanceSource,
        provenance: StashProvenance,
    ) {
        self.ensure_package(from_package.clone(), range, None);
        self.ensure_package(to_package.clone(), range, None);
        self.stash_graph.inheritance_edges.push(PackageInheritanceEdge {
            from_package,
            to_package,
            range,
            declaration_item,
            source,
            provenance,
            confidence: StashConfidence::High,
        });
    }

    fn record_dynamic_stash_boundary(
        &mut self,
        package: Option<String>,
        symbol: Option<String>,
        range: SourceLocation,
        boundary_item: Option<HirId>,
        kind: StashDynamicBoundaryKind,
        reason: &str,
    ) {
        if let Some(package) = &package {
            self.ensure_package(package.clone(), range, None);
        }
        self.stash_graph.dynamic_boundaries.push(StashDynamicBoundary {
            package,
            symbol,
            range,
            boundary_item,
            kind,
            reason: reason.to_string(),
            provenance: StashProvenance::DynamicBoundary,
            confidence: StashConfidence::Low,
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

fn slot_kind_for_sigil(sigil: &str) -> Option<GlobSlotKind> {
    match sigil {
        "$" => Some(GlobSlotKind::Scalar),
        "@" => Some(GlobSlotKind::Array),
        "%" => Some(GlobSlotKind::Hash),
        "&" => Some(GlobSlotKind::Code),
        _ => None,
    }
}

fn stash_slot(
    name: String,
    kind: GlobSlotKind,
    range: SourceLocation,
    declaration_item: Option<HirId>,
    source: GlobSlotSource,
    alias_target: Option<String>,
    provenance: StashProvenance,
    confidence: StashConfidence,
) -> GlobSlot {
    GlobSlot { name, kind, range, declaration_item, source, alias_target, provenance, confidence }
}

fn package_and_symbol(name: &str, package_context: Option<&str>) -> (String, String) {
    if let Some((package, symbol)) = name.rsplit_once("::") {
        let package = if package.is_empty() { "main" } else { package };
        return (package.to_string(), symbol.to_string());
    }

    (package_context.unwrap_or("main").to_string(), name.to_string())
}

// Only an explicit empty prototype marks a sub as constant-like.
fn has_empty_prototype(node: Option<&Node>) -> bool {
    matches!(node.map(|node| &node.kind), Some(NodeKind::Prototype { content }) if content.trim().is_empty())
}

fn static_code_alias_target(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::Unary { op, operand } if op == "\\" => match &operand.kind {
            NodeKind::FunctionCall { name, args } if args.is_empty() => Some(name.clone()),
            NodeKind::Typeglob { name } => Some(name.clone()),
            _ => None,
        },
        NodeKind::Typeglob { name } => Some(name.clone()),
        _ => None,
    }
}

fn export_declaration_symbol(kind: ExportDeclarationKind) -> &'static str {
    match kind {
        ExportDeclarationKind::Default => "EXPORT",
        ExportDeclarationKind::Optional => "EXPORT_OK",
        ExportDeclarationKind::Tag => "EXPORT_TAGS",
    }
}

fn static_export_symbols_from_node(node: &Node) -> Option<Vec<String>> {
    match &node.kind {
        NodeKind::ArrayLiteral { elements } => {
            if elements.len() == 1
                && matches!(
                    elements.first().map(|node| &node.kind),
                    Some(NodeKind::ArrayLiteral { .. })
                )
            {
                return static_export_symbols_from_node(&elements[0]);
            }

            let mut symbols = Vec::new();
            for element in elements {
                symbols.push(static_export_symbol_from_node(element)?);
            }
            Some(symbols)
        }
        NodeKind::String { .. } | NodeKind::Identifier { .. } => {
            static_export_symbol_from_node(node).map(|symbol| vec![symbol])
        }
        _ => None,
    }
}

fn static_export_symbol_from_node(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::String { value, interpolated } => {
            if *interpolated && contains_interpolation_marker(value) {
                return None;
            }
            clean_export_symbol(value)
        }
        NodeKind::Identifier { name } => clean_export_symbol(name),
        _ => None,
    }
}

fn static_export_tags_from_node(node: &Node) -> Option<Vec<(String, Vec<String>)>> {
    match &node.kind {
        NodeKind::HashLiteral { pairs } => {
            let mut tags = Vec::new();
            for (key, value) in pairs {
                tags.push((
                    static_export_tag_name_from_node(key)?,
                    static_export_symbols_from_node(value)?,
                ));
            }
            Some(tags)
        }
        _ => None,
    }
}

fn static_export_tag_name_from_node(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::String { value, interpolated } => {
            if *interpolated && contains_interpolation_marker(value) {
                return None;
            }
            clean_export_tag(value)
        }
        NodeKind::Identifier { name } => clean_export_tag(name),
        _ => None,
    }
}

fn clean_export_symbol(value: &str) -> Option<String> {
    let cleaned = value.trim().trim_matches(',').trim_matches('"').trim_matches('\'');
    if is_export_symbol_name(cleaned) { Some(cleaned.to_string()) } else { None }
}

fn clean_export_tag(value: &str) -> Option<String> {
    let cleaned =
        value.trim().trim_matches(',').trim_matches('"').trim_matches('\'').trim_start_matches(':');
    if is_bareword_like(cleaned) { Some(cleaned.to_string()) } else { None }
}

fn is_export_symbol_name(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };
    let body = if matches!(first, '$' | '@' | '%' | '&' | '*') {
        &value[first.len_utf8()..]
    } else {
        value
    };
    is_bareword_like(body)
}

fn is_bareword_like(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && value.chars().all(|ch| ch == '_' || ch == ':' || ch.is_ascii_alphanumeric())
}

fn contains_interpolation_marker(value: &str) -> bool {
    value.contains('$') || value.contains('@') || value.contains('%')
}

fn is_symbolic_reference_deref_op(op: &str) -> bool {
    matches!(op, "${}" | "@{}" | "%{}" | "&{}" | "*{}")
}

fn pragma_state_fact(
    range: SourceLocation,
    snapshot: &PragmaSnapshot,
    directive_item: Option<HirId>,
    scope_id: Option<HirScopeId>,
    package_context: Option<String>,
) -> PragmaStateFact {
    let state = snapshot.state();
    PragmaStateFact {
        range,
        anchor_id: AnchorId(range.start as u64),
        directive_item,
        scope_id,
        package_context,
        strict_vars: state.strict_vars,
        strict_subs: state.strict_subs,
        strict_refs: state.strict_refs,
        warnings: state.warnings,
        disabled_warning_categories: state.disabled_warning_categories.clone(),
        features: state.features.iter().map(|feature| (*feature).to_string()).collect(),
        provenance: CompileProvenance::ExactAst,
        confidence: CompileConfidence::High,
    }
}

fn static_pragma_args(pragma: &str, args: &[String]) -> Option<(PragmaArgumentKind, Vec<String>)> {
    if args.is_empty() {
        return Some((PragmaArgumentKind::Broad, Vec::new()));
    }

    let mut normalized = Vec::new();
    for arg in args {
        for item in static_pragma_arg_items(arg)? {
            if !is_valid_static_pragma_arg(pragma, &item) {
                return None;
            }
            normalized.push(item);
        }
    }

    if normalized.is_empty() { None } else { Some((PragmaArgumentKind::Categories, normalized)) }
}

fn static_pragma_arg_items(arg: &str) -> Option<Vec<String>> {
    let trimmed = arg.trim().trim_matches(',');
    if trimmed.is_empty() || contains_interpolation_marker(trimmed) {
        return None;
    }

    let unquoted = trimmed.trim_matches('"').trim_matches('\'');
    let body = if let Some(inner) =
        unquoted.strip_prefix("qw(").and_then(|value| value.strip_suffix(')'))
    {
        inner
    } else {
        unquoted
    };

    let items = body.split_whitespace().map(clean_static_pragma_arg).collect::<Option<Vec<_>>>()?;
    if items.is_empty() { None } else { Some(items) }
}

fn clean_static_pragma_arg(arg: &str) -> Option<String> {
    let cleaned = arg.trim().trim_matches(',').trim_matches('"').trim_matches('\'');
    if cleaned.is_empty() || contains_interpolation_marker(cleaned) {
        return None;
    }
    if cleaned.chars().any(|ch| {
        matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '\\' | ';' | '=' | '>' | '<' | '&' | '*')
    }) {
        return None;
    }
    Some(cleaned.to_string())
}

fn is_valid_static_pragma_arg(pragma: &str, arg: &str) -> bool {
    match pragma {
        "strict" => matches!(arg, "vars" | "subs" | "refs"),
        "warnings" | "feature" => true,
        _ => false,
    }
}

fn static_package_args(args: &[String]) -> Vec<String> {
    args.iter()
        .flat_map(|arg| static_names_from_arg(arg))
        .filter(|arg| arg != "-norequire")
        .collect()
}

fn static_path_args(args: &[String]) -> Vec<String> {
    args.iter()
        .flat_map(|arg| {
            let trimmed = arg.trim();
            if let Some(inner) =
                trimmed.strip_prefix("qw(").and_then(|value| value.strip_suffix(')'))
            {
                return inner
                    .split_whitespace()
                    .map(clean_static_path)
                    .filter(|path| !path.is_empty())
                    .collect::<Vec<_>>();
            }
            vec![clean_static_path(trimmed)].into_iter().filter(|path| !path.is_empty()).collect()
        })
        .collect()
}

fn clean_static_path(path: &str) -> String {
    path.trim().trim_matches(',').trim_matches('"').trim_matches('\'').to_string()
}

fn compile_directive_kind(module: &str) -> CompileDirectiveKind {
    match module {
        "strict" => CompileDirectiveKind::Strict,
        "warnings" => CompileDirectiveKind::Warnings,
        "feature" => CompileDirectiveKind::Feature,
        "lib" => CompileDirectiveKind::Lib,
        "parent" | "base" => CompileDirectiveKind::Inheritance,
        "constant" => CompileDirectiveKind::Constant,
        _ => CompileDirectiveKind::Module,
    }
}

fn compile_phase(phase: &str) -> CompilePhase {
    match phase {
        "BEGIN" => CompilePhase::Begin,
        "UNITCHECK" => CompilePhase::UnitCheck,
        "CHECK" => CompilePhase::Check,
        "INIT" => CompilePhase::Init,
        "END" => CompilePhase::End,
        _ => CompilePhase::Unknown,
    }
}

fn constant_names_from_use_args(args: &[String]) -> Vec<String> {
    if args.is_empty() {
        return Vec::new();
    }

    if args.len() == 1 {
        return static_names_from_arg(&args[0])
            .into_iter()
            .filter(|name| is_constant_name(name))
            .collect();
    }

    if args.first().is_some_and(|arg| arg == "{") {
        let mut names = Vec::new();
        for pair in args.windows(2) {
            if pair[1] == "=>" {
                let name = clean_static_name(&pair[0]);
                if is_constant_name(&name) {
                    names.push(name);
                }
            }
        }
        names.sort();
        names.dedup();
        return names;
    }

    let name = clean_static_name(&args[0]);
    if is_constant_name(&name) {
        return vec![name];
    }

    Vec::new()
}

fn static_names_from_arg(arg: &str) -> Vec<String> {
    let trimmed = arg.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if let Some(inner) = trimmed.strip_prefix("qw(").and_then(|value| value.strip_suffix(')')) {
        return inner
            .split_whitespace()
            .map(clean_static_name)
            .filter(|name| !name.is_empty())
            .collect();
    }

    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return trimmed
            .trim_matches(|ch| ch == '{' || ch == '}')
            .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == '=' || ch == '>')
            .map(clean_static_name)
            .filter(|name| is_constant_name(name))
            .collect();
    }

    vec![clean_static_name(trimmed)].into_iter().filter(|name| !name.is_empty()).collect()
}

fn clean_static_name(name: &str) -> String {
    name.trim().trim_matches(',').trim_matches('"').trim_matches('\'').trim_matches(':').to_string()
}

fn is_constant_name(name: &str) -> bool {
    let Some(first) = name.chars().next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && name.chars().all(|ch| ch == '_' || ch.is_ascii_alphanumeric() || ch == ':')
}

fn static_package_names_from_node(node: &Node) -> Vec<String> {
    match &node.kind {
        NodeKind::ArrayLiteral { elements } => {
            elements.iter().flat_map(static_package_names_from_node).collect()
        }
        NodeKind::String { value, .. } | NodeKind::Identifier { name: value } => {
            static_names_from_arg(value)
        }
        _ => Vec::new(),
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
