//! Read-only semantic query facade that composes parser, semantic, and workspace layers.
//!
//! This module intentionally exposes a narrow, incremental surface for common
//! semantic lookups used by IDE integrations.

use crate::analysis::class_model::ClassModel;
use crate::analysis::semantic::SemanticAnalyzer;
use crate::analysis::symbol::SymbolKind;
use crate::ast::{Node, NodeKind};
use crate::pragma_tracker::{PragmaState, PragmaTracker};
use crate::workspace_index::{Location, WorkspaceIndex};
use crate::{SourceLocation, symbol::Symbol};
use std::collections::HashMap;

/// Read-only semantic query facade over parser, semantic, and optional workspace index state.
pub struct SemanticQueryFacade<'a> {
    analyzer: SemanticAnalyzer,
    ast: &'a Node,
    pragma_map: Vec<(std::ops::Range<usize>, PragmaState)>,
    workspace_index: Option<&'a WorkspaceIndex>,
}

/// A resolved symbol with semantic metadata.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ResolvedSymbol {
    /// Symbol name without package qualification.
    pub name: String,
    /// Fully qualified symbol name.
    pub qualified_name: String,
    /// Symbol classification.
    pub kind: SymbolKind,
    /// Byte range of the symbol definition in the current file.
    pub definition: SourceLocation,
}

/// A definition location resolved from local semantics and optional workspace index.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct DefinitionLocation {
    /// Local byte location in the current file.
    pub local: SourceLocation,
    /// Cross-file location if resolved via the workspace index.
    pub workspace: Option<Location>,
}

/// Visible import declaration in the current file.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct VisibleImport {
    /// Imported module name from `use`/`require`.
    pub module: String,
    /// Raw import arguments.
    pub args: Vec<String>,
    /// Source byte range of the import declaration.
    pub location: SourceLocation,
}

/// Parent-chain lookup result for inheritance-aware queries.
#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct ParentChain {
    /// Receiver class requested by the caller.
    pub class_name: String,
    /// Ordered parent names in effective lookup order.
    pub parents: Vec<String>,
    /// The class name that actually provides the queried method.
    pub inherited_origin: Option<String>,
}

/// Effective pragma state at an offset.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct EffectivePragmaState {
    /// Byte offset the query was evaluated at.
    pub offset: usize,
    /// Effective pragma state for the provided offset.
    pub state: PragmaState,
}

impl<'a> SemanticQueryFacade<'a> {
    /// Build a facade for a parsed AST and source text.
    #[must_use]
    pub fn new(ast: &'a Node, source: &str, workspace_index: Option<&'a WorkspaceIndex>) -> Self {
        Self {
            analyzer: SemanticAnalyzer::analyze_with_source(ast, source),
            ast,
            pragma_map: PragmaTracker::build(ast),
            workspace_index,
        }
    }

    /// Resolve a symbol definition at the provided byte position.
    #[must_use]
    pub fn resolved_symbol_at(&self, byte_offset: usize) -> Option<ResolvedSymbol> {
        let symbol = self.analyzer.find_definition(byte_offset)?;
        Some(Self::resolved_symbol(symbol))
    }

    /// Resolve definition location details at the provided byte position.
    #[must_use]
    pub fn definition_location_at(&self, byte_offset: usize) -> Option<DefinitionLocation> {
        let symbol = self.analyzer.find_definition(byte_offset)?;
        let workspace = self.workspace_definition(symbol);

        Some(DefinitionLocation { local: symbol.location, workspace })
    }

    /// Return all visible import declarations in source order.
    #[must_use]
    pub fn visible_imports(&self) -> Vec<VisibleImport> {
        let mut imports = Vec::new();
        Self::collect_imports(self.ast, &mut imports);
        imports.sort_by_key(|import| import.location.start);
        imports
    }

    /// Return the parent chain and inherited method origin (if known).
    #[must_use]
    pub fn parent_chain_for_method(&self, class_name: &str, method_name: &str) -> ParentChain {
        let models_by_name: HashMap<&str, &ClassModel> =
            self.analyzer.class_models.iter().map(|model| (model.name.as_str(), model)).collect();

        let parents =
            models_by_name.get(class_name).map(|model| model.parents.clone()).unwrap_or_default();

        let inherited_origin =
            self.analyzer.resolve_inherited_method_hover(class_name, method_name).and_then(
                |hover| {
                    hover.details.iter().find_map(|detail| {
                        detail.strip_prefix("Inherited from ").map(str::to_string)
                    })
                },
            );

        ParentChain { class_name: class_name.to_string(), parents, inherited_origin }
    }

    /// Compute the effective pragma state at a byte offset.
    #[must_use]
    pub fn effective_pragma_state(&self, byte_offset: usize) -> EffectivePragmaState {
        let state = PragmaTracker::state_for_offset(&self.pragma_map, byte_offset);
        EffectivePragmaState { offset: byte_offset, state }
    }

    fn resolved_symbol(symbol: &Symbol) -> ResolvedSymbol {
        ResolvedSymbol {
            name: symbol.name.clone(),
            qualified_name: symbol.qualified_name.clone(),
            kind: symbol.kind,
            definition: symbol.location,
        }
    }

    fn workspace_definition(&self, symbol: &Symbol) -> Option<Location> {
        let index = self.workspace_index?;

        index
            .find_definition(&symbol.qualified_name)
            .or_else(|| index.find_definition(&symbol.name))
    }

    fn collect_imports(node: &Node, out: &mut Vec<VisibleImport>) {
        if let NodeKind::Use { module, args, .. } = &node.kind {
            out.push(VisibleImport {
                module: module.clone(),
                args: args.clone(),
                location: node.location,
            });
        }

        for child in node.children() {
            Self::collect_imports(child, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SemanticQueryFacade;
    use crate::Parser;

    #[test]
    fn facade_exposes_visible_imports_and_pragma_state() -> Result<(), Box<dyn std::error::Error>> {
        let code = "use strict;\nuse warnings;\nmy $x = 1;\n";
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;

        let facade = SemanticQueryFacade::new(&ast, code, None);
        let imports = facade.visible_imports();

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].module, "strict");
        assert_eq!(imports[1].module, "warnings");

        let pragma = facade.effective_pragma_state(code.find("$x").ok_or("missing variable")?);
        assert!(pragma.state.strict_vars);
        assert!(pragma.state.warnings);

        Ok(())
    }

    #[test]
    fn facade_resolves_symbol_definition() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $value = 1;\n$value += 1;\n";
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;

        let facade = SemanticQueryFacade::new(&ast, code, None);
        let use_offset = code.rfind("$value").ok_or("missing symbol use")?;

        let symbol = facade.resolved_symbol_at(use_offset).ok_or("definition not found")?;
        assert_eq!(symbol.name, "value");

        let location = facade.definition_location_at(use_offset).ok_or("location not found")?;
        assert_eq!(location.local.start, code.find("$value").ok_or("missing declaration")?);

        Ok(())
    }
}
