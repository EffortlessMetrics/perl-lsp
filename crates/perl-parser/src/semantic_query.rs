//! Read-only semantic query façade spanning parser, semantic, and workspace layers.
//!
//! This module provides a narrow integration surface that composes existing
//! parser/semantic/workspace capabilities without introducing new semantic
//! behavior. The API is intentionally small so consumers can adopt it
//! incrementally.

use crate::analysis::class_model::{ClassModelBuilder, MethodResolutionOrder};
use crate::analysis::semantic::SemanticAnalyzer;
use crate::analysis::symbol::{Symbol, SymbolKind};
use crate::ast::{Node, NodeKind};
use crate::position::Range;
use crate::pragma_tracker::PragmaTracker;
use crate::workspace::workspace_index::WorkspaceIndex;
use crate::SourceLocation;

/// Stable resolved symbol information for read-only semantic queries.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ResolvedSymbol {
    /// Symbol name in source form (without package qualification for variables).
    pub name: String,
    /// Fully qualified symbol name when available.
    pub qualified_name: String,
    /// Semantic kind (subroutine, variable, package, ...).
    pub kind: SymbolKind,
    /// Definition location.
    pub definition: DefinitionLocation,
    /// Declaration style when known (`my`, `our`, `state`, ...).
    pub declaration: Option<String>,
    /// Optional documentation attached to the declaration.
    pub documentation: Option<String>,
}

/// Definition target location from same-file semantic analysis or workspace index.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum DefinitionLocation {
    /// Definition in the current file/AST.
    CurrentFile {
        /// Byte-based source span for the definition.
        location: SourceLocation,
    },
    /// Definition resolved via workspace index.
    Workspace {
        /// URI of the file containing the definition.
        uri: String,
        /// LSP-style line/column range inside `uri`.
        range: Range,
    },
}

/// Import made visible by a `use` statement.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct VisibleImport {
    /// Imported module name.
    pub module: String,
    /// Explicit import list from the `use` statement arguments.
    pub imports: Vec<String>,
    /// Source span for the `use` statement.
    pub location: SourceLocation,
}

/// Parent relationship chain for a class/package.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ParentChain {
    /// Receiver class/package name.
    pub class_name: String,
    /// Parent class order as discovered from semantic class model.
    pub parents: Vec<String>,
    /// Method-resolution order configured for this class.
    pub mro: MethodResolutionOrder,
}

/// Effective lexical pragma state at a byte offset.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct EffectivePragmaState {
    /// Whether strict vars is active.
    pub strict_vars: bool,
    /// Whether strict subs is active.
    pub strict_subs: bool,
    /// Whether strict refs is active.
    pub strict_refs: bool,
    /// Whether warnings are enabled.
    pub warnings: bool,
    /// Effective enabled features.
    pub features: Vec<&'static str>,
    /// Lexically imported builtin names.
    pub builtin_imports: Vec<String>,
}

/// Narrow read-only façade for semantic navigation-style queries.
pub struct SemanticQueryFacade<'a> {
    analyzer: SemanticAnalyzer,
    class_models: Vec<crate::analysis::class_model::ClassModel>,
    pragma_map: Vec<(std::ops::Range<usize>, crate::pragma_tracker::PragmaState)>,
    root: &'a Node,
    workspace_index: Option<&'a WorkspaceIndex>,
}

impl<'a> SemanticQueryFacade<'a> {
    /// Build a query façade from parsed AST/source and optional workspace index.
    pub fn new(
        root: &'a Node,
        source: &'a str,
        workspace_index: Option<&'a WorkspaceIndex>,
    ) -> Self {
        Self {
            analyzer: SemanticAnalyzer::analyze_with_source(root, source),
            class_models: ClassModelBuilder::new().build(root),
            pragma_map: PragmaTracker::build(root),
            root,
            workspace_index,
        }
    }

    /// Resolve the symbol definition associated with a byte offset.
    pub fn resolved_symbol_at(&self, byte_offset: usize) -> Option<ResolvedSymbol> {
        self.analyzer.find_definition(byte_offset).map(Self::resolved_symbol_from)
    }

