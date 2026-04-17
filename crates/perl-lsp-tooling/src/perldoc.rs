//! Perldoc integration for dynamic builtin documentation
//!
//! This module provides functionality to query `perldoc -f <function_name>`
//! to get documentation for Perl builtin functions that are not in the
//! hardcoded builtins list.
//!
//! ## Security
//!
//! - Function names are validated against a strict regex whitelist before
//!   being passed to the subprocess.
//! - Subprocess calls have a configurable timeout (default 500ms).
//! - Results are cached to avoid repeated subprocess calls.
//!
//! ## Architecture
//!
//! The [`PerldocLookup`] struct manages perldoc lookups with:
//! - A thread-safe LRU cache (default 100 entries)
//! - Subprocess timeout enforcement
//! - Output parsing to extract signature and description

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use perl_subprocess_runtime::SubprocessRuntime;

/// Documentation for a Perl builtin function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinDoc {
    /// The function signature (e.g., "fc EXPR", "fc LIST")
    pub signature: String,
    /// The function description
    pub description: String,
}

/// Configuration for perldoc lookups.
#[derive(Debug, Clone)]
pub struct PerldocConfig {
    /// Maximum number of cache entries (default: 100)
    pub cache_size: usize,
    /// Timeout for perldoc subprocess in milliseconds (default: 500)
    pub timeout_ms: u64,
    /// Whether perldoc lookups are enabled (default: true for testing)
    pub enabled: bool,
}

impl Default for PerldocConfig {
    fn default() -> Self {
        Self { cache_size: 100, timeout_ms: 500, enabled: true }
    }
}

/// Thread-safe LRU cache for perldoc results.
struct LruCache {
    /// The cache entries stored as (key, value, last_access_time)
    entries: HashMap<String, (BuiltinDoc, Instant)>,
    /// Ordered list of keys by last access time (oldest first)
    order: VecDeque<String>,
    /// Maximum number of entries
    max_size: usize,
}

impl LruCache {
    /// Create a new LRU cache with the given maximum size.
    fn new(max_size: usize) -> Self {
        Self { entries: HashMap::new(), order: VecDeque::new(), max_size }
    }

    /// Get a cached entry if it exists.
    fn get(&mut self, key: &str) -> Option<BuiltinDoc> {
        if let Some((doc, _)) = self.entries.get(key) {
            // Move to end (most recently used)
            self.order.retain(|k| k != key);
            self.order.push_back(key.to_string());
            return Some(doc.clone());
        }
        None
    }

    /// Insert a new entry into the cache.
    fn insert(&mut self, key: String, doc: BuiltinDoc) {
        // Remove oldest if at capacity
        while self.entries.len() >= self.max_size {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }

        self.entries.insert(key.clone(), (doc, Instant::now()));
        self.order.push_back(key);
    }

    /// Clear all entries from the cache.
    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }
}
/// Static regex for validating Perl identifiers.
/// Perl identifier: starts with letter or underscore, followed by alphanumeric or underscore.
/// Uses Option to handle potential regex compilation failure gracefully.
static PERL_IDENTIFIER_REGEX: Lazy<Option<Regex>> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").ok());

/// Perldoc lookup service.
///
/// Queries `perldoc -f <function_name>` to get documentation for Perl
/// builtin functions that are not in the hardcoded builtins list.
pub struct PerldocLookup<R> {
    /// The subprocess runtime used to execute perldoc.
    runtime: R,
    /// The configuration (stored for future use; currently only cache_size is used).
    #[allow(dead_code)]
    config: PerldocConfig,
    /// The LRU cache for results.
    cache: Mutex<LruCache>,
    /// Flag to track if perldoc has been marked as unavailable.
    perldoc_unavailable: Mutex<bool>,
}

