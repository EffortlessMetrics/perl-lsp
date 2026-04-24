//! Read-only semantic query facade that composes parser, semantic, and workspace data.

use crate::analysis::class_model::ClassModel;
use crate::analysis::semantic::SemanticModel;
use crate::ast::Node;
use crate::pragma_tracker::{PragmaState, PragmaTracker};
use crate::symbol::SymbolKind;
use crate::workspace_index::WorkspaceIndex;

/// Stable read-only symbol view returned by semantic queries.
#[derive(Debug, Clone)]
pub struct ResolvedSymbol {
    /// Bare symbol name.
    pub name: String,
    /// Fully-qualified symbol name.
    pub qualified_name: String,
    /// Symbol kind.
    pub kind: SymbolKind,
    /// Byte-span of the symbol in the source document.
    pub span: crate::SourceLocation,
}

/// Location of a symbol definition.
#[derive(Debug, Clone)]
pub struct DefinitionLocation {
    /// File URI when available (workspace-backed query).
    pub uri: Option<String>,
    /// Byte-span of the definition in the source document.
    pub span: crate::SourceLocation,
}

/// Import or export-like name visible for a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleImport {
    /// Package exposing the symbol.
    pub package: String,
    /// Importable symbol name.
    pub symbol: String,
    /// Whether this symbol is exported by default.
    pub exported_by_default: bool,
}

/// Parent package chain metadata for a class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentChain {
    /// Receiver class queried.
    pub class_name: String,
    /// Ordered ancestors as declared in the class model.
    pub ancestors: Vec<String>,
}

/// Effective pragma state at a byte offset.
#[derive(Debug, Clone)]
pub struct EffectivePragmaState {
    /// Byte offset queried.
    pub offset: usize,
    /// Effective pragma state at `offset`.
    pub state: PragmaState,
}

/// Read-only semantic query entry point over parser, semantic, and optional workspace index.
pub struct SemanticQueryFacade<'a> {
    semantic_model: &'a SemanticModel,
    workspace_index: Option<&'a WorkspaceIndex>,
    class_models: &'a [ClassModel],
    pragma_map: Vec<(std::ops::Range<usize>, PragmaState)>,
}

impl<'a> SemanticQueryFacade<'a> {
    /// Create a facade for read-only semantic queries.
    pub fn new(root: &Node, semantic_model: &'a SemanticModel) -> Self {
        Self {
            semantic_model,
            workspace_index: None,
            class_models: semantic_model.class_models(),
            pragma_map: PragmaTracker::build(root),
        }
    }

    /// Attach a workspace index for cross-file lookups.
    pub fn with_workspace_index(mut self, workspace_index: &'a WorkspaceIndex) -> Self {
        self.workspace_index = Some(workspace_index);
        self
    }

    /// Resolve the most-specific symbol at a byte offset.
    pub fn resolve_symbol_at(&self, offset: usize) -> Option<ResolvedSymbol> {
        let symbol = self.semantic_model.definition_at(offset)?;
        Some(ResolvedSymbol {
            name: symbol.name.clone(),
            qualified_name: symbol.qualified_name.clone(),
            kind: symbol.kind,
            span: symbol.location,
        })
    }

    /// Resolve definition location at a byte offset.
    pub fn definition_location_at(&self, offset: usize) -> Option<DefinitionLocation> {
        let symbol = self.semantic_model.definition_at(offset)?;

        let uri = self.workspace_index.and_then(|workspace| {
            let mut symbols = workspace.find_symbols(&symbol.qualified_name);
            symbols.pop().map(|found| found.uri)
        });

        Some(DefinitionLocation { uri, span: symbol.location })
    }

    /// Collect importable symbols visible from a package by reading export metadata.
    pub fn visible_imports_for_package(&self, package_name: &str) -> Vec<VisibleImport> {
        let mut imports = Vec::new();
        for model in self.class_models.iter().filter(|model| model.name == package_name) {
            for symbol in &model.exports {
                imports.push(VisibleImport {
                    package: model.name.clone(),
                    symbol: symbol.clone(),
                    exported_by_default: true,
                });
            }
            for symbol in &model.export_ok {
                imports.push(VisibleImport {
                    package: model.name.clone(),
                    symbol: symbol.clone(),
                    exported_by_default: false,
                });
            }
        }

        imports
    }

    /// Return declared direct parents for a class.
    pub fn parent_chain_for_class(&self, class_name: &str) -> Option<ParentChain> {
        let class_model = self.class_models.iter().find(|model| model.name == class_name)?;
        Some(ParentChain {
            class_name: class_name.to_string(),
            ancestors: class_model.parents.clone(),
        })
    }

    /// Return effective pragma state at a byte offset.
    pub fn effective_pragma_state_at(&self, offset: usize) -> EffectivePragmaState {
        EffectivePragmaState {
            offset,
            state: PragmaTracker::state_for_offset(&self.pragma_map, offset),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    use perl_tdd_support::must_some;

    #[test]
    fn resolves_local_symbol_and_definition() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $value = 41;\n$value += 1;\n";
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let model = SemanticModel::build(&ast, code);
        let facade = SemanticQueryFacade::new(&ast, &model);

        let symbol = must_some(facade.resolve_symbol_at(17));
        assert_eq!(symbol.name, "value");

        let definition = must_some(facade.definition_location_at(17));
        assert_eq!(definition.span.start, 3);
        Ok(())
    }

    #[test]
    fn collects_exports_and_pragma_state() -> Result<(), Box<dyn std::error::Error>> {
        let code = "package My::Exporter;\nuse strict;\nour @EXPORT = qw(foo);\nour @EXPORT_OK = qw(bar);\n";
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let model = SemanticModel::build(&ast, code);
        let facade = SemanticQueryFacade::new(&ast, &model);

        let imports = facade.visible_imports_for_package("My::Exporter");
        assert!(imports.iter().any(|item| item.symbol == "foo" && item.exported_by_default));
        assert!(imports.iter().any(|item| item.symbol == "bar" && !item.exported_by_default));

        let pragma = facade.effective_pragma_state_at(25);
        assert!(pragma.state.strict_vars);
        Ok(())
    }
}
