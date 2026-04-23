use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Assertion type for gold corpus diagnostics expectations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "assertion")]
pub enum GoldAssertion {
    /// No diagnostics should be emitted for this fixture
    #[serde(rename = "no_diagnostics")]
    NoDiagnostics,

    /// A specific diagnostic should NOT be present
    #[serde(rename = "no_diagnostic")]
    NoDiagnostic { code: String },

    /// A diagnostic with the given code should be present
    #[serde(rename = "diagnostic_present")]
    DiagnosticPresent {
        code: String,
        #[serde(default)]
        byte_offset: Option<usize>,
        #[serde(default)]
        message_contains: Option<String>,
    },

    /// Expect exactly N diagnostics with the given code
    #[serde(rename = "diagnostic_count")]
    DiagnosticCount { code: String, count: usize },
}

/// Expected diagnostics for a gold corpus fixture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldExpected {
    pub diagnostics: Vec<GoldAssertion>,
}

/// A gold corpus test fixture with expected results
#[derive(Debug, Clone)]
pub struct GoldFixture {
    pub name: String,
    pub fixture_path: PathBuf,
    pub expected: GoldExpected,
}

/// Load a single gold fixture from a directory
///
/// Expects a directory with:
/// - `fixture.pl` — the Perl source code to test
/// - `expected.json` — JSON file with GoldExpected assertions
pub fn load_gold_fixture<P: AsRef<Path>>(
    dir: P,
) -> Result<GoldFixture, Box<dyn std::error::Error>> {
    let dir = dir.as_ref();
    let name = dir.file_name().ok_or("No directory name")?.to_string_lossy().to_string();

    let fixture_path = dir.join("fixture.pl");
    let expected_path = dir.join("expected.json");

    if !fixture_path.exists() {
        return Err(format!("fixture.pl not found in {}", dir.display()).into());
    }
    if !expected_path.exists() {
        return Err(format!("expected.json not found in {}", dir.display()).into());
    }

    let expected_json = fs::read_to_string(&expected_path)?;
    let expected: GoldExpected = serde_json::from_str(&expected_json)?;

    Ok(GoldFixture { name, fixture_path, expected })
}

/// Load all gold fixtures from a directory
///
/// Walks the directory and loads all subdirectories as fixtures.
pub fn load_gold_fixtures<P: AsRef<Path>>(
    root: P,
) -> Result<Vec<GoldFixture>, Box<dyn std::error::Error>> {
    load_gold_fixtures_from(root)
}

/// Load all gold fixtures from a directory
///
/// Walks the directory and loads all subdirectories as fixtures.
pub fn load_gold_fixtures_from<P: AsRef<Path>>(
    root: P,
) -> Result<Vec<GoldFixture>, Box<dyn std::error::Error>> {
    let root = root.as_ref();
    let mut fixtures = Vec::new();

    if !root.exists() {
        return Err(format!("Gold fixtures directory not found: {}", root.display()).into());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            match load_gold_fixture(&path) {
                Ok(fixture) => fixtures.push(fixture),
                Err(e) => {
                    // Use tracing instead of eprintln
                    tracing::warn!("Failed to load fixture from {}: {}", path.display(), e);
                }
            }
        }
    }

    // Sort by name for consistent ordering
    fixtures.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(fixtures)
}

// ---------------------------------------------------------------------------
// Editor Intelligence — Hover
// ---------------------------------------------------------------------------

/// Assertion kind for hover gold corpus entries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HoverAssertionKind {
    /// Response must have non-null, non-empty content
    HoverNonNull,
    /// Response must be null (no hover available)
    HoverNull,
    /// Response content must contain `needle` (case-sensitive)
    HoverContains { needle: String },
    /// Response content must NOT contain `needle`
    HoverAbsent { needle: String },
}

/// A single hover assertion at a given (line, character) position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverAssertion {
    #[serde(flatten)]
    pub kind: HoverAssertionKind,
    pub line: u32,
    pub character: u32,
    #[serde(default)]
    pub rationale: String,
}

/// On-disk representation of `expected_hover.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverGoldExpected {
    pub version: u32,
    pub fixture: String,
    pub assertions: Vec<HoverAssertion>,
}

/// A hover gold fixture (fixture.pl + expected_hover.json)
#[derive(Debug, Clone)]
pub struct HoverGoldFixture {
    pub name: String,
    pub fixture_path: PathBuf,
    pub hover_assertions: Vec<HoverAssertion>,
}

