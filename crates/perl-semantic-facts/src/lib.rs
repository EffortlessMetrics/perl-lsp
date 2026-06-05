//! Neutral semantic fact vocabulary for Perl analysis layers.
//!
//! This crate defines strongly-typed IDs and serializable fact records that can be shared
//! between parser-derived semantics, semantic analyzer synthesis, and workspace indexing.
//!
//! It intentionally does **not** parse Perl, implement LSP providers, or own workspace
//! storage backends.

use serde::{Deserialize, Serialize};

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        pub struct $name(pub u64);
    };
}

id_newtype!(FileId);
id_newtype!(ScopeId);
id_newtype!(EntityId);
id_newtype!(AnchorId);
id_newtype!(OccurrenceId);
id_newtype!(EdgeId);
id_newtype!(DiagnosticId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EntityKind {
    Package,
    Class,
    Role,
    Subroutine,
    Method,
    Variable,
    Constant,
    Field,
    Label,
    Format,
    Module,
    GeneratedMember,
    ExternalSymbol,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OccurrenceKind {
    Definition,
    Reference,
    Read,
    Write,
    Call,
    MethodCall,
    StaticMethodCall,
    CoderefReference,
    TypeglobReference,
    Import,
    Export,
    Inheritance,
    RoleComposition,
    GeneratedUse,
    DynamicBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    Defines,
    References,
    Reads,
    Writes,
    Calls,
    ImportsModule,
    ImportsSymbol,
    ExportsSymbol,
    ExportsGroup,
    Inherits,
    ComposesRole,
    MemberOf,
    GeneratedFrom,
    AliasOf,
    DependsOn,
    DynamicBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Provenance {
    ExactAst,
    DesugaredAst,
    SemanticAnalyzer,
    FrameworkSynthesis,
    ImportExportInference,
    PragmaInference,
    NameHeuristic,
    SearchFallback,
    DynamicBoundary,
    /// Exact `require Module; Module->import(literal list)` pattern where all
    /// import arguments are literal strings or `qw(...)` words — no variables,
    /// no computed expressions.  More precise than `ExactAst` because it
    /// guarantees the symbol list is fully statically known.
    LiteralRequireImport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorFact {
    pub id: AnchorId,
    pub file_id: FileId,
    pub span_start_byte: u32,
    pub span_end_byte: u32,
    pub scope_id: Option<ScopeId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityFact {
    pub id: EntityId,
    pub kind: EntityKind,
    pub canonical_name: String,
    pub anchor_id: Option<AnchorId>,
    pub scope_id: Option<ScopeId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OccurrenceFact {
    pub id: OccurrenceId,
    pub kind: OccurrenceKind,
    pub entity_id: Option<EntityId>,
    pub anchor_id: AnchorId,
    pub scope_id: Option<ScopeId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeFact {
    pub id: EdgeId,
    pub kind: EdgeKind,
    pub from_entity_id: EntityId,
    pub to_entity_id: EntityId,
    pub via_occurrence_id: Option<OccurrenceId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticFact {
    pub id: DiagnosticId,
    pub code: Option<String>,
    pub message: String,
    pub primary_anchor_id: AnchorId,
    pub related_anchor_ids: Vec<AnchorId>,
    pub scope_id: Option<ScopeId>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

/// Canonical export facts inferred for a Perl package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportSet {
    /// Package symbols exported by default (`@EXPORT`).
    pub default_exports: Vec<String>,
    /// Package symbols exported on request (`@EXPORT_OK`).
    pub optional_exports: Vec<String>,
    /// Named export groups (`%EXPORT_TAGS`).
    pub tags: Vec<ExportTag>,
    /// How this export set was inferred.
    pub provenance: Provenance,
    /// Confidence for the inferred export set.
    pub confidence: Confidence,
    /// Exporting module or package name, when known.
    pub module_name: Option<String>,
    /// Source anchor of the export declaration, when available.
    pub anchor_id: Option<AnchorId>,
}

/// Named `%EXPORT_TAGS` entry and its members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportTag {
    /// Tag name (for example `all` from `:all`).
    pub name: String,
    /// Symbols in this tag.
    pub members: Vec<String>,
}

/// Canonical import specification inferred for a single import site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSpec {
    /// Imported module or package name.
    pub module: String,
    /// Syntactic import shape used at the site.
    pub kind: ImportKind,
    /// Symbol selection policy represented at the site.
    pub symbols: ImportSymbols,
    /// Import site provenance.
    pub provenance: Provenance,
    /// Confidence for inferred import semantics.
    pub confidence: Confidence,
    /// File containing this import site, when known.
    pub file_id: Option<FileId>,
    /// Source anchor for this import site, when available.
    pub anchor_id: Option<AnchorId>,
    /// Scope enclosing this import site, when known.
    pub scope_id: Option<ScopeId>,
    /// Byte offset of the start of this import statement in the source file.
    ///
    /// Used for order-aware suppression: a dynamic import only suppresses
    /// barewords that appear **after** the import statement in the file.
    /// `None` means the position is unknown; callers should be conservative
    /// (no suppression) when this is absent.
    pub span_start_byte: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportKind {
    Use,
    UseEmpty,
    UseExplicitList,
    UseTag,
    Require,
    RequireThenImport,
    UseConstant,
    DynamicRequire,
    /// A `Class->import(...)` method call — not a `use` statement.
    ///
    /// Used when a class's `import` method is called directly, typically with
    /// a dynamic argument list (`Foo->import(@names)`).  This is distinct from
    /// `Use` (which is a `use Foo` statement) and `Require` (which is a bare
    /// `require Foo` statement).
    ManualImport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportSymbols {
    Default,
    None,
    Explicit(Vec<String>),
    Tags(Vec<String>),
    Mixed { tags: Vec<String>, names: Vec<String> },
    Dynamic,
}

/// A single `use lib`/`no lib` include-path entry extracted from a Perl file.
///
/// Distinct from [`ImportSpec`]: `ImportSpec` is import-site-scoped (what module
/// is being imported), while `UseLibFact` is path-entry-scoped (what directory
/// is being added to or removed from `@INC`).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UseLibFact {
    /// The literal path string as it appeared in the source (after unquoting).
    ///
    /// For `use lib '../lib'`, this is `"../lib"`.
    /// Dynamic args (`use lib $var`, `use lib @dirs`) are never emitted —
    /// no fact is created for those cases.
    pub path: String,
    /// `true` = added by `use lib`; `false` = removed by `no lib`.
    ///
    /// Facts are per-statement, not net state — both facts are emitted when
    /// `use lib 'x'` is followed by `no lib 'x'`. Callers compute net state
    /// from the sequence.
    pub is_active: bool,
    /// File containing this statement.
    pub file_id: FileId,
    /// Anchor of the `use lib`/`no lib` statement.
    pub anchor_id: Option<AnchorId>,
    /// `ExactAst` for static literal entries; dynamic args are not emitted.
    pub provenance: Provenance,
    /// `High` for static string literals. Dynamic args are never emitted.
    pub confidence: Confidence,
}

impl UseLibFact {
    /// Create a new `UseLibFact`.
    pub fn new(
        path: String,
        is_active: bool,
        file_id: FileId,
        anchor_id: Option<AnchorId>,
        provenance: Provenance,
        confidence: Confidence,
    ) -> Self {
        Self { path, is_active, file_id, anchor_id, provenance, confidence }
    }
}

/// One symbol visible at a query point with source attribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleSymbol {
    /// Symbol display name visible at the query point.
    pub name: String,
    /// Optional backing entity when known.
    pub entity_id: Option<EntityId>,
    /// Visibility source classification.
    pub source: VisibleSymbolSource,
    /// Visibility confidence.
    pub confidence: Confidence,
    /// Optional origin metadata for hover explanations and rename safety.
    pub context: Option<VisibleSymbolContext>,
}

/// Origin metadata for a [`VisibleSymbol`], enabling hover explanations
/// and rename safety analysis.
///
/// Tracks the source module and the import/export anchor IDs that made
/// the symbol visible at the query point.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleSymbolContext {
    /// Module that originally defined or exported the symbol.
    pub source_module: Option<String>,
    /// Anchor of the `use`/`require` statement that imported the symbol.
    pub source_import_anchor_id: Option<AnchorId>,
    /// Anchor of the `@EXPORT`/`@EXPORT_OK` declaration that exported the symbol.
    pub source_export_anchor_id: Option<AnchorId>,
}

impl VisibleSymbolContext {
    /// Create a new `VisibleSymbolContext`.
    ///
    /// Required because `#[non_exhaustive]` prevents struct-literal
    /// construction outside this crate.
    pub fn new(
        source_module: Option<String>,
        source_import_anchor_id: Option<AnchorId>,
        source_export_anchor_id: Option<AnchorId>,
    ) -> Self {
        Self { source_module, source_import_anchor_id, source_export_anchor_id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisibleSymbolSource {
    LocalLexical,
    LocalPackage,
    ExplicitImport,
    DefaultExport,
    ExportTag,
    Constant,
    Generated,
    External,
    DynamicUnknown,
}

// ── Definition Ranking ──

/// Coarse ranking tier for a definition candidate.
///
/// Variants are ordered from most specific (best) to least specific (worst).
/// The `Ord` derive reflects this ordering so that sorting a candidate list
/// places `ExactQualified` first and `Heuristic` last.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DefinitionRank {
    ExactQualified,
    SamePackage,
    ExplicitImport,
    DefaultExport,
    WorkspaceCandidate,
    Heuristic,
}

/// Structured reason explaining why a [`DefinitionCandidate`] received its rank.
///
/// Variants that carry a `module` field identify the specific module that
/// contributed the import or export relationship.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefinitionRankReason {
    ExactQualifiedName,
    SamePackage,
    ExplicitImport { module: String },
    DefaultExport { module: String },
    WorkspaceSymbol,
    HeuristicNameMatch,
}

/// A ranked entry in a definition candidate list.
///
/// Produced by the workspace definition index and consumed by
/// `SemanticQueries::definitions` to present ordered results to providers.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionCandidate {
    /// The entity this candidate refers to.
    pub entity_id: EntityId,
    /// Source anchor for the definition site.
    pub anchor_id: AnchorId,
    /// Fully qualified canonical name of the entity.
    pub canonical_name: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Owning package, if known.
    pub package: Option<String>,
    /// Entity kind (Subroutine, Method, Variable, etc.).
    pub kind: EntityKind,
    /// How this candidate was discovered.
    pub provenance: Provenance,
    /// Confidence in the candidate.
    pub confidence: Confidence,
    /// Coarse ranking tier.
    pub rank: DefinitionRank,
    /// Structured reason for the assigned rank.
    pub rank_reason: DefinitionRankReason,
}

impl DefinitionCandidate {
    /// Construct a new `DefinitionCandidate`.
    ///
    /// Required because the struct is `#[non_exhaustive]` and cannot be
    /// constructed with struct-literal syntax outside this crate.
    #[allow(clippy::too_many_arguments)] // mirrors the struct fields 1-to-1
    pub fn new(
        entity_id: EntityId,
        anchor_id: AnchorId,
        canonical_name: String,
        display_name: String,
        package: Option<String>,
        kind: EntityKind,
        provenance: Provenance,
        confidence: Confidence,
        rank: DefinitionRank,
        rank_reason: DefinitionRankReason,
    ) -> Self {
        Self {
            entity_id,
            anchor_id,
            canonical_name,
            display_name,
            package,
            kind,
            provenance,
            confidence,
            rank,
            rank_reason,
        }
    }
}

/// Occurrence-based reference edge linking a reference site to zero, one, or many
/// target entity candidates.
///
/// Unlike [`EdgeFact`] which connects two known entities, a `ReferenceEdge` is
/// anchored on an [`OccurrenceFact`] and carries a candidate list that may be empty
/// (unresolved), singular (exact), or plural (ambiguous).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceEdge {
    /// The occurrence that produced this reference.
    pub occurrence_id: OccurrenceId,
    /// Source anchor for the reference site.
    pub anchor_id: AnchorId,
    /// File containing the reference.
    pub file_id: FileId,
    /// Bare or qualified symbol key used at the reference site.
    pub symbol_key: String,
    /// Zero, one, or many candidate target entities.
    pub target_candidates: Vec<EntityId>,
    /// Occurrence classification (Read, Call, MethodCall, etc.).
    pub kind: OccurrenceKind,
    /// How this reference was inferred.
    pub provenance: Provenance,
    /// Confidence in the resolution.
    pub confidence: Confidence,
}

// ── Package Graph ──

/// A node in the package/class/role graph.
///
/// Represents a known package, class, or role in the workspace.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageNode {
    /// Entity backing this package node.
    pub entity_id: EntityId,
    /// Fully qualified package name.
    pub name: String,
    /// Classification of this package node.
    pub kind: PackageKind,
    /// Source anchor for the package declaration, when available.
    pub anchor_id: Option<AnchorId>,
    /// File containing this package declaration, when known.
    pub file_id: Option<FileId>,
}

impl PackageNode {
    /// Construct a new `PackageNode`.
    ///
    /// Required because the struct is `#[non_exhaustive]` and cannot be
    /// constructed with struct-literal syntax outside this crate.
    pub fn new(
        entity_id: EntityId,
        name: String,
        kind: PackageKind,
        anchor_id: Option<AnchorId>,
        file_id: Option<FileId>,
    ) -> Self {
        Self { entity_id, name, kind, anchor_id, file_id }
    }
}

/// Classification of a package graph node.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageKind {
    /// A plain Perl package.
    Package,
    /// A class (Moo/Moose/native).
    Class,
    /// A role (Moo::Role/Moose::Role/Role::Tiny).
    Role,
    /// An external package not found in the workspace.
    External,
}

/// A directed edge in the package graph.
///
/// Connects a source package to a target package with a relationship kind
/// (inheritance, role composition, or dependency).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageEdge {
    /// Source package name (the inheriting/consuming package).
    pub from_package: String,
    /// Target package name (the inherited/composed package).
    pub to_package: String,
    /// Relationship kind.
    pub kind: PackageEdgeKind,
    /// Source anchor for the statement that established this edge.
    pub anchor_id: Option<AnchorId>,
    /// How this edge was inferred.
    pub provenance: Provenance,
    /// Confidence in this edge.
    pub confidence: Confidence,
}

impl PackageEdge {
    /// Construct a new `PackageEdge`.
    ///
    /// Required because the struct is `#[non_exhaustive]` and cannot be
    /// constructed with struct-literal syntax outside this crate.
    pub fn new(
        from_package: String,
        to_package: String,
        kind: PackageEdgeKind,
        anchor_id: Option<AnchorId>,
        provenance: Provenance,
        confidence: Confidence,
    ) -> Self {
        Self { from_package, to_package, kind, anchor_id, provenance, confidence }
    }
}

/// Kind of relationship in the package graph.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageEdgeKind {
    /// Inheritance: `use parent`, `use base`, `@ISA`, `extends`.
    Inherits,
    /// Role composition: `with 'Role'`.
    ComposesRole,
    /// General dependency (e.g. `use Module`).
    DependsOn,
}

