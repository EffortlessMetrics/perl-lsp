//! HIR data model.

use crate::SourceLocation;
use perl_semantic_facts::{
    AnchorId, Confidence, ExportSet, ExportTag, FileId, ImportKind, ImportSpec, ImportSymbols,
    Provenance, ScopeId, VisibleSymbol, VisibleSymbolContext, VisibleSymbolSource,
};
use std::collections::BTreeMap;

/// Stable identifier for a HIR item within one lowered file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct HirId {
    index: u32,
}

impl HirId {
    /// Create an identifier from a zero-based lowering index.
    #[inline]
    pub const fn from_index(index: u32) -> Self {
        Self { index }
    }

    /// Return the zero-based lowering index.
    #[inline]
    pub const fn index(self) -> u32 {
        self.index
    }
}

/// Stable identifier for a HIR scope frame within one lowered file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct HirScopeId {
    index: u32,
}

impl HirScopeId {
    /// Create a scope identifier from a zero-based lowering index.
    #[inline]
    pub const fn from_index(index: u32) -> Self {
        Self { index }
    }

    /// Return the zero-based lowering index.
    #[inline]
    pub const fn index(self) -> u32 {
        self.index
    }
}

/// Stable identifier for a HIR binding within one lowered file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct HirBindingId {
    index: u32,
}

impl HirBindingId {
    /// Create a binding identifier from a zero-based lowering index.
    #[inline]
    pub const fn from_index(index: u32) -> Self {
        Self { index }
    }

    /// Return the zero-based lowering index.
    #[inline]
    pub const fn index(self) -> u32 {
        self.index
    }
}

/// Parser AST location that produced a HIR item.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AstAnchor {
    /// Parser AST node kind name.
    pub node_kind: &'static str,
    /// Full AST node source range.
    pub range: SourceLocation,
    /// Precise name range when the AST exposes one.
    pub name_range: Option<SourceLocation>,
}

/// Recovery quality for a lowered HIR item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RecoveryConfidence {
    /// Lowered from a normally parsed AST node.
    Parsed,
    /// Lowered from a parser recovery wrapper with a partial valid tree.
    Recovered,
    /// Lowered from a partially known or placeholder AST shape.
    Partial,
    /// Lowering could not classify recovery confidence yet.
    Unknown,
}

/// HIR for one parsed file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct HirFile {
    /// Items lowered in stable depth-first source order.
    pub items: Vec<HirItem>,
    /// Scope and binding graph lowered beside HIR items.
    pub scope_graph: ScopeGraph,
    /// Package stash graph lowered beside HIR items.
    pub stash_graph: StashGraph,
    /// Compile-environment facts lowered beside HIR items.
    pub compile_environment: CompileEnvironment,
    /// Source-backed subroutine prototype facts lowered beside HIR items.
    pub prototype_table: PrototypeTable,
}

impl HirFile {
    /// Return true when no HIR items were lowered.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Project compile-time effects using the default model metadata.
    ///
    /// This is a compiler-substrate proof surface only. It links existing HIR
    /// facts to the state mutations that produced them without changing LSP
    /// provider behavior.
    #[must_use]
    pub fn compile_effects(&self) -> Vec<CompileEffect> {
        self.compile_effects_with_source_hash(None)
    }

    /// Project compile-time effects and attach a caller-supplied source hash.
    ///
    /// Parser-core does not own a source database, so persisted workspace
    /// callers can pass the source hash they use for freshness. Fixture-only
    /// callers may use [`HirFile::compile_effects`].
    #[must_use]
    pub fn compile_effects_with_source_hash(
        &self,
        source_hash: Option<String>,
    ) -> Vec<CompileEffect> {
        compile_effects_from_file(self, source_hash)
    }

    /// Project framework-adapter facts using the default registry.
    ///
    /// This is a compiler-substrate proof surface only. It does not change LSP
    /// provider behavior.
    #[must_use]
    pub fn framework_facts(&self) -> FrameworkFactGraph {
        FrameworkAdapterRegistry::default().project_file(self)
    }
}

/// Source-backed subroutine prototype facts lowered from parsed declarations.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct PrototypeTable {
    /// Prototype facts in stable source order.
    pub facts: Vec<PrototypeFact>,
}

/// One source-backed prototype fact for a named subroutine declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PrototypeFact {
    /// Subroutine name as written in the declaration.
    pub sub_name: String,
    /// Package context active at the declaration.
    pub package_context: Option<String>,
    /// Prototype content without the surrounding parentheses.
    pub content: String,
    /// Precise source range for the prototype node.
    pub range: SourceLocation,
    /// Full declaration source range.
    pub declaration_range: SourceLocation,
    /// HIR item that declared the subroutine.
    pub declaration_item: HirId,
    /// Scope owning the subroutine declaration.
    pub scope_id: Option<HirScopeId>,
    /// Source anchor for this prototype fact.
    pub anchor_id: AnchorId,
    /// Provenance for the lowered prototype fact.
    pub provenance: CompileProvenance,
    /// Confidence for the lowered prototype fact.
    pub confidence: CompileConfidence,
}

/// One lowered HIR item with common metadata required by compiler layers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HirItem {
    /// Stable item id for this file.
    pub id: HirId,
    /// Lowered language construct.
    pub kind: HirKind,
    /// Source range for the construct.
    pub range: SourceLocation,
    /// Parser AST anchor for this item.
    pub anchor: AstAnchor,
    /// Recovery quality inherited from parser recovery.
    pub recovery_confidence: RecoveryConfidence,
    /// Package context known at lowering time.
    pub package_context: Option<String>,
    /// Scope context known at lowering time.
    pub scope_context: Option<HirScopeId>,
}

/// Current HIR compile-effect model version.
pub const COMPILE_EFFECT_MODEL_VERSION: u32 = 1;

/// One Rust-modeled Perl compile-time effect.
///
/// Effects connect source constructs to compiler state mutations and the
/// semantic fact categories emitted from those mutations. They are proof data
/// for compiler-substrate work and do not change provider behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CompileEffect {
    /// Stable ordinal after source-order sorting.
    pub ordinal: u32,
    /// Effect category.
    pub kind: CompileEffectKind,
    /// Source construct category.
    pub source_kind: CompileEffectSourceKind,
    /// Semantic fact category emitted by this effect.
    pub fact_kind: CompileEffectFactKind,
    /// Human-readable fact name.
    pub fact_name: Option<String>,
    /// Source range for the effect.
    pub range: SourceLocation,
    /// HIR item that produced this effect, when available.
    pub source_item: Option<HirId>,
    /// Scope containing this effect, when known.
    pub scope_id: Option<HirScopeId>,
    /// Package context active at the effect, when known.
    pub package_context: Option<String>,
    /// Source anchor of the emitted fact, when available.
    pub fact_anchor_id: Option<AnchorId>,
    /// Dynamic-boundary reason, when this effect records unsupported behavior.
    pub dynamic_reason: Option<String>,
    /// Caller-supplied source hash used for freshness, when available.
    pub source_hash: Option<String>,
    /// Compile-effect model version.
    pub model_version: u32,
    /// How this effect was produced.
    pub provenance: CompileProvenance,
    /// Confidence in this effect.
    pub confidence: CompileConfidence,
}

/// Compiler state mutation represented by an effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompileEffectKind {
    /// Declare a package/stash.
    DeclarePackage,
    /// Declare a subroutine code slot.
    DeclareSub,
    /// Declare a method code slot.
    DeclareMethod,
    /// Declare a lexical or package binding.
    DeclareBinding,
    /// Set effective pragma or feature state.
    SetPragmaState,
    /// Add an include path.
    AddIncludePath,
    /// Remove an include path.
    RemoveIncludePath,
    /// Record a module load or resolution request.
    RequestModule,
    /// Record an import-symbol relationship.
    ImportSymbols,
    /// Assign an inheritance edge.
    AssignInheritance,
    /// Assign a simple typeglob alias.
    AssignGlobAlias,
    /// Define a constant-like code slot.
    DefineConstant,
    /// Register a prototype-bearing subroutine.
    RegisterPrototype,
    /// Emit a dynamic-boundary fact instead of guessing.
    EmitDynamicBoundary,
}

/// Source construct that produced a compile effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompileEffectSourceKind {
    /// `package` declaration.
    PackageDecl,
    /// `sub` declaration.
    SubDecl,
    /// `method` declaration.
    MethodDecl,
    /// Variable declaration.
    VariableDecl,
    /// `use` directive.
    UseDirective,
    /// `no` directive.
    NoDirective,
    /// `require` directive.
    RequireDirective,
    /// Compile-time phase block.
    PhaseBlock,
    /// Symbolic-reference dereference.
    SymbolicReferenceDeref,
    /// Assignment expression.
    Assignment,
    /// Typeglob assignment.
    TypeglobAssignment,
    /// Derived HIR scope graph fact.
    ScopeGraph,
    /// Derived HIR stash graph fact.
    StashGraph,
    /// Derived compile-environment fact.
    CompileEnvironment,
}

/// Semantic fact category emitted by a compile effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompileEffectFactKind {
    /// Package fact.
    Package,
    /// Subroutine fact.
    Sub,
    /// Method fact.
    Method,
    /// Binding fact.
    Binding,
    /// Pragma-state fact.
    PragmaState,
    /// Include-root fact.
    IncludeRoot,
    /// Module-request fact.
    ModuleRequest,
    /// Import specification fact.
    ImportSpec,
    /// Inheritance edge fact.
    InheritanceEdge,
    /// Glob slot fact.
    GlobSlot,
    /// Constant fact.
    Constant,
    /// Prototype fact.
    Prototype,
    /// Dynamic-boundary fact.
    DynamicBoundary,
}

#[derive(Debug)]
struct CompileEffectEntry {
    source_order: u32,
    effect: CompileEffect,
}

