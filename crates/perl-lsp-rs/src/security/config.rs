/// Security configuration for the LSP server
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum file size for parsing (bytes).
    ///
    /// Defaults to the value from [`perl_lsp_rs_core::runtime::limits::LspLimits`] (1MB).
    /// Adjust via `perl.limits.maxFileSizeBytes` in LSP settings.
    pub max_file_size: usize,
    /// Maximum path length
    pub max_path_length: usize,
    /// Allowed file extensions
    pub allowed_extensions: Vec<String>,
    /// Whether to enable strict validation
    pub strict_mode: bool,
    /// Maximum LSP parameter size
    pub max_parameter_size: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_file_size: perl_lsp_rs_core::runtime::limits::max_file_size_bytes(),
            max_path_length: 4096,
            allowed_extensions: vec![
                "pl".to_string(),
                "pm".to_string(),
                "t".to_string(),
                "pod".to_string(),
            ],
            strict_mode: true,
            max_parameter_size: 1_000_000,
        }
    }
}
