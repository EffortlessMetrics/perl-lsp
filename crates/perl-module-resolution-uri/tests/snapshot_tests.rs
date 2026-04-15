//! Snapshot tests for perl-module-resolution-uri structured outputs.
//!
//! These tests capture the Debug representation of key types as baselines.
//! Any change to the Debug format of these types will be detected by these tests.
//!
//! ## Types Snapshot Tested
//!
//! 1. `ModuleUriResolution` enum - all three variants
//! 2. `IncRoot` struct - path-based resolution metadata
//! 3. `IncRootKind` enum - include root category labels
//!
//! ## Coverage
//!
//! - ModuleUriResolution::Resolved(String) - captures full URI string
//! - ModuleUriResolution::NotFound - unit variant
//! - ModuleUriResolution::TimedOut - unit variant
//! - IncRoot with each IncRootKind variant
//! - IncRootKind::FileLocalLexical
//! - IncRootKind::WorkspaceRelative
//! - IncRootKind::ExternalAbsolute
//! - IncRootKind::InterpreterStartup
//! - IncRootKind::RuntimeDerived

use perl_module_resolution_uri::{IncRoot, IncRootKind, ModuleUriResolution};
use std::path::PathBuf;

// =============================================================================
// ModuleUriResolution enum snapshots
// =============================================================================

// Snapshot: ModuleUriResolution::Resolved variant Debug format
const SNAPSHOT_MODULE_URI_RESOLUTION_RESOLVED: &str = "Resolved(\"file:///path/to/module.pm\")";

#[test]
fn snapshot_module_uri_resolution_resolved_debug() {
    let resolved = ModuleUriResolution::Resolved("file:///path/to/module.pm".to_string());
    let debug_output = format!("{:?}", resolved);
    assert_eq!(
        debug_output, SNAPSHOT_MODULE_URI_RESOLUTION_RESOLVED,
        "ModuleUriResolution::Resolved Debug format changed. \
         If this is intentional, update SNAPSHOT_MODULE_URI_RESOLUTION_RESOLVED."
    );
}

// Snapshot: ModuleUriResolution::NotFound variant Debug format
const SNAPSHOT_MODULE_URI_RESOLUTION_NOT_FOUND: &str = "NotFound";

#[test]
fn snapshot_module_uri_resolution_not_found_debug() {
    let not_found = ModuleUriResolution::NotFound;
    let debug_output = format!("{:?}", not_found);
    assert_eq!(
        debug_output, SNAPSHOT_MODULE_URI_RESOLUTION_NOT_FOUND,
        "ModuleUriResolution::NotFound Debug format changed. \
         If this is intentional, update SNAPSHOT_MODULE_URI_RESOLUTION_NOT_FOUND."
    );
}

// Snapshot: ModuleUriResolution::TimedOut variant Debug format
const SNAPSHOT_MODULE_URI_RESOLUTION_TIMED_OUT: &str = "TimedOut";

#[test]
fn snapshot_module_uri_resolution_timed_out_debug() {
    let timed_out = ModuleUriResolution::TimedOut;
    let debug_output = format!("{:?}", timed_out);
    assert_eq!(
        debug_output, SNAPSHOT_MODULE_URI_RESOLUTION_TIMED_OUT,
        "ModuleUriResolution::TimedOut Debug format changed. \
         If this is intentional, update SNAPSHOT_MODULE_URI_RESOLUTION_TIMED_OUT."
    );
}

// =============================================================================
// IncRootKind enum snapshots
// =============================================================================

// Snapshot: IncRootKind::FileLocalLexical Debug format
const SNAPSHOT_INC_ROOT_KIND_FILE_LOCAL_LEXICAL: &str = "FileLocalLexical";

#[test]
fn snapshot_inc_root_kind_file_local_lexical_debug() {
    let kind = IncRootKind::FileLocalLexical;
    let debug_output = format!("{:?}", kind);
    assert_eq!(
        debug_output, SNAPSHOT_INC_ROOT_KIND_FILE_LOCAL_LEXICAL,
        "IncRootKind::FileLocalLexical Debug format changed. \
         If this is intentional, update SNAPSHOT_INC_ROOT_KIND_FILE_LOCAL_LEXICAL."
    );
}

// Snapshot: IncRootKind::WorkspaceRelative Debug format
const SNAPSHOT_INC_ROOT_KIND_WORKSPACE_RELATIVE: &str = "WorkspaceRelative";

#[test]
fn snapshot_inc_root_kind_workspace_relative_debug() {
    let kind = IncRootKind::WorkspaceRelative;
    let debug_output = format!("{:?}", kind);
    assert_eq!(
        debug_output, SNAPSHOT_INC_ROOT_KIND_WORKSPACE_RELATIVE,
        "IncRootKind::WorkspaceRelative Debug format changed. \
         If this is intentional, update SNAPSHOT_INC_ROOT_KIND_WORKSPACE_RELATIVE."
    );
}