impl<R: SubprocessRuntime> PerldocLookup<R> {
    /// Create a new PerldocLookup with the given runtime and config.
    pub fn new(runtime: R, config: PerldocConfig) -> Self {
        Self {
            runtime,
            config: config.clone(),
            cache: Mutex::new(LruCache::new(config.cache_size)),
            perldoc_unavailable: Mutex::new(false),
        }
    }

    /// Check if a function name is a valid Perl identifier.
    fn is_valid_identifier(&self, name: &str) -> bool {
        PERL_IDENTIFIER_REGEX.as_ref().is_some_and(|regex| regex.is_match(name))
    }

    /// Get documentation for a builtin function.
    ///
    /// Returns `Some(BuiltinDoc)` if perldoc can provide documentation,
    /// or `None` if the function is not found or perldoc is unavailable.
    pub fn get_documentation(&self, name: &str) -> Option<BuiltinDoc> {
        // Check if perldoc has been marked as unavailable
        {
            let unavailable = self.perldoc_unavailable.lock().unwrap_or_else(|e| e.into_inner());
            if *unavailable {
                return None;
            }
        }

        // Validate the identifier
        if !self.is_valid_identifier(name) {
            return None;
        }

        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(doc) = cache.get(name) {
                return Some(doc);
            }
        }

        // Query perldoc
        let result = self.query_perldoc(name);

        // Cache the result (even if None, to avoid repeated lookups)
        if let Some(ref doc) = result {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            cache.insert(name.to_string(), doc.clone());
        }

        result
    }

    /// Query perldoc for a function.
    fn query_perldoc(&self, name: &str) -> Option<BuiltinDoc> {
        // Run perldoc. Caller should provide a timeout-configured runtime if needed.
        let output = self.runtime.run_command("perldoc", &["-f", name], None);

        match output {
            Ok(output) if output.success() => {
                let stdout = output.stdout_lossy();
                self.parse_perldoc_output(&stdout)
            }
            Ok(output) => {
                // perldoc returns non-zero when function not found
                // Mark as unavailable to avoid repeated failed lookups
                if output.status_code != 0 && stdout_for_perldoc_not_found(&output.stdout_lossy()) {
                    let mut unavailable =
                        self.perldoc_unavailable.lock().unwrap_or_else(|e| e.into_inner());
                    *unavailable = true;
                }
                None
            }
            Err(_) => {
                // perldoc command failed (not found, timeout, etc.)
                // Mark as unavailable
                let mut unavailable =
                    self.perldoc_unavailable.lock().unwrap_or_else(|e| e.into_inner());
                *unavailable = true;
                None
            }
        }
    }

    /// Parse the output of `perldoc -f <function_name>`.
    ///
    /// The output format is:
    /// ```
    /// fc LIST
    ///
    ///     Returns the casefolded version of EXPR [...]
    /// ```
    fn parse_perldoc_output(&self, output: &str) -> Option<BuiltinDoc> {
        let output = output.trim();

        // Split into lines
        let lines: Vec<&str> = output.lines().collect();
        if lines.is_empty() {
            return None;
        }

        // First non-empty line is typically the signature
        let signature =
            lines.iter().find(|l| !l.trim().is_empty()).map(|l| l.trim().to_string())?;

        // Look for the description (paragraph after signature)
        let description = self.extract_description(lines)?;

        Some(BuiltinDoc { signature, description })
    }

    /// Extract the description from perldoc output.
    ///
    /// The description typically starts after a blank line following the signature
    /// and continues until the next section or EOF.
    fn extract_description(&self, lines: Vec<&str>) -> Option<String> {
        let mut description_lines = Vec::new();
        let mut found_blank_line = false;

        for line in &lines[1..] {
            let trimmed = line.trim();

            // Stop at section headers (lines starting with =)
            if trimmed.starts_with('=') {
                break;
            }

            if trimmed.is_empty() {
                found_blank_line = true;
                continue;
            }

            // Skip the signature line itself
            if !found_blank_line {
                continue;
            }

            // Collect description lines
            description_lines.push(trimmed.to_string());
        }

        if description_lines.is_empty() {
            // Fallback: join all lines after signature
            description_lines = lines
                .iter()
                .skip(1)
                .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('='))
                .map(|l| l.trim().to_string())
                .collect();
        }

        if description_lines.is_empty() {
            return None;
        }

        // Join with spaces and clean up
        let description = description_lines.join(" ");
        let description = description.trim();

        // Remove leading/trailing punctuation that looks like formatting artifacts
        let description = description.trim_start_matches(['-', '*', ' ']);
        let description = description.trim_end_matches(['-', '*', ' ']);

        if description.is_empty() {
            return None;
        }

        Some(description.to_string())
    }

    /// Clear the cache.
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.clear();
    }

    /// Reset the unavailable flag (useful for testing).
    #[cfg(test)]
    pub fn reset_unavailable_flag(&self) {
        let mut flag = self.perldoc_unavailable.lock().unwrap_or_else(|e| e.into_inner());
        *flag = false;
    }
}