/// Load all hover gold fixtures from a directory.
///
/// Walks the directory and loads all subdirectories that contain both
/// `fixture.pl` and `expected_hover.json`. Directories that lack
/// `expected_hover.json` are silently skipped.
pub fn load_hover_gold_fixtures<P: AsRef<Path>>(
    root: P,
) -> Result<Vec<HoverGoldFixture>, Box<dyn std::error::Error>> {
    let root = root.as_ref();
    let mut fixtures = Vec::new();

    if !root.exists() {
        return Err(format!("Gold fixtures directory not found: {}", root.display()).into());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let fixture_path = path.join("fixture.pl");
        let hover_path = path.join("expected_hover.json");

        if !fixture_path.exists() || !hover_path.exists() {
            continue; // not a hover fixture
        }

        let name = path.file_name().ok_or("No directory name")?.to_string_lossy().to_string();
        let json = fs::read_to_string(&hover_path)?;
        let expected: HoverGoldExpected = serde_json::from_str(&json)
            .map_err(|e| format!("Parsing {}: {e}", hover_path.display()))?;

        fixtures.push(HoverGoldFixture {
            name,
            fixture_path,
            hover_assertions: expected.assertions,
        });
    }

    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(fixtures)
}

// ---------------------------------------------------------------------------
// Editor Intelligence — Goto-Definition
// ---------------------------------------------------------------------------

/// Assertion kind for goto-definition gold corpus entries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum GotoAssertionKind {
    /// Response must return at least one location
    GotoNonNull,
    /// Response must return no locations
    GotoNull,
    /// The first returned location must be on the given line
    GotoLine { expected_line: u32 },
}

/// A single goto-definition assertion at a given (line, character) position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GotoAssertion {
    #[serde(flatten)]
    pub kind: GotoAssertionKind,
    pub line: u32,
    pub character: u32,
    #[serde(default)]
    pub rationale: String,
}

/// On-disk representation of `expected_goto.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GotoGoldExpected {
    pub version: u32,
    pub fixture: String,
    pub assertions: Vec<GotoAssertion>,
}

/// A goto-definition gold fixture (fixture.pl + expected_goto.json)
#[derive(Debug, Clone)]
pub struct GotoGoldFixture {
    pub name: String,
    pub fixture_path: PathBuf,
    pub goto_assertions: Vec<GotoAssertion>,
}

/// Load all goto-definition gold fixtures from a directory.
///
/// Silently skips directories that lack `expected_goto.json`.
pub fn load_goto_gold_fixtures<P: AsRef<Path>>(
    root: P,
) -> Result<Vec<GotoGoldFixture>, Box<dyn std::error::Error>> {
    let root = root.as_ref();
    let mut fixtures = Vec::new();

    if !root.exists() {
        return Err(format!("Gold fixtures directory not found: {}", root.display()).into());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let fixture_path = path.join("fixture.pl");
        let goto_path = path.join("expected_goto.json");

        if !fixture_path.exists() || !goto_path.exists() {
            continue;
        }

        let name = path.file_name().ok_or("No directory name")?.to_string_lossy().to_string();
        let json = fs::read_to_string(&goto_path)?;
        let expected: GotoGoldExpected = serde_json::from_str(&json)
            .map_err(|e| format!("Parsing {}: {e}", goto_path.display()))?;

        fixtures.push(GotoGoldFixture { name, fixture_path, goto_assertions: expected.assertions });
    }

    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(fixtures)
}

// ---------------------------------------------------------------------------
// Editor Intelligence — Completion
// ---------------------------------------------------------------------------

/// Assertion kind for completion gold corpus entries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CompletionAssertionKind {
    /// Completion list must not be empty
    CompletionNonEmpty,
    /// Item must be at position [0] in the list
    CompletionTop1 { expected_label: String },
    /// Item must be in positions [0..4]
    CompletionTop5 { expected_label: String },
    /// Item must appear anywhere in the list
    CompletionPresent { expected_label: String },
    /// Label must NOT appear in the list
    CompletionNoiseAbsent { forbidden_label: String },
}

/// A single completion assertion at a given (line, character) position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionAssertion {
    #[serde(flatten)]
    pub kind: CompletionAssertionKind,
    pub line: u32,
    pub character: u32,
    #[serde(default)]
    pub rationale: String,
}

/// On-disk representation of `expected_completion.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionGoldExpected {
    pub version: u32,
    pub fixture: String,
    pub assertions: Vec<CompletionAssertion>,
}

/// A completion gold fixture (fixture.pl + expected_completion.json)
#[derive(Debug, Clone)]
pub struct CompletionGoldFixture {
    pub name: String,
    pub fixture_path: PathBuf,
    pub completion_assertions: Vec<CompletionAssertion>,
}

