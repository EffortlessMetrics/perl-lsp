use super::Value;

/// Intermediate result from phase-1 hover extraction (under document lock).
///
/// The document lock must be released before calling module resolution to avoid
/// deadlock, so we extract what we need first and resolve afterwards.
pub(super) enum HoverExtracted {
    /// Hover content fully built (symbol, builtin, or token hover).
    Complete(Value),
    /// A `use Module` was found; module name needs resolution without lock.
    /// Carries (module_name, doc_text, doc_uri, doc_offset) for use lib / FindBin wiring.
    UseModule(String, String, String, usize),
    /// Cursor is on a `->method()` call where the method belongs to an inherited or
    /// role-composed ancestor class. Carries (receiver_pkg, method_name, doc_uri).
    /// Phase 2 resolves the hover using the workspace index BFS (same logic as
    /// `inherited_method_definition_location` in navigation.rs).
    InheritedMethod(String, String, String),
    /// Cursor is on a package-name token (contains `::`) that was not handled by an
    /// earlier semantic or `use` path. Carries (package_name, doc_text, doc_uri, doc_offset).
    /// Phase 2 resolves it via `build_module_hover` (same as `UseModule`).
    PossiblePackage(String, String, String, usize),
    /// Nothing hoverable at this position.
    None,
}
