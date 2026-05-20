//! Constants used by runtime input validation.

/// Maximum allowed path length.
pub(crate) const MAX_PATH_LENGTH: usize = 4096;

/// Maximum allowed URI length.
pub(crate) const MAX_URI_LENGTH: usize = 4096;

/// Maximum allowed JSON payload size for LSP params.
pub(crate) const MAX_PARAMS_SIZE: usize = 1_000_000;

/// Maximum allowed LSP method name length.
pub(crate) const MAX_METHOD_LENGTH: usize = 100;

/// Maximum allowed per-line text length.
pub(crate) const MAX_LINE_LENGTH: usize = 100_000;

/// Allowed file extensions for Perl files.
pub(crate) const ALLOWED_EXTENSIONS: &[&str] = &["pl", "pm", "t", "pod"];

/// Allowed URI schemes for text document synchronization.
pub(crate) const ALLOWED_TEXT_DOCUMENT_URI_SCHEMES: &[&str] =
    &["file://", "untitled:", "opencode:"];

/// Allowed execute-command entries.
pub(crate) const ALLOWED_COMMANDS: &[&str] = &[
    "perl.runCritic",
    "perl.formatDocument",
    "perl.extractVariable",
    "perl.extractSubroutine",
    "perl.optimizeImports",
    "perl.workspaceTrustReport",
    "perl.previewSafeDelete",
    "perl.safeDeleteSymbol",
    "perl.previewPackageRename",
    "perl.explainMissingModuleLookup",
];

/// Suspicious patterns rejected in generic payloads.
pub(crate) const SUSPICIOUS_PATTERNS: &[&str] =
    &["<script", "javascript:", "data:text/html", "<?php", "<%"];
