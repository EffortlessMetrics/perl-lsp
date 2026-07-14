//! AST-to-HIR lowering.

use crate::{Node, NodeKind, SourceLocation};
use perl_pragma::{CompileTimePragmaEnvironment, PragmaSnapshot};
use perl_semantic_facts::AnchorId;
use std::collections::BTreeMap;

use super::body::{
    AccessMode, Arena, AssignMode, BinaryOp, BodyOwner, BodyOwnerKind, BodySourceMap,
    DeclStorageClass, HirBlock, HirBlockId, HirBody, HirBodyId, HirExpr, HirExprId, HirStmt,
    HirStmtId, HirVariable, Sigil, UnaryMode, VariableKind,
};
use super::model::{
    AstAnchor, BarewordExpr, BarewordFact, BarewordRole, BarewordTable, Binding, BindingReference,
    BlockShell, BranchKeyword, BranchShell, CallExpr, CallForm, CompileConfidence,
    CompileDirective, CompileDirectiveAction, CompileDirectiveKind, CompileEnvironment,
    CompileEnvironmentBoundary, CompileEnvironmentBoundaryKind, CompilePhase, CompilePhaseBlock,
    CompileProvenance, ControlTransfer, ControlTransferKind, DynamicBoundary, DynamicBoundaryKind,
    ExportDeclaration, ExportDeclarationKind, GlobSlot, GlobSlotKind, GlobSlotSource,
    HIR_BODY_MODEL_VERSION, HirBindingId, HirFile, HirId, HirItem, HirKind, HirScopeId,
    IncRootAction, IncRootFact, IncRootKind, IndirectCallExpr, InheritanceSource, LiteralExpr,
    LiteralKind, LoopKind, LoopShell, MethodCallExpr, MethodDecl, ModuleRequest, ModuleRequestKind,
    ModuleResolutionStatus, PackageDecl, PackageInheritanceEdge, PackageStash, PragmaArgumentKind,
    PragmaEffect, PragmaStateFact, PrototypeFact, PrototypeTable, RecoveryConfidence, RequireDecl,
    ScopeFrame, ScopeGraph, ScopeKind, StashConfidence, StashDynamicBoundary,
    StashDynamicBoundaryKind, StashGraph, StashProvenance, StatementModifierKind,
    StatementModifierShell, StorageClass, SubDecl, UseDecl, VariableBinding, VariableDecl,
};