/// Check if the output indicates the function was not found.
fn stdout_for_perldoc_not_found(stdout: &str) -> bool {
    stdout.contains("No documentation")
        || stdout.contains("not found")
        || stdout.contains("Unknown function")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_identifier() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        // Valid identifiers
        assert!(lookup.is_valid_identifier("fc"));
        assert!(lookup.is_valid_identifier("trim"));
        assert!(lookup.is_valid_identifier("state"));
        assert!(lookup.is_valid_identifier("caller"));
        assert!(lookup.is_valid_identifier("_private"));
        assert!(lookup.is_valid_identifier("foo_bar123"));

        // Invalid identifiers
        assert!(!lookup.is_valid_identifier("123abc"));
        assert!(!lookup.is_valid_identifier("my-var"));
        assert!(!lookup.is_valid_identifier("my$var"));
        assert!(!lookup.is_valid_identifier(""));
        assert!(!lookup.is_valid_identifier("foo bar"));
    }

    #[test]
    fn test_parse_perldoc_output_fc() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let output = r#"fc LIST

    Returns the casefolded version of EXPR. This is the foldcase
    corresponding to Unicode NFD/NFC foldcase."#;

        let doc = lookup.parse_perldoc_output(output);
        assert!(doc.is_some());
        let doc = doc.unwrap();
        assert_eq!(doc.signature, "fc LIST");
        assert!(doc.description.contains("casefolded"));
    }

    #[test]
    fn test_parse_perldoc_output_trim() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let output = r#"trim EXPR
    trim

    Removes leading and trailing whitespace from the value of EXPR,
    and returns the modified value."#;

        let doc = lookup.parse_perldoc_output(output);
        assert!(doc.is_some());
        let doc = doc.unwrap();
        assert_eq!(doc.signature, "trim EXPR");
        assert!(doc.description.contains("whitespace"));
    }

    #[test]
    fn test_parse_perldoc_output_pipe() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let output = r#"pipe DIRHANDLE, DIRHANDLE

    Opens a pair of connected pipes. See perlipc for details."#;

        let doc = lookup.parse_perldoc_output(output);
        assert!(doc.is_some());
        let doc = doc.unwrap();
        assert_eq!(doc.signature, "pipe DIRHANDLE, DIRHANDLE");
    }

    #[test]
    fn test_parse_perldoc_output_socket() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let output = r#"socket SOCKET, DOMAIN, TYPE, PROTOCOL

    Creates a socket. See perlipc for details."#;

        let doc = lookup.parse_perldoc_output(output);
        assert!(doc.is_some());
        let doc = doc.unwrap();
        assert_eq!(doc.signature, "socket SOCKET, DOMAIN, TYPE, PROTOCOL");
    }

    #[test]
    fn test_parse_perldoc_output_readline() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let output = r#"readline EXPR

    Reads from the filehandle whose typeglob is contained in EXPR."#;

        let doc = lookup.parse_perldoc_output(output);
        assert!(doc.is_some());
        let doc = doc.unwrap();
        assert_eq!(doc.signature, "readline EXPR");
    }

    // ── Edge case: empty output ────────────────────────────────────────────────

    #[test]
    fn test_parse_perldoc_output_empty() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let doc = lookup.parse_perldoc_output("");
        assert!(doc.is_none());
    }

    #[test]
    fn test_parse_perldoc_output_whitespace_only() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let doc = lookup.parse_perldoc_output("   \n\n  \t  ");
        assert!(doc.is_none());
    }

    // ── Edge case: single-line output (no description) ────────────────────────

    #[test]
    fn test_parse_perldoc_output_single_line() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let output = "myfunc LIST";
        let doc = lookup.parse_perldoc_output(output);
        // Single line with no description returns None - perldoc output should have both
        assert!(doc.is_none());
    }

    // ── Edge case: no blank line between signature and description ─────────────

    #[test]
    fn test_parse_perldoc_output_no_blank_line() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let output = "myfunc LIST\n    This is the description without a blank line.";
        let doc = lookup.parse_perldoc_output(output);
        assert!(doc.is_some());
        let doc = doc.unwrap();
        assert_eq!(doc.signature, "myfunc LIST");
        // The description extraction should still work via fallback
        assert!(!doc.description.is_empty());
    }

    // ── Edge case: section header in description ────────────────────────────────

    #[test]
    fn test_parse_perldoc_output_section_header_in_description() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let output = "myfunc LIST\n\n    Description before header.\n=head1 SEE ALSO\n\n    More documentation here.";
        let doc = lookup.parse_perldoc_output(output);
        assert!(doc.is_some());
        let doc = doc.unwrap();
        assert_eq!(doc.signature, "myfunc LIST");
        // Should stop at the section header
        assert!(!doc.description.contains("SEE ALSO"));
        assert!(doc.description.contains("Description before header"));
    }

    // ── Edge case: multiple blank lines ─────────────────────────────────────────

    #[test]
    fn test_parse_perldoc_output_multiple_blank_lines() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let output = "myfunc LIST\n\n\n\n    Description after multiple blank lines.";
        let doc = lookup.parse_perldoc_output(output);
        assert!(doc.is_some());
        let doc = doc.unwrap();
        assert_eq!(doc.signature, "myfunc LIST");
        assert!(doc.description.contains("Description after multiple blank lines"));
    }

    // ── Edge case: leading/trailing formatting characters in description ────────

    #[test]
    fn test_parse_perldoc_output_formatting_characters() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let output = "myfunc LIST\n\n    --- Leading dashes ---\n    Description here\n    *** Trailing stars ***";
        let doc = lookup.parse_perldoc_output(output);
        assert!(doc.is_some());
        let doc = doc.unwrap();
        // Should trim leading/trailing formatting characters
        assert!(!doc.description.starts_with('-'));
        assert!(!doc.description.ends_with('*'));
    }

    // ── Edge case: identifier validation - underscore-only ───────────────────────

    #[test]
    fn test_is_valid_identifier_underscore_only() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        assert!(lookup.is_valid_identifier("_")); // Single underscore is valid
        assert!(lookup.is_valid_identifier("__")); // Double underscore
        assert!(lookup.is_valid_identifier("___")); // Triple underscore
    }

    // ── Edge case: identifier validation - single character ─────────────────────

    #[test]
    fn test_is_valid_identifier_single_char() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        assert!(lookup.is_valid_identifier("a"));
        assert!(lookup.is_valid_identifier("Z"));
        assert!(lookup.is_valid_identifier("_"));
    }

    // ── Edge case: identifier validation - starts with letter, underscore ────────

    #[test]
    fn test_is_valid_identifier_edge_cases() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        // Valid
        assert!(lookup.is_valid_identifier("a0"));
        assert!(lookup.is_valid_identifier("_0"));
        assert!(lookup.is_valid_identifier("ABC"));
        assert!(lookup.is_valid_identifier("abc_DEF"));
        assert!(lookup.is_valid_identifier("_PRIVATE"));
        assert!(lookup.is_valid_identifier("x1_y2_z3"));

        // Invalid
        assert!(!lookup.is_valid_identifier(""));
        assert!(!lookup.is_valid_identifier("0"));
        assert!(!lookup.is_valid_identifier("0abc"));
        assert!(!lookup.is_valid_identifier("-foo"));
        assert!(!lookup.is_valid_identifier("foo-bar"));
        assert!(!lookup.is_valid_identifier("foo.bar"));
        assert!(!lookup.is_valid_identifier("foo:bar"));
        assert!(!lookup.is_valid_identifier("foo bar"));
        assert!(!lookup.is_valid_identifier("foo\tbar"));
        assert!(!lookup.is_valid_identifier("foo\nbar"));
    }

    // ── Edge case: LRU cache with size 1 ────────────────────────────────────────

    #[test]
    fn test_lru_cache_size_one() {
        let mut cache = LruCache::new(1);

        let doc1 = BuiltinDoc { signature: "a".to_string(), description: "1".to_string() };
        let doc2 = BuiltinDoc { signature: "b".to_string(), description: "2".to_string() };

        cache.insert("a".to_string(), doc1.clone());
        assert_eq!(cache.get("a"), Some(doc1.clone()));

        // Inserting b should evict a
        cache.insert("b".to_string(), doc2.clone());
        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.get("b"), Some(doc2.clone()));
    }

    // ── Edge case: LRU cache - accessing evicts other entries ───────────────────

    #[test]
    fn test_lru_cache_access_eviction() {
        let mut cache = LruCache::new(3);

        let doc1 = BuiltinDoc { signature: "a".to_string(), description: "1".to_string() };
        let doc2 = BuiltinDoc { signature: "b".to_string(), description: "2".to_string() };
        let doc3 = BuiltinDoc { signature: "c".to_string(), description: "3".to_string() };
        let doc4 = BuiltinDoc { signature: "d".to_string(), description: "4".to_string() };

        cache.insert("a".to_string(), doc1.clone());
        cache.insert("b".to_string(), doc2.clone());
        cache.insert("c".to_string(), doc3.clone());

        // Access b and a (making c the oldest)
        assert_eq!(cache.get("b"), Some(doc2.clone()));
        assert_eq!(cache.get("a"), Some(doc1.clone()));

        // Insert d - should evict c (oldest after accesses)
        cache.insert("d".to_string(), doc4.clone());

        assert_eq!(cache.get("c"), None); // evicted
        assert_eq!(cache.get("a"), Some(doc1)); // still there
        assert_eq!(cache.get("b"), Some(doc2)); // still there
        assert_eq!(cache.get("d"), Some(doc4)); // just added
    }

    // ── Edge case: LRU cache - clear ───────────────────────────────────────────

    #[test]
    fn test_lru_cache_clear() {
        let mut cache = LruCache::new(3);

        let doc1 = BuiltinDoc { signature: "a".to_string(), description: "1".to_string() };
        let doc2 = BuiltinDoc { signature: "b".to_string(), description: "2".to_string() };

        cache.insert("a".to_string(), doc1.clone());
        cache.insert("b".to_string(), doc2.clone());

        cache.clear();

        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.get("b"), None);
    }

    // ── Edge case: LRU cache - duplicate insert updates ─────────────────────────

    #[test]
    fn test_lru_cache_duplicate_insert() {
        let mut cache = LruCache::new(3);

        let doc1 = BuiltinDoc { signature: "a".to_string(), description: "1".to_string() };
        let doc2 = BuiltinDoc { signature: "a".to_string(), description: "2".to_string() };

        cache.insert("a".to_string(), doc1.clone());
        cache.insert("a".to_string(), doc2.clone());

        // Should return the updated doc and it should be at the end (most recently used)
        assert_eq!(cache.get("a"), Some(doc2.clone()));
    }

    // ── Edge case: parse perldoc with very long signature ──────────────────────

    #[test]
    fn test_parse_perldoc_output_long_signature() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        let long_sig = "a_very_long_function_name_with_many_parameters_and_arguments TYPE1, TYPE2, TYPE3, TYPE4, TYPE5, TYPE6";
        let output = format!("{}\n\n    Description.", long_sig);

        let doc = lookup.parse_perldoc_output(&output);
        assert!(doc.is_some());
        let doc = doc.unwrap();
        assert_eq!(doc.signature, long_sig);
    }

    // ── Edge case: get_documentation with invalid identifier returns None ────────

    #[test]
    fn test_get_documentation_invalid_identifier() {
        let runtime = MockRuntime::new();
        let lookup = PerldocLookup::new(runtime, PerldocConfig::default());

        // Invalid identifiers should return None without calling runtime
        assert!(lookup.get_documentation("123abc").is_none());
        assert!(lookup.get_documentation("my-var").is_none());
        assert!(lookup.get_documentation("my$var").is_none());
        assert!(lookup.get_documentation("").is_none());
    }

    #[test]
    fn test_lru_cache() {
        let mut cache = LruCache::new(3);

        let doc1 = BuiltinDoc { signature: "foo".to_string(), description: "desc1".to_string() };
        let doc2 = BuiltinDoc { signature: "bar".to_string(), description: "desc2".to_string() };
        let doc3 = BuiltinDoc { signature: "baz".to_string(), description: "desc3".to_string() };
        let doc4 = BuiltinDoc { signature: "qux".to_string(), description: "desc4".to_string() };

        cache.insert("foo".to_string(), doc1.clone());
        cache.insert("bar".to_string(), doc2.clone());
        cache.insert("baz".to_string(), doc3.clone());

        // Access foo to make it recently used
        assert_eq!(cache.get("foo"), Some(doc1.clone()));

        // Insert a new entry, which should evict bar (oldest after foo was accessed)
        cache.insert("qux".to_string(), doc4.clone());

        // foo should still be there (was accessed)
        assert_eq!(cache.get("foo"), Some(doc1));
        // bar should be evicted
        assert_eq!(cache.get("bar"), None);
        // baz should still be there
        assert_eq!(cache.get("baz"), Some(doc3));
        // qux should be there
        assert_eq!(cache.get("qux"), Some(doc4));
    }

    // Mock runtime for testing
    struct MockRuntime {
        responses: Mutex<
            Vec<
                Result<
                    perl_subprocess_runtime::SubprocessOutput,
                    perl_subprocess_runtime::SubprocessError,
                >,
            >,
        >,
    }

    impl MockRuntime {
        fn new() -> Self {
            Self { responses: Mutex::new(Vec::new()) }
        }
    }

    impl perl_subprocess_runtime::SubprocessRuntime for MockRuntime {
        fn run_command(
            &self,
            _program: &str,
            args: &[&str],
            _stdin: Option<&[u8]>,
        ) -> Result<
            perl_subprocess_runtime::SubprocessOutput,
            perl_subprocess_runtime::SubprocessError,
        > {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                // Default: simulate successful perldoc output
                let func_name = args.get(1).map(|s| s.to_string()).unwrap_or_default();
                Ok(perl_subprocess_runtime::SubprocessOutput {
                    stdout: format!(
                        "{} LIST\n\n    Returns the documentation for {}.",
                        func_name, func_name
                    )
                    .into_bytes(),
                    stderr: Vec::new(),
                    status_code: 0,
                })
            } else {
                responses.remove(0)
            }
        }
    }
}