fn compile_effects_from_file(file: &HirFile, source_hash: Option<String>) -> Vec<CompileEffect> {
    let mut entries = Vec::new();
    let mut next_order = 0;

    for item in &file.items {
        push_item_effects(item, &source_hash, &mut entries, &mut next_order);
    }
    for fact in &file.prototype_table.facts {
        push_compile_effect(
            &mut entries,
            &mut next_order,
            CompileEffectSeed {
                kind: CompileEffectKind::RegisterPrototype,
                source_kind: CompileEffectSourceKind::SubDecl,
                fact_kind: CompileEffectFactKind::Prototype,
                fact_name: Some(fact.sub_name.clone()),
                range: fact.range,
                source_item: Some(fact.declaration_item),
                scope_id: fact.scope_id,
                package_context: fact.package_context.clone(),
                fact_anchor_id: Some(fact.anchor_id),
                dynamic_reason: None,
                source_hash: source_hash.clone(),
                provenance: fact.provenance,
                confidence: fact.confidence,
            },
        );
    }
    for binding in &file.scope_graph.bindings {
        push_compile_effect(
            &mut entries,
            &mut next_order,
            CompileEffectSeed {
                kind: CompileEffectKind::DeclareBinding,
                source_kind: CompileEffectSourceKind::ScopeGraph,
                fact_kind: CompileEffectFactKind::Binding,
                fact_name: Some(format!("{}{}", binding.sigil, binding.name)),
                range: binding.range,
                source_item: binding.declaration_item,
                scope_id: Some(binding.scope_id),
                package_context: binding.package_context.clone(),
                fact_anchor_id: Some(AnchorId(binding.range.start as u64)),
                dynamic_reason: None,
                source_hash: source_hash.clone(),
                provenance: CompileProvenance::ExactAst,
                confidence: CompileConfidence::High,
            },
        );
    }
    for fact in &file.compile_environment.pragma_state_facts {
        push_compile_effect(
            &mut entries,
            &mut next_order,
            CompileEffectSeed {
                kind: CompileEffectKind::SetPragmaState,
                source_kind: CompileEffectSourceKind::CompileEnvironment,
                fact_kind: CompileEffectFactKind::PragmaState,
                fact_name: Some("strict/warnings/feature".to_string()),
                range: fact.range,
                source_item: fact.directive_item,
                scope_id: fact.scope_id,
                package_context: fact.package_context.clone(),
                fact_anchor_id: Some(fact.anchor_id),
                dynamic_reason: None,
                source_hash: source_hash.clone(),
                provenance: fact.provenance,
                confidence: fact.confidence,
            },
        );
    }
    for root in &file.compile_environment.inc_roots {
        let kind = match root.action {
            IncRootAction::Add => CompileEffectKind::AddIncludePath,
            IncRootAction::Remove => CompileEffectKind::RemoveIncludePath,
        };
        push_compile_effect(
            &mut entries,
            &mut next_order,
            CompileEffectSeed {
                kind,
                source_kind: match root.action {
                    IncRootAction::Add => CompileEffectSourceKind::UseDirective,
                    IncRootAction::Remove => CompileEffectSourceKind::NoDirective,
                },
                fact_kind: CompileEffectFactKind::IncludeRoot,
                fact_name: Some(root.path.clone()),
                range: root.range,
                source_item: root.directive_item,
                scope_id: root.scope_id,
                package_context: root.package_context.clone(),
                fact_anchor_id: Some(AnchorId(root.range.start as u64)),
                dynamic_reason: None,
                source_hash: source_hash.clone(),
                provenance: root.provenance,
                confidence: root.confidence,
            },
        );
    }
    for request in &file.compile_environment.module_requests {
        push_compile_effect(
            &mut entries,
            &mut next_order,
            CompileEffectSeed {
                kind: CompileEffectKind::RequestModule,
                source_kind: module_request_source_kind(request.kind),
                fact_kind: CompileEffectFactKind::ModuleRequest,
                fact_name: request.target.clone().or_else(|| Some("<dynamic>".to_string())),
                range: request.range,
                source_item: request.directive_item,
                scope_id: request.scope_id,
                package_context: request.package_context.clone(),
                fact_anchor_id: Some(AnchorId(request.range.start as u64)),
                dynamic_reason: None,
                source_hash: source_hash.clone(),
                provenance: request.provenance,
                confidence: request.confidence,
            },
        );
    }
    for spec in file.compile_environment.import_specs(FileId(0)) {
        push_compile_effect(
            &mut entries,
            &mut next_order,
            CompileEffectSeed {
                kind: CompileEffectKind::ImportSymbols,
                source_kind: import_spec_source_kind(&spec),
                fact_kind: CompileEffectFactKind::ImportSpec,
                fact_name: Some(spec.module.clone()),
                range: SourceLocation::new(
                    spec.span_start_byte.unwrap_or_default() as usize,
                    spec.span_start_byte.unwrap_or_default() as usize,
                ),
                source_item: None,
                scope_id: spec.scope_id.map(|scope| HirScopeId::from_index(scope.0 as u32)),
                package_context: None,
                fact_anchor_id: spec.anchor_id,
                dynamic_reason: None,
                source_hash: source_hash.clone(),
                provenance: fact_provenance_to_compile(spec.provenance),
                confidence: fact_confidence_to_compile(spec.confidence),
            },
        );
    }
    for edge in &file.stash_graph.inheritance_edges {
        push_compile_effect(
            &mut entries,
            &mut next_order,
            CompileEffectSeed {
                kind: CompileEffectKind::AssignInheritance,
                source_kind: CompileEffectSourceKind::StashGraph,
                fact_kind: CompileEffectFactKind::InheritanceEdge,
                fact_name: Some(format!("{}->{}", edge.from_package, edge.to_package)),
                range: edge.range,
                source_item: edge.declaration_item,
                scope_id: None,
                package_context: Some(edge.from_package.clone()),
                fact_anchor_id: Some(AnchorId(edge.range.start as u64)),
                dynamic_reason: None,
                source_hash: source_hash.clone(),
                provenance: stash_provenance_to_compile(edge.provenance),
                confidence: stash_confidence_to_compile(edge.confidence),
            },
        );
    }
    for package in &file.stash_graph.packages {
        for slot in &package.slots {
            push_slot_effects(package, slot, &source_hash, &mut entries, &mut next_order);
        }
    }
    for boundary in &file.compile_environment.dynamic_boundaries {
        push_compile_effect(
            &mut entries,
            &mut next_order,
            CompileEffectSeed {
                kind: CompileEffectKind::EmitDynamicBoundary,
                source_kind: compile_boundary_source_kind(boundary.kind),
                fact_kind: CompileEffectFactKind::DynamicBoundary,
                fact_name: Some(format!("{:?}", boundary.kind)),
                range: boundary.range,
                source_item: boundary.boundary_item,
                scope_id: boundary.scope_id,
                package_context: boundary.package_context.clone(),
                fact_anchor_id: Some(AnchorId(boundary.range.start as u64)),
                dynamic_reason: Some(boundary.reason.clone()),
                source_hash: source_hash.clone(),
                provenance: boundary.provenance,
                confidence: boundary.confidence,
            },
        );
    }
    for boundary in &file.stash_graph.dynamic_boundaries {
        push_compile_effect(
            &mut entries,
            &mut next_order,
            CompileEffectSeed {
                kind: CompileEffectKind::EmitDynamicBoundary,
                source_kind: CompileEffectSourceKind::StashGraph,
                fact_kind: CompileEffectFactKind::DynamicBoundary,
                fact_name: boundary.symbol.clone(),
                range: boundary.range,
                source_item: boundary.boundary_item,
                scope_id: None,
                package_context: boundary.package.clone(),
                fact_anchor_id: Some(AnchorId(boundary.range.start as u64)),
                dynamic_reason: Some(boundary.reason.clone()),
                source_hash: source_hash.clone(),
                provenance: stash_provenance_to_compile(boundary.provenance),
                confidence: stash_confidence_to_compile(boundary.confidence),
            },
        );
    }

    entries.sort_by_key(|entry| (entry.effect.range.start, entry.source_order));
    entries
        .into_iter()
        .enumerate()
        .map(|(ordinal, mut entry)| {
            entry.effect.ordinal = ordinal as u32;
            entry.effect
        })
        .collect()
}

fn push_item_effects(
    item: &HirItem,
    source_hash: &Option<String>,
    entries: &mut Vec<CompileEffectEntry>,
    next_order: &mut u32,
) {
    match &item.kind {
        HirKind::PackageDecl(decl) => {
            push_compile_effect(
                entries,
                next_order,
                CompileEffectSeed {
                    kind: CompileEffectKind::DeclarePackage,
                    source_kind: CompileEffectSourceKind::PackageDecl,
                    fact_kind: CompileEffectFactKind::Package,
                    fact_name: Some(decl.name.clone()),
                    range: item.range,
                    source_item: Some(item.id),
                    scope_id: item.scope_context,
                    package_context: Some(decl.name.clone()),
                    fact_anchor_id: Some(AnchorId(item.range.start as u64)),
                    dynamic_reason: None,
                    source_hash: source_hash.clone(),
                    provenance: CompileProvenance::ExactAst,
                    confidence: CompileConfidence::High,
                },
            );
        }
        HirKind::SubDecl(decl) => {
            let Some(name) = &decl.name else {
                return;
            };
            push_compile_effect(
                entries,
                next_order,
                CompileEffectSeed {
                    kind: CompileEffectKind::DeclareSub,
                    source_kind: CompileEffectSourceKind::SubDecl,
                    fact_kind: CompileEffectFactKind::Sub,
                    fact_name: Some(name.clone()),
                    range: item.range,
                    source_item: Some(item.id),
                    scope_id: item.scope_context,
                    package_context: item.package_context.clone(),
                    fact_anchor_id: Some(AnchorId(item.range.start as u64)),
                    dynamic_reason: None,
                    source_hash: source_hash.clone(),
                    provenance: CompileProvenance::ExactAst,
                    confidence: CompileConfidence::High,
                },
            );
        }
        HirKind::MethodDecl(decl) => {
            push_compile_effect(
                entries,
                next_order,
                CompileEffectSeed {
                    kind: CompileEffectKind::DeclareMethod,
                    source_kind: CompileEffectSourceKind::MethodDecl,
                    fact_kind: CompileEffectFactKind::Method,
                    fact_name: Some(decl.name.clone()),
                    range: item.range,
                    source_item: Some(item.id),
                    scope_id: item.scope_context,
                    package_context: item.package_context.clone(),
                    fact_anchor_id: Some(AnchorId(item.range.start as u64)),
                    dynamic_reason: None,
                    source_hash: source_hash.clone(),
                    provenance: CompileProvenance::ExactAst,
                    confidence: CompileConfidence::High,
                },
            );
        }
        _ => {}
    }
}

fn push_slot_effects(
    package: &PackageStash,
    slot: &GlobSlot,
    source_hash: &Option<String>,
    entries: &mut Vec<CompileEffectEntry>,
    next_order: &mut u32,
) {
    let (kind, fact_kind) = match slot.source {
        GlobSlotSource::TypeglobAlias => {
            (CompileEffectKind::AssignGlobAlias, CompileEffectFactKind::GlobSlot)
        }
        GlobSlotSource::ConstantDeclaration => {
            (CompileEffectKind::DefineConstant, CompileEffectFactKind::Constant)
        }
        _ => return,
    };

    push_compile_effect(
        entries,
        next_order,
        CompileEffectSeed {
            kind,
            source_kind: match slot.source {
                GlobSlotSource::TypeglobAlias => CompileEffectSourceKind::TypeglobAssignment,
                _ => CompileEffectSourceKind::StashGraph,
            },
            fact_kind,
            fact_name: Some(format!("{}::{}", package.package, slot.name)),
            range: slot.range,
            source_item: slot.declaration_item,
            scope_id: None,
            package_context: Some(package.package.clone()),
            fact_anchor_id: Some(AnchorId(slot.range.start as u64)),
            dynamic_reason: None,
            source_hash: source_hash.clone(),
            provenance: stash_provenance_to_compile(slot.provenance),
            confidence: stash_confidence_to_compile(slot.confidence),
        },
    );
}

