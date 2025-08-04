//! LSP feature modules
//!
//! This module organizes all LSP feature providers in a modular way.

pub mod document_symbols;
pub mod workspace_symbols;
pub mod semantic_tokens;
pub mod code_lens;
pub mod call_hierarchy;
pub mod folding_range;
pub mod inlay_hints;

use crate::ast::Node;
use std::sync::Arc;

/// Trait for LSP feature providers
pub trait FeatureProvider: Send + Sync {
    /// Get the feature name
    fn name(&self) -> &'static str;
    
    /// Check if this feature is enabled
    fn is_enabled(&self) -> bool {
        true
    }
    
    /// Initialize the feature
    fn initialize(&mut self, _params: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

/// Document-based feature provider
pub trait DocumentFeatureProvider: FeatureProvider {
    /// Process a document
    fn process_document(&self, uri: &str, content: &str, ast: &Node) -> Result<(), Box<dyn std::error::Error>>;
}

/// Workspace-wide feature provider
pub trait WorkspaceFeatureProvider: FeatureProvider {
    /// Index a document for workspace-wide features
    fn index_document(&mut self, uri: &str, ast: &Node);
    
    /// Remove a document from the index
    fn remove_document(&mut self, uri: &str);
    
    /// Clear all indexed data
    fn clear(&mut self);
}

/// Feature manager that coordinates all providers
pub struct FeatureManager {
    providers: Vec<Box<dyn FeatureProvider>>,
    document_providers: Vec<Arc<dyn DocumentFeatureProvider>>,
    workspace_providers: Vec<Arc<dyn WorkspaceFeatureProvider>>,
}

impl FeatureManager {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            document_providers: Vec::new(),
            workspace_providers: Vec::new(),
        }
    }
    
    /// Register a feature provider
    pub fn register<T: FeatureProvider + 'static>(&mut self, provider: T) {
        self.providers.push(Box::new(provider));
    }
    
    /// Register a document feature provider
    pub fn register_document<T: DocumentFeatureProvider + 'static>(&mut self, provider: Arc<T>) {
        self.document_providers.push(provider);
    }
    
    /// Register a workspace feature provider
    pub fn register_workspace<T: WorkspaceFeatureProvider + 'static>(&mut self, provider: Arc<T>) {
        self.workspace_providers.push(provider);
    }
    
    /// Initialize all features
    pub fn initialize(&mut self, params: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
        for provider in &mut self.providers {
            if provider.is_enabled() {
                provider.initialize(params)?;
            }
        }
        Ok(())
    }
    
    /// Process a document through all document providers
    pub fn process_document(&self, uri: &str, content: &str, ast: &Node) -> Result<(), Box<dyn std::error::Error>> {
        for provider in &self.document_providers {
            if provider.is_enabled() {
                provider.process_document(uri, content, ast)?;
            }
        }
        Ok(())
    }
}

impl Default for FeatureManager {
    fn default() -> Self {
        Self::new()
    }
}