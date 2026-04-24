//! Read-only semantic query façade spanning parser, semantic, and workspace layers.

use crate::analysis::class_model::ClassModel;
use crate::analysis::semantic::SemanticAnalyzer;
use crate::pragma_tracker::{PragmaState, PragmaTracker};
use crate::symbol::SymbolKind;
use crate::workspace::workspace_index::{Location, WorkspaceIndex};
use crate::{Node, Parser, SourceLocation};

/// Stable identifier for a resolved definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionId {
    /// Fully qualified identifier when available.
    pub qualified_name: String,
}

/// Stable location payload for a definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionLocation {
    /// Optional file URI when resolved through workspace indexing.
    pub uri: Option<String>,
    /// Byte offset where the definition starts.
    pub start: usize,
    /// Byte offset where the definition ends.
    pub end: usize,
}

/// A symbol resolved at a byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSymbol {
    /// Symbol name.
    pub name: String,
    /// Symbol kind.
    pub kind: SymbolKind,
    /// Definition identifier.
    pub definition_id: DefinitionId,
    /// Best-known definition location.
    pub definition_location: DefinitionLocation,
}

/// A visible import-like contribution in the current semantic context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleImport {
    /// Import source package/module.
    pub source: String,
    /// Imported item if known.
    pub item: Option<String>,
    /// Import category.
    pub kind: VisibleImportKind,
}

/// Classification for `VisibleImport` entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibleImportKind {
    /// Parent class visibility contribution.
    Parent,
    /// Lexically imported builtin from `use builtin`.
    Builtin,
}

/// A single parent in a package inheritance chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentLink {
    /// Parent package name.
    pub package: String,
    /// Origin location if the package can be resolved in the workspace.
    pub origin: Option<DefinitionLocation>,
}

/// Parent traversal response for a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentChain {
    /// Queried package.
    pub package: String,
    /// Ordered parent packages.
    pub parents: Vec<ParentLink>,
}

/// Snapshot of effective pragma state at a byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectivePragmaState {
    /// `strict vars` flag.
    pub strict_vars: bool,
    /// `strict subs` flag.
    pub strict_subs: bool,
    /// `strict refs` flag.
    pub strict_refs: bool,
    /// Warnings global state.
    pub warnings: bool,
    /// UTF-8 source flag.
    pub utf8: bool,
    /// Enabled language features.
    pub features: Vec<String>,
    /// Lexically imported builtins.
    pub builtin_imports: Vec<String>,
}

impl From<&PragmaState> for EffectivePragmaState {
    fn from(value: &PragmaState) -> Self {
        Self {
            strict_vars: value.strict_vars,
            strict_subs: value.strict_subs,
            strict_refs: value.strict_refs,
            warnings: value.warnings,
            utf8: value.utf8,
            features: value.features.iter().map(|feature| (*feature).to_string()).collect(),
            builtin_imports: value.builtin_imports.clone(),
        }
    }
}

/// Narrow read-only semantic query façade for incremental consumer adoption.
pub struct SemanticQueryFacade {
    analyzer: SemanticAnalyzer,
    pragma_map: Vec<(std::ops::Range<usize>, PragmaState)>,
}

impl SemanticQueryFacade {
    /// Build the façade from a parsed root node and source text.
    pub fn new(root: &Node, source: &str) -> Self {
        Self {
            analyzer: SemanticAnalyzer::analyze_with_source(root, source),
            pragma_map: PragmaTracker::build(root),
        }
    }

    /// Parse source and build the façade in one step.
    pub fn from_source(source: &str) -> Result<Self, crate::ParseError> {
        let mut parser = Parser::new(source);
        let root = parser.parse()?;
        Ok(Self::new(&root, source))
    }

    /// Resolve a symbol at a byte offset.
    pub fn resolved_symbol_at(&self, offset: usize) -> Option<ResolvedSymbol> {
        let location = SourceLocation { start: offset, end: offset };
        let symbol = self.analyzer.symbol_at(location)?;
        let definition = self.analyzer.find_definition(offset).unwrap_or(symbol);

        Some(ResolvedSymbol {
            name: symbol.name.clone(),
            kind: symbol.kind,
            definition_id: DefinitionId { qualified_name: definition.qualified_name.clone() },
            definition_location: DefinitionLocation {
                uri: None,
                start: definition.location.start,
                end: definition.location.end,
            },
        })
    }