#[derive(Debug)]
struct CompileEffectSeed {
    kind: CompileEffectKind,
    source_kind: CompileEffectSourceKind,
    fact_kind: CompileEffectFactKind,
    fact_name: Option<String>,
    range: SourceLocation,
    source_item: Option<HirId>,
    scope_id: Option<HirScopeId>,
    package_context: Option<String>,
    fact_anchor_id: Option<AnchorId>,
    dynamic_reason: Option<String>,
    source_hash: Option<String>,
    provenance: CompileProvenance,
    confidence: CompileConfidence,
}

fn push_compile_effect(
    entries: &mut Vec<CompileEffectEntry>,
    next_order: &mut u32,
    seed: CompileEffectSeed,
) {
    let order = *next_order;
    *next_order += 1;
    entries.push(CompileEffectEntry {
        source_order: order,
        effect: CompileEffect {
            ordinal: 0,
            kind: seed.kind,
            source_kind: seed.source_kind,
            fact_kind: seed.fact_kind,
            fact_name: seed.fact_name,
            range: seed.range,
            source_item: seed.source_item,
            scope_id: seed.scope_id,
            package_context: seed.package_context,
            fact_anchor_id: seed.fact_anchor_id,
            dynamic_reason: seed.dynamic_reason,
            source_hash: seed.source_hash,
            model_version: COMPILE_EFFECT_MODEL_VERSION,
            provenance: seed.provenance,
            confidence: seed.confidence,
        },
    });
}

fn module_request_source_kind(kind: ModuleRequestKind) -> CompileEffectSourceKind {
    match kind {
        ModuleRequestKind::Require => CompileEffectSourceKind::RequireDirective,
        _ => CompileEffectSourceKind::UseDirective,
    }
}

fn import_spec_source_kind(spec: &ImportSpec) -> CompileEffectSourceKind {
    match spec.kind {
        ImportKind::Require | ImportKind::DynamicRequire => {
            CompileEffectSourceKind::RequireDirective
        }
        _ => CompileEffectSourceKind::UseDirective,
    }
}

fn compile_boundary_source_kind(kind: CompileEnvironmentBoundaryKind) -> CompileEffectSourceKind {
    match kind {
        CompileEnvironmentBoundaryKind::DynamicRequire => CompileEffectSourceKind::RequireDirective,
        CompileEnvironmentBoundaryKind::DynamicPragmaArgs
        | CompileEnvironmentBoundaryKind::DynamicIncRoot => CompileEffectSourceKind::UseDirective,
        CompileEnvironmentBoundaryKind::PhaseBlockExecution => CompileEffectSourceKind::PhaseBlock,
        CompileEnvironmentBoundaryKind::SymbolicReferenceDeref => {
            CompileEffectSourceKind::SymbolicReferenceDeref
        }
    }
}

fn stash_provenance_to_compile(provenance: StashProvenance) -> CompileProvenance {
    match provenance {
        StashProvenance::ExactAst => CompileProvenance::ExactAst,
        StashProvenance::DesugaredAst => CompileProvenance::DesugaredAst,
        StashProvenance::DynamicBoundary => CompileProvenance::DynamicBoundary,
    }
}

fn stash_confidence_to_compile(confidence: StashConfidence) -> CompileConfidence {
    match confidence {
        StashConfidence::High => CompileConfidence::High,
        StashConfidence::Medium => CompileConfidence::Medium,
        StashConfidence::Low => CompileConfidence::Low,
    }
}

fn fact_provenance_to_compile(provenance: Provenance) -> CompileProvenance {
    match provenance {
        // Exact AST-derived facts (fully statically known).
        Provenance::ExactAst | Provenance::LiteralRequireImport => CompileProvenance::ExactAst,
        Provenance::DesugaredAst
        | Provenance::SemanticAnalyzer
        | Provenance::FrameworkSynthesis
        | Provenance::ImportExportInference
        | Provenance::PragmaInference => CompileProvenance::DesugaredAst,
        Provenance::NameHeuristic | Provenance::SearchFallback | Provenance::DynamicBoundary => {
            CompileProvenance::DynamicBoundary
        }
    }
}

fn fact_confidence_to_compile(confidence: Confidence) -> CompileConfidence {
    match confidence {
        Confidence::High => CompileConfidence::High,
        Confidence::Medium => CompileConfidence::Medium,
        Confidence::Low => CompileConfidence::Low,
    }
}

/// HIR-local scope graph for compiler-substrate proof.
///
/// The graph is intentionally parser-core-local. Later compiler fact export can
/// map these ids to `perl-semantic-facts` ids without changing provider
/// behavior in this first scope/pad slice.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ScopeGraph {
    /// Scope frames in stable creation order.
    pub scopes: Vec<ScopeFrame>,
    /// Bindings in stable declaration order.
    pub bindings: Vec<Binding>,
    /// Variable references observed while lowering.
    pub references: Vec<BindingReference>,
}

impl ScopeGraph {
    /// Return the root file scope, when present.
    #[inline]
    pub fn root_scope(&self) -> Option<&ScopeFrame> {
        self.scopes.first()
    }
}

/// One lexical/package scope frame.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScopeFrame {
    /// Stable scope id.
    pub id: HirScopeId,
    /// Parent scope id, absent for the file scope.
    pub parent: Option<HirScopeId>,
    /// Scope category.
    pub kind: ScopeKind,
    /// Source range covered by the scope.
    pub range: SourceLocation,
    /// Package context active for this scope, when known.
    pub package_context: Option<String>,
}

/// Scope frame category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ScopeKind {
    /// Whole-file root scope.
    File,
    /// Package context scope.
    Package,
    /// Plain block scope.
    Block,
    /// Subroutine pad scope.
    Subroutine,
    /// Method pad scope.
    Method,
    /// Signature parameter scope.
    Signature,
    /// Legacy `format` declaration scope.
    Format,
    /// Dynamic/string eval scope boundary.
    EvalString,
    /// Compile-time phase block scope, such as `BEGIN`.
    PhaseBlock,
}

/// Compiler binding produced from a HIR declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Binding {
    /// Stable binding id.
    pub id: HirBindingId,
    /// Scope that owns this binding.
    pub scope_id: HirScopeId,
    /// Variable sigil.
    pub sigil: String,
    /// Variable name without sigil.
    pub name: String,
    /// Source range of the binding declaration token.
    pub range: SourceLocation,
    /// Storage class represented by the declaration.
    pub storage: StorageClass,
    /// Package context active for this binding, when known.
    pub package_context: Option<String>,
    /// HIR item that declared this binding.
    pub declaration_item: Option<HirId>,
    /// Earlier visible binding shadowed by this declaration, when known.
    pub shadows: Option<HirBindingId>,
}

/// Storage class represented by a binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StorageClass {
    /// Lexical `my` variable.
    LexicalMy,
    /// Persistent lexical `state` variable.
    LexicalState,
    /// `our` package variable made lexically visible.
    PackageOur,
    /// `local` package variable localization.
    LocalizedPackage,
    /// Signature parameter binding.
    Parameter,
    /// Method invocant binding.
    MethodInvocant,
    /// Implicit lexical binding such as `$_`.
    Implicit,
    /// Package global observed without a lexical binding.
    PackageGlobal,
}

/// Variable reference and its lexical binding resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BindingReference {
    /// Scope containing the reference.
    pub scope_id: HirScopeId,
    /// Variable sigil.
    pub sigil: String,
    /// Variable name without sigil.
    pub name: String,
    /// Source range for the reference token.
    pub range: SourceLocation,
    /// Resolved binding, if one was visible in the scope chain.
    pub resolved_binding: Option<HirBindingId>,
}

/// HIR-local package stash graph for compiler-substrate proof.
///
/// This graph is intentionally parser-core-local. It records package/stash
/// facts with provenance and confidence, but no LSP provider consumes it yet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct StashGraph {
    /// Package stashes in stable first-seen order.
    pub packages: Vec<PackageStash>,
    /// Inheritance edges in stable source order.
    pub inheritance_edges: Vec<PackageInheritanceEdge>,
    /// Static Exporter-style declarations in stable source order.
    pub export_declarations: Vec<ExportDeclaration>,
    /// Dynamic stash boundaries in stable source order.
    pub dynamic_boundaries: Vec<StashDynamicBoundary>,
}

impl StashGraph {
    /// Project static HIR/stash export declarations into canonical export facts.
    ///
    /// This is a compiler-substrate projection only. It does not execute Perl,
    /// inspect the filesystem, or change workspace/LSP provider behavior.
    #[must_use]
    pub fn export_sets(&self) -> Vec<ExportSet> {
        let mut builders = BTreeMap::<String, ExportSetBuilder>::new();

        for declaration in &self.export_declarations {
            let builder = builders.entry(declaration.package.clone()).or_insert_with(|| {
                ExportSetBuilder::new(
                    declaration.package.clone(),
                    declaration.range,
                    stash_provenance_to_fact(declaration.provenance),
                    stash_confidence_to_fact(declaration.confidence),
                )
            });
            builder.absorb(declaration);
        }

        builders.into_values().map(ExportSetBuilder::into_export_set).collect()
    }

    /// Project static constant-like code slots into a compile-time constant table.
    ///
    /// The table is a compiler-substrate receipt only. It records facts already
    /// present in the stash graph and does not execute Perl, evaluate constant
    /// values, or change provider behavior.
    #[must_use]
    pub fn constant_table(&self) -> ConstantTable {
        let mut entries = Vec::new();

        for package in &self.packages {
            for slot in &package.slots {
                if slot.kind != GlobSlotKind::Code
                    || slot.source != GlobSlotSource::ConstantDeclaration
                {
                    continue;
                }

                entries.push(ConstantTableEntry {
                    package: package.package.clone(),
                    name: slot.name.clone(),
                    canonical_name: format!("{}::{}", package.package, slot.name),
                    range: slot.range,
                    declaration_item: slot.declaration_item,
                    source: slot.source,
                    provenance: slot.provenance,
                    confidence: slot.confidence,
                });
            }
        }

        ConstantTable { entries }
    }
}