// ── Generated Members ──────────────────────────────────────────────────

/// A framework-synthesized member (e.g. Moo/Moose accessor from `has`).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedMember {
    /// Deterministic entity ID for this generated member.
    pub entity_id: EntityId,
    /// Name of the generated method (e.g. `x` for `has 'x'`).
    pub name: String,
    /// What kind of generated member this is.
    pub kind: GeneratedMemberKind,
    /// Anchor of the `has` declaration that generated this member.
    pub source_anchor_id: AnchorId,
    /// Package that owns this generated member.
    pub package: String,
    /// Always `FrameworkSynthesis` for generated members.
    pub provenance: Provenance,
    /// Always `Medium` for generated members.
    pub confidence: Confidence,
}

impl GeneratedMember {
    /// Construct a new `GeneratedMember`.
    ///
    /// Required because the struct is `#[non_exhaustive]` and cannot be
    /// constructed with struct-literal syntax outside this crate.
    #[allow(clippy::too_many_arguments)] // mirrors the struct fields 1-to-1
    pub fn new(
        entity_id: EntityId,
        name: String,
        kind: GeneratedMemberKind,
        source_anchor_id: AnchorId,
        package: String,
        provenance: Provenance,
        confidence: Confidence,
    ) -> Self {
        Self { entity_id, name, kind, source_anchor_id, package, provenance, confidence }
    }
}

/// Classification of a framework-generated member.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneratedMemberKind {
    /// Read-only accessor (getter).
    Getter,
    /// Read-write accessor (setter).
    Setter,
    /// Combined read-write accessor (single method for both get and set).
    Accessor,
    /// Predicate method (`has_<attr>`).
    Predicate,
    /// Clearer method (`clear_<attr>`).
    Clearer,
    /// Builder method (`_build_<attr>`).
    Builder,
    /// Constant value.
    Constant,
}

// ── Value Shape ─────────────────────────────────────────────────────