    /// Resolve a definition by symbol name via workspace index, if available.
    pub fn definition_for_symbol(
        &self,
        symbol_name: &str,
        workspace: Option<&WorkspaceIndex>,
    ) -> Option<DefinitionLocation> {
        let Some(index) = workspace else {
            return None;
        };
        let location = index.find_definition(symbol_name)?;
        Some(location_from_workspace(location))
    }

    /// Return visible import-like entries inferred from semantic and pragma state.
    pub fn visible_imports(&self) -> Vec<VisibleImport> {
        let mut imports = Vec::new();

        for model in &self.analyzer.class_models {
            for parent in &model.parents {
                imports.push(VisibleImport {
                    source: parent.clone(),
                    item: None,
                    kind: VisibleImportKind::Parent,
                });
            }
        }

        let state = PragmaTracker::state_for_offset(&self.pragma_map, usize::MAX);
        for imported in &state.builtin_imports {
            imports.push(VisibleImport {
                source: "builtin".to_string(),
                item: Some(imported.clone()),
                kind: VisibleImportKind::Builtin,
            });
        }

        imports
    }

    /// Resolve direct parent chain for a package and attach workspace origin data when possible.
    pub fn parent_chain(
        &self,
        package: &str,
        workspace: Option<&WorkspaceIndex>,
    ) -> Option<ParentChain> {
        let model = self.package_model(package)?;
        let parents = model
            .parents
            .iter()
            .map(|parent| ParentLink {
                package: parent.clone(),
                origin: workspace
                    .and_then(|index| index.find_definition(parent).map(location_from_workspace)),
            })
            .collect();

        Some(ParentChain { package: package.to_string(), parents })
    }

    /// Effective pragma state at a byte offset.
    pub fn effective_pragma_state_at(&self, offset: usize) -> EffectivePragmaState {
        let state = PragmaTracker::state_for_offset(&self.pragma_map, offset);
        EffectivePragmaState::from(&state)
    }

    fn package_model(&self, package: &str) -> Option<&ClassModel> {
        self.analyzer.class_models.iter().find(|model| model.name == package)
    }
}

fn location_from_workspace(location: Location) -> DefinitionLocation {
    DefinitionLocation {
        uri: Some(location.uri),
        start: location.range.start.line as usize,
        end: location.range.end.line as usize,
    }
}

#[cfg(test)]
mod tests {
    use super::{SemanticQueryFacade, VisibleImportKind};
    use crate::workspace::workspace_index::WorkspaceIndex;
    use std::error::Error;
    use url::Url;

    #[test]
    fn resolves_symbol_and_pragma_state() -> Result<(), Box<dyn Error>> {
        let source = r#"
            use strict;
            use warnings;
            use builtin qw(true false);
            package Child;
            use parent 'Base';
            my $value = true;
            $value;
        "#;

        let facade = SemanticQueryFacade::from_source(source)?;
        let symbol_offset = source
            .rfind("$value")
            .ok_or_else(|| "expected reference to $value in test source".to_string())?;

        let resolved = facade
            .resolved_symbol_at(symbol_offset)
            .ok_or_else(|| "expected symbol to resolve at $value reference".to_string())?;

        assert_eq!(resolved.name, "value");

        let pragma = facade.effective_pragma_state_at(symbol_offset);
        assert!(pragma.strict_vars);
        assert!(pragma.warnings);

        let imports = facade.visible_imports();
        assert!(
            imports.iter().any(|import| {
                import.kind == VisibleImportKind::Parent && import.source == "Base"
            })
        );
        assert!(imports.iter().any(|import| {
            import.kind == VisibleImportKind::Builtin && import.item.as_deref() == Some("true")
        }));

        Ok(())
    }

    #[test]
    fn resolves_parent_origin_through_workspace() -> Result<(), Box<dyn Error>> {
        let source = "package Child; use parent 'Base';";
        let facade = SemanticQueryFacade::from_source(source)?;

        let index = WorkspaceIndex::new();
        let uri = Url::parse("file:///tmp/Base.pm")?;
        let base_code = "package Base; sub inherited { 1 }";
        index.index_file(uri, base_code.to_string())?;

        let chain = facade
            .parent_chain("Child", Some(&index))
            .ok_or_else(|| "expected parent chain for Child package".to_string())?;

        assert_eq!(chain.parents.len(), 1);
        assert!(chain.parents.first().and_then(|entry| entry.origin.as_ref()).is_some());

        Ok(())
    }
}