/// Compile-time constant projection derived from the HIR stash graph.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ConstantTable {
    /// Static constant-like code slots in stable source order.
    pub entries: Vec<ConstantTableEntry>,
}

impl ConstantTable {
    /// Returns true when no static constants were recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// One static constant-like code slot.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConstantTableEntry {
    /// Package that owns the constant.
    pub package: String,
    /// Bare constant name.
    pub name: String,
    /// Fully qualified constant name.
    pub canonical_name: String,
    /// Source range for the declaration or desugaring that produced the slot.
    pub range: SourceLocation,
    /// HIR item that produced the slot, when available.
    pub declaration_item: Option<HirId>,
    /// Source shape that produced this constant-like code slot.
    pub source: GlobSlotSource,
    /// How this constant fact was produced.
    pub provenance: StashProvenance,
    /// Confidence in this constant fact.
    pub confidence: StashConfidence,
}

#[derive(Debug)]
struct ExportSetBuilder {
    module_name: String,
    anchor_range: SourceLocation,
    default_exports: Vec<String>,
    optional_exports: Vec<String>,
    tags: BTreeMap<String, Vec<String>>,
    provenance: Provenance,
    confidence: Confidence,
}

impl ExportSetBuilder {
    fn new(
        module_name: String,
        anchor_range: SourceLocation,
        provenance: Provenance,
        confidence: Confidence,
    ) -> Self {
        Self {
            module_name,
            anchor_range,
            default_exports: Vec::new(),
            optional_exports: Vec::new(),
            tags: BTreeMap::new(),
            provenance,
            confidence,
        }
    }

    fn absorb(&mut self, declaration: &ExportDeclaration) {
        if declaration.range.start < self.anchor_range.start {
            self.anchor_range = declaration.range;
        }
        self.provenance =
            combine_provenance(self.provenance, stash_provenance_to_fact(declaration.provenance));
        self.confidence =
            combine_confidence(self.confidence, stash_confidence_to_fact(declaration.confidence));

        match declaration.kind {
            ExportDeclarationKind::Default => {
                self.default_exports.extend(declaration.symbols.iter().cloned());
            }
            ExportDeclarationKind::Optional => {
                self.optional_exports.extend(declaration.symbols.iter().cloned());
            }
            ExportDeclarationKind::Tag => {
                if let Some(tag_name) = &declaration.tag_name {
                    self.tags
                        .entry(tag_name.clone())
                        .or_default()
                        .extend(declaration.symbols.iter().cloned());
                }
            }
        }
    }

    fn into_export_set(mut self) -> ExportSet {
        sort_dedup(&mut self.default_exports);
        sort_dedup(&mut self.optional_exports);

        let tags = self
            .tags
            .into_iter()
            .map(|(name, mut members)| {
                sort_dedup(&mut members);
                ExportTag { name, members }
            })
            .collect();

        ExportSet {
            default_exports: self.default_exports,
            optional_exports: self.optional_exports,
            tags,
            provenance: self.provenance,
            confidence: self.confidence,
            module_name: Some(self.module_name),
            anchor_id: Some(AnchorId(self.anchor_range.start as u64)),
        }
    }
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn combine_provenance(current: Provenance, next: Provenance) -> Provenance {
    if current == Provenance::DynamicBoundary || next == Provenance::DynamicBoundary {
        Provenance::DynamicBoundary
    } else if current == Provenance::ImportExportInference
        || next == Provenance::ImportExportInference
    {
        Provenance::ImportExportInference
    } else {
        current
    }
}

fn combine_confidence(current: Confidence, next: Confidence) -> Confidence {
    match (current, next) {
        (Confidence::Low, _) | (_, Confidence::Low) => Confidence::Low,
        (Confidence::Medium, _) | (_, Confidence::Medium) => Confidence::Medium,
        (Confidence::High, Confidence::High) => Confidence::High,
    }
}

fn stash_provenance_to_fact(provenance: StashProvenance) -> Provenance {
    match provenance {
        StashProvenance::ExactAst => Provenance::ExactAst,
        StashProvenance::DesugaredAst => Provenance::DesugaredAst,
        StashProvenance::DynamicBoundary => Provenance::DynamicBoundary,
    }
}

fn stash_confidence_to_fact(confidence: StashConfidence) -> Confidence {
    match confidence {
        StashConfidence::High => Confidence::High,
        StashConfidence::Medium => Confidence::Medium,
        StashConfidence::Low => Confidence::Low,
    }
}

/// One Perl package stash.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PackageStash {
    /// Package name.
    pub package: String,
    /// Source range that first established this package.
    pub range: SourceLocation,
    /// HIR item that first established this package, when available.
    pub declaration_item: Option<HirId>,
    /// Symbol slots observed for this package.
    pub slots: Vec<GlobSlot>,
    /// How this stash fact was produced.
    pub provenance: StashProvenance,
    /// Confidence in this stash fact.
    pub confidence: StashConfidence,
}

/// One slot inside a Perl typeglob.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GlobSlot {
    /// Symbol name without sigil.
    pub name: String,
    /// Slot category.
    pub kind: GlobSlotKind,
    /// Source range for the declaration or mutation that produced this slot.
    pub range: SourceLocation,
    /// HIR item that produced this slot, when available.
    pub declaration_item: Option<HirId>,
    /// Source shape that produced this slot.
    pub source: GlobSlotSource,
    /// Static alias target, when this slot is an alias.
    pub alias_target: Option<String>,
    /// How this slot fact was produced.
    pub provenance: StashProvenance,
    /// Confidence in this slot fact.
    pub confidence: StashConfidence,
}

/// Perl typeglob slot category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GlobSlotKind {
    /// Scalar slot: `$Package::name`.
    Scalar,
    /// Array slot: `@Package::name`.
    Array,
    /// Hash slot: `%Package::name`.
    Hash,
    /// Code slot: `Package::name()`.
    Code,
    /// IO slot / filehandle slot.
    Io,
    /// Format slot.
    Format,
}

/// Source shape that populated a glob slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GlobSlotSource {
    /// `package` declaration.
    PackageDeclaration,
    /// `sub` declaration.
    SubDeclaration,
    /// `method` declaration.
    MethodDeclaration,
    /// `our` declaration.
    OurDeclaration,
    /// Legacy `format` declaration.
    FormatDeclaration,
    /// `use constant` compile-time declaration.
    ConstantDeclaration,
    /// Package variable assignment such as `@ISA = ...`.
    PackageAssignment,
    /// Static typeglob alias assignment.
    TypeglobAlias,
}

/// Provenance for HIR-local stash facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StashProvenance {
    /// Fact came directly from parser AST syntax.
    ExactAst,
    /// Fact came from a simple compile-time desugaring such as `use parent`.
    DesugaredAst,
    /// Fact came from conservative dynamic-boundary classification.
    DynamicBoundary,
}

/// Confidence for HIR-local stash facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StashConfidence {
    /// High-confidence exact or simple desugared fact.
    High,
    /// Medium-confidence static interpretation.
    Medium,
    /// Low-confidence dynamic-boundary fact.
    Low,
}

/// Inheritance edge established by `@ISA`, `use parent`, or `use base`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PackageInheritanceEdge {
    /// Package inheriting from the target.
    pub from_package: String,
    /// Parent package.
    pub to_package: String,
    /// Source range for the edge.
    pub range: SourceLocation,
    /// HIR item that produced this edge, when available.
    pub declaration_item: Option<HirId>,
    /// Source shape that produced this edge.
    pub source: InheritanceSource,
    /// How this edge fact was produced.
    pub provenance: StashProvenance,
    /// Confidence in this edge fact.
    pub confidence: StashConfidence,
}

/// Source shape that established an inheritance edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InheritanceSource {
    /// `our @ISA = ...`.
    IsaAssignment,
    /// `use parent ...`.
    UseParent,
    /// `use base ...`.
    UseBase,
}

/// Static Exporter-style declaration observed in a package stash.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExportDeclaration {
    /// Package declaring the export list.
    pub package: String,
    /// Export declaration category.
    pub kind: ExportDeclarationKind,
    /// Tag name for `%EXPORT_TAGS` entries.
    pub tag_name: Option<String>,
    /// Static exported symbols.
    pub symbols: Vec<String>,
    /// Source range for the declaration.
    pub range: SourceLocation,
    /// HIR item that produced this declaration, when available.
    pub declaration_item: Option<HirId>,
    /// How this export declaration was produced.
    pub provenance: StashProvenance,
    /// Confidence in this export declaration.
    pub confidence: StashConfidence,
}

/// Export declaration category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExportDeclarationKind {
    /// `@EXPORT`.
    Default,
    /// `@EXPORT_OK`.
    Optional,
    /// `%EXPORT_TAGS`.
    Tag,
}

/// Dynamic stash mutation boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StashDynamicBoundary {
    /// Package affected by the boundary, when known.
    pub package: Option<String>,
    /// Symbol affected by the boundary, when statically known.
    pub symbol: Option<String>,
    /// Source range for the boundary.
    pub range: SourceLocation,
    /// HIR item that also records this boundary, when available.
    pub boundary_item: Option<HirId>,
    /// Boundary category.
    pub kind: StashDynamicBoundaryKind,
    /// Short reason for status/proof output.
    pub reason: String,
    /// How this boundary fact was produced.
    pub provenance: StashProvenance,
    /// Confidence in this boundary fact.
    pub confidence: StashConfidence,
}

/// Dynamic stash boundary category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StashDynamicBoundaryKind {
    /// Stash/typeglob assignment with a non-static RHS.
    DynamicStashMutation,
    /// Export declaration has a non-static member list or tag shape.
    DynamicExportDeclaration,
    /// `AUTOLOAD` makes method lookup dynamic for this package.
    Autoload,
}

/// HIR-local compile environment for compiler-substrate proof.
///
/// This model records compile-time directives, pragma state changes, include
/// roots, module requests, phase blocks, and dynamic boundaries without
/// changing LSP provider behavior.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct CompileEnvironment {
    /// `use`, `no`, and `require` directives in stable source order.
    pub directives: Vec<CompileDirective>,
    /// Pragma or feature effects in stable source order.
    pub pragma_effects: Vec<PragmaEffect>,
    /// Effective strict/warnings/feature state facts in source order.
    pub pragma_state_facts: Vec<PragmaStateFact>,
    /// Include-root effects such as `use lib` and `no lib`.
    pub inc_roots: Vec<IncRootFact>,
    /// Static and dynamic module requests observed in the file.
    pub module_requests: Vec<ModuleRequest>,
    /// Compile-time phase blocks observed in source order.
    pub phase_blocks: Vec<CompilePhaseBlock>,
    /// Unsupported or dynamic compile-environment boundaries.
    pub dynamic_boundaries: Vec<CompileEnvironmentBoundary>,
}