/// Lightweight type approximation for a variable or expression.
///
/// Used for method candidate filtering — not a full type system.
/// `bless` is not a separate top-level shape; it produces
/// [`ValueShape::Object`] with low confidence.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueShape {
    /// Shape could not be determined.
    Unknown,
    /// Plain scalar value.
    Scalar,
    /// Array reference (`[...]` or `\@arr`).
    ArrayRef,
    /// Hash reference (`{...}` or `\%hash`).
    HashRef,
    /// Code reference (`sub { ... }` or `\&sub`).
    CodeRef,
    /// A package name used as a class (e.g. `Foo` in `Foo->method`).
    PackageName {
        /// Fully qualified package name.
        package: String,
    },
    /// An object instance (blessed reference).
    Object {
        /// Package the object was blessed into.
        package: String,
        /// Confidence in the inferred package.
        confidence: Confidence,
    },
}

// ── Provider Fact-Source Tracing ─────────────────────────────────────

/// LSP provider surface that consumed or considered a semantic fact.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProviderSurface {
    Diagnostics,
    Completion,
    Hover,
    Definition,
    References,
    Rename,
    SafeDelete,
    WorkspaceSymbols,
    DocumentSymbols,
    SemanticTokens,
}

/// Coarse source class for a provider answer.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProviderFactSourceKind {
    /// Parser tokens or AST shape.
    ParserSyntax,
    /// Legacy workspace index or provider-local data.
    LegacyWorkspace,
    /// Canonical semantic fact graph.
    SemanticFact,
    /// Rust compiler-substrate fact.
    CompilerFact,
    /// Framework adapter projection.
    FrameworkAdapter,
    /// Dynamic-boundary fact used to avoid false precision.
    DynamicBoundary,
    /// Fallback behavior because no stronger fact was available.
    Fallback,
    /// Source is intentionally unknown or unavailable.
    Unknown,
}

/// Freshness state for a provider fact source.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProviderFactFreshness {
    Fresh,
    Stale,
    Unknown,
    NotApplicable,
}

/// How a provider used a traced fact source.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProviderFallbackState {
    /// Primary answer path used this source.
    Primary,
    /// Source was measured but not used for live behavior.
    Shadow,
    /// Provider fell back from a stronger unavailable source.
    Fallback,
    /// Source existed but could not answer this request.
    Unavailable,
    /// Source blocked an unsafe provider action.
    Blocked,
}

/// Source/provenance trace for a provider answer.
///
/// This is an additive contract for provider cutover proof. It lets providers
/// report where an answer came from before any broad provider behavior change.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderFactTrace {
    /// Provider surface that produced the answer.
    pub surface: ProviderSurface,
    /// Coarse source class used by the provider.
    pub source: ProviderFactSourceKind,
    /// Semantic provenance for the underlying fact, when known.
    pub provenance: Provenance,
    /// Confidence in the underlying fact or fallback.
    pub confidence: Confidence,
    /// Freshness of the underlying fact relative to the request.
    pub freshness: ProviderFactFreshness,
    /// Whether this source drove live behavior, shadow proof, fallback, or a blocker.
    pub fallback_state: ProviderFallbackState,
    /// Optional stable source hash used by the producer.
    pub source_hash: Option<String>,
    /// Optional semantic anchor used by the producer.
    pub anchor_id: Option<AnchorId>,
    /// Optional fact/model version used by the producer.
    pub model_version: Option<u32>,
}

impl ProviderFactTrace {
    /// Construct a new provider fact trace.
    #[allow(clippy::too_many_arguments)] // mirrors the public trace fields 1-to-1
    pub fn new(
        surface: ProviderSurface,
        source: ProviderFactSourceKind,
        provenance: Provenance,
        confidence: Confidence,
        freshness: ProviderFactFreshness,
        fallback_state: ProviderFallbackState,
        source_hash: Option<String>,
        anchor_id: Option<AnchorId>,
        model_version: Option<u32>,
    ) -> Self {
        Self {
            surface,
            source,
            provenance,
            confidence,
            freshness,
            fallback_state,
            source_hash,
            anchor_id,
            model_version,
        }
    }
}

// ── Rename and Safe Delete Plans ────────────────────────────────────

/// A conservative rename plan enumerating affected occurrences and blockers.
///
/// Produced by `SemanticQueries::rename_plan` and consumed by the rename
/// provider to decide whether the rename is safe to apply.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenamePlan {
    /// The entity being renamed.
    pub entity_id: EntityId,
    /// Current name of the entity.
    pub old_name: String,
    /// Proposed new name.
    pub new_name: String,
    /// Planned text edits for the rename.
    pub edits: Vec<PlannedEdit>,
    /// Conditions that block the rename.
    pub blockers: Vec<PlanBlocker>,
    /// Non-blocking warnings for the rename.
    pub warnings: Vec<PlanWarning>,
}

impl RenamePlan {
    /// Construct a new `RenamePlan`.
    ///
    /// Required because the struct is `#[non_exhaustive]` and cannot be
    /// constructed with struct-literal syntax outside this crate.
    pub fn new(
        entity_id: EntityId,
        old_name: String,
        new_name: String,
        edits: Vec<PlannedEdit>,
        blockers: Vec<PlanBlocker>,
        warnings: Vec<PlanWarning>,
    ) -> Self {
        Self { entity_id, old_name, new_name, edits, blockers, warnings }
    }
}

/// A conservative safe-delete plan enumerating blockers that prevent deletion.
///
/// Produced by `SemanticQueries::safe_delete_plan` and consumed by the
/// safe-delete provider to decide whether deletion is safe.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeDeletePlan {
    /// The entity being considered for deletion.
    pub entity_id: EntityId,
    /// Name of the entity being considered for deletion.
    pub name: String,
    /// Conditions that block the deletion.
    pub blockers: Vec<PlanBlocker>,
    /// Non-blocking warnings for the deletion.
    pub warnings: Vec<PlanWarning>,
}

impl SafeDeletePlan {
    /// Construct a new `SafeDeletePlan`.
    ///
    /// Required because the struct is `#[non_exhaustive]` and cannot be
    /// constructed with struct-literal syntax outside this crate.
    pub fn new(
        entity_id: EntityId,
        name: String,
        blockers: Vec<PlanBlocker>,
        warnings: Vec<PlanWarning>,
    ) -> Self {
        Self { entity_id, name, blockers, warnings }
    }
}

/// A condition that blocks a rename or safe-delete operation.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanBlocker {
    /// Why the operation is blocked.
    pub reason: PlanBlockerReason,
    /// Source anchor for the blocking reference, when available.
    pub anchor_id: Option<AnchorId>,
    /// Human-readable description of the blocker.
    pub description: String,
}

impl PlanBlocker {
    /// Construct a new `PlanBlocker`.
    ///
    /// Required because the struct is `#[non_exhaustive]` and cannot be
    /// constructed with struct-literal syntax outside this crate.
    pub fn new(
        reason: PlanBlockerReason,
        anchor_id: Option<AnchorId>,
        description: String,
    ) -> Self {
        Self { reason, anchor_id, description }
    }
}

/// Reason a rename or safe-delete operation is blocked.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanBlockerReason {
    /// Reference crosses a dynamic boundary (string eval, symbolic deref, AUTOLOAD).
    DynamicBoundary,
    /// Reference is ambiguous (multiple candidate targets).
    AmbiguousReference,
    /// Symbol is exported and referenced from other modules.
    CrossModuleExport,
    /// Symbol is imported by another file.
    ImportedSymbol,
    /// Symbol is listed in an ExportSet.
    ExportedSymbol,
    /// Symbol has remaining references in the workspace.
    ReferencesExist,
    /// Symbol is a generated member without a generator-specific edit plan.
    GeneratedMember,
    /// Fact freshness is stale, so the plan cannot authorize edits or deletion.
    StaleFact,
    /// Occurrence could not be classified into a known category.
    UnclassifiedOccurrence,
}

/// A non-blocking warning attached to a rename or safe-delete plan.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanWarning {
    /// Human-readable warning message.
    pub message: String,
    /// Source anchor for the warning site, when available.
    pub anchor_id: Option<AnchorId>,
}

impl PlanWarning {
    /// Construct a new `PlanWarning`.
    ///
    /// Required because the struct is `#[non_exhaustive]` and cannot be
    /// constructed with struct-literal syntax outside this crate.
    pub fn new(message: String, anchor_id: Option<AnchorId>) -> Self {
        Self { message, anchor_id }
    }
}

/// A planned text edit within a rename operation.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedEdit {
    /// Source anchor for the edit site.
    pub anchor_id: AnchorId,
    /// File containing the edit site.
    pub file_id: FileId,
    /// Classification of this edit (definition, reference, import, export).
    pub category: PlannedEditCategory,
    /// Text being replaced.
    pub old_text: String,
    /// Replacement text.
    pub new_text: String,
}