    /// Resolve a definition by symbol name, preferring workspace lookups when available.
    pub fn definition_location(&self, symbol_name: &str) -> Option<DefinitionLocation> {
        self.workspace_index.and_then(|index| index.find_definition(symbol_name)).map_or_else(
            || {
                self.analyzer
                    .symbol_table()
                    .symbols
                    .get(symbol_name)
                    .and_then(|symbols| symbols.first())
                    .map(|symbol| DefinitionLocation::CurrentFile { location: symbol.location })
            },
            |location| {
                Some(DefinitionLocation::Workspace { uri: location.uri, range: location.range })
            },
        )
    }

    /// Return all visible imports (`use`) in the current file.
    pub fn visible_imports(&self) -> Vec<VisibleImport> {
        let mut imports = Vec::new();
        collect_visible_imports(self.root, &mut imports);
        imports
    }

    /// Return parent metadata for a class/package when present in same-file class models.
    pub fn parent_chain(&self, class_name: &str) -> Option<ParentChain> {
        self.class_models.iter().find(|model| model.name == class_name).map(|model| ParentChain {
            class_name: model.name.clone(),
            parents: model.parents.clone(),
            mro: model.mro,
        })
    }

    /// Resolve inherited method origin in the current file's class hierarchy.
    pub fn inherited_origin(
        &self,
        receiver_class: &str,
        method_name: &str,
    ) -> Option<DefinitionLocation> {
        self.analyzer
            .resolve_inherited_method_location(receiver_class, method_name)
            .map(|location| DefinitionLocation::CurrentFile { location })
    }

    /// Return effective lexical pragma state at the provided byte offset.
    pub fn effective_pragma_state(&self, byte_offset: usize) -> EffectivePragmaState {
        let state = PragmaTracker::state_for_offset(&self.pragma_map, byte_offset);
        EffectivePragmaState {
            strict_vars: state.strict_vars,
            strict_subs: state.strict_subs,
            strict_refs: state.strict_refs,
            warnings: state.warnings,
            features: state.features,
            builtin_imports: state.builtin_imports,
        }
    }

    fn resolved_symbol_from(symbol: &Symbol) -> ResolvedSymbol {
        ResolvedSymbol {
            name: symbol.name.clone(),
            qualified_name: symbol.qualified_name.clone(),
            kind: symbol.kind,
            definition: DefinitionLocation::CurrentFile { location: symbol.location },
            declaration: symbol.declaration.clone(),
            documentation: symbol.documentation.clone(),
        }
    }
}

fn collect_visible_imports(node: &Node, imports: &mut Vec<VisibleImport>) {
    if let NodeKind::Use { module, args, .. } = &node.kind {
        imports.push(VisibleImport {
            module: module.clone(),
            imports: args.clone(),
            location: node.location,
        });
    }

    for child in node.children() {
        collect_visible_imports(child, imports);
    }
}

#[cfg(test)]
mod tests {
    use super::{DefinitionLocation, SemanticQueryFacade};
    use crate::Parser;

    #[test]
    fn facade_exposes_read_only_queries() -> Result<(), Box<dyn std::error::Error>> {
        let source = r#"
            package Parent;
            sub greet { return "hi" }

            package Child;
            use parent 'Parent';
            use feature 'say';
            my $value = 1;
            $value += 2;
        "#;

        let mut parser = Parser::new(source);
        let ast = parser.parse()?;
        let facade = SemanticQueryFacade::new(&ast, source, None);

        let value_ref_offset = source.find("$value +=").ok_or("missing test offset")?;
        let resolved = facade.resolved_symbol_at(value_ref_offset).ok_or("missing symbol")?;
        assert_eq!(resolved.name, "value");

        let imports = facade.visible_imports();
        assert!(imports.iter().any(|import| import.module == "parent"));

        let parent_chain = facade.parent_chain("Child").ok_or("missing parent chain")?;
        assert_eq!(parent_chain.parents, vec!["Parent"]);

        if let Some(inherited) = facade.inherited_origin("Child", "greet") {
            match inherited {
                DefinitionLocation::CurrentFile { .. } => {}
                DefinitionLocation::Workspace { .. } => {
                    return Err("expected same-file inherited origin".into());
                }
            }
        }

        let feature_offset = source.find("use feature").ok_or("missing feature offset")?;
        let pragma_state = facade.effective_pragma_state(feature_offset);
        assert!(pragma_state.features.iter().any(|feature| *feature == "say"));

        Ok(())
    }
}