/// Load all completion gold fixtures from a directory.
///
/// Silently skips directories that lack `expected_completion.json`.
pub fn load_completion_gold_fixtures<P: AsRef<Path>>(
    root: P,
) -> Result<Vec<CompletionGoldFixture>, Box<dyn std::error::Error>> {
    let root = root.as_ref();
    let mut fixtures = Vec::new();

    if !root.exists() {
        return Err(format!("Gold fixtures directory not found: {}", root.display()).into());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let fixture_path = path.join("fixture.pl");
        let completion_path = path.join("expected_completion.json");

        if !fixture_path.exists() || !completion_path.exists() {
            continue;
        }

        let name = path.file_name().ok_or("No directory name")?.to_string_lossy().to_string();
        let json = fs::read_to_string(&completion_path)?;
        let expected: CompletionGoldExpected = serde_json::from_str(&json)
            .map_err(|e| format!("Parsing {}: {e}", completion_path.display()))?;

        fixtures.push(CompletionGoldFixture {
            name,
            fixture_path,
            completion_assertions: expected.assertions,
        });
    }

    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(fixtures)
}

// ---------------------------------------------------------------------------
// Editor Intelligence — Document Symbols (STUBS — RED TEST REQUIRED)
// ---------------------------------------------------------------------------

/// Assertion kind for document symbols gold corpus entries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SymbolsAssertionKind {
    /// Document symbols list must not be empty
    SymbolsNonEmpty,
    /// Document symbols list must be empty
    SymbolsEmpty,
    /// A symbol with the given name must be present
    SymbolsContains { name: String },
    /// A symbol with the given name must NOT be present
    SymbolsAbsent { name: String },
    /// A symbol with the given name must have the expected LSP SymbolKind
    SymbolsKindMatches { name: String, expected_kind: i64 },
    /// Document must have at least N top-level symbols
    SymbolsCountAtLeast { min: usize },
    /// Document must have at most N top-level symbols
    SymbolsCountAtMost { max: usize },
    /// A package symbol with the given name must be present (kind 4 or 2)
    SymbolsPackagePresent { name: String },
    /// A subroutine/function symbol with the given name must be present (kind 12)
    SymbolsSubroutinePresent { name: String },
    /// A variable symbol with the given name must be present (kind 13)
    SymbolsVariablePresent { name: String },
}

/// A single document symbols assertion (no position needed — whole-document query)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(expecting = "a document symbols assertion")]
pub struct SymbolsAssertion {
    #[serde(flatten)]
    pub kind: SymbolsAssertionKind,
}

/// On-disk representation of `expected_symbols.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolsGoldExpected {
    pub version: u32,
    pub fixture: String,
    pub assertions: Vec<SymbolsAssertion>,
}

/// A document symbols gold fixture (fixture.pl + expected_symbols.json)
#[derive(Debug, Clone)]
pub struct SymbolsGoldFixture {
    pub name: String,
    pub fixture_path: PathBuf,
    pub symbols_assertions: Vec<SymbolsAssertion>,
}

/// Load all document symbols gold fixtures from a directory.
///
/// Silently skips directories that lack `expected_symbols.json`.
pub fn load_symbols_gold_fixtures<P: AsRef<Path>>(
    root: P,
) -> Result<Vec<SymbolsGoldFixture>, Box<dyn std::error::Error>> {
    let root = root.as_ref();
    let mut fixtures = Vec::new();

    if !root.exists() {
        return Err(format!("Gold fixtures directory not found: {}", root.display()).into());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let fixture_path = path.join("fixture.pl");
        let symbols_path = path.join("expected_symbols.json");

        if !fixture_path.exists() || !symbols_path.exists() {
            continue;
        }

        let name = path.file_name().ok_or("No directory name")?.to_string_lossy().to_string();
        let json = fs::read_to_string(&symbols_path)?;
        let expected: SymbolsGoldExpected = serde_json::from_str(&json)
            .map_err(|e| format!("Parsing {}: {e}", symbols_path.display()))?;

        fixtures.push(SymbolsGoldFixture {
            name,
            fixture_path,
            symbols_assertions: expected.assertions,
        });
    }

    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(fixtures)
}

// ---------------------------------------------------------------------------
// Editor Intelligence — Rename (STUBS — RED TEST REQUIRED)
// ---------------------------------------------------------------------------

/// Assertion kind for rename gold corpus entries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RenameAssertionKind {
    /// Rename must return non-null WorkspaceEdit for renamable symbols
    RenameNonNull,
    /// Rename must return null for non-renamable positions
    RenameNull,
    /// prepareRename must return a valid range at renamable positions
    RenamePrepareValid,
    /// prepareRename must return null at non-renamable positions
    RenamePrepareNull,
    /// WorkspaceEdit changes must contain the expected text
    RenameChangesContainText { expected_text: String },
    /// WorkspaceEdit must touch the expected number of distinct file URIs
    RenameChangesMatchCount { expected_count: usize },
}