impl PlannedEdit {
    /// Construct a new `PlannedEdit`.
    ///
    /// Required because the struct is `#[non_exhaustive]` and cannot be
    /// constructed with struct-literal syntax outside this crate.
    pub fn new(
        anchor_id: AnchorId,
        file_id: FileId,
        category: PlannedEditCategory,
        old_text: String,
        new_text: String,
    ) -> Self {
        Self { anchor_id, file_id, category, old_text, new_text }
    }
}

/// Classification of a planned edit within a rename operation.
///
/// Distinguishes definition edits from reference edits, import list edits,
/// and export list edits so that the rename provider can handle each
/// category appropriately.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlannedEditCategory {
    /// Edit to the symbol's definition site.
    Definition,
    /// Edit to a reference (call site, read, write).
    Reference,
    /// Edit to an import list (`use Foo qw(...)` argument).
    ImportList,
    /// Edit to an export list (`@EXPORT`, `@EXPORT_OK`, `%EXPORT_TAGS` entry).
    ExportList,
}

impl ReferenceEdge {
    /// Construct a new `ReferenceEdge`.
    ///
    /// Required because the struct is `#[non_exhaustive]` and cannot be
    /// constructed with struct-literal syntax outside this crate.
    #[allow(clippy::too_many_arguments)] // mirrors the struct fields 1-to-1
    pub fn new(
        occurrence_id: OccurrenceId,
        anchor_id: AnchorId,
        file_id: FileId,
        symbol_key: String,
        target_candidates: Vec<EntityId>,
        kind: OccurrenceKind,
        provenance: Provenance,
        confidence: Confidence,
    ) -> Self {
        Self {
            occurrence_id,
            anchor_id,
            file_id,
            symbol_key,
            target_candidates,
            kind,
            provenance,
            confidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_fact_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let fact = EntityFact {
            id: EntityId(100),
            kind: EntityKind::Method,
            canonical_name: "Foo::bar".to_string(),
            anchor_id: Some(AnchorId(12)),
            scope_id: Some(ScopeId(3)),
            provenance: Provenance::SemanticAnalyzer,
            confidence: Confidence::High,
        };

        let serialized = serde_json::to_string(&fact)?;
        let decoded: EntityFact = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, fact);
        Ok(())
    }

    #[test]
    fn deterministic_debug_for_edge_fact() {
        let fact = EdgeFact {
            id: EdgeId(7),
            kind: EdgeKind::Calls,
            from_entity_id: EntityId(11),
            to_entity_id: EntityId(22),
            via_occurrence_id: Some(OccurrenceId(33)),
            provenance: Provenance::ExactAst,
            confidence: Confidence::Medium,
        };

        assert_eq!(
            format!("{fact:?}"),
            "EdgeFact { id: EdgeId(7), kind: Calls, from_entity_id: EntityId(11), to_entity_id: EntityId(22), via_occurrence_id: Some(OccurrenceId(33)), provenance: ExactAst, confidence: Medium }"
        );
    }

    #[test]
    fn pretty_json_for_anchor_fact_is_stable() -> Result<(), serde_json::Error> {
        let fact = AnchorFact {
            id: AnchorId(5),
            file_id: FileId(1),
            span_start_byte: 10,
            span_end_byte: 15,
            scope_id: None,
            provenance: Provenance::DesugaredAst,
            confidence: Confidence::Low,
        };

        let json = serde_json::to_string_pretty(&fact)?;
        assert_eq!(
            json,
            "{\n  \"id\": 5,\n  \"file_id\": 1,\n  \"span_start_byte\": 10,\n  \"span_end_byte\": 15,\n  \"scope_id\": null,\n  \"provenance\": \"DesugaredAst\",\n  \"confidence\": \"Low\"\n}"
        );
        Ok(())
    }