// Snapshot: IncRootKind::ExternalAbsolute Debug format
const SNAPSHOT_INC_ROOT_KIND_EXTERNAL_ABSOLUTE: &str = "ExternalAbsolute";

#[test]
fn snapshot_inc_root_kind_external_absolute_debug() {
    let kind = IncRootKind::ExternalAbsolute;
    let debug_output = format!("{:?}", kind);
    assert_eq!(
        debug_output, SNAPSHOT_INC_ROOT_KIND_EXTERNAL_ABSOLUTE,
        "IncRootKind::ExternalAbsolute Debug format changed. \
         If this is intentional, update SNAPSHOT_INC_ROOT_KIND_EXTERNAL_ABSOLUTE."
    );
}

// Snapshot: IncRootKind::InterpreterStartup Debug format
const SNAPSHOT_INC_ROOT_KIND_INTERPRETER_STARTUP: &str = "InterpreterStartup";

#[test]
fn snapshot_inc_root_kind_interpreter_startup_debug() {
    let kind = IncRootKind::InterpreterStartup;
    let debug_output = format!("{:?}", kind);
    assert_eq!(
        debug_output, SNAPSHOT_INC_ROOT_KIND_INTERPRETER_STARTUP,
        "IncRootKind::InterpreterStartup Debug format changed. \
         If this is intentional, update SNAPSHOT_INC_ROOT_KIND_INTERPRETER_STARTUP."
    );
}

// Snapshot: IncRootKind::RuntimeDerived Debug format
const SNAPSHOT_INC_ROOT_KIND_RUNTIME_DERIVED: &str = "RuntimeDerived";

#[test]
fn snapshot_inc_root_kind_runtime_derived_debug() {
    let kind = IncRootKind::RuntimeDerived;
    let debug_output = format!("{:?}", kind);
    assert_eq!(
        debug_output, SNAPSHOT_INC_ROOT_KIND_RUNTIME_DERIVED,
        "IncRootKind::RuntimeDerived Debug format changed. \
         If this is intentional, update SNAPSHOT_INC_ROOT_KIND_RUNTIME_DERIVED."
    );
}

// =============================================================================
// IncRoot struct snapshots (one per IncRootKind variant)
// =============================================================================

// Snapshot: IncRoot with FileLocalLexical
const SNAPSHOT_INC_ROOT_FILE_LOCAL_LEXICAL: &str = "IncRoot { kind: FileLocalLexical, path: \"/workspace/lib\", precedence: 0, source: \"use lib\" }";

#[test]
fn snapshot_inc_root_file_local_lexical_debug() {
    let root = IncRoot {
        kind: IncRootKind::FileLocalLexical,
        path: PathBuf::from("/workspace/lib"),
        precedence: 0,
        source: "use lib".to_string(),
    };
    let debug_output = format!("{:?}", root);
    assert_eq!(
        debug_output, SNAPSHOT_INC_ROOT_FILE_LOCAL_LEXICAL,
        "IncRoot (FileLocalLexical) Debug format changed. \
         If this is intentional, update SNAPSHOT_INC_ROOT_FILE_LOCAL_LEXICAL."
    );
}

// Snapshot: IncRoot with WorkspaceRelative
const SNAPSHOT_INC_ROOT_WORKSPACE_RELATIVE: &str =
    "IncRoot { kind: WorkspaceRelative, path: \"lib\", precedence: 0, source: \"includePaths\" }";

#[test]
fn snapshot_inc_root_workspace_relative_debug() {
    let root = IncRoot {
        kind: IncRootKind::WorkspaceRelative,
        path: PathBuf::from("lib"),
        precedence: 0,
        source: "includePaths".to_string(),
    };
    let debug_output = format!("{:?}", root);
    assert_eq!(
        debug_output, SNAPSHOT_INC_ROOT_WORKSPACE_RELATIVE,
        "IncRoot (WorkspaceRelative) Debug format changed. \
         If this is intentional, update SNAPSHOT_INC_ROOT_WORKSPACE_RELATIVE."
    );
}

// Snapshot: IncRoot with ExternalAbsolute
const SNAPSHOT_INC_ROOT_EXTERNAL_ABSOLUTE: &str = "IncRoot { kind: ExternalAbsolute, path: \"/opt/perl/lib\", precedence: 5, source: \"external\" }";

#[test]
fn snapshot_inc_root_external_absolute_debug() {
    let root = IncRoot {
        kind: IncRootKind::ExternalAbsolute,
        path: PathBuf::from("/opt/perl/lib"),
        precedence: 5,
        source: "external".to_string(),
    };
    let debug_output = format!("{:?}", root);
    assert_eq!(
        debug_output, SNAPSHOT_INC_ROOT_EXTERNAL_ABSOLUTE,
        "IncRoot (ExternalAbsolute) Debug format changed. \
         If this is intentional, update SNAPSHOT_INC_ROOT_EXTERNAL_ABSOLUTE."
    );
}

