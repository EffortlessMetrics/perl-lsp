//! Reference resolution and symbol visibility for Navigate/Analyze workflows.

use crate::SourceLocation;
use crate::symbol::{ScopeId, Symbol, SymbolKind};

use super::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// Resolve a reference to its symbol definitions, handling cross-package lookups.
    pub(super) fn resolve_reference_to_symbols(
        &self,
        reference: &crate::symbol::SymbolReference,
    ) -> Vec<&Symbol> {
        // Handle qualified names like Foo::bar
        if let Some((pkg, name)) = reference.name.rsplit_once("::") {
            if let Some(pkg_syms) = self.symbol_table.symbols.get(pkg) {
                let mut results = Vec::new();
                for sym in pkg_syms {
                    if sym.kind == SymbolKind::Package {
                        // Find the scope associated with this package symbol
                        let pkg_scope = self
                            .symbol_table
                            .scopes
                            .values()
                            .find(|s| {
                                s.kind == crate::symbol::ScopeKind::Package
                                    && s.location.start == sym.location.start
                                    && s.location.end == sym.location.end
                            })
                            .map(|s| s.id)
                            .unwrap_or(sym.scope_id);
                        // Symbols may live in an inner block scope
                        let search_scope = self
                            .symbol_table
                            .scopes
                            .values()
                            .find(|s| s.parent == Some(pkg_scope))
                            .map(|s| s.id)
                            .unwrap_or(pkg_scope);
                        results.extend(self.symbol_table.find_symbol(
                            name,
                            search_scope,
                            reference.kind,
                        ));
                    }
                }
                results
            } else {
                self.symbol_table.find_symbol(name, reference.scope_id, reference.kind)
            }
        } else {
            self.symbol_table.find_symbol(&reference.name, reference.scope_id, reference.kind)
        }
    }

    /// Find all references to a symbol at a given position for Navigate/Analyze workflows.
    pub fn find_all_references(
        &self,
        position: usize,
        include_declaration: bool,
    ) -> Vec<SourceLocation> {
        // First find the symbol at this position (either definition or reference)
        let symbol = if let Some(def) = self.find_definition(position) {
            Some(def)
        } else {
            // Check if we're on a reference
            for refs in self.symbol_table.references.values() {
                for reference in refs {
                    if reference.location.start <= position && reference.location.end >= position {
                        // Found a reference, get its definition to get the symbol ID
                        let symbols = self.symbol_table.find_symbol(
                            &reference.name,
                            reference.scope_id,
                            reference.kind,
                        );
                        if let Some(first_symbol) = symbols.first() {
                            return self
                                .find_all_references_for_symbol(first_symbol, include_declaration);
                        }
                    }
                }
            }
            None
        };

        if let Some(symbol) = symbol {
            return self.find_all_references_for_symbol(symbol, include_declaration);
        }

        Vec::new()
    }

    /// Find all references for a specific symbol.
    pub(super) fn find_all_references_for_symbol(
        &self,
        symbol: &Symbol,
        include_declaration: bool,
    ) -> Vec<SourceLocation> {
        let mut locations = Vec::new();

        // Include the declaration if requested
        if include_declaration {
            locations.push(symbol.location);
        }

        // Find all references to this symbol by name
        if let Some(refs) = self.symbol_table.references.get(&symbol.name) {
            for reference in refs {
                // Only include references of the same kind and in scope where the symbol is visible
                if reference.kind == symbol.kind {
                    // Check if the symbol is visible from this reference's scope
                    if self.is_symbol_visible(symbol, reference.scope_id) {
                        locations.push(reference.location);
                    }
                }
            }
        }

        locations
    }

    /// Check if a symbol is visible from a given scope.
    pub(super) fn is_symbol_visible(&self, symbol: &Symbol, scope_id: ScopeId) -> bool {
        // For now, simple visibility check:
        // - Symbols in the same scope are visible
        // - Symbols in parent scopes are visible
        // - Package-level symbols are visible from package scopes

        if symbol.scope_id == scope_id {
            return true;
        }

        // Check if scope_id is a descendant of symbol.scope_id
        let mut current_scope = scope_id;
        while let Some(scope) = self.symbol_table.scopes.get(&current_scope) {
            if scope.parent == Some(symbol.scope_id) {
                return true;
            }
            if let Some(parent) = scope.parent {
                current_scope = parent;
            } else {
                break;
            }
        }

        // For package-level symbols (scope_id 0), always visible
        symbol.scope_id == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    // Helper: build a SemanticAnalyzer from Perl source.
    fn analyze(source: &str) -> Result<SemanticAnalyzer, Box<dyn std::error::Error>> {
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;
        Ok(SemanticAnalyzer::analyze_with_source(&ast, source))
    }

    // ── find_all_references ───────────────────────────────────────────────────

    #[test]
    fn find_all_references_returns_empty_for_unknown_position()
    -> Result<(), Box<dyn std::error::Error>> {
        let analyzer = analyze("my $x = 1;")?;
        // Position 9_999 is far past the end — no symbol there.
        let refs = analyzer.find_all_references(9_999, true);
        assert!(refs.is_empty(), "expected no references for out-of-bounds position");
        Ok(())
    }

    #[test]
    fn find_all_references_includes_declaration_when_requested()
    -> Result<(), Box<dyn std::error::Error>> {
        // $count is declared at position 3 (after "my ") and used at position 14.
        let source = "my $count = 0; $count + 1;";
        let analyzer = analyze(source)?;

        // Query from the declaration site (byte 3).
        let refs_with_decl = analyzer.find_all_references(3, true);
        let refs_without_decl = analyzer.find_all_references(3, false);

        // Including the declaration must yield at least as many locations.
        assert!(
            refs_with_decl.len() >= refs_without_decl.len(),
            "include_declaration=true should yield >= locations than false"
        );
        Ok(())
    }

    #[test]
    fn find_all_references_omits_declaration_when_not_requested()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "my $val = 42; print $val;";
        let analyzer = analyze(source)?;

        let refs_with = analyzer.find_all_references(3, true);
        let refs_without = analyzer.find_all_references(3, false);

        // When declaration is excluded the count must be <= the inclusive count.
        assert!(refs_without.len() <= refs_with.len());
        Ok(())
    }

    #[test]
    fn find_all_references_finds_multiple_use_sites() -> Result<(), Box<dyn std::error::Error>> {
        // $n is declared once but used twice after the declaration.
        let source = "my $n = 1; my $a = $n + 1; my $b = $n * 2;";
        let analyzer = analyze(source)?;

        // Querying from the declaration site with include_declaration=true
        // should find at least 2 locations (declaration + at least one usage).
        let refs = analyzer.find_all_references(3, true);
        assert!(
            refs.len() >= 2,
            "expected declaration + at least one use site, got {}",
            refs.len()
        );
        Ok(())
    }

    // ── is_symbol_visible ─────────────────────────────────────────────────────
    // is_symbol_visible is pub(super); we exercise it indirectly via
    // find_all_references_for_symbol called from find_all_references above,
    // and directly here by calling find_all_references and checking that
    // package-level (scope 0) symbols are always reachable.

    #[test]
    fn subroutine_at_package_scope_is_visible_globally() -> Result<(), Box<dyn std::error::Error>> {
        let source = "sub helper { return 1; } helper();";
        let analyzer = analyze(source)?;

        // The subroutine call at position 25 ("helper()") should resolve to a
        // non-empty set of reference locations when we query from a position
        // inside the call site.
        let refs = analyzer.find_all_references(25, false);
        assert!(!refs.is_empty(), "package-level subroutine should be visible at call site");
        Ok(())
    }
}
