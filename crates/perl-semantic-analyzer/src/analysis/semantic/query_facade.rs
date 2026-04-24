//! Read-only semantic query facade over parser, semantic, and workspace layers.

use std::ops::Range;

use crate::SourceLocation;
use crate::ast::Node;
use crate::pragma_tracker::{PragmaState, PragmaTracker};
use crate::symbol::{Symbol, SymbolKind};
use crate::workspace_index::WorkspaceIndex;

use super::SemanticModel;

/// Stable read-only semantic query surface for incremental consumer adoption.
#[derive(Debug)]
pub struct SemanticQueryFacade {
    model: SemanticModel,
    pragma_map: Vec<(Range<usize>, PragmaState)>,
}

/// Read-only symbol projection returned by semantic query lookups.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResolvedSymbol {
    /// Symbol name (without sigil for variables).
    pub name: String,
    /// Fully qualified symbol name when known.
    pub qualified_name: String,
    /// Symbol kind.
    pub kind: SymbolKind,
    /// Definition source location.
    pub location: SourceLocation,
}

/// Definition location that may include a workspace URI.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DefinitionLocation {
    /// File URI if known for this definition.
    pub uri: Option<String>,
    /// Byte-range location inside the source file.
    pub location: SourceLocation,
}

/// Imported module visible to the current document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VisibleImport {
    /// Imported module name.
    pub module_name: String,
}

/// Ordered inheritance information for a class.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParentChain {
    /// Class/package name that owns the chain.
    pub class_name: String,
    /// Ancestors in method-resolution order.
    pub ancestors: Vec<String>,
}

/// Effective pragma state at a byte offset.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct EffectivePragmaState {
    /// Byte offset where state was requested.
    pub offset: usize,
    /// Effective tracked pragma state.
    pub state: PragmaState,
}

impl SemanticQueryFacade {
    /// Build a read-only query facade from parser output and source text.
    pub fn build(root: &Node, source: &str) -> Self {
        Self { model: SemanticModel::build(root, source), pragma_map: PragmaTracker::build(root) }
    }

    /// Access the underlying semantic model for incremental migration.
    pub fn semantic_model(&self) -> &SemanticModel {
        &self.model
    }

    /// Resolve a symbol definition at `position`.
    pub fn resolved_symbol_at(&self, position: usize) -> Option<ResolvedSymbol> {
        self.model.definition_at(position).map(ResolvedSymbol::from)
    }

    /// Resolve a definition location at `position`.
    pub fn definition_location_at(
        &self,
        position: usize,
        current_uri: Option<&str>,
    ) -> Option<DefinitionLocation> {
        self.model.definition_at(position).map(|symbol| DefinitionLocation {
            uri: current_uri.map(std::string::ToString::to_string),
            location: symbol.location,
        })
    }

    /// Return imports visible to `uri` from workspace indexing.
    pub fn visible_imports(&self, workspace: &WorkspaceIndex, uri: &str) -> Vec<VisibleImport> {
        let mut imports: Vec<_> = workspace
            .file_dependencies(uri)
            .into_iter()
            .map(|module_name| VisibleImport { module_name })
            .collect();
        imports.sort_by(|left, right| left.module_name.cmp(&right.module_name));
        imports
    }

    /// Return class parent chain in analyzer-configured resolution order.
    pub fn parent_chain(&self, class_name: &str) -> Option<ParentChain> {
        self.model
            .parent_chain(class_name)
            .map(|ancestors| ParentChain { class_name: class_name.to_string(), ancestors })
    }

    /// Resolve inherited method origin for a class and method name.
    pub fn inherited_origin(
        &self,
        class_name: &str,
        method_name: &str,
        current_uri: Option<&str>,
    ) -> Option<DefinitionLocation> {
        self.model.resolve_inherited_method_location(class_name, method_name).map(|location| {
            DefinitionLocation { uri: current_uri.map(std::string::ToString::to_string), location }
        })
    }

    /// Return effective tracked pragma state for `offset`.
    pub fn effective_pragma_state(&self, offset: usize) -> EffectivePragmaState {
        EffectivePragmaState {
            offset,
            state: PragmaTracker::state_for_offset(&self.pragma_map, offset),
        }
    }
}

impl From<&Symbol> for ResolvedSymbol {
    fn from(value: &Symbol) -> Self {
        Self {
            name: value.name.clone(),
            qualified_name: value.qualified_name.clone(),
            kind: value.kind,
            location: value.location,
        }
    }
}

#[cfg(test)]
mod tests {
    use perl_tdd_support::{must, must_some};

    use super::*;
    use crate::parser::Parser;
    use crate::workspace_index;

    #[test]
    fn query_facade_resolves_symbol_and_pragmas() {
        let code = "use strict;\nmy $value = 1;\n$value;\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let facade = SemanticQueryFacade::build(&ast, code);
        let usage_offset = must_some(code.rfind("$value"));

        let symbol = must_some(facade.resolved_symbol_at(usage_offset));
        assert_eq!(symbol.name, "value");

        let pragma_state = facade.effective_pragma_state(usage_offset);
        assert!(pragma_state.state.strict_vars);
    }

    #[test]
    fn query_facade_reads_workspace_imports_and_parent_chain() {
        let code = "package Child;\nuse parent 'Base';\n1;\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let facade = SemanticQueryFacade::build(&ast, code);

        let index = workspace_index::WorkspaceIndex::new();
        must(index.index_file_str("file:///test/child.pm", code));

        let imports = facade.visible_imports(&index, "file:///test/child.pm");
        assert!(imports.iter().any(|import| import.module_name == "Base"));

        let chain = must_some(facade.parent_chain("Child"));
        assert_eq!(chain.ancestors, vec!["Base"]);
    }
}