impl CompileEnvironment {
    /// Effective pragma state facts in source order.
    #[must_use]
    pub fn pragma_state_facts(&self) -> &[PragmaStateFact] {
        &self.pragma_state_facts
    }

    /// Return the latest effective pragma state fact at or before `offset`.
    #[must_use]
    pub fn pragma_state_at(&self, offset: usize) -> Option<&PragmaStateFact> {
        let idx = self.pragma_state_facts.partition_point(|fact| fact.range.start <= offset);
        if idx > 0 { self.pragma_state_facts.get(idx - 1) } else { None }
    }

    /// Project HIR compile directives into canonical import facts.
    ///
    /// This is a compiler-substrate projection only. It does not inspect the
    /// filesystem, execute Perl, or change workspace/LSP provider behavior.
    #[must_use]
    pub fn import_specs(&self, file_id: FileId) -> Vec<ImportSpec> {
        self.directives
            .iter()
            .filter_map(|directive| import_spec_from_directive(directive, file_id))
            .collect()
    }

    /// Build module-resolution candidate facts from static module requests.
    ///
    /// The HIR layer records lexical include-root effects and module requests,
    /// but it does not read process environment, inspect the filesystem, or
    /// depend on the downstream `perl-module` resolver. Callers provide
    /// configured, `PERL5LIB`, and system roots explicitly; this method combines
    /// them with source-order lexical `use lib` roots active at each request.
    #[must_use]
    pub fn module_resolution_candidates(
        &self,
        supplied_roots: &[ModuleResolutionRoot],
    ) -> Vec<ModuleResolutionCandidate> {
        self.module_requests
            .iter()
            .enumerate()
            .filter_map(|(request_index, request)| {
                let target = request.target.as_ref()?;
                let normalized_target = normalize_module_target(target);
                let relative_path = module_target_to_relative_path(&normalized_target)?;
                let candidate_roots =
                    self.candidate_roots_for_request(request, &relative_path, supplied_roots);
                let status = if candidate_roots.is_empty() {
                    ModuleResolutionCandidateStatus::NotFound
                } else {
                    ModuleResolutionCandidateStatus::CandidateBuilt
                };

                Some(ModuleResolutionCandidate {
                    request_index,
                    directive_item: request.directive_item,
                    request_kind: request.kind,
                    target: normalized_target,
                    relative_path,
                    roots: candidate_roots,
                    status,
                    resolved_path: None,
                    range: request.range,
                    package_context: request.package_context.clone(),
                    provenance: request.provenance,
                    confidence: request.confidence,
                })
            })
            .collect()
    }

    /// Resolve static module candidate facts using a caller-supplied path predicate.
    ///
    /// This preserves the HIR layer's explicit boundary: the caller supplies
    /// roots and the existence check, so parser-core still does not read
    /// ambient process state or spawn Perl.
    #[must_use]
    pub fn resolved_module_resolution_candidates(
        &self,
        supplied_roots: &[ModuleResolutionRoot],
        mut path_exists: impl FnMut(&str) -> bool,
    ) -> Vec<ModuleResolutionCandidate> {
        let mut candidates = self.module_resolution_candidates(supplied_roots);

        for candidate in &mut candidates {
            if candidate.status != ModuleResolutionCandidateStatus::CandidateBuilt {
                continue;
            }

            if let Some(root) =
                candidate.roots.iter().find(|root| path_exists(&root.candidate_path))
            {
                candidate.status = ModuleResolutionCandidateStatus::Resolved;
                candidate.resolved_path = Some(root.candidate_path.clone());
            } else {
                candidate.status = ModuleResolutionCandidateStatus::NotFound;
            }
        }

        candidates
    }

    fn candidate_roots_for_request(
        &self,
        request: &ModuleRequest,
        relative_path: &str,
        supplied_roots: &[ModuleResolutionRoot],
    ) -> Vec<ModuleResolutionCandidateRoot> {
        let active_lexical_roots = self.active_lexical_roots_for_request(request);
        active_lexical_roots
            .iter()
            .map(|root| ModuleResolutionRoot {
                path: root.path.clone(),
                kind: root.kind,
                source: root.source.clone(),
            })
            .chain(supplied_roots.iter().cloned())
            .enumerate()
            .map(|(precedence, root)| ModuleResolutionCandidateRoot {
                path: root.path.clone(),
                kind: root.kind,
                source: root.source,
                candidate_path: join_candidate_path(&root.path, relative_path),
                precedence,
            })
            .collect()
    }

    fn active_lexical_roots_for_request(&self, request: &ModuleRequest) -> Vec<ActiveLexicalRoot> {
        let mut active = Vec::new();

        for (order, root) in self.inc_roots.iter().enumerate() {
            if root.range.start > request.range.start {
                continue;
            }
            if root.kind != IncRootKind::UseLib {
                continue;
            }

            match root.action {
                IncRootAction::Add => {
                    active.push(ActiveLexicalRoot {
                        path: root.path.clone(),
                        kind: root.kind,
                        source: "use-lib-lexical".to_string(),
                        range_start: root.range.start,
                        order,
                    });
                }
                IncRootAction::Remove => {
                    active.retain(|active_root| active_root.path != root.path);
                }
            }
        }

        active.sort_by(|left, right| {
            right.range_start.cmp(&left.range_start).then_with(|| left.order.cmp(&right.order))
        });

        active
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveLexicalRoot {
    path: String,
    kind: IncRootKind,
    source: String,
    range_start: usize,
    order: usize,
}

/// One compile-time directive.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CompileDirective {
    /// Directive action.
    pub action: CompileDirectiveAction,
    /// Module or pragma name.
    pub module: Option<String>,
    /// Static arguments captured by the parser.
    pub args: Vec<String>,
    /// Source range for the directive.
    pub range: SourceLocation,
    /// HIR item attached to this directive, when one exists.
    pub item_id: Option<HirId>,
    /// Scope containing the directive.
    pub scope_id: Option<HirScopeId>,
    /// Package context active at the directive.
    pub package_context: Option<String>,
    /// Directive classification.
    pub kind: CompileDirectiveKind,
    /// How this fact was produced.
    pub provenance: CompileProvenance,
    /// Confidence in this fact.
    pub confidence: CompileConfidence,
}

/// Compile-time directive action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompileDirectiveAction {
    /// `use Module ...`.
    Use,
    /// `no Module ...`.
    No,
    /// `require Module`.
    Require,
}

/// Compile-time directive classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompileDirectiveKind {
    /// `strict` pragma.
    Strict,
    /// `warnings` pragma.
    Warnings,
    /// `feature` pragma.
    Feature,
    /// `lib` include-path pragma.
    Lib,
    /// Inheritance helper such as `parent` or `base`.
    Inheritance,
    /// Constant declaration helper.
    Constant,
    /// Ordinary module load/import directive.
    Module,
    /// Dynamic or unsupported directive shape.
    Dynamic,
}

/// Pragma or feature state change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PragmaEffect {
    /// Pragma name.
    pub pragma: String,
    /// Whether the pragma is being enabled (`use`) or disabled (`no`).
    pub enabled: bool,
    /// Static, normalized categories or feature names captured by the parser.
    pub args: Vec<String>,
    /// Whether this effect applies broadly or to listed categories/features.
    pub argument_kind: PragmaArgumentKind,
    /// Source range for the effect.
    pub range: SourceLocation,
    /// Directive that produced this effect.
    pub directive_item: Option<HirId>,
    /// Scope containing the effect.
    pub scope_id: Option<HirScopeId>,
    /// Package context active at the effect.
    pub package_context: Option<String>,
    /// How this fact was produced.
    pub provenance: CompileProvenance,
    /// Confidence in this fact.
    pub confidence: CompileConfidence,
}

/// Static pragma argument shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PragmaArgumentKind {
    /// No arguments were supplied, so the pragma transition applies broadly.
    Broad,
    /// Static category, warning, or feature names were supplied.
    Categories,
}

/// Effective strict/warnings/feature state after a compile-time transition.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PragmaStateFact {
    /// Source range for the transition that produced this state.
    pub range: SourceLocation,
    /// Stable source anchor for this transition.
    pub anchor_id: AnchorId,
    /// Directive that produced this state, when HIR has one.
    pub directive_item: Option<HirId>,
    /// Scope containing this transition.
    pub scope_id: Option<HirScopeId>,
    /// Package context active at this transition.
    pub package_context: Option<String>,
    /// Effective `strict vars` state.
    pub strict_vars: bool,
    /// Effective `strict subs` state.
    pub strict_subs: bool,
    /// Effective `strict refs` state.
    pub strict_refs: bool,
    /// Effective global warnings state.
    pub warnings: bool,
    /// Warning categories explicitly disabled in this state.
    pub disabled_warning_categories: Vec<String>,
    /// Effective feature names in this state.
    pub features: Vec<String>,
    /// How this fact was produced.
    pub provenance: CompileProvenance,
    /// Confidence in this fact.
    pub confidence: CompileConfidence,
}

impl PragmaStateFact {
    /// Whether all strict categories are active in this state.
    #[must_use]
    pub fn strict_enabled(&self) -> bool {
        self.strict_vars && self.strict_subs && self.strict_refs
    }

    /// Whether warnings are active for a category in this state.
    #[must_use]
    pub fn warning_active(&self, category: &str) -> bool {
        self.warnings && !self.disabled_warning_categories.iter().any(|name| name == category)
    }

    /// Whether a feature is active in this state.
    #[must_use]
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|name| name == feature)
    }
}

/// Registry for compiler-substrate framework adapters.
///
/// Adapters consume HIR/stash/import compiler facts and emit more compiler
/// facts. They must not directly special-case diagnostics, completion, hover,
/// or navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FrameworkAdapterRegistry {
    adapters: Vec<FrameworkAdapterKind>,
}

impl Default for FrameworkAdapterRegistry {
    fn default() -> Self {
        Self { adapters: vec![FrameworkAdapterKind::ExporterFamily] }
    }
}

impl FrameworkAdapterRegistry {
    /// Create a registry with an explicit adapter set.
    #[must_use]
    pub fn new(adapters: Vec<FrameworkAdapterKind>) -> Self {
        Self { adapters }
    }

    /// Return the adapter kinds enabled in this registry.
    #[must_use]
    pub fn adapters(&self) -> &[FrameworkAdapterKind] {
        &self.adapters
    }

    /// Project framework compiler facts from a lowered HIR file.
    #[must_use]
    pub fn project_file(&self, file: &HirFile) -> FrameworkFactGraph {
        let mut graph = FrameworkFactGraph::default();

        for adapter in &self.adapters {
            match adapter {
                FrameworkAdapterKind::ExporterFamily => {
                    project_exporter_family_facts(file, &mut graph);
                }
            }
        }

        graph
    }
}