/// Lower a parser AST into first-slice HIR items plus canonical body arenas.
///
/// This is intentionally conservative: it emits only package, subroutine,
/// method, use, require, variable-declaration, and expression-shell items. It
/// records local scope, stash, and compile-environment side graphs. A second
/// pass lowers body arenas and attaches them to [`HirFile::bodies`].
pub fn lower_ast(ast: &Node) -> HirFile {
    let pragma_environment = CompileTimePragmaEnvironment::build(ast);
    let mut lowerer = Lowerer::new(ast.location, pragma_environment);
    lowerer.visit(ast, RecoveryConfidence::Parsed);
    lowerer.record_pragma_state_facts();
    let mut file = lowerer.finish();
    // Second pass: lower bodies and attach to HirFile.
    // body_model_version is set AFTER the second pass succeeds so that
    // consumers see version==1 only when bodies are actually attached.
    lower_bodies_into_file(ast, &mut file);
    file.body_model_version = HIR_BODY_MODEL_VERSION;
    file
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
    /// Label inherited from an enclosing `LABEL:` statement, consumed by the
    /// loop it directly wraps.
    pending_label: Option<String>,
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
            pending_label: None,
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
            bodies: Vec::new(),
            body_owners: BTreeMap::new(),
            // Version 0 = bodies not yet attached. lower_ast() sets this to
            // HIR_BODY_MODEL_VERSION after lower_bodies_into_file() returns.
            body_model_version: 0,
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
                // A `LABEL: { ... }` labeled bare block absorbs the pending
                // label here.  `BlockShell` does not carry a label field, so if
                // the pending label were left alive it would silently propagate
                // to the first loop found inside the block — which is the thing
                // *inside* the labeled block, not the labeled block itself.
                // Drop it so the loop gets no spurious label.
                let _ = self.pending_label.take();
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
            NodeKind::Subroutine {
                name,
                name_span,
                prototype,
                signature,
                attributes,
                body,
                ..
            } => {
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
            NodeKind::Method { name, name_span: _, signature, attributes, body } => {
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
                self.record_phase_execution_boundary(phase, block, node.location);
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
            NodeKind::If { condition, elsif_branches, else_branch, keyword, .. } => {
                let keyword = match keyword.as_deref() {
                    Some("unless") => BranchKeyword::Unless,
                    _ => BranchKeyword::If,
                };
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::BranchShell(BranchShell {
                        keyword,
                        condition_range: condition.location,
                        elsif_count: elsif_branches.len(),
                        has_else: else_branch.is_some(),
                    }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::Ternary { condition, .. } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::BranchShell(BranchShell {
                        keyword: BranchKeyword::Ternary,
                        condition_range: condition.location,
                        elsif_count: 0,
                        has_else: true,
                    }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::While { continue_block, keyword, .. } => {
                let kind = match keyword.as_deref() {
                    Some("until") => LoopKind::Until,
                    _ => LoopKind::While,
                };
                let label = self.pending_label.take();
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::LoopShell(LoopShell {
                        kind,
                        has_condition: true,
                        has_continue: continue_block.is_some(),
                        declares_iterator: false,
                        label,
                    }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::For { init, condition, continue_block, .. } => {
                let label = self.pending_label.take();
                let declares_iterator = init.as_deref().is_some_and(is_lexical_declaration_node);
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::LoopShell(LoopShell {
                        kind: LoopKind::CStyleFor,
                        has_condition: condition.is_some(),
                        has_continue: continue_block.is_some(),
                        declares_iterator,
                        label,
                    }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::Foreach { variable, continue_block, .. } => {
                let label = self.pending_label.take();
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::LoopShell(LoopShell {
                        kind: LoopKind::Foreach,
                        has_condition: false,
                        has_continue: continue_block.is_some(),
                        declares_iterator: is_lexical_declaration_node(variable),
                        label,
                    }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::LabeledStatement { label, statement } => {
                let previous = self.pending_label.replace(label.clone());
                self.visit(statement, confidence);
                self.pending_label = previous;
            }
            NodeKind::Return { value } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::ControlTransfer(ControlTransfer {
                        kind: ControlTransferKind::Return,
                        label: None,
                        has_value: value.is_some(),
                    }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
                self.visit_children(node, confidence);
            }
            NodeKind::LoopControl { op, label } => {
                let kind = match op.as_str() {
                    "last" => ControlTransferKind::Last,
                    "redo" => ControlTransferKind::Redo,
                    _ => ControlTransferKind::Next,
                };
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::ControlTransfer(ControlTransfer {
                        kind,
                        label: label.clone(),
                        has_value: false,
                    }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
            }
            NodeKind::Goto { target, .. } => {
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::ControlTransfer(ControlTransfer {
                        kind: ControlTransferKind::Goto,
                        label: goto_label(target),
                        has_value: false,
                    }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
                self.visit(target, confidence);
            }
            NodeKind::StatementModifier { modifier, condition, .. } => {
                let modifier_kind = match modifier.as_str() {
                    "if" => StatementModifierKind::If,
                    "unless" => StatementModifierKind::Unless,
                    "while" => StatementModifierKind::While,
                    "until" => StatementModifierKind::Until,
                    "for" | "foreach" => StatementModifierKind::Foreach,
                    _ => StatementModifierKind::Other,
                };
                // Loop-form modifiers can inherit an enclosing `LABEL:`; branch
                // forms are not loop targets, so they must not consume it.
                let label = match modifier_kind {
                    StatementModifierKind::While
                    | StatementModifierKind::Until
                    | StatementModifierKind::Foreach => self.pending_label.take(),
                    StatementModifierKind::If
                    | StatementModifierKind::Unless
                    | StatementModifierKind::Other => None,
                };
                self.push_item(
                    node,
                    None,
                    confidence,
                    HirKind::StatementModifierShell(StatementModifierShell {
                        modifier: modifier_kind,
                        condition_range: condition.location,
                        label,
                    }),
                    self.package_context.clone(),
                    Some(self.current_scope()),
                );
                self.visit_children(node, confidence);
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
        let id = HirScopeId::from_index(to_u32_saturating(self.scope_graph.scopes.len()));
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
            NodeKind::MandatoryParameter { variable, .. }
            | NodeKind::SlurpyParameter { variable, .. } => {
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
            NodeKind::NamedParameter { variable, default_value, .. } => {
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
                if let Some(default_value) = default_value {
                    self.visit(default_value, RecoveryConfidence::Parsed);
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
        let id = HirBindingId::from_index(to_u32_saturating(self.scope_graph.bindings.len()));
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
        let phase = compile_phase(phase);
        self.compile_environment.phase_blocks.push(CompilePhaseBlock {
            phase,
            range,
            scope_id: Some(self.current_scope()),
            package_context: self.package_context.clone(),
            provenance: CompileProvenance::ExactAst,
            confidence: CompileConfidence::High,
        });
    }

    fn record_phase_execution_boundary(
        &mut self,
        phase: &str,
        block: &Node,
        range: SourceLocation,
    ) {
        let phase = compile_phase(phase);
        // Only BEGIN executes immediately, at parse time. Per perlmod
        // (https://perldoc.perl.org/perlmod), the compile/run phase order is
        // BEGIN -> UNITCHECK -> CHECK -> INIT -> END: UNITCHECK, CHECK, INIT,
        // and END bodies are all compiled with the surrounding program but
        // execute later in Perl's lifecycle (UNITCHECK right after their
        // compilation unit finishes compiling, CHECK at the end of
        // compilation, INIT just before the main runtime, END at the end).
        // Preserve their phase facts without treating ordinary compile
        // analysis as an attempt to execute those bodies.
        if matches!(
            phase,
            CompilePhase::UnitCheck | CompilePhase::Check | CompilePhase::Init | CompilePhase::End
        ) || !phase_block_requires_compile_execution(block)
        {
            return;
        }

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
                if let Some((slot_kind, alias_target)) = static_glob_alias_target(rhs) {
                    self.record_slot(
                        package,
                        stash_slot(
                            symbol,
                            slot_kind,
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

fn static_glob_alias_target(node: &Node) -> Option<(GlobSlotKind, String)> {
    match &node.kind {
        NodeKind::Unary { op, operand } if op == "\\" => match &operand.kind {
            NodeKind::FunctionCall { name, args } if args.is_empty() => {
                Some((GlobSlotKind::Code, name.clone()))
            }
            NodeKind::Typeglob { name } => Some((GlobSlotKind::Code, name.clone())),
            NodeKind::Variable { sigil, name } => {
                slot_kind_for_sigil(sigil).map(|slot_kind| (slot_kind, name.clone()))
            }
            _ => None,
        },
        NodeKind::Typeglob { name } => Some((GlobSlotKind::Code, name.clone())),
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

/// Whether `op` is a block/sigil dereference form (`${}` / `@{}` / `%{}` /
/// `&{}` / `*{}`) that Perl treats as a *symbolic* reference dereference when
/// `strict 'refs'` is disabled — see perlref.
///
/// Per perlref, hard-vs-symbolic dereference is a **runtime-value** decision,
/// not an AST-shape one: `$name = "foo"; $$name = 1;` dereferences a
/// *variable* operand and still sets the package global `$foo` symbolically,
/// because the variable's string contents are the referent's name. So this
/// classifier intentionally does not special-case `Variable` operands (e.g.
/// `${$x}`, `@$x`) as "always a hard reference" — under `no strict 'refs'`,
/// any block/sigil dereference is *potentially* a symbolic reference,
/// regardless of whether its operand is a literal, a variable, or an
/// expression. This is a conservative, fail-closed heuristic (flag when
/// unsure), not a proof that the operand is actually used as a symbol name at
/// runtime — some idiomatic forms (`$$_`, `@$tuple` over an already-a-ref
/// value) are always hard references in practice and never resolve a symbol
/// table entry, but that can only be known from the runtime value, so callers
/// that need to suppress known-safe idioms do so narrowly (e.g. the CPAN
/// corpus test-runner's per-file allowlist), not by weakening this check.
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

/// Whether a phase block may alter the compilation environment without a
/// statically modeled effect. Pure data-only phase bodies still compile and
/// retain their phase fact, but do not require compile-time evaluation.
fn phase_block_requires_compile_execution(block: &Node) -> bool {
    match &block.kind {
        NodeKind::FunctionCall { .. }
        | NodeKind::MethodCall { .. }
        | NodeKind::IndirectCall { .. }
        | NodeKind::Eval { .. }
        | NodeKind::Do { .. }
        | NodeKind::Use { .. }
        | NodeKind::No { .. }
        | NodeKind::PhaseBlock { .. } => true,
        NodeKind::Assignment { lhs, .. } if is_compile_environment_target(lhs) => true,
        NodeKind::VariableDeclaration { declarator, variable, .. }
            if matches!(declarator.as_str(), "local" | "our")
                && is_compile_environment_target(variable) =>
        {
            true
        }
        _ => block.children().into_iter().any(phase_block_requires_compile_execution),
    }
}

fn is_compile_environment_target(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Variable { sigil, name } => matches!(
            (sigil.as_str(), name.as_str()),
            ("$", "INC") | ("@", "INC") | ("%", "INC") | ("$", "^H") | ("%", "^H") | ("$", "^OPEN")
        ),
        NodeKind::VariableWithAttributes { variable, .. } => {
            is_compile_environment_target(variable)
        }
        NodeKind::Binary { op, left, .. } if op == "{}" || op == "[]" => {
            is_compile_environment_target(left)
        }
        _ => false,
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

/// Whether a loop's iterator/init position introduces a lexical declaration
/// (`foreach my $x` or `for (my $i = 0; ...)`).
fn is_lexical_declaration_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::VariableDeclaration { .. } | NodeKind::VariableListDeclaration { .. }
    )
}

/// Static label target for a `goto`, when the target is a plain label
/// identifier rather than a sub reference or dynamic expression.
fn goto_label(target: &Node) -> Option<String> {
    match &target.kind {
        NodeKind::Identifier { name } => Some(name.clone()),
        _ => None,
    }
}

// ── Second pass: canonical body lowering ──────────────────────────────────────
//
// After `Lowerer` finishes the first pass (items + scope graph), a second walk
// lowers each body owner into a `HirBody` arena and attaches all bodies to the
// `HirFile`. The second pass reuses the scope/binding graph from pass 1 for
// accurate variable-kind resolution (lexical vs. package), implementing the
// #2575 correctness requirements without disturbing the mature first pass.

/// Entry point for the second-pass body lowering.
///
/// Lowers the program-root body and all subroutine/method bodies, then stores
/// them in `file.bodies` and `file.body_owners`.
fn lower_bodies_into_file(ast: &Node, file: &mut HirFile) {
    // Program-root body (index 0 in bodies vec).
    {
        let root_scope = file
            .scope_graph
            .scopes
            .first()
            .map(|s| s.id)
            .unwrap_or_else(|| HirScopeId::from_index(0));
        let root_body =
            lower_body_from_ast(ast, BodyOwnerKind::ProgramRoot, root_scope, &file.scope_graph);
        let body_id = HirBodyId(file.bodies.len() as u32);
        file.body_owners.insert(BodyOwner::new(BodyOwnerKind::ProgramRoot, 0), body_id);
        file.bodies.push(root_body);
    }

    // Sub/method bodies: walk the top-level AST for Subroutine and Method nodes.
    collect_sub_bodies(ast, file, &mut 0u32, &mut 0u32);
}

/// Walk `node` looking for `Subroutine` and `Method` AST nodes and lower their bodies.
fn collect_sub_bodies(
    node: &Node,
    file: &mut HirFile,
    sub_ordinal: &mut u32,
    method_ordinal: &mut u32,
) {
    match &node.kind {
        NodeKind::Program { statements } => {
            for stmt in statements {
                collect_sub_bodies(stmt, file, sub_ordinal, method_ordinal);
            }
        }
        NodeKind::Subroutine { name, body, .. } => {
            let owner_kind = BodyOwnerKind::Subroutine { name: name.clone() };
            let ordinal = *sub_ordinal;
            *sub_ordinal += 1;
            // Find the scope frame created for this sub's body by matching
            // the body block's source range and scope kind.
            let body_scope = find_body_scope(&file.scope_graph, body.location);
            let hir_body =
                lower_body_from_ast(body, owner_kind.clone(), body_scope, &file.scope_graph);
            let body_id = HirBodyId(file.bodies.len() as u32);
            file.body_owners.insert(BodyOwner::new(owner_kind, ordinal), body_id);
            file.bodies.push(hir_body);
            // Recurse into the body for nested subs/methods
            collect_sub_bodies(body, file, sub_ordinal, method_ordinal);
        }
        NodeKind::Method { name, body, .. } => {
            let owner_kind = BodyOwnerKind::Method { name: name.clone() };
            let ordinal = *method_ordinal;
            *method_ordinal += 1;
            let body_scope = find_body_scope(&file.scope_graph, body.location);
            let hir_body =
                lower_body_from_ast(body, owner_kind.clone(), body_scope, &file.scope_graph);
            let body_id = HirBodyId(file.bodies.len() as u32);
            file.body_owners.insert(BodyOwner::new(owner_kind, ordinal), body_id);
            file.bodies.push(hir_body);
            collect_sub_bodies(body, file, sub_ordinal, method_ordinal);
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                collect_sub_bodies(stmt, file, sub_ordinal, method_ordinal);
            }
        }
        NodeKind::Package { block, .. } => {
            if let Some(block) = block {
                collect_sub_bodies(block, file, sub_ordinal, method_ordinal);
            }
        }
        _ => {
            // Recurse into children via the AST's child-iteration.
            node.for_each_child(|child: &Node| {
                collect_sub_bodies(child, file, sub_ordinal, method_ordinal);
            });
        }
    }
}

/// Lower one body owner's AST node into a [`HirBody`] arena.
///
/// For the program root, `ast` is the `Program` node.
/// For a subroutine body, `ast` is the `Block` node inside the sub.
///
/// `start_scope` is the innermost scope frame that encloses this body — used
/// to seed the scope chain walk for variable-kind resolution.
fn lower_body_from_ast(
    ast: &Node,
    owner: BodyOwnerKind,
    start_scope: HirScopeId,
    scope_graph: &ScopeGraph,
) -> HirBody {
    let mut builder = BodyBuilder2::new(scope_graph, start_scope);

    let stmts = match &ast.kind {
        NodeKind::Program { statements } => statements.as_slice(),
        NodeKind::Block { statements } => statements.as_slice(),
        _ => std::slice::from_ref(ast),
    };

    let root_range = ast.location;
    let mut root_block = HirBlock::default();

    for stmt_node in stmts {
        let stmt_id = builder.lower_statement(stmt_node);
        root_block.stmts.push(stmt_id);
    }

    let root_id = builder.alloc_block(root_block, root_range);
    builder.finish(root_id, owner)
}

// ── BodyBuilder2: body arena builder for the second pass ─────────────────────
//
// This mirrors `body::BodyBuilder` from the first-slice body.rs but adds:
//   - Scope-based variable-kind resolution (lexical vs. package)
//   - Compound-assignment ReadModifyWrite distinction
//   - Recovery-confidence propagation (no exact fact through contamination)

struct BodyBuilder2<'a> {
    exprs: Arena<HirExpr>,
    stmts: Arena<HirStmt>,
    blocks: Arena<HirBlock>,
    source_map: BodySourceMap,
    /// Full scope graph from pass 1 — used for parent-chain resolution.
    scope_graph: &'a ScopeGraph,
    /// The innermost scope that encloses this body's root block.
    ///
    /// All variable-kind resolution starts from this scope and walks up through
    /// parent pointers in `scope_graph.scopes`. This is the fix for the scope-blind
    /// bug: previously `scope_depth` was initialised to [scope 0] and never updated,
    /// so every variable in a sub body incorrectly resolved to scope 0 (file root),
    /// making all bindings declared inside the sub invisible.
    start_scope: HirScopeId,
}

impl<'a> BodyBuilder2<'a> {
    fn new(scope_graph: &'a ScopeGraph, start_scope: HirScopeId) -> Self {
        Self {
            exprs: Arena::default(),
            stmts: Arena::default(),
            blocks: Arena::default(),
            source_map: BodySourceMap::default(),
            scope_graph,
            start_scope,
        }
    }

    fn alloc_expr(&mut self, expr: HirExpr, range: SourceLocation) -> HirExprId {
        let idx = self.exprs.alloc(expr);
        self.source_map.expr_ranges.push(range);
        HirExprId(idx)
    }

    fn alloc_stmt(&mut self, stmt: HirStmt, range: SourceLocation) -> HirStmtId {
        let idx = self.stmts.alloc(stmt);
        self.source_map.stmt_ranges.push(range);
        HirStmtId(idx)
    }

    fn alloc_block(&mut self, block: HirBlock, range: SourceLocation) -> HirBlockId {
        let idx = self.blocks.alloc(block);
        self.source_map.block_ranges.push(range);
        HirBlockId(idx)
    }

    fn finish(self, root_block: HirBlockId, owner: BodyOwnerKind) -> HirBody {
        HirBody {
            exprs: self.exprs,
            stmts: self.stmts,
            blocks: self.blocks,
            source_map: self.source_map,
            root_block,
            owner,
        }
    }

    /// Resolve whether a variable is lexically bound or package-global.
    ///
    /// A variable is `Lexical` if a `my`/`state` binding for it is visible in
    /// the current scope chain. An `our` binding resolves to `Package` (package
    /// alias). A qualified name (`Foo::x`) is always `Package`.
    ///
    /// Uses the same parent-chain walk as the first-pass `resolve_visible_binding`
    /// (lower.rs ~1892). Starting from `start_scope`, walk up through
    /// `scope_graph.scopes[id].parent` until None — matching the identical
    /// algorithm used in pass 1.
    fn resolve_variable_kind(&self, sigil: &str, name: &str) -> VariableKind {
        // Qualified names are always package-qualified.
        if name.contains("::") {
            return VariableKind::Package;
        }
        let mut cursor = Some(self.start_scope);
        while let Some(current_scope) = cursor {
            for binding in self.scope_graph.bindings.iter().rev() {
                if binding.scope_id == current_scope
                    && binding.sigil == sigil
                    && binding.name == name
                {
                    return match binding.storage {
                        StorageClass::LexicalMy
                        | StorageClass::LexicalState
                        | StorageClass::Parameter => VariableKind::Lexical,
                        StorageClass::PackageOur
                        | StorageClass::LocalizedPackage
                        | StorageClass::PackageGlobal
                        | StorageClass::MethodInvocant
                        | StorageClass::Implicit => VariableKind::Package,
                    };
                }
            }
            // Walk up to the parent scope — identical to first-pass resolve_visible_binding.
            cursor =
                self.scope_graph.scopes.get(current_scope.index() as usize).and_then(|s| s.parent);
        }
        // No binding found in any ancestor scope — treat as package global.
        VariableKind::Package
    }

    fn lower_statement(&mut self, node: &Node) -> HirStmtId {
        let range = node.location;

        match &node.kind {
            // Peel through the expression-statement wrapper.
            NodeKind::ExpressionStatement { expression } => self.lower_statement(expression),

            NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
                let (sigil_str, var_name) = match &variable.kind {
                    NodeKind::Variable { sigil, name } => (sigil.as_str(), name.clone()),
                    NodeKind::VariableWithAttributes { variable, .. } => match &variable.kind {
                        NodeKind::Variable { sigil, name } => (sigil.as_str(), name.clone()),
                        _ => ("$", String::from("<unknown>")),
                    },
                    _ => ("$", String::from("<unknown>")),
                };
                let sigil = sigil_from_str(sigil_str);
                let storage = storage_class_for_decl(declarator);

                let init_expr_id = initializer.as_ref().map(|init_node| {
                    // Allocate the write-place for the declared variable.
                    // Always Lexical regardless of declarator — the place IS the
                    // declaration site, not a resolved binding.
                    let place_kind = match declarator.as_str() {
                        "our" => VariableKind::Package,
                        _ => VariableKind::Lexical,
                    };
                    let place_expr = HirExpr::Variable(HirVariable {
                        sigil: sigil_from_str(sigil_str),
                        name: var_name.clone(),
                        kind: place_kind,
                        access: AccessMode::Write,
                    });
                    let place_id = self.alloc_expr(place_expr, variable.location);

                    // Lower the RHS.
                    let rhs_id = self.lower_expr(init_node);

                    // Assign node spanning from variable to end of initializer.
                    let assign_range = SourceLocation {
                        start: variable.location.start,
                        end: init_node.location.end,
                    };
                    let assign_expr =
                        HirExpr::Assign { lhs: place_id, rhs: rhs_id, mode: AssignMode::Simple };
                    self.alloc_expr(assign_expr, assign_range)
                });

                self.alloc_stmt(
                    HirStmt::Let { name: var_name, sigil, storage, init: init_expr_id },
                    range,
                )
            }

            _ => {
                let expr_id = self.lower_expr(node);
                self.alloc_stmt(HirStmt::Expr(expr_id), range)
            }
        }
    }

    fn lower_expr(&mut self, node: &Node) -> HirExprId {
        let range = node.location;

        match &node.kind {
            // Peel through expression-statement wrapper when lower_expr is called on one.
            NodeKind::ExpressionStatement { expression } => self.lower_expr(expression),

            NodeKind::Variable { sigil, name } => {
                let kind = self.resolve_variable_kind(sigil, name);
                let var = HirVariable {
                    sigil: sigil_from_str(sigil),
                    name: name.clone(),
                    kind,
                    access: AccessMode::Read,
                };
                self.alloc_expr(HirExpr::Variable(var), range)
            }

            NodeKind::Binary { op, left, right } => {
                let lhs_id = self.lower_expr(left);
                let rhs_id = self.lower_expr(right);
                let binary_op = binary_op_from_str(op);
                self.alloc_expr(HirExpr::Binary { lhs: lhs_id, op: binary_op, rhs: rhs_id }, range)
            }

            NodeKind::Assignment { lhs, rhs, op } => {
                // Plain `=` is Simple; compound operators (`+=`, `-=`, etc.) are RMW.
                let (mode, lhs_access) = if op == "=" {
                    (AssignMode::Simple, AccessMode::Write)
                } else {
                    (AssignMode::ReadModifyWrite, AccessMode::ReadModifyWrite)
                };

                // Lower LHS with the correct access mode applied to the variable node.
                let lhs_id = self.lower_expr_as_place(lhs, lhs_access);
                let rhs_id = self.lower_expr(rhs);
                self.alloc_expr(HirExpr::Assign { lhs: lhs_id, rhs: rhs_id, mode }, range)
            }

            NodeKind::Unary { op, operand } => {
                // Prefix `++`/`--` are ReadModifyWrite on the operand.
                let unary_mode = if op == "++" || op == "--" {
                    UnaryMode::ReadModifyWrite
                } else {
                    UnaryMode::Read
                };
                let operand_id = if matches!(unary_mode, UnaryMode::ReadModifyWrite) {
                    self.lower_expr_as_place(operand, AccessMode::ReadModifyWrite)
                } else {
                    self.lower_expr(operand)
                };
                self.alloc_expr(
                    HirExpr::Unary { operand: operand_id, mode: unary_mode, op: op.clone() },
                    range,
                )
            }

            NodeKind::FunctionCall { args, .. } => {
                // Lower each argument as a read expression so variable references
                // in call-arg positions produce correct LexicalRead PIR nodes.
                // The call itself is still not modeled (recorded as unsupported
                // in the PIR lowerer), but its arguments are correctly extracted.
                let arg_ids: Vec<HirExprId> = args.iter().map(|a| self.lower_expr(a)).collect();
                self.alloc_expr(
                    HirExpr::Call { args: arg_ids, ast_kind: "FunctionCall".to_string() },
                    range,
                )
            }

            NodeKind::Return { value } => {
                // Lower the return value as a read expression when present.
                // This surfaces variable reads in `return $x` positions.
                let arg_ids: Vec<HirExprId> =
                    if let Some(v) = value { vec![self.lower_expr(v)] } else { vec![] };
                self.alloc_expr(
                    HirExpr::Call { args: arg_ids, ast_kind: "Return".to_string() },
                    range,
                )
            }

            _ => {
                // Everything else: emit Opaque. This is the "fail closed" path.
                let kind_name = node.kind.kind_name().to_string();
                self.alloc_expr(HirExpr::Opaque { ast_kind: kind_name }, range)
            }
        }
    }

    /// Lower an expression node that appears in lvalue (place) position.
    ///
    /// For a `Variable` node the access mode is set to `access`; for everything
    /// else the node is lowered normally (the access mode is carried only on
    /// terminal variable nodes).
    fn lower_expr_as_place(&mut self, node: &Node, access: AccessMode) -> HirExprId {
        let range = node.location;
        match &node.kind {
            NodeKind::Variable { sigil, name } => {
                let kind = self.resolve_variable_kind(sigil, name);
                let var =
                    HirVariable { sigil: sigil_from_str(sigil), name: name.clone(), kind, access };
                self.alloc_expr(HirExpr::Variable(var), range)
            }
            _ => self.lower_expr(node),
        }
    }
}

// ── Helpers for the second-pass body lowerer ─────────────────────────────────

/// Find the innermost scope frame that covers `body_loc` (the source range of
/// a subroutine or method body block).
///
/// The first pass creates a `Subroutine` or `Method` scope frame whose `.range`
/// covers the body block. We find it by scanning all scope frames for the one
/// with the smallest enclosing range that still fully contains `body_loc`.
/// Falls back to the root scope (index 0) if no matching frame is found.
fn find_body_scope(scope_graph: &ScopeGraph, body_loc: SourceLocation) -> HirScopeId {
    let mut best: Option<(usize, HirScopeId)> = None; // (range_size, id)
    for frame in &scope_graph.scopes {
        let range = frame.range;
        // Check that this scope frame fully contains the body location.
        if range.start <= body_loc.start && range.end >= body_loc.end {
            let size = range.end - range.start;
            let is_better = best.is_none_or(|(prev_size, _)| size < prev_size);
            if is_better {
                best = Some((size, frame.id));
            }
        }
    }
    best.map(|(_, id)| id)
        .or_else(|| scope_graph.scopes.first().map(|s| s.id))
        .unwrap_or_else(|| HirScopeId::from_index(0))
}

fn sigil_from_str(s: &str) -> Sigil {
    match s {
        "$" => Sigil::Scalar,
        "@" => Sigil::Array,
        "%" => Sigil::Hash,
        "&" => Sigil::Code,
        "*" => Sigil::Glob,
        _ => Sigil::Scalar,
    }
}

fn binary_op_from_str(s: &str) -> BinaryOp {
    match s {
        "+" => BinaryOp::Add,
        "-" => BinaryOp::Sub,
        "*" => BinaryOp::Mul,
        "/" => BinaryOp::Div,
        "." => BinaryOp::Concat,
        other => BinaryOp::Other(other.to_string()),
    }
}

/// Convert a `usize` length/index into a `u32`, saturating at [`u32::MAX`]
/// instead of silently truncating (wrapping) on overflow.
///
/// HIR scope and binding identifiers are `u32`-backed. A naive `len() as u32`
/// cast wraps modulo `2^32` once the count exceeds [`u32::MAX`], producing a
/// low identifier that collides with an existing scope/binding and corrupts the
/// scope graph. Saturating keeps the value monotonic and non-colliding at the
/// boundary; downstream code already treats `u32::MAX` as an unreachable upper
/// bound for a single lowered file.
#[inline]
fn to_u32_saturating(value: usize) -> u32 {
    value.min(u32::MAX as usize) as u32
}

fn storage_class_for_decl(declarator: &str) -> DeclStorageClass {
    match declarator {
        "my" => DeclStorageClass::My,
        "our" => DeclStorageClass::Our,
        "local" => DeclStorageClass::Local,
        "state" => DeclStorageClass::State,
        _ => DeclStorageClass::My,
    }
}

#[cfg(test)]
mod saturating_id_tests {
    use super::to_u32_saturating;

    /// Counts well within `u32` range convert losslessly.
    #[test]
    fn small_counts_are_lossless() {
        assert_eq!(to_u32_saturating(0), 0);
        assert_eq!(to_u32_saturating(1), 1);
        assert_eq!(to_u32_saturating(42), 42);
        assert_eq!(to_u32_saturating(u32::MAX as usize - 1), u32::MAX - 1);
    }

    /// A count exactly at `u32::MAX` maps to `u32::MAX`.
    #[test]
    fn count_at_u32_max_clamps() {
        assert_eq!(to_u32_saturating(u32::MAX as usize), u32::MAX);
    }

    /// Regression: a count *above* `u32::MAX` must clamp to `u32::MAX`, not wrap
    /// to a low id that would collide with an existing scope/binding. On 32-bit
    /// targets `usize` cannot exceed `u32::MAX`, so this case is skipped there.
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn count_above_u32_max_does_not_wrap() {
        let above = u32::MAX as usize + 1;
        // A naive `above as u32` cast would wrap to 0 (a colliding low id).
        assert_eq!(above as u32, 0, "precondition: naive cast wraps to a colliding id");
        // The saturating conversion clamps to the maximum instead.
        assert_eq!(to_u32_saturating(above), u32::MAX);
        assert_ne!(to_u32_saturating(above), 0);

        // A far-overflowing count also clamps rather than wrapping.
        let far = u32::MAX as usize + 12_345;
        assert_eq!(to_u32_saturating(far), u32::MAX);
        assert_ne!(to_u32_saturating(far), far as u32);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod goto_lowering_tests {
    //! `--lib` coverage for the `NodeKind::Goto` HIR-lowering arm (#1923).
    //! goto lowering is otherwise exercised only by integration tests under
    //! `tests/`, which do not count toward Codecov / Patch 95 (`--lib` only).
    use super::*;
    use crate::parser::Parser;
    use perl_tdd_support::must;

    #[test]
    fn goto_lowers_to_control_transfer_item() {
        let mut parser = Parser::new("goto &handler;");
        let ast = must(parser.parse());
        let file = lower_ast(&ast);
        assert!(
            file.items.iter().any(|item| matches!(
                &item.kind,
                HirKind::ControlTransfer(transfer)
                    if matches!(transfer.kind, ControlTransferKind::Goto)
            )),
            "goto must lower to a ControlTransfer HIR item of kind Goto"
        );
    }
}