    /// Verify OccurrenceFact with a None entity_id round-trips correctly — this
    /// exercises the optional foreign-key path that EntityFact's test does not cover.
    #[test]
    fn occurrence_fact_with_null_entity_id_roundtrips() -> Result<(), serde_json::Error> {
        let fact = OccurrenceFact {
            id: OccurrenceId(42),
            kind: OccurrenceKind::Call,
            entity_id: None,
            anchor_id: AnchorId(10),
            scope_id: Some(ScopeId(2)),
            provenance: Provenance::NameHeuristic,
            confidence: Confidence::Low,
        };
        let serialized = serde_json::to_string(&fact)?;
        let decoded: OccurrenceFact = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, fact);
        // entity_id: None must serialize as JSON null, not be omitted.
        assert!(
            serialized.contains("\"entity_id\":null"),
            "entity_id null must be explicit in JSON"
        );
        Ok(())
    }

    /// Verify that u64::MAX is preserved through JSON serialization without
    /// truncation — serde_json serializes u64 as a JSON number, which can
    /// exceed JS safe-integer range but round-trips correctly in Rust.
    #[test]
    fn id_u64_max_roundtrips() -> Result<(), serde_json::Error> {
        let id = EntityId(u64::MAX);
        let serialized = serde_json::to_string(&id)?;
        let decoded: EntityId = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, id);
        Ok(())
    }

    #[test]
    fn import_spec_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let spec = ImportSpec {
            module: "Foo::Bar".to_string(),
            kind: ImportKind::RequireThenImport,
            symbols: ImportSymbols::Mixed {
                tags: vec!["all".to_string()],
                names: vec!["$X".to_string(), "@Y".to_string()],
            },
            provenance: Provenance::ImportExportInference,
            confidence: Confidence::Medium,
            file_id: None,
            anchor_id: None,
            scope_id: None,
            span_start_byte: None,
        };

        let serialized = serde_json::to_string(&spec)?;
        let decoded: ImportSpec = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, spec);
        Ok(())
    }

    #[test]
    fn import_symbols_debug_is_deterministic() {
        let symbols = ImportSymbols::Mixed {
            tags: vec!["io".to_string(), "all".to_string()],
            names: vec!["open".to_string(), "close".to_string()],
        };
        assert_eq!(
            format!("{symbols:?}"),
            "Mixed { tags: [\"io\", \"all\"], names: [\"open\", \"close\"] }"
        );
    }

    #[test]
    fn visible_symbol_pretty_json_is_stable() -> Result<(), serde_json::Error> {
        let visible = VisibleSymbol {
            name: "slurp".to_string(),
            entity_id: Some(EntityId(17)),
            source: VisibleSymbolSource::ExplicitImport,
            confidence: Confidence::High,
            context: None,
        };

        let json = serde_json::to_string_pretty(&visible)?;
        assert_eq!(
            json,
            "{\n  \"name\": \"slurp\",\n  \"entity_id\": 17,\n  \"source\": \"ExplicitImport\",\n  \"confidence\": \"High\",\n  \"context\": null\n}"
        );
        Ok(())
    }

    /// ReferenceEdge with multiple target candidates round-trips through JSON.
    #[test]
    fn reference_edge_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let edge = ReferenceEdge {
            occurrence_id: OccurrenceId(50),
            anchor_id: AnchorId(20),
            file_id: FileId(3),
            symbol_key: "Foo::bar".to_string(),
            target_candidates: vec![EntityId(100), EntityId(200)],
            kind: OccurrenceKind::Call,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        };

        let serialized = serde_json::to_string(&edge)?;
        let decoded: ReferenceEdge = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, edge);
        Ok(())
    }

    /// ReferenceEdge with empty target_candidates (unresolved) round-trips correctly.
    #[test]
    fn reference_edge_empty_candidates_roundtrips() -> Result<(), serde_json::Error> {
        let edge = ReferenceEdge {
            occurrence_id: OccurrenceId(51),
            anchor_id: AnchorId(21),
            file_id: FileId(4),
            symbol_key: "unknown_sub".to_string(),
            target_candidates: vec![],
            kind: OccurrenceKind::Reference,
            provenance: Provenance::NameHeuristic,
            confidence: Confidence::Low,
        };

        let serialized = serde_json::to_string(&edge)?;
        let decoded: ReferenceEdge = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, edge);
        // Empty candidates must serialize as an empty array, not null.
        assert!(
            serialized.contains("\"target_candidates\":[]"),
            "empty target_candidates must be an empty JSON array"
        );
        Ok(())
    }

    /// DefinitionRank round-trips through JSON for every variant.
    #[test]
    fn definition_rank_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let variants = [
            DefinitionRank::ExactQualified,
            DefinitionRank::SamePackage,
            DefinitionRank::ExplicitImport,
            DefinitionRank::DefaultExport,
            DefinitionRank::WorkspaceCandidate,
            DefinitionRank::Heuristic,
        ];
        for variant in &variants {
            let serialized = serde_json::to_string(variant)?;
            let decoded: DefinitionRank = serde_json::from_str(&serialized)?;
            assert_eq!(&decoded, variant);
        }
        Ok(())
    }

    /// DefinitionRank ordering: ExactQualified < SamePackage < … < Heuristic.
    #[test]
    fn definition_rank_ordering_matches_design() {
        assert!(DefinitionRank::ExactQualified < DefinitionRank::SamePackage);
        assert!(DefinitionRank::SamePackage < DefinitionRank::ExplicitImport);
        assert!(DefinitionRank::ExplicitImport < DefinitionRank::DefaultExport);
        assert!(DefinitionRank::DefaultExport < DefinitionRank::WorkspaceCandidate);
        assert!(DefinitionRank::WorkspaceCandidate < DefinitionRank::Heuristic);
    }

    /// DefinitionRankReason round-trips through JSON for all variants,
    /// including those carrying a module field.
    #[test]
    fn definition_rank_reason_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let reasons = [
            DefinitionRankReason::ExactQualifiedName,
            DefinitionRankReason::SamePackage,
            DefinitionRankReason::ExplicitImport { module: "Foo::Bar".to_string() },
            DefinitionRankReason::DefaultExport { module: "Baz::Qux".to_string() },
            DefinitionRankReason::WorkspaceSymbol,
            DefinitionRankReason::HeuristicNameMatch,
        ];
        for reason in &reasons {
            let serialized = serde_json::to_string(reason)?;
            let decoded: DefinitionRankReason = serde_json::from_str(&serialized)?;
            assert_eq!(&decoded, reason);
        }
        Ok(())
    }

    /// DefinitionCandidate with all fields populated round-trips through JSON.
    #[test]
    fn definition_candidate_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let candidate = DefinitionCandidate {
            entity_id: EntityId(300),
            anchor_id: AnchorId(40),
            canonical_name: "Foo::Bar::baz".to_string(),
            display_name: "baz".to_string(),
            package: Some("Foo::Bar".to_string()),
            kind: EntityKind::Subroutine,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            rank: DefinitionRank::ExactQualified,
            rank_reason: DefinitionRankReason::ExactQualifiedName,
        };

        let serialized = serde_json::to_string(&candidate)?;
        let decoded: DefinitionCandidate = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, candidate);
        Ok(())
    }

    /// DefinitionCandidate with None package round-trips correctly.
    #[test]
    fn definition_candidate_none_package_roundtrips() -> Result<(), serde_json::Error> {
        let candidate = DefinitionCandidate {
            entity_id: EntityId(301),
            anchor_id: AnchorId(41),
            canonical_name: "main::helper".to_string(),
            display_name: "helper".to_string(),
            package: None,
            kind: EntityKind::Subroutine,
            provenance: Provenance::NameHeuristic,
            confidence: Confidence::Low,
            rank: DefinitionRank::Heuristic,
            rank_reason: DefinitionRankReason::HeuristicNameMatch,
        };

        let serialized = serde_json::to_string(&candidate)?;
        let decoded: DefinitionCandidate = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, candidate);
        // package: None must serialize as JSON null, not be omitted.
        assert!(serialized.contains("\"package\":null"), "package null must be explicit in JSON");
        Ok(())
    }

    /// DefinitionCandidate with import-based rank reason round-trips the module field.
    #[test]
    fn definition_candidate_import_reason_roundtrips() -> Result<(), serde_json::Error> {
        let candidate = DefinitionCandidate {
            entity_id: EntityId(302),
            anchor_id: AnchorId(42),
            canonical_name: "List::Util::first".to_string(),
            display_name: "first".to_string(),
            package: Some("List::Util".to_string()),
            kind: EntityKind::Subroutine,
            provenance: Provenance::ImportExportInference,
            confidence: Confidence::Medium,
            rank: DefinitionRank::ExplicitImport,
            rank_reason: DefinitionRankReason::ExplicitImport { module: "List::Util".to_string() },
        };

        let serialized = serde_json::to_string(&candidate)?;
        let decoded: DefinitionCandidate = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, candidate);
        Ok(())
    }

    /// ProviderFactTrace round-trips through JSON with source hash and model version.
    #[test]
    fn provider_fact_trace_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let trace = ProviderFactTrace::new(
            ProviderSurface::Completion,
            ProviderFactSourceKind::CompilerFact,
            Provenance::ImportExportInference,
            Confidence::High,
            ProviderFactFreshness::Fresh,
            ProviderFallbackState::Shadow,
            Some("fixture-source-sha".to_string()),
            Some(AnchorId(10)),
            Some(1),
        );

        let serialized = serde_json::to_string(&trace)?;
        let decoded: ProviderFactTrace = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, trace);
        Ok(())
    }

    /// ProviderFactTrace keeps null freshness metadata explicit when unavailable.
    #[test]
    fn provider_fact_trace_optional_metadata_roundtrips() -> Result<(), serde_json::Error> {
        let trace = ProviderFactTrace::new(
            ProviderSurface::Diagnostics,
            ProviderFactSourceKind::Fallback,
            Provenance::SearchFallback,
            Confidence::Low,
            ProviderFactFreshness::NotApplicable,
            ProviderFallbackState::Fallback,
            None,
            None,
            None,
        );

        let serialized = serde_json::to_string(&trace)?;
        let decoded: ProviderFactTrace = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, trace);
        assert!(
            serialized.contains("\"source_hash\":null")
                && serialized.contains("\"anchor_id\":null")
                && serialized.contains("\"model_version\":null"),
            "optional trace metadata should remain explicit for downstream consumers"
        );
        Ok(())
    }

    /// Provider trace enums round-trip through JSON for every current variant.
    #[test]
    fn provider_fact_trace_enums_roundtrip_through_json() -> Result<(), serde_json::Error> {
        for surface in [
            ProviderSurface::Diagnostics,
            ProviderSurface::Completion,
            ProviderSurface::Hover,
            ProviderSurface::Definition,
            ProviderSurface::References,
            ProviderSurface::Rename,
            ProviderSurface::SafeDelete,
            ProviderSurface::WorkspaceSymbols,
            ProviderSurface::DocumentSymbols,
            ProviderSurface::SemanticTokens,
        ] {
            let serialized = serde_json::to_string(&surface)?;
            let decoded: ProviderSurface = serde_json::from_str(&serialized)?;
            assert_eq!(decoded, surface);
        }

        for source in [
            ProviderFactSourceKind::ParserSyntax,
            ProviderFactSourceKind::LegacyWorkspace,
            ProviderFactSourceKind::SemanticFact,
            ProviderFactSourceKind::CompilerFact,
            ProviderFactSourceKind::FrameworkAdapter,
            ProviderFactSourceKind::DynamicBoundary,
            ProviderFactSourceKind::Fallback,
            ProviderFactSourceKind::Unknown,
        ] {
            let serialized = serde_json::to_string(&source)?;
            let decoded: ProviderFactSourceKind = serde_json::from_str(&serialized)?;
            assert_eq!(decoded, source);
        }

        for freshness in [
            ProviderFactFreshness::Fresh,
            ProviderFactFreshness::Stale,
            ProviderFactFreshness::Unknown,
            ProviderFactFreshness::NotApplicable,
        ] {
            let serialized = serde_json::to_string(&freshness)?;
            let decoded: ProviderFactFreshness = serde_json::from_str(&serialized)?;
            assert_eq!(decoded, freshness);
        }

        for fallback_state in [
            ProviderFallbackState::Primary,
            ProviderFallbackState::Shadow,
            ProviderFallbackState::Fallback,
            ProviderFallbackState::Unavailable,
            ProviderFallbackState::Blocked,
        ] {
            let serialized = serde_json::to_string(&fallback_state)?;
            let decoded: ProviderFallbackState = serde_json::from_str(&serialized)?;
            assert_eq!(decoded, fallback_state);
        }

        Ok(())
    }

    // ── Package Graph round-trip tests ──

    /// PackageEdge round-trips through JSON.
    #[test]
    fn package_edge_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let edge = PackageEdge {
            from_package: "Child".to_string(),
            to_package: "Parent".to_string(),
            kind: PackageEdgeKind::Inherits,
            anchor_id: Some(AnchorId(99)),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        };

        let serialized = serde_json::to_string(&edge)?;
        let decoded: PackageEdge = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, edge);
        Ok(())
    }

    /// PackageEdgeKind round-trips through JSON for every variant.
    #[test]
    fn package_edge_kind_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let variants =
            [PackageEdgeKind::Inherits, PackageEdgeKind::ComposesRole, PackageEdgeKind::DependsOn];
        for variant in &variants {
            let serialized = serde_json::to_string(variant)?;
            let decoded: PackageEdgeKind = serde_json::from_str(&serialized)?;
            assert_eq!(&decoded, variant);
        }
        Ok(())
    }

    /// PackageNode round-trips through JSON.
    #[test]
    fn package_node_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let node = PackageNode {
            entity_id: EntityId(500),
            name: "My::Package".to_string(),
            kind: PackageKind::Class,
            anchor_id: Some(AnchorId(10)),
            file_id: Some(FileId(2)),
        };

        let serialized = serde_json::to_string(&node)?;
        let decoded: PackageNode = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, node);
        Ok(())
    }

    /// PackageKind round-trips through JSON for every variant.
    #[test]
    fn package_kind_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let variants =
            [PackageKind::Package, PackageKind::Class, PackageKind::Role, PackageKind::External];
        for variant in &variants {
            let serialized = serde_json::to_string(variant)?;
            let decoded: PackageKind = serde_json::from_str(&serialized)?;
            assert_eq!(&decoded, variant);
        }
        Ok(())
    }

    /// PackageEdge with None anchor_id round-trips correctly.
    #[test]
    fn package_edge_none_anchor_roundtrips() -> Result<(), serde_json::Error> {
        let edge = PackageEdge {
            from_package: "App::Worker".to_string(),
            to_package: "Unknown::External".to_string(),
            kind: PackageEdgeKind::DependsOn,
            anchor_id: None,
            provenance: Provenance::NameHeuristic,
            confidence: Confidence::Low,
        };

        let serialized = serde_json::to_string(&edge)?;
        let decoded: PackageEdge = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, edge);
        assert!(
            serialized.contains("\"anchor_id\":null"),
            "anchor_id null must be explicit in JSON"
        );
        Ok(())
    }

    // ── GeneratedMember tests ───────────────────────────────────────────

    /// GeneratedMember round-trips through JSON.
    #[test]
    fn generated_member_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let member = GeneratedMember {
            entity_id: EntityId(600),
            name: "username".to_string(),
            kind: GeneratedMemberKind::Getter,
            source_anchor_id: AnchorId(50),
            package: "MyApp::User".to_string(),
            provenance: Provenance::FrameworkSynthesis,
            confidence: Confidence::Medium,
        };

        let serialized = serde_json::to_string(&member)?;
        let decoded: GeneratedMember = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, member);
        Ok(())
    }

    /// GeneratedMemberKind round-trips through JSON for every variant.
    #[test]
    fn generated_member_kind_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let variants = [
            GeneratedMemberKind::Getter,
            GeneratedMemberKind::Setter,
            GeneratedMemberKind::Accessor,
            GeneratedMemberKind::Predicate,
            GeneratedMemberKind::Clearer,
            GeneratedMemberKind::Builder,
            GeneratedMemberKind::Constant,
        ];
        for variant in &variants {
            let serialized = serde_json::to_string(variant)?;
            let decoded: GeneratedMemberKind = serde_json::from_str(&serialized)?;
            assert_eq!(&decoded, variant);
        }
        Ok(())
    }

    /// GeneratedMember constructed via `new()` matches struct literal.
    #[test]
    fn generated_member_new_constructor() -> Result<(), serde_json::Error> {
        let via_new = GeneratedMember::new(
            EntityId(700),
            "email".to_string(),
            GeneratedMemberKind::Accessor,
            AnchorId(60),
            "MyApp::User".to_string(),
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
        );
        let via_literal = GeneratedMember {
            entity_id: EntityId(700),
            name: "email".to_string(),
            kind: GeneratedMemberKind::Accessor,
            source_anchor_id: AnchorId(60),
            package: "MyApp::User".to_string(),
            provenance: Provenance::FrameworkSynthesis,
            confidence: Confidence::Medium,
        };
        assert_eq!(via_new, via_literal);
        Ok(())
    }

    // ── ValueShape tests ───────────────────────────────────────────────

    /// ValueShape::Unknown round-trips through JSON.
    #[test]
    fn value_shape_unknown_roundtrips() -> Result<(), serde_json::Error> {
        let shape = ValueShape::Unknown;
        let serialized = serde_json::to_string(&shape)?;
        let decoded: ValueShape = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, shape);
        Ok(())
    }

    /// ValueShape::Object round-trips through JSON preserving package and confidence.
    #[test]
    fn value_shape_object_roundtrips() -> Result<(), serde_json::Error> {
        let shape =
            ValueShape::Object { package: "My::Class".to_string(), confidence: Confidence::High };
        let serialized = serde_json::to_string(&shape)?;
        let decoded: ValueShape = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, shape);
        Ok(())
    }

    /// ValueShape::PackageName round-trips through JSON.
    #[test]
    fn value_shape_package_name_roundtrips() -> Result<(), serde_json::Error> {
        let shape = ValueShape::PackageName { package: "Foo::Bar".to_string() };
        let serialized = serde_json::to_string(&shape)?;
        let decoded: ValueShape = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, shape);
        Ok(())
    }

    /// All ValueShape variants round-trip through JSON.
    #[test]
    fn value_shape_all_variants_roundtrip() -> Result<(), serde_json::Error> {
        let variants: Vec<ValueShape> = vec![
            ValueShape::Unknown,
            ValueShape::Scalar,
            ValueShape::ArrayRef,
            ValueShape::HashRef,
            ValueShape::CodeRef,
            ValueShape::PackageName { package: "Foo".to_string() },
            ValueShape::Object { package: "Bar::Baz".to_string(), confidence: Confidence::Low },
        ];
        for shape in &variants {
            let serialized = serde_json::to_string(shape)?;
            let decoded: ValueShape = serde_json::from_str(&serialized)?;
            assert_eq!(&decoded, shape);
        }
        Ok(())
    }

    // ── RenamePlan / SafeDeletePlan round-trip tests ─────────────────

    /// RenamePlan with edits, blockers, and warnings round-trips through JSON.
    #[test]
    fn rename_plan_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let plan = RenamePlan {
            entity_id: EntityId(400),
            old_name: "foo".to_string(),
            new_name: "bar".to_string(),
            edits: vec![
                PlannedEdit {
                    anchor_id: AnchorId(80),
                    file_id: FileId(1),
                    category: PlannedEditCategory::Definition,
                    old_text: "foo".to_string(),
                    new_text: "bar".to_string(),
                },
                PlannedEdit {
                    anchor_id: AnchorId(81),
                    file_id: FileId(2),
                    category: PlannedEditCategory::Reference,
                    old_text: "foo".to_string(),
                    new_text: "bar".to_string(),
                },
            ],
            blockers: vec![PlanBlocker {
                reason: PlanBlockerReason::DynamicBoundary,
                anchor_id: Some(AnchorId(90)),
                description: "reference crosses eval boundary".to_string(),
            }],
            warnings: vec![PlanWarning {
                message: "symbol also appears in comments".to_string(),
                anchor_id: None,
            }],
        };

        let serialized = serde_json::to_string(&plan)?;
        let decoded: RenamePlan = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, plan);
        Ok(())
    }

    /// RenamePlan with empty edits, blockers, and warnings round-trips correctly.
    #[test]
    fn rename_plan_empty_collections_roundtrip() -> Result<(), serde_json::Error> {
        let plan = RenamePlan {
            entity_id: EntityId(401),
            old_name: "x".to_string(),
            new_name: "y".to_string(),
            edits: vec![],
            blockers: vec![],
            warnings: vec![],
        };

        let serialized = serde_json::to_string(&plan)?;
        let decoded: RenamePlan = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, plan);
        assert!(serialized.contains("\"edits\":[]"), "empty edits must be an empty JSON array");
        assert!(
            serialized.contains("\"blockers\":[]"),
            "empty blockers must be an empty JSON array"
        );
        assert!(
            serialized.contains("\"warnings\":[]"),
            "empty warnings must be an empty JSON array"
        );
        Ok(())
    }

    /// RenamePlan constructed via `new()` matches struct literal.
    #[test]
    fn rename_plan_new_constructor() {
        let via_new = RenamePlan::new(
            EntityId(402),
            "old".to_string(),
            "new".to_string(),
            vec![],
            vec![],
            vec![],
        );
        let via_literal = RenamePlan {
            entity_id: EntityId(402),
            old_name: "old".to_string(),
            new_name: "new".to_string(),
            edits: vec![],
            blockers: vec![],
            warnings: vec![],
        };
        assert_eq!(via_new, via_literal);
    }

    /// SafeDeletePlan with blockers and warnings round-trips through JSON.
    #[test]
    fn safe_delete_plan_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let plan = SafeDeletePlan {
            entity_id: EntityId(500),
            name: "unused_sub".to_string(),
            blockers: vec![
                PlanBlocker {
                    reason: PlanBlockerReason::ReferencesExist,
                    anchor_id: Some(AnchorId(70)),
                    description: "3 references remain".to_string(),
                },
                PlanBlocker {
                    reason: PlanBlockerReason::ExportedSymbol,
                    anchor_id: Some(AnchorId(71)),
                    description: "symbol in @EXPORT_OK".to_string(),
                },
            ],
            warnings: vec![],
        };

        let serialized = serde_json::to_string(&plan)?;
        let decoded: SafeDeletePlan = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, plan);
        Ok(())
    }

    /// SafeDeletePlan with no blockers round-trips correctly.
    #[test]
    fn safe_delete_plan_no_blockers_roundtrips() -> Result<(), serde_json::Error> {
        let plan = SafeDeletePlan {
            entity_id: EntityId(501),
            name: "dead_code".to_string(),
            blockers: vec![],
            warnings: vec![PlanWarning {
                message: "symbol appears in pod documentation".to_string(),
                anchor_id: Some(AnchorId(72)),
            }],
        };

        let serialized = serde_json::to_string(&plan)?;
        let decoded: SafeDeletePlan = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, plan);
        Ok(())
    }

    /// SafeDeletePlan constructed via `new()` matches struct literal.
    #[test]
    fn safe_delete_plan_new_constructor() {
        let via_new = SafeDeletePlan::new(EntityId(502), "helper".to_string(), vec![], vec![]);
        let via_literal = SafeDeletePlan {
            entity_id: EntityId(502),
            name: "helper".to_string(),
            blockers: vec![],
            warnings: vec![],
        };
        assert_eq!(via_new, via_literal);
    }

    /// PlanBlockerReason round-trips through JSON for every variant.
    #[test]
    fn plan_blocker_reason_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let variants = [
            PlanBlockerReason::DynamicBoundary,
            PlanBlockerReason::AmbiguousReference,
            PlanBlockerReason::CrossModuleExport,
            PlanBlockerReason::ImportedSymbol,
            PlanBlockerReason::ExportedSymbol,
            PlanBlockerReason::ReferencesExist,
            PlanBlockerReason::GeneratedMember,
            PlanBlockerReason::StaleFact,
            PlanBlockerReason::UnclassifiedOccurrence,
        ];
        for variant in &variants {
            let serialized = serde_json::to_string(variant)?;
            let decoded: PlanBlockerReason = serde_json::from_str(&serialized)?;
            assert_eq!(&decoded, variant);
        }
        Ok(())
    }

    /// PlanBlocker with None anchor_id round-trips correctly.
    #[test]
    fn plan_blocker_none_anchor_roundtrips() -> Result<(), serde_json::Error> {
        let blocker = PlanBlocker {
            reason: PlanBlockerReason::GeneratedMember,
            anchor_id: None,
            description: "generated accessor without edit plan".to_string(),
        };

        let serialized = serde_json::to_string(&blocker)?;
        let decoded: PlanBlocker = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, blocker);
        assert!(
            serialized.contains("\"anchor_id\":null"),
            "anchor_id null must be explicit in JSON"
        );
        Ok(())
    }

    /// PlanBlocker constructed via `new()` matches struct literal.
    #[test]
    fn plan_blocker_new_constructor() {
        let via_new = PlanBlocker::new(
            PlanBlockerReason::ImportedSymbol,
            Some(AnchorId(99)),
            "imported by other file".to_string(),
        );
        let via_literal = PlanBlocker {
            reason: PlanBlockerReason::ImportedSymbol,
            anchor_id: Some(AnchorId(99)),
            description: "imported by other file".to_string(),
        };
        assert_eq!(via_new, via_literal);
    }

    /// PlanWarning round-trips through JSON.
    #[test]
    fn plan_warning_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let warning = PlanWarning {
            message: "symbol also used in string interpolation".to_string(),
            anchor_id: Some(AnchorId(85)),
        };

        let serialized = serde_json::to_string(&warning)?;
        let decoded: PlanWarning = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, warning);
        Ok(())
    }

    /// PlanWarning constructed via `new()` matches struct literal.
    #[test]
    fn plan_warning_new_constructor() {
        let via_new = PlanWarning::new("check pod docs".to_string(), None);
        let via_literal = PlanWarning { message: "check pod docs".to_string(), anchor_id: None };
        assert_eq!(via_new, via_literal);
    }

    /// PlannedEdit round-trips through JSON.
    #[test]
    fn planned_edit_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let edit = PlannedEdit {
            anchor_id: AnchorId(60),
            file_id: FileId(5),
            category: PlannedEditCategory::ImportList,
            old_text: "foo".to_string(),
            new_text: "bar".to_string(),
        };

        let serialized = serde_json::to_string(&edit)?;
        let decoded: PlannedEdit = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, edit);
        Ok(())
    }

    /// PlannedEdit constructed via `new()` matches struct literal.
    #[test]
    fn planned_edit_new_constructor() {
        let via_new = PlannedEdit::new(
            AnchorId(61),
            FileId(6),
            PlannedEditCategory::ExportList,
            "old_sym".to_string(),
            "new_sym".to_string(),
        );
        let via_literal = PlannedEdit {
            anchor_id: AnchorId(61),
            file_id: FileId(6),
            category: PlannedEditCategory::ExportList,
            old_text: "old_sym".to_string(),
            new_text: "new_sym".to_string(),
        };
        assert_eq!(via_new, via_literal);
    }

    /// PlannedEditCategory round-trips through JSON for every variant.
    #[test]
    fn planned_edit_category_roundtrips_through_json() -> Result<(), serde_json::Error> {
        let variants = [
            PlannedEditCategory::Definition,
            PlannedEditCategory::Reference,
            PlannedEditCategory::ImportList,
            PlannedEditCategory::ExportList,
        ];
        for variant in &variants {
            let serialized = serde_json::to_string(variant)?;
            let decoded: PlannedEditCategory = serde_json::from_str(&serialized)?;
            assert_eq!(&decoded, variant);
        }
        Ok(())
    }

    // ── ID newtype semantics ──────────────────────────────────────────────────
    //
    // The id_newtype! macro derives Debug, Clone, Copy, PartialEq, Eq, PartialOrd,
    // Ord, Hash, Serialize, and Deserialize.  The existing `id_u64_max_roundtrips`
    // test only exercises EntityId at u64::MAX.  The tests below cover boundary
    // values (0, 1, mid-range), ordering comparisons, HashMap key identity, and
    // Debug format for all seven ID types.

    /// All seven ID newtypes produce the correct Debug representation.
    #[test]
    fn id_newtype_debug_format() {
        assert_eq!(format!("{:?}", FileId(0)), "FileId(0)");
        assert_eq!(format!("{:?}", ScopeId(1)), "ScopeId(1)");
        assert_eq!(format!("{:?}", EntityId(42)), "EntityId(42)");
        assert_eq!(format!("{:?}", AnchorId(999)), "AnchorId(999)");
        assert_eq!(format!("{:?}", OccurrenceId(u64::MAX)), "OccurrenceId(18446744073709551615)");
        assert_eq!(format!("{:?}", EdgeId(0)), "EdgeId(0)");
        assert_eq!(format!("{:?}", DiagnosticId(1)), "DiagnosticId(1)");
    }

    /// All seven ID newtypes support equality and inequality at boundary values.
    #[test]
    fn id_newtype_equality_at_boundaries() {
        // zero identity
        assert_eq!(FileId(0), FileId(0));
        assert_ne!(FileId(0), FileId(1));

        // one identity
        assert_eq!(ScopeId(1), ScopeId(1));
        assert_ne!(ScopeId(1), ScopeId(2));

        // mid-range identity
        assert_eq!(EntityId(500), EntityId(500));
        assert_ne!(EntityId(499), EntityId(500));

        // u64::MAX identity
        assert_eq!(AnchorId(u64::MAX), AnchorId(u64::MAX));
        assert_ne!(AnchorId(u64::MAX - 1), AnchorId(u64::MAX));

        // remaining types at zero
        assert_eq!(OccurrenceId(0), OccurrenceId(0));
        assert_ne!(OccurrenceId(0), OccurrenceId(1));
        assert_eq!(EdgeId(0), EdgeId(0));
        assert_ne!(EdgeId(0), EdgeId(1));
        assert_eq!(DiagnosticId(0), DiagnosticId(0));
        assert_ne!(DiagnosticId(0), DiagnosticId(1));
    }

    /// All seven ID newtypes support total ordering consistent with their inner u64.
    #[test]
    fn id_newtype_ordering() {
        assert!(FileId(0) < FileId(1));
        assert!(FileId(1) < FileId(u64::MAX));
        assert!(ScopeId(10) > ScopeId(9));
        assert!(EntityId(0) <= EntityId(0));
        assert!(AnchorId(100) >= AnchorId(100));
        assert!(OccurrenceId(1) < OccurrenceId(2));
        assert!(EdgeId(u64::MAX - 1) < EdgeId(u64::MAX));
        assert!(DiagnosticId(3) > DiagnosticId(2));
    }

    /// ID newtypes can be used as HashMap keys (exercises Hash + Eq).
    #[test]
    fn id_newtype_as_hashmap_key() {
        use std::collections::HashMap;

        let mut map: HashMap<FileId, &str> = HashMap::new();
        map.insert(FileId(0), "zero");
        map.insert(FileId(1), "one");
        map.insert(FileId(u64::MAX), "max");

        assert_eq!(map[&FileId(0)], "zero");
        assert_eq!(map[&FileId(1)], "one");
        assert_eq!(map[&FileId(u64::MAX)], "max");
        assert!(!map.contains_key(&FileId(2)));

        // Verify the same for ScopeId, EntityId, AnchorId, OccurrenceId, EdgeId, DiagnosticId.
        let mut scope_map: HashMap<ScopeId, u32> = HashMap::new();
        scope_map.insert(ScopeId(0), 0);
        scope_map.insert(ScopeId(1), 1);
        assert_eq!(scope_map[&ScopeId(0)], 0);

        let mut entity_map: HashMap<EntityId, u32> = HashMap::new();
        entity_map.insert(EntityId(42), 42);
        assert_eq!(entity_map[&EntityId(42)], 42);

        let mut anchor_map: HashMap<AnchorId, u32> = HashMap::new();
        anchor_map.insert(AnchorId(99), 99);
        assert_eq!(anchor_map[&AnchorId(99)], 99);

        let mut occ_map: HashMap<OccurrenceId, u32> = HashMap::new();
        occ_map.insert(OccurrenceId(7), 7);
        assert_eq!(occ_map[&OccurrenceId(7)], 7);

        let mut edge_map: HashMap<EdgeId, u32> = HashMap::new();
        edge_map.insert(EdgeId(3), 3);
        assert_eq!(edge_map[&EdgeId(3)], 3);

        let mut diag_map: HashMap<DiagnosticId, u32> = HashMap::new();
        diag_map.insert(DiagnosticId(5), 5);
        assert_eq!(diag_map[&DiagnosticId(5)], 5);
    }

    /// All seven ID newtypes round-trip through JSON at boundary values (0, 1, mid, MAX).
    ///
    /// The `id_newtype!` macro produces `pub struct $name(pub u64)`.  serde's
    /// default representation for a newtype struct over a u64 is a bare JSON
    /// number — **not** a JSON object.  The shape assertion below pins this
    /// contract so that adding `#[serde(rename_all = ...)]` or similar
    /// attributes would surface as a test failure.
    #[test]
    fn id_newtype_json_roundtrip_boundary_values() -> Result<(), serde_json::Error> {
        // Shape contract: bare JSON number, not an object or array.
        assert_eq!(serde_json::to_string(&FileId(42))?, "42");
        assert_eq!(serde_json::to_string(&ScopeId(42))?, "42");
        assert_eq!(serde_json::to_string(&EntityId(42))?, "42");
        assert_eq!(serde_json::to_string(&AnchorId(42))?, "42");
        assert_eq!(serde_json::to_string(&OccurrenceId(42))?, "42");
        assert_eq!(serde_json::to_string(&EdgeId(42))?, "42");
        assert_eq!(serde_json::to_string(&DiagnosticId(42))?, "42");

        for v in [0u64, 1, 500, u64::MAX] {
            let s = serde_json::to_string(&FileId(v))?;
            assert_eq!(serde_json::from_str::<FileId>(&s)?, FileId(v));

            let s = serde_json::to_string(&ScopeId(v))?;
            assert_eq!(serde_json::from_str::<ScopeId>(&s)?, ScopeId(v));

            let s = serde_json::to_string(&EntityId(v))?;
            assert_eq!(serde_json::from_str::<EntityId>(&s)?, EntityId(v));

            let s = serde_json::to_string(&AnchorId(v))?;
            assert_eq!(serde_json::from_str::<AnchorId>(&s)?, AnchorId(v));

            let s = serde_json::to_string(&OccurrenceId(v))?;
            assert_eq!(serde_json::from_str::<OccurrenceId>(&s)?, OccurrenceId(v));

            let s = serde_json::to_string(&EdgeId(v))?;
            assert_eq!(serde_json::from_str::<EdgeId>(&s)?, EdgeId(v));

            let s = serde_json::to_string(&DiagnosticId(v))?;
            assert_eq!(serde_json::from_str::<DiagnosticId>(&s)?, DiagnosticId(v));
        }
        Ok(())
    }

    // ── Constructor coverage ──────────────────────────────────────────────────

    /// `VisibleSymbolContext::new()` produces a value equal to the struct literal.
    ///
    /// Required because `#[non_exhaustive]` prevents struct-literal construction
    /// outside the crate — callers must use `new()`.
    #[test]
    fn visible_symbol_context_new_constructor() -> Result<(), serde_json::Error> {
        let via_new = VisibleSymbolContext::new(
            Some("Foo::Bar".to_string()),
            Some(AnchorId(10)),
            Some(AnchorId(20)),
        );
        assert_eq!(via_new.source_module.as_deref(), Some("Foo::Bar"));
        assert_eq!(via_new.source_import_anchor_id, Some(AnchorId(10)));
        assert_eq!(via_new.source_export_anchor_id, Some(AnchorId(20)));

        // All-None variant.
        let none_ctx = VisibleSymbolContext::new(None, None, None);
        assert!(none_ctx.source_module.is_none());
        assert!(none_ctx.source_import_anchor_id.is_none());
        assert!(none_ctx.source_export_anchor_id.is_none());

        // Serde roundtrip with populated fields.
        let serialized = serde_json::to_string(&via_new)?;
        let decoded: VisibleSymbolContext = serde_json::from_str(&serialized)?;
        assert_eq!(decoded.source_module, via_new.source_module);
        assert_eq!(decoded.source_import_anchor_id, via_new.source_import_anchor_id);
        assert_eq!(decoded.source_export_anchor_id, via_new.source_export_anchor_id);
        Ok(())
    }

    /// `PackageNode::new()` produces the correct field values.
    #[test]
    fn package_node_new_constructor() -> Result<(), serde_json::Error> {
        let node = PackageNode::new(
            EntityId(10),
            "My::Package".to_string(),
            PackageKind::Class,
            Some(AnchorId(5)),
            Some(FileId(1)),
        );
        assert_eq!(node.entity_id, EntityId(10));
        assert_eq!(node.name, "My::Package");
        assert_eq!(node.kind, PackageKind::Class);
        assert_eq!(node.anchor_id, Some(AnchorId(5)));
        assert_eq!(node.file_id, Some(FileId(1)));

        // None-anchor, None-file variant.
        let bare = PackageNode::new(
            EntityId(99),
            "External::Pkg".to_string(),
            PackageKind::External,
            None,
            None,
        );
        assert!(bare.anchor_id.is_none());
        assert!(bare.file_id.is_none());
        assert_eq!(
            format!("{bare:?}"),
            "PackageNode { entity_id: EntityId(99), name: \"External::Pkg\", kind: External, anchor_id: None, file_id: None }"
        );

        // Serde roundtrip.
        let serialized = serde_json::to_string(&node)?;
        let decoded: PackageNode = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, node);
        Ok(())
    }

    /// `PackageEdge::new()` produces the correct field values.
    #[test]
    fn package_edge_new_constructor() -> Result<(), serde_json::Error> {
        let edge = PackageEdge::new(
            "Child::Class".to_string(),
            "Parent::Class".to_string(),
            PackageEdgeKind::Inherits,
            Some(AnchorId(77)),
            Provenance::ExactAst,
            Confidence::High,
        );
        assert_eq!(edge.from_package, "Child::Class");
        assert_eq!(edge.to_package, "Parent::Class");
        assert_eq!(edge.kind, PackageEdgeKind::Inherits);
        assert_eq!(edge.anchor_id, Some(AnchorId(77)));
        assert_eq!(edge.provenance, Provenance::ExactAst);
        assert_eq!(edge.confidence, Confidence::High);

        // No-anchor variant.
        let inferred = PackageEdge::new(
            "App::Worker".to_string(),
            "Moo".to_string(),
            PackageEdgeKind::DependsOn,
            None,
            Provenance::NameHeuristic,
            Confidence::Low,
        );
        assert!(inferred.anchor_id.is_none());

        // Serde roundtrip.
        let serialized = serde_json::to_string(&edge)?;
        let decoded: PackageEdge = serde_json::from_str(&serialized)?;
        assert_eq!(decoded, edge);
        Ok(())
    }
}