/// Framework adapter kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FrameworkAdapterKind {
    /// Exporter and Exporter::Tiny-style export declarations.
    ExporterFamily,
}

/// Facts emitted by framework adapters.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct FrameworkFactGraph {
    /// Framework-exported symbol facts in stable source order.
    pub exported_symbols: Vec<FrameworkExportedSymbolFact>,
    /// Dynamic or unsupported framework boundaries in stable source order.
    pub dynamic_boundaries: Vec<FrameworkDynamicBoundaryFact>,
}

/// One framework-exported symbol fact.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FrameworkExportedSymbolFact {
    /// Adapter that emitted this fact.
    pub adapter: FrameworkAdapterKind,
    /// Package declaring the export.
    pub package: String,
    /// Exported symbol name.
    pub name: String,
    /// Export relationship represented by this fact.
    pub kind: FrameworkExportedSymbolKind,
    /// Tag name when this is a `%EXPORT_TAGS` member.
    pub tag_name: Option<String>,
    /// Source range for the export declaration.
    pub range: SourceLocation,
    /// HIR item that produced the export declaration, when available.
    pub declaration_item: Option<HirId>,
    /// Backing visible-symbol fact for provider-shadow proof.
    pub visible_symbol: VisibleSymbol,
    /// Source declaration provenance.
    pub source_provenance: Provenance,
    /// Source declaration confidence.
    pub source_confidence: Confidence,
    /// Adapter fact provenance.
    pub provenance: Provenance,
    /// Adapter fact confidence.
    pub confidence: Confidence,
}

/// Export relationship represented by a framework fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FrameworkExportedSymbolKind {
    /// Symbol exported by default via `@EXPORT`.
    Default,
    /// Symbol available for explicit import via `@EXPORT_OK`.
    Optional,
    /// Symbol included in a `%EXPORT_TAGS` tag.
    TagMember,
}

/// Dynamic or unsupported framework-adapter boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FrameworkDynamicBoundaryFact {
    /// Adapter that emitted this boundary.
    pub adapter: FrameworkAdapterKind,
    /// Package affected by the boundary, when known.
    pub package: Option<String>,
    /// Symbol affected by the boundary, when statically known.
    pub symbol: Option<String>,
    /// Source range for the boundary.
    pub range: SourceLocation,
    /// HIR item that also records this boundary, when available.
    pub boundary_item: Option<HirId>,
    /// Boundary category.
    pub kind: StashDynamicBoundaryKind,
    /// Short reason for status/proof output.
    pub reason: String,
    /// Adapter fact provenance.
    pub provenance: Provenance,
    /// Adapter fact confidence.
    pub confidence: Confidence,
}

fn project_exporter_family_facts(file: &HirFile, graph: &mut FrameworkFactGraph) {
    for declaration in &file.stash_graph.export_declarations {
        match declaration.kind {
            ExportDeclarationKind::Default => {
                project_exported_symbols_from_declaration(
                    declaration,
                    FrameworkExportedSymbolKind::Default,
                    None,
                    graph,
                );
            }
            ExportDeclarationKind::Optional => {
                project_exported_symbols_from_declaration(
                    declaration,
                    FrameworkExportedSymbolKind::Optional,
                    None,
                    graph,
                );
            }
            ExportDeclarationKind::Tag => {
                project_exported_symbols_from_declaration(
                    declaration,
                    FrameworkExportedSymbolKind::TagMember,
                    declaration.tag_name.as_deref(),
                    graph,
                );
            }
        }
    }

    for boundary in &file.stash_graph.dynamic_boundaries {
        if boundary.kind != StashDynamicBoundaryKind::DynamicExportDeclaration {
            continue;
        }
        graph.dynamic_boundaries.push(FrameworkDynamicBoundaryFact {
            adapter: FrameworkAdapterKind::ExporterFamily,
            package: boundary.package.clone(),
            symbol: boundary.symbol.clone(),
            range: boundary.range,
            boundary_item: boundary.boundary_item,
            kind: boundary.kind,
            reason: boundary.reason.clone(),
            provenance: Provenance::DynamicBoundary,
            confidence: Confidence::Low,
        });
    }
}

fn project_exported_symbols_from_declaration(
    declaration: &ExportDeclaration,
    kind: FrameworkExportedSymbolKind,
    tag_name: Option<&str>,
    graph: &mut FrameworkFactGraph,
) {
    for symbol in &declaration.symbols {
        graph.exported_symbols.push(FrameworkExportedSymbolFact {
            adapter: FrameworkAdapterKind::ExporterFamily,
            package: declaration.package.clone(),
            name: symbol.clone(),
            kind,
            tag_name: tag_name.map(str::to_string),
            range: declaration.range,
            declaration_item: declaration.declaration_item,
            visible_symbol: visible_symbol_for_export(declaration, symbol, kind),
            source_provenance: stash_provenance_to_fact(declaration.provenance),
            source_confidence: stash_confidence_to_fact(declaration.confidence),
            provenance: Provenance::FrameworkSynthesis,
            confidence: Confidence::Medium,
        });
    }
}

fn visible_symbol_for_export(
    declaration: &ExportDeclaration,
    symbol: &str,
    kind: FrameworkExportedSymbolKind,
) -> VisibleSymbol {
    let source = match kind {
        FrameworkExportedSymbolKind::Default => VisibleSymbolSource::DefaultExport,
        FrameworkExportedSymbolKind::Optional => VisibleSymbolSource::ExplicitImport,
        FrameworkExportedSymbolKind::TagMember => VisibleSymbolSource::ExportTag,
    };

    VisibleSymbol {
        name: symbol.to_string(),
        entity_id: None,
        source,
        confidence: Confidence::Medium,
        context: Some(VisibleSymbolContext::new(
            Some(declaration.package.clone()),
            None,
            Some(AnchorId(declaration.range.start as u64)),
        )),
    }
}

/// Include-root effect.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct IncRootFact {
    /// Include root path as written after static cleanup.
    pub path: String,
    /// Whether the root is added or removed.
    pub action: IncRootAction,
    /// Source of the include root.
    pub kind: IncRootKind,
    /// Source range for the effect.
    pub range: SourceLocation,
    /// Directive that produced this effect.
    pub directive_item: Option<HirId>,
    /// Scope containing the effect.
    pub scope_id: Option<HirScopeId>,
    /// Package context active at the effect.
    pub package_context: Option<String>,
    /// How this fact was produced.
    pub provenance: CompileProvenance,
    /// Confidence in this fact.
    pub confidence: CompileConfidence,
}

/// Include-root action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IncRootAction {
    /// Add an include root.
    Add,
    /// Remove an include root.
    Remove,
}

/// Include-root source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IncRootKind {
    /// Root came from `use lib` / `no lib`.
    UseLib,
    /// Root came from configured include paths.
    Configured,
    /// Root came from `PERL5LIB`.
    Perl5Lib,
    /// Root came from system `@INC`.
    SystemInc,
}

/// Caller-supplied include root for module-resolution candidate facts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ModuleResolutionRoot {
    /// Include root path as configured or observed by the caller.
    pub path: String,
    /// Root source category.
    pub kind: IncRootKind,
    /// Human-readable source label for diagnostics/status output.
    pub source: String,
}

impl ModuleResolutionRoot {
    /// Create an explicit include root for module candidate projection.
    #[must_use]
    pub fn new(path: impl Into<String>, kind: IncRootKind, source: impl Into<String>) -> Self {
        Self { path: path.into(), kind, source: source.into() }
    }
}

/// Module load request.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ModuleRequest {
    /// Static target, when known.
    pub target: Option<String>,
    /// Source shape that requested the module.
    pub kind: ModuleRequestKind,
    /// Source range for the request.
    pub range: SourceLocation,
    /// Directive that produced this request.
    pub directive_item: Option<HirId>,
    /// Scope containing the request.
    pub scope_id: Option<HirScopeId>,
    /// Package context active at the request.
    pub package_context: Option<String>,
    /// Static resolution status for this first slice.
    pub resolution: ModuleResolutionStatus,
    /// How this fact was produced.
    pub provenance: CompileProvenance,
    /// Confidence in this fact.
    pub confidence: CompileConfidence,
}

/// Source shape for a module load request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ModuleRequestKind {
    /// `use Module`.
    Use,
    /// `require Module`.
    Require,
    /// `use parent`.
    Parent,
    /// `use base`.
    Base,
}

/// Static module-resolution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ModuleResolutionStatus {
    /// Static module target was recorded, but path resolution is intentionally deferred.
    Deferred,
    /// Module target is dynamic and cannot be resolved statically.
    Dynamic,
}

/// Derived module-resolution candidate fact keyed to a HIR module request.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ModuleResolutionCandidate {
    /// Zero-based request index in [`CompileEnvironment::module_requests`].
    pub request_index: usize,
    /// Directive HIR item that produced this request.
    pub directive_item: Option<HirId>,
    /// Source shape that requested the module.
    pub request_kind: ModuleRequestKind,
    /// Static module target.
    pub target: String,
    /// Relative module path, for example `Foo/Bar.pm`.
    pub relative_path: String,
    /// Ordered candidate roots considered for this request.
    pub roots: Vec<ModuleResolutionCandidateRoot>,
    /// Resolution status for this candidate packet.
    pub status: ModuleResolutionCandidateStatus,
    /// Candidate path selected by the resolver, when a matching file exists.
    pub resolved_path: Option<String>,
    /// Source range for the request.
    pub range: SourceLocation,
    /// Package context active at the request.
    pub package_context: Option<String>,
    /// How this fact was produced.
    pub provenance: CompileProvenance,
    /// Confidence in this fact.
    pub confidence: CompileConfidence,
}

impl ModuleResolutionCandidate {
    /// Build the cache key for this module-resolution candidate.
    ///
    /// The key intentionally records request identity, root provenance/order,
    /// candidate paths, source anchor, and resolver epoch, but not the current
    /// resolution outcome. Candidate existence is tracked separately as an
    /// invalidation input so file appearance/removal can invalidate a cached
    /// result without changing the request identity.
    #[must_use]
    pub fn cache_key(&self, resolver_epoch: u64) -> ModuleResolutionCacheKey {
        ModuleResolutionCacheKey {
            resolver_epoch,
            request_index: self.request_index,
            directive_item: self.directive_item,
            request_kind: self.request_kind,
            target: self.target.clone(),
            relative_path: self.relative_path.clone(),
            roots: self
                .roots
                .iter()
                .map(ModuleResolutionCacheRootKey::from_candidate_root)
                .collect(),
            range: self.range,
            package_context: self.package_context.clone(),
        }
    }

