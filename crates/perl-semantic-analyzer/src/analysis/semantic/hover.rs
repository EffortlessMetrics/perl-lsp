//! Hover information types and documentation extraction.

/// Hover information for symbols displayed in LSP hover requests.
///
/// Provides comprehensive symbol information including signature,
/// documentation, and contextual details for enhanced developer experience.
///
/// Used during Navigate/Analyze stages to answer hover queries.
///
/// # Performance Characteristics
/// - Computation: <100μs for typical symbol lookup
/// - Memory: Cached per symbol for repeated access
/// - LSP response: <50ms end-to-end including network
///
/// # Perl Context Integration
/// - Subroutine signatures with parameter information
/// - Package qualification and scope context
/// - POD documentation extraction and formatting
/// - Variable type inference and usage patterns
///
/// Workflow: Navigate/Analyze hover details for LSP.
#[derive(Debug, Clone)]
pub struct HoverInfo {
    /// Symbol signature or declaration
    pub signature: String,
    /// Documentation extracted from POD or comments
    pub documentation: Option<String>,
    /// Additional contextual details
    pub details: Vec<String>,
}