/// A single rename assertion at a given (line, character) position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameAssertion {
    #[serde(flatten)]
    pub kind: RenameAssertionKind,
    pub line: u32,
    pub character: u32,
    pub new_name: String,
    #[serde(default)]
    pub rationale: String,
}

/// On-disk representation of `expected_rename.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameGoldExpected {
    pub version: u32,
    pub fixture: String,
    pub assertions: Vec<RenameAssertion>,
}

/// A rename gold fixture (fixture.pl + expected_rename.json)
#[derive(Debug, Clone)]
pub struct RenameGoldFixture {
    pub name: String,
    pub fixture_path: PathBuf,
    pub rename_assertions: Vec<RenameAssertion>,
}

/// Load all rename gold fixtures from a directory.
///
/// Silently skips directories that lack `expected_rename.json`.
pub fn load_rename_gold_fixtures<P: AsRef<Path>>(
    root: P,
) -> Result<Vec<RenameGoldFixture>, Box<dyn std::error::Error>> {
    let root = root.as_ref();
    let mut fixtures = Vec::new();

    if !root.exists() {
        return Err(format!("Gold fixtures directory not found: {}", root.display()).into());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let fixture_path = path.join("fixture.pl");
        let rename_path = path.join("expected_rename.json");

        if !fixture_path.exists() || !rename_path.exists() {
            continue;
        }

        let name = path.file_name().ok_or("No directory name")?.to_string_lossy().to_string();
        let json = fs::read_to_string(&rename_path)?;
        let expected: RenameGoldExpected = serde_json::from_str(&json)
            .map_err(|e| format!("Parsing {}: {e}", rename_path.display()))?;

        fixtures.push(RenameGoldFixture {
            name,
            fixture_path,
            rename_assertions: expected.assertions,
        });
    }

    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(fixtures)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gold_assertion_deserialization() {
        let json = r#"{"assertion": "no_diagnostics"}"#;
        let assertion: GoldAssertion = serde_json::from_str(json).unwrap();
        assert!(matches!(assertion, GoldAssertion::NoDiagnostics));
    }

    #[test]
    fn test_gold_assertion_diagnostic_present() {
        let json = r#"{"assertion": "diagnostic_present", "code": "PL100", "byte_offset": 24}"#;
        let assertion: GoldAssertion = serde_json::from_str(json).unwrap();
        assert!(
            matches!(
                &assertion,
                GoldAssertion::DiagnosticPresent {
                    code,
                    byte_offset: Some(24),
                    ..
                } if code == "PL100"
            ),
            "Expected DiagnosticPresent variant with code PL100 and byte_offset 24"
        );
    }

    #[test]
    fn test_gold_expected_deserialization() {
        let json = r#"{"diagnostics": [{"assertion": "no_diagnostics"}]}"#;
        let expected: GoldExpected = serde_json::from_str(json).unwrap();
        assert_eq!(expected.diagnostics.len(), 1);
    }

    #[test]
    fn test_malformed_json_returns_error() {
        // Malformed expected.json must produce a serde error, not panic
        let bad_json = r#"{"diagnostics": [{"assertion": "unknown_variant"}]}"#;
        let result: Result<GoldExpected, _> = serde_json::from_str(bad_json);
        assert!(result.is_err(), "unknown assertion variant should fail to deserialize");
    }

    #[test]
    fn test_empty_diagnostics_array_deserializes() {
        // empty array is valid JSON but the harness will catch it as vacuous
        let json = r#"{"diagnostics": []}"#;
        let expected: GoldExpected = serde_json::from_str(json).unwrap();
        assert_eq!(expected.diagnostics.len(), 0);
    }

    #[test]
    fn test_no_diagnostic_deserialization() {
        let json = r#"{"assertion": "no_diagnostic", "code": "PL100"}"#;
        let assertion: GoldAssertion = serde_json::from_str(json).unwrap();
        assert!(
            matches!(&assertion, GoldAssertion::NoDiagnostic { code } if code == "PL100"),
            "Expected NoDiagnostic variant with code PL100"
        );
    }

    #[test]
    fn test_diagnostic_count_deserialization() {
        let json = r#"{"assertion": "diagnostic_count", "code": "PL001", "count": 3}"#;
        let assertion: GoldAssertion = serde_json::from_str(json).unwrap();
        assert!(
            matches!(&assertion, GoldAssertion::DiagnosticCount { code, count: 3 } if code == "PL001"),
            "Expected DiagnosticCount variant with code PL001 and count 3"
        );
    }
}