    /// Build cache invalidation inputs for this candidate.
    ///
    /// The caller supplies the path-existence predicate; parser-core still does
    /// not read ambient process state or inspect the filesystem directly.
    #[must_use]
    pub fn cache_invalidation(
        &self,
        resolver_epoch: u64,
        mut path_exists: impl FnMut(&str) -> bool,
    ) -> ModuleResolutionCacheInvalidation {
        let path_states = self
            .roots
            .iter()
            .map(|root| ModuleResolutionCandidatePathState {
                candidate_path: root.candidate_path.clone(),
                exists: path_exists(&root.candidate_path),
            })
            .collect();

        ModuleResolutionCacheInvalidation { key: self.cache_key(resolver_epoch), path_states }
    }
}

/// A single candidate root/path pair for a static module request.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ModuleResolutionCandidateRoot {
    /// Include root path as configured or observed by the caller.
    pub path: String,
    /// Root source category.
    pub kind: IncRootKind,
    /// Human-readable source label.
    pub source: String,
    /// Candidate module path under this root.
    pub candidate_path: String,
    /// Search precedence; lower values are searched first.
    pub precedence: usize,
}

/// Cache key for a static module-resolution candidate.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ModuleResolutionCacheKey {
    /// Caller-supplied resolver epoch, policy version, or filesystem snapshot id.
    pub resolver_epoch: u64,
    /// Zero-based request index in [`CompileEnvironment::module_requests`].
    pub request_index: usize,
    /// Directive HIR item that produced this request.
    pub directive_item: Option<HirId>,
    /// Source shape that requested the module.
    pub request_kind: ModuleRequestKind,
    /// Static module target.
    pub target: String,
    /// Relative module path, for example `Foo/Bar.pm`.
    pub relative_path: String,
    /// Ordered candidate roots included in the cache identity.
    pub roots: Vec<ModuleResolutionCacheRootKey>,
    /// Source range for the request.
    pub range: SourceLocation,
    /// Package context active at the request.
    pub package_context: Option<String>,
}

/// Root identity included in module-resolution cache keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ModuleResolutionCacheRootKey {
    /// Include root path as configured or observed by the caller.
    pub path: String,
    /// Root source category.
    pub kind: IncRootKind,
    /// Human-readable source label.
    pub source: String,
    /// Candidate module path under this root.
    pub candidate_path: String,
    /// Search precedence; lower values are searched first.
    pub precedence: usize,
}

impl ModuleResolutionCacheRootKey {
    fn from_candidate_root(root: &ModuleResolutionCandidateRoot) -> Self {
        Self {
            path: root.path.clone(),
            kind: root.kind,
            source: root.source.clone(),
            candidate_path: root.candidate_path.clone(),
            precedence: root.precedence,
        }
    }
}

/// Candidate path state used to invalidate module-resolution cache entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ModuleResolutionCandidatePathState {
    /// Candidate module path under a searched root.
    pub candidate_path: String,
    /// Whether the caller observed the candidate path as existing.
    pub exists: bool,
}

/// Cache invalidation inputs for a module-resolution candidate.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ModuleResolutionCacheInvalidation {
    /// Stable cache key for the module-resolution request and root set.
    pub key: ModuleResolutionCacheKey,
    /// Candidate path existence states observed by the caller.
    pub path_states: Vec<ModuleResolutionCandidatePathState>,
}

/// Static resolution state for a module candidate packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ModuleResolutionCandidateStatus {
    /// Candidate paths were built but not resolved against the filesystem.
    CandidateBuilt,
    /// Dynamic module target cannot produce candidate paths.
    Dynamic,
    /// Static request has no roots to search.
    NotFound,
    /// Downstream resolver found a matching module.
    Resolved,
    /// Downstream resolver exhausted its timeout budget.
    TimedOut,
}

/// Compile-time phase block.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CompilePhaseBlock {
    /// Phase kind.
    pub phase: CompilePhase,
    /// Source range for the block.
    pub range: SourceLocation,
    /// Scope containing the block.
    pub scope_id: Option<HirScopeId>,
    /// Package context active at the block.
    pub package_context: Option<String>,
    /// How this fact was produced.
    pub provenance: CompileProvenance,
    /// Confidence in this fact.
    pub confidence: CompileConfidence,
}

/// Perl compile/runtime phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompilePhase {
    /// `BEGIN`.
    Begin,
    /// `UNITCHECK`.
    UnitCheck,
    /// `CHECK`.
    Check,
    /// `INIT`.
    Init,
    /// `END`.
    End,
    /// Unknown phase spelling.
    Unknown,
}

/// Dynamic compile-environment boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CompileEnvironmentBoundary {
    /// Boundary category.
    pub kind: CompileEnvironmentBoundaryKind,
    /// Source range for the boundary.
    pub range: SourceLocation,
    /// HIR item that also records this boundary, when available.
    pub boundary_item: Option<HirId>,
    /// Scope containing the boundary.
    pub scope_id: Option<HirScopeId>,
    /// Package context active at the boundary.
    pub package_context: Option<String>,
    /// Short reason for status/proof output.
    pub reason: String,
    /// How this fact was produced.
    pub provenance: CompileProvenance,
    /// Confidence in this fact.
    pub confidence: CompileConfidence,
}

/// Dynamic compile-environment boundary category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompileEnvironmentBoundaryKind {
    /// `require` target could not be determined statically.
    DynamicRequire,
    /// Pragma or feature arguments could not be determined statically.
    DynamicPragmaArgs,
    /// Include-root effect is dynamic or unsupported.
    DynamicIncRoot,
    /// Phase block contains compile-time execution that is not evaluated here.
    PhaseBlockExecution,
    /// Symbolic-reference dereference is possible while `strict refs` is disabled.
    SymbolicReferenceDeref,
}

/// Provenance for HIR-local compile-environment facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompileProvenance {
    /// Fact came directly from parser AST syntax.
    ExactAst,
    /// Fact came from a simple compile-time desugaring.
    DesugaredAst,
    /// Fact came from conservative dynamic-boundary classification.
    DynamicBoundary,
}

/// Confidence for HIR-local compile-environment facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompileConfidence {
    /// High-confidence exact or simple desugared fact.
    High,
    /// Medium-confidence static interpretation.
    Medium,
    /// Low-confidence dynamic-boundary fact.
    Low,
}

fn import_spec_from_directive(directive: &CompileDirective, file_id: FileId) -> Option<ImportSpec> {
    match directive.action {
        CompileDirectiveAction::Use => {
            let module = directive.module.as_deref()?;
            if is_version_pragma(module) {
                return None;
            }
            if module == "constant" {
                return Some(classify_constant_import(directive, file_id));
            }

            let (kind, symbols, provenance, confidence) =
                classify_import_args(&directive.args, module, directive.range);
            Some(import_spec(
                module.to_string(),
                kind,
                symbols,
                provenance,
                confidence,
                directive,
                file_id,
            ))
        }
        CompileDirectiveAction::Require => {
            let (module, kind, symbols, provenance, confidence) =
                if let Some(module) = directive.module.as_ref() {
                    (
                        module.clone(),
                        ImportKind::Require,
                        ImportSymbols::Default,
                        Provenance::ExactAst,
                        Confidence::High,
                    )
                } else {
                    (
                        String::new(),
                        ImportKind::DynamicRequire,
                        ImportSymbols::Dynamic,
                        Provenance::DynamicBoundary,
                        Confidence::Low,
                    )
                };
            Some(import_spec(module, kind, symbols, provenance, confidence, directive, file_id))
        }
        CompileDirectiveAction::No => None,
    }
}

fn import_spec(
    module: String,
    kind: ImportKind,
    symbols: ImportSymbols,
    provenance: Provenance,
    confidence: Confidence,
    directive: &CompileDirective,
    file_id: FileId,
) -> ImportSpec {
    ImportSpec {
        module,
        kind,
        symbols,
        provenance,
        confidence,
        file_id: Some(file_id),
        anchor_id: Some(AnchorId(directive.range.start as u64)),
        scope_id: directive.scope_id.map(|id| ScopeId(id.index() as u64)),
        span_start_byte: Some(directive.range.start as u32),
    }
}

fn classify_import_args(
    args: &[String],
    module: &str,
    range: SourceLocation,
) -> (ImportKind, ImportSymbols, Provenance, Confidence) {
    if args.is_empty() {
        let bare_len = "use ".len() + module.len() + 1;
        let span_len = range.end.saturating_sub(range.start);
        if span_len > bare_len {
            return (
                ImportKind::UseEmpty,
                ImportSymbols::None,
                Provenance::ExactAst,
                Confidence::High,
            );
        }
        return (ImportKind::Use, ImportSymbols::Default, Provenance::ExactAst, Confidence::High);
    }

    let mut explicit_names = Vec::new();
    let mut tags = Vec::new();
    let mut has_dynamic_arg = false;

    for arg in args {
        let trimmed = arg.trim();
        if trimmed == "=>" || trimmed == "," || trimmed == "\\" {
            continue;
        }

        if let Some(inner) = parse_qw_content(trimmed) {
            collect_qw_import_words(inner, &mut explicit_names, &mut tags);
            continue;
        }

        let was_quoted = is_quoted(trimmed);
        let unquoted = unquote(trimmed);
        if !was_quoted && looks_like_dynamic_import_arg(unquoted) {
            has_dynamic_arg = true;
            continue;
        }

        if let Some(tag) = unquoted.strip_prefix(':') {
            tags.push(tag.to_string());
            continue;
        }

        if looks_like_symbol_name(unquoted) {
            explicit_names.push(unquoted.to_string());
        }
    }

    if has_dynamic_arg {
        return (
            ImportKind::UseExplicitList,
            ImportSymbols::Dynamic,
            Provenance::DynamicBoundary,
            Confidence::Low,
        );
    }

    if !tags.is_empty() && explicit_names.is_empty() {
        return (
            ImportKind::UseTag,
            ImportSymbols::Tags(tags),
            Provenance::ExactAst,
            Confidence::High,
        );
    }

    if !tags.is_empty() && !explicit_names.is_empty() {
        return (
            ImportKind::UseExplicitList,
            ImportSymbols::Mixed { tags, names: explicit_names },
            Provenance::ExactAst,
            Confidence::High,
        );
    }

    if !explicit_names.is_empty() {
        return (
            ImportKind::UseExplicitList,
            ImportSymbols::Explicit(explicit_names),
            Provenance::ExactAst,
            Confidence::High,
        );
    }

    (ImportKind::UseEmpty, ImportSymbols::None, Provenance::ExactAst, Confidence::High)
}

