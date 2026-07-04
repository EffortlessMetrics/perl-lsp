//! `perl-workspace-core` — the LSP-free project-facts substrate for Perl.
//!
//! This crate owns the shared, deterministic project model that every product
//! surface consumes: the LSP server, the DAP server, native critic/tidy, the
//! RIPR exporter, Kwalitee scoring, and (later) a tree-sitter-compatible
//! adapter. It sits **below** the editor/LSP runtime and **above** the raw
//! parser — see
//! [PLSP-ADR-0006](../../../docs/adr/PLSP-ADR-0006-perl-workspace-core-facts-substrate.md)
//! and `docs/reference/NATIVE_STACK_POLICY.md`.
//!
//! # What it owns
//!
//! - A typed [`ProjectModel`] with per-fact records for files, packages, and
//!   symbols, plus explicit [`DynamicBoundary`]s and [`ModelLimitation`]s.
//! - Deterministic, host-path-free identity ([`FileId`], [`PackageId`],
//!   [`SymbolId`]) and content [`Digest`]s.
//! - One internal range format ([`SourceRange`]): byte offsets + 0-based UTF-8
//!   line/column. UTF-16 LSP positions are produced only at the LSP boundary,
//!   never stored here.
//! - [`Provenance`] + [`Confidence`] + [`EvidenceSource`] on every fact.
//! - A [`FactClasses`] selector so a request only pays for what it asks for.
//!
//! # What it must never depend on
//!
//! `perl-lsp-rs`, `perl-lsp-rs-core`, `perllsp`, `perl-dap`, `lsp-types`,
//! `tokio`, `tower-lsp`, or `perl-workspace` (which transitively pulls
//! `lsp-types`). This is asserted by `tests/dependency_contract.rs`.
//!
//! # Quick start
//!
//! ```no_run
//! use perl_workspace_core::{build_project_model, FactClasses, ProjectModelRequest};
//!
//! let model = build_project_model(&ProjectModelRequest {
//!     root: "lib",
//!     fact_classes: FactClasses::FILES | FactClasses::SYMBOLS,
//! })?;
//! for file in &model.files {
//!     println!("{} ({:?})", file.relative_path, file.role);
//! }
//! # Ok::<(), perl_workspace_core::WorkspaceCoreError>(())
//! ```

#![warn(missing_docs)]

pub mod boundary;
pub mod builder;
pub mod dist;
pub mod effects;
pub mod error;
pub mod export;
pub mod fact_classes;
pub mod file;
pub mod id;
pub mod import;
mod import_walk;
pub mod model;
pub mod package;
pub mod pod;
pub mod provenance;
pub mod range;
pub mod relation;
pub mod symbol;
pub mod test;

/// The fact-schema version this crate emits. Bump on any breaking model change.
pub const SCHEMA_VERSION: u32 = 1;

// ── Curated public surface ──────────────────────────────────────────────────
pub use boundary::{DynamicBoundary, DynamicBoundaryKind};
pub use builder::{ProjectModelRequest, build_project_model};
pub use dist::{DistMetadataFacts, DistMetadataSource, Prereq};
pub use effects::CompileEffectFacts;
pub use error::{ModelLimitation, WorkspaceCoreError};
pub use export::{ExportFact, ExportKind};
pub use fact_classes::FactClasses;
pub use file::{FileRecord, FileRole, ParseStatus};
pub use id::{Digest, FileId, PackageId, SymbolId, fnv1a};
pub use import::{ImportFact, ImportKind};
pub use model::ProjectModel;
pub use package::PackageRecord;
pub use pod::{PodFact, PodSection, PodSectionKind};
pub use provenance::{Confidence, EvidenceSource, Producer, Provenance};
pub use range::{SourceRange, Utf8LineIndex};
pub use relation::{RelationFact, RelationKind};
pub use symbol::{SymbolFactKind, SymbolRecord, Visibility};
pub use test::TestFact;