// Snapshot: IncRoot with InterpreterStartup
const SNAPSHOT_INC_ROOT_INTERPRETER_STARTUP: &str = "IncRoot { kind: InterpreterStartup, path: \"/usr/lib/perl5\", precedence: 10, source: \"interpreter-startup-inc\" }";

#[test]
fn snapshot_inc_root_interpreter_startup_debug() {
    let root = IncRoot {
        kind: IncRootKind::InterpreterStartup,
        path: PathBuf::from("/usr/lib/perl5"),
        precedence: 10,
        source: "interpreter-startup-inc".to_string(),
    };
    let debug_output = format!("{:?}", root);
    assert_eq!(
        debug_output, SNAPSHOT_INC_ROOT_INTERPRETER_STARTUP,
        "IncRoot (InterpreterStartup) Debug format changed. \
         If this is intentional, update SNAPSHOT_INC_ROOT_INTERPRETER_STARTUP."
    );
}

// Snapshot: IncRoot with RuntimeDerived
const SNAPSHOT_INC_ROOT_RUNTIME_DERIVED: &str = "IncRoot { kind: RuntimeDerived, path: \"/tmp/perl-lib\", precedence: 15, source: \"runtime\" }";

#[test]
fn snapshot_inc_root_runtime_derived_debug() {
    let root = IncRoot {
        kind: IncRootKind::RuntimeDerived,
        path: PathBuf::from("/tmp/perl-lib"),
        precedence: 15,
        source: "runtime".to_string(),
    };
    let debug_output = format!("{:?}", root);
    assert_eq!(
        debug_output, SNAPSHOT_INC_ROOT_RUNTIME_DERIVED,
        "IncRoot (RuntimeDerived) Debug format changed. \
         If this is intentional, update SNAPSHOT_INC_ROOT_RUNTIME_DERIVED."
    );
}

// =============================================================================
// Structural snapshots - ensuring no unexpected fields
// =============================================================================

#[test]
fn snapshot_inc_root_has_exactly_four_fields() {
    // This test verifies the IncRoot struct has exactly 4 public fields
    // by checking its Debug output contains only the expected field names.
    let root = IncRoot {
        kind: IncRootKind::WorkspaceRelative,
        path: PathBuf::from("/test"),
        precedence: 0,
        source: "test".to_string(),
    };
    let debug_str = format!("{:?}", root);

    // Should contain exactly these 4 field names
    assert!(debug_str.contains("kind:"), "IncRoot should have 'kind' field");
    assert!(debug_str.contains("path:"), "IncRoot should have 'path' field");
    assert!(debug_str.contains("precedence:"), "IncRoot should have 'precedence' field");
    assert!(debug_str.contains("source:"), "IncRoot should have 'source' field");

    // Should NOT contain signature-related fields
    let debug_lower = debug_str.to_lowercase();
    assert!(
        !debug_lower.contains("signature"),
        "IncRoot should NOT have 'signature' field (trust boundary: no signature status)"
    );
    assert!(
        !debug_lower.contains("trust"),
        "IncRoot should NOT have 'trust' field (trust boundary: no trust levels)"
    );
    assert!(
        !debug_lower.contains("provenance"),
        "IncRoot should NOT have 'provenance' field (trust boundary: no provenance info)"
    );
    assert!(
        !debug_lower.contains("integrity"),
        "IncRoot should NOT have 'integrity' field (trust boundary: no integrity fields)"
    );
}

#[test]
fn snapshot_inc_root_kind_has_exactly_five_variants() {
    // This test documents that IncRootKind has exactly 5 variants
    let variants = [
        IncRootKind::FileLocalLexical,
        IncRootKind::WorkspaceRelative,
        IncRootKind::ExternalAbsolute,
        IncRootKind::InterpreterStartup,
        IncRootKind::RuntimeDerived,
    ];

    // Each should have a distinct Debug representation
    let debug_outputs: Vec<String> = variants.iter().map(|v| format!("{:?}", v)).collect();
    let unique_outputs: std::collections::HashSet<_> = debug_outputs.iter().collect();
    assert_eq!(
        unique_outputs.len(),
        5,
        "IncRootKind should have exactly 5 distinct variants, found {}",
        unique_outputs.len()
    );
}

#[test]
fn snapshot_module_uri_resolution_has_three_variants() {
    // This test documents that ModuleUriResolution has exactly 3 variants
    let variants = [
        ModuleUriResolution::Resolved("test.pm".to_string()),
        ModuleUriResolution::NotFound,
        ModuleUriResolution::TimedOut,
    ];

    // Each should have a distinct Debug representation
    let debug_outputs: Vec<String> = variants.iter().map(|v| format!("{:?}", v)).collect();
    let unique_outputs: std::collections::HashSet<_> = debug_outputs.iter().collect();
    assert_eq!(
        unique_outputs.len(),
        3,
        "ModuleUriResolution should have exactly 3 distinct variants, found {}",
        unique_outputs.len()
    );
}