fn classify_constant_import(directive: &CompileDirective, file_id: FileId) -> ImportSpec {
    let mut constant_names = Vec::new();
    let args = &directive.args;

    if args.first().map(String::as_str) == Some("{") {
        let mut index = 1;
        while index < args.len() {
            let token = args[index].trim();
            if token == "}" || token == "=>" || token == "," {
                index += 1;
                continue;
            }
            if index + 1 < args.len() && args[index + 1].trim() == "=>" {
                constant_names.push(unquote(token).to_string());
                index += 3;
            } else {
                index += 1;
            }
        }
    } else if let Some(inner) = args.first().and_then(|arg| parse_qw_content(arg.trim())) {
        constant_names.extend(inner.split_whitespace().map(str::to_string));
    } else if let Some(name) = args.first() {
        let trimmed = unquote(name.trim());
        if looks_like_constant_name(trimmed) {
            constant_names.push(trimmed.to_string());
        }
    }

    let mut seen = std::collections::HashSet::new();
    constant_names.retain(|name| seen.insert(name.clone()));

    let symbols = if constant_names.is_empty() {
        ImportSymbols::None
    } else {
        ImportSymbols::Explicit(constant_names)
    };

    import_spec(
        "constant".to_string(),
        ImportKind::UseConstant,
        symbols,
        Provenance::ExactAst,
        Confidence::High,
        directive,
        file_id,
    )
}

fn collect_qw_import_words(inner: &str, explicit_names: &mut Vec<String>, tags: &mut Vec<String>) {
    for word in inner.split_whitespace() {
        if let Some(tag) = word.strip_prefix(':') {
            tags.push(tag.to_string());
        } else {
            explicit_names.push(word.to_string());
        }
    }
}

fn is_version_pragma(module: &str) -> bool {
    if module.chars().next().is_some_and(|character| character.is_ascii_digit()) {
        return true;
    }
    module.starts_with('v')
        && module.len() > 1
        && module[1..].chars().all(|character| character.is_ascii_digit() || character == '.')
}

fn parse_qw_content(value: &str) -> Option<&str> {
    let rest = value.strip_prefix("qw")?.trim_start();
    let mut chars = rest.chars();
    let open = chars.next()?;
    let close = match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        other => other,
    };
    let inner_start = open.len_utf8();
    let inner_end = rest.len().checked_sub(close.len_utf8())?;
    if inner_start > inner_end || !rest.ends_with(close) {
        return None;
    }
    Some(&rest[inner_start..inner_end])
}

fn is_quoted(value: &str) -> bool {
    (value.starts_with('\'') && value.ends_with('\''))
        || (value.starts_with('"') && value.ends_with('"'))
}

fn unquote(value: &str) -> &str {
    if is_quoted(value) && value.len() >= 2 { &value[1..value.len() - 1] } else { value }
}

fn looks_like_dynamic_import_arg(value: &str) -> bool {
    value.starts_with('$')
        || value.starts_with('@')
        || value.starts_with('%')
        || value.starts_with('&')
        || value.starts_with('*')
}

fn looks_like_symbol_name(value: &str) -> bool {
    let value = unquote(value);
    if value.is_empty() {
        return false;
    }
    if value.starts_with(':') {
        return true;
    }
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
}

fn looks_like_constant_name(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
}

fn module_target_to_relative_path(target: &str) -> Option<String> {
    let relative_path =
        if target.ends_with(".pm") || target.ends_with(".pl") || target.contains(['/', '\\']) {
            target.replace('\\', "/")
        } else {
            let canonical = target.replace('\'', "::");
            format!("{}.pm", canonical.replace("::", "/"))
        };

    is_safe_relative_module_path(&relative_path).then_some(relative_path)
}

fn normalize_module_target(target: &str) -> String {
    target.trim().trim_matches('"').trim_matches('\'').to_string()
}

fn is_safe_relative_module_path(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.contains(':') {
        return false;
    }

    path.split('/').all(|segment| !matches!(segment, "" | "." | ".."))
}

fn join_candidate_path(root: &str, relative_path: &str) -> String {
    let normalized_root = root.replace('\\', "/");
    let trimmed_root = normalized_root.trim_end_matches('/');
    if trimmed_root.is_empty() {
        relative_path.to_string()
    } else {
        format!("{trimmed_root}/{relative_path}")
    }
}

/// First-slice HIR constructs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HirKind {
    /// `package Foo;` or block package declaration.
    PackageDecl(PackageDecl),
    /// `sub foo { ... }` declaration.
    SubDecl(SubDecl),
    /// `method foo { ... }` declaration.
    MethodDecl(MethodDecl),
    /// `use Module ...;` declaration.
    UseDecl(UseDecl),
    /// `require Module;` call recognized as a compile-time declaration shape.
    RequireDecl(RequireDecl),
    /// `my`, `our`, `state`, or `local` variable declaration.
    VariableDecl(VariableDecl),
    /// Function-like call expression shell.
    CallExpr(CallExpr),
    /// Method-call expression shell.
    MethodCallExpr(MethodCallExpr),
    /// Indirect-object method-call expression shell.
    IndirectCallExpr(IndirectCallExpr),
    /// Bareword expression shell.
    BarewordExpr(BarewordExpr),
    /// Literal expression shell.
    LiteralExpr(LiteralExpr),
    /// Block expression shell without scope construction.
    BlockShell(BlockShell),
    /// Unsupported or intentionally dynamic Perl boundary.
    DynamicBoundary(DynamicBoundary),
}

impl HirKind {
    /// Canonical names for all first-slice HIR construct variants.
    ///
    /// Metrics and status generators should use this list instead of keeping a
    /// separate copy of the current HIR surface.
    pub const ALL_KIND_NAMES: &[&'static str] = &[
        "BarewordExpr",
        "BlockShell",
        "CallExpr",
        "DynamicBoundary",
        "IndirectCallExpr",
        "LiteralExpr",
        "MethodCallExpr",
        "MethodDecl",
        "PackageDecl",
        "RequireDecl",
        "SubDecl",
        "UseDecl",
        "VariableDecl",
    ];
}

/// Package declaration HIR payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PackageDecl {
    /// Package name.
    pub name: String,
    /// Precise package-name source range.
    pub name_range: SourceLocation,
    /// Whether this declaration owns an inline block.
    pub has_block: bool,
}

/// Subroutine declaration HIR payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SubDecl {
    /// Subroutine name, absent for anonymous subs.
    pub name: Option<String>,
    /// Precise subroutine-name source range when available.
    pub name_range: Option<SourceLocation>,
    /// Whether the declaration has a prototype.
    pub has_prototype: bool,
    /// Whether the declaration has a signature.
    pub has_signature: bool,
    /// Number of parsed attributes.
    pub attribute_count: usize,
}

/// Method declaration HIR payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MethodDecl {
    /// Method name.
    pub name: String,
    /// Whether the declaration has a signature.
    pub has_signature: bool,
    /// Number of parsed attributes.
    pub attribute_count: usize,
}

/// Use declaration HIR payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct UseDecl {
    /// Module or pragma name.
    pub module: String,
    /// Parsed import arguments.
    pub args: Vec<String>,
    /// Whether the parser classified the module as a source-filter risk.
    pub has_filter_risk: bool,
}

/// Require declaration HIR payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RequireDecl {
    /// Statically recognized require target when available.
    pub target: Option<String>,
    /// Number of parser arguments on the underlying function call.
    pub arg_count: usize,
}

/// Variable declaration HIR payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VariableDecl {
    /// Scope/storage declarator: `my`, `our`, `state`, or `local`.
    pub declarator: String,
    /// Variables statically visible in the declaration.
    pub variables: Vec<VariableBinding>,
    /// Number of parsed attributes on the declaration.
    pub attribute_count: usize,
    /// Whether the declaration has an initializer expression.
    pub has_initializer: bool,
    /// Whether this came from a list declaration.
    pub is_list: bool,
}

/// One variable binding named by a declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VariableBinding {
    /// Variable sigil.
    pub sigil: String,
    /// Variable name without sigil.
    pub name: String,
    /// Source range for the variable token.
    pub range: SourceLocation,
}

/// Function-like call shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CallExpr {
    /// Callee name, or parser sentinel for dynamic call forms.
    pub name: String,
    /// Number of parsed arguments.
    pub arg_count: usize,
    /// Parser-observed call shape.
    pub form: CallForm,
}

/// Parser-observed call shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CallForm {
    /// A named function call such as `foo(...)`.
    NamedFunction,
    /// A coderef/dynamic callee call such as `$callback->(...)`.
    Coderef,
}

/// Method-call shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MethodCallExpr {
    /// Method name.
    pub method: String,
    /// Number of parsed arguments.
    pub arg_count: usize,
    /// Parser AST kind for the receiver expression.
    pub object_kind: &'static str,
}

/// Indirect-object call shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct IndirectCallExpr {
    /// Method name.
    pub method: String,
    /// Number of parsed arguments.
    pub arg_count: usize,
    /// Parser AST kind for the receiver/class expression.
    pub object_kind: &'static str,
}

/// Bareword expression shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BarewordExpr {
    /// Bareword text as parsed.
    pub name: String,
}

/// Literal expression shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LiteralExpr {
    /// Literal category.
    pub kind: LiteralKind,
    /// Preserved value for compact scalar literals.
    pub value: Option<String>,
    /// Whether the literal can interpolate variables.
    pub interpolated: Option<bool>,
    /// Element count for aggregate literals.
    pub element_count: Option<usize>,
    /// Pair count for hash literals.
    pub pair_count: Option<usize>,
}

/// Literal category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LiteralKind {
    /// Numeric literal.
    Number,
    /// String literal.
    String,
    /// `undef`.
    Undef,
    /// Array/list literal.
    Array,
    /// Hash literal.
    Hash,
}

/// Block shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BlockShell {
    /// Number of parsed statements directly inside the block.
    pub statement_count: usize,
}

/// Dynamic-boundary shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DynamicBoundary {
    /// Boundary category.
    pub kind: DynamicBoundaryKind,
    /// Short human-readable reason for the boundary.
    pub reason: String,
}

/// Dynamic-boundary category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DynamicBoundaryKind {
    /// Coderef/dynamic callee call through `->()`.
    CoderefCall,
    /// `eval` whose body is not a statically parsed block.
    EvalExpression,
    /// `do` whose body is not a statically parsed block.
    DoExpression,
    /// Stash/typeglob assignment whose effect cannot be modeled statically.
    DynamicStashMutation,
    /// `AUTOLOAD` declaration introduces dynamic method dispatch.
    Autoload,
    /// Symbolic-reference dereference whose target cannot be modeled statically.
    SymbolicReferenceDeref,
}
