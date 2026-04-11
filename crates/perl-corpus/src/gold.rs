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
}
