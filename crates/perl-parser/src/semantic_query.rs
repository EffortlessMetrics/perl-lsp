//! Read-only semantic query facade spanning parser, semantic analysis, and workspace indexing.
//!
//! This module provides a small, intentionally incomplete API for consumers that
//! need stable semantic queries without depending on crate-internal structures.

use crate::analysis::class_model::ClassModel;
use crate::analysis::semantic::{SemanticAnalyzer, SemanticModel};
use crate::pragma_tracker::{PragmaState, PragmaTracker};
use crate::workspace_index::WorkspaceIndex;
use crate::{ParseError, Parser, SourceLocation, SymbolKind};
use std::collections::HashSet;
use std::ops::Range;

/// Opaque identifier for a symbol definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct DefinitionId(pub String);

/// Resolved source location for a symbol definition.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DefinitionLocation {
    /// Stable definition identifier.
    pub id: DefinitionId,
    /// Document URI containing the definition.
    pub uri: String,
    /// Byte-range location of the definition in the source document.
    pub range: SourceLocation,
}

/// Typed read-only view of a resolved symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResolvedSymbol {
    /// Symbol name as written in source.
    pub name: String,
    /// Fully-qualified symbol name when available.
    pub qualified_name: Option<String>,
    /// Semantic symbol kind.
    pub kind: SymbolKind,
    /// Resolved definition location.
    pub definition: DefinitionLocation,
}

/// Import visible from the current document's workspace index entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VisibleImport {
    /// Imported module name.
    pub module: String,
    /// URI of document where import is visible.
    pub uri: String,
}

/// Parent package chain and inherited resolution origin.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParentChain {
    /// Queried package/class name.
    pub package: String,
    /// Ordered parent packages (nearest parent first).
    pub parents: Vec<String>,
    /// Package where the queried inherited method was resolved.
    pub inherited_origin: Option<String>,
}

/// Snapshot of effective pragma state at a source offset.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EffectivePragmaState {
    /// `use strict 'vars'` effective state.
    pub strict_vars: bool,
    /// `use strict 'subs'` effective state.
    pub strict_subs: bool,
    /// `use strict 'refs'` effective state.
    pub strict_refs: bool,
    /// Global warnings state.
    pub warnings: bool,
    /// Active feature names.
    pub features: Vec<String>,
}

impl From<PragmaState> for EffectivePragmaState {
    fn from(state: PragmaState) -> Self {
        Self {
            strict_vars: state.strict_vars,
            strict_subs: state.strict_subs,
            strict_refs: state.strict_refs,
            warnings: state.warnings,
            features: state.features.into_iter().map(ToString::to_string).collect(),
        }
    }
}

/// Small read-only semantic query facade for a single indexed document.
pub struct SemanticQueryFacade {
    uri: String,
    semantic_model: SemanticModel,
    semantic_analyzer: SemanticAnalyzer,
    workspace_index: WorkspaceIndex,
    pragma_map: Vec<(Range<usize>, PragmaState)>,
}

impl SemanticQueryFacade {
    /// Parse, analyze, and index a document for read-only semantic queries.
    pub fn build(uri: impl Into<String>, source: &str) -> Result<Self, ParseError> {
        let uri = uri.into();
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let semantic_model = SemanticModel::build(&ast, source);
        let semantic_analyzer = SemanticAnalyzer::analyze_with_source(&ast, source);
        let pragma_map = PragmaTracker::build(&ast);

        let workspace_index = WorkspaceIndex::new();
        let _ = workspace_index.index_file_str(&uri, source);

        Ok(Self { uri, semantic_model, semantic_analyzer, workspace_index, pragma_map })
    }

    /// Resolve the symbol definition at byte `offset`.
    pub fn resolve_symbol_at(&self, offset: usize) -> Option<ResolvedSymbol> {
        self.semantic_model.definition_at(offset).map(|symbol| {
            let qualified_name = optional_string(&symbol.qualified_name);
            let def_id =
                definition_id_for(qualified_name.as_deref(), &symbol.name, symbol.location);
            let definition = DefinitionLocation {
                id: def_id,
                uri: self.uri.clone(),
                range: symbol.location,
            };

            ResolvedSymbol {
                name: symbol.name.clone(),
                qualified_name,
                kind: symbol.kind,
                definition,
            }
        })
    }

    /// Resolve only the definition location at byte `offset`.
    pub fn definition_location_at(&self, offset: usize) -> Option<DefinitionLocation> {
        self.resolve_symbol_at(offset).map(|resolved| resolved.definition)
    }

    /// List document imports visible through workspace indexing.
    pub fn visible_imports(&self) -> Vec<VisibleImport> {
        let mut modules: Vec<String> = self.workspace_index.file_dependencies(&self.uri).into_iter().collect();
        modules.sort();
        modules
            .into_iter()
            .map(|module| VisibleImport { module, uri: self.uri.clone() })
            .collect()
    }

    /// Build parent chain for `package`, optionally resolving inherited `method_name` origin.
    pub fn parent_chain(
        &self,
        package: &str,
        method_name: Option<&str>,
    ) -> Option<ParentChain> {
        let model = self.semantic_analyzer.class_models.iter().find(|model| model.name == package)?;
        let mut seen = HashSet::new();
        let mut parents = Vec::new();
        collect_parents(model, &self.semantic_analyzer.class_models, &mut seen, &mut parents);

        let inherited_origin = method_name.and_then(|method| {
            let from_models = parents.iter().find_map(|parent| {
                self.semantic_analyzer
                    .class_models
                    .iter()
                    .find(|model| &model.name == parent)
                    .and_then(|model| {
                        if model.methods.iter().any(|item| item.name == method) {
                            Some(model.name.clone())
                        } else {
                            None
                        }
                    })
            });

            from_models.or_else(|| {
                self.semantic_analyzer
                    .resolve_inherited_method_hover(package, method)
                    .and_then(|hover| {
                        hover.details.into_iter().find_map(|detail| {
                            detail
                                .strip_prefix("Inherited from ")
                                .map(std::string::ToString::to_string)
                        })
                    })
            })
        });

        Some(ParentChain { package: package.to_string(), parents, inherited_origin })
    }

    /// Get effective pragma state for the given byte `offset`.
    pub fn effective_pragma_state(&self, offset: usize) -> EffectivePragmaState {
        PragmaTracker::state_for_offset(&self.pragma_map, offset).into()
    }
}

fn definition_id_for(qualified_name: Option<&str>, name: &str, location: SourceLocation) -> DefinitionId {
    let stable_name = qualified_name.unwrap_or(name);
    DefinitionId(format!("{stable_name}:{}:{}", location.start, location.end))
}

fn optional_string(value: &str) -> Option<String> {
    if value.is_empty() { None } else { Some(value.to_string()) }
}

fn collect_parents(
    model: &ClassModel,
    models: &[ClassModel],
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
) {
    for parent in &model.parents {
        if seen.insert(parent.clone()) {
            out.push(parent.clone());
            if let Some(parent_model) = models.iter().find(|candidate| candidate.name == *parent) {
                collect_parents(parent_model, models, seen, out);
            }
        }
    }
}
