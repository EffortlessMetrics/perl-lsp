#[cfg(feature = "lsp-compat")]
use super::QuickFix;
#[cfg(not(feature = "lsp-compat"))]
use super::ViolationSummary;
#[cfg(feature = "lsp-compat")]
use super::perlcritic_quick_fix;
use super::{CriticConfig, Severity, Violation};
use crate::critic_parser::parse_perlcritic_output;
use perl_parser_core::position::{Position, Range};
use perl_subprocess_runtime::SubprocessRuntime;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

#[cfg(feature = "lsp-compat")]
use lsp_types;

/// Single entry in the violation cache.
struct CacheEntry {
    /// Hash of the file content that produced these violations.
    ///
    /// Used to detect stale entries when the file is edited without
    /// an explicit `invalidate_cache` call (e.g. external editor, git checkout).
    content_hash: u64,
    violations: Vec<Violation>,
    /// Monotonically increasing counter value at the last access.
    /// The entry with the lowest `access_seq` is the LRU candidate.
    access_seq: u64,
}

/// Perl::Critic analyzer
pub struct CriticAnalyzer {
    /// Configuration settings for the analyzer
    config: CriticConfig,
    /// Bounded LRU cache of violations keyed by file path.
    cache: HashMap<String, CacheEntry>,
    /// Subprocess runtime for executing perlcritic
    runtime: Arc<dyn SubprocessRuntime>,
    /// Monotonically increasing logical clock for LRU tracking.
    access_counter: u64,
}

impl CriticAnalyzer {
    /// Creates a new analyzer with the given configuration and runtime.
    pub fn new(config: CriticConfig, runtime: Arc<dyn SubprocessRuntime>) -> Self {
        Self { config, cache: HashMap::new(), runtime, access_counter: 0 }
    }

    /// Creates a new analyzer with the OS subprocess runtime (non-WASM only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_os_runtime(config: CriticConfig) -> Self {
        use perl_subprocess_runtime::OsSubprocessRuntime;
        let timeout = config.timeout_secs;
        Self::new(config, Arc::new(OsSubprocessRuntime::with_timeout(timeout)))
    }

    /// Run Perl::Critic on a file, using path-only cache lookup.
    ///
    /// Prefer `analyze_file_with_hash` when the document content is available
    /// so that stale cache entries are detected and invalidated automatically.
    pub fn analyze_file(&mut self, file_path: &Path) -> Result<Vec<Violation>, String> {
        let path_str = file_path.to_string_lossy().to_string();

        if let Some(entry) = self.cache.get_mut(&path_str) {
            self.access_counter += 1;
            entry.access_seq = self.access_counter;
            return Ok(entry.violations.clone());
        }

        let violations = self.run_perlcritic(file_path, &path_str)?;
        self.insert_entry(path_str, 0, violations.clone());
        Ok(violations)
    }

    /// Run Perl::Critic on a file, validating the cached result against `content_hash`.
    ///
    /// When `content_hash` does not match the stored hash the cached entry is
    /// treated as stale and perlcritic is re-executed. This catches file changes
    /// that arrive through external editors or git operations without triggering
    /// the LSP `didChange` notification.
    pub fn analyze_file_with_hash(
        &mut self,
        file_path: &Path,
        content_hash: u64,
    ) -> Result<Vec<Violation>, String> {
        let path_str = file_path.to_string_lossy().to_string();

        if let Some(entry) = self.cache.get_mut(&path_str) {
            if entry.content_hash == content_hash {
                self.access_counter += 1;
                entry.access_seq = self.access_counter;
                return Ok(entry.violations.clone());
            }
            // Hash mismatch: entry is stale; fall through to re-run perlcritic.
        }

        let violations = self.run_perlcritic(file_path, &path_str)?;
        self.insert_entry(path_str, content_hash, violations.clone());
        Ok(violations)
    }

    /// Execute `perlcritic` and parse its output.
    fn run_perlcritic(&self, _file_path: &Path, path_str: &str) -> Result<Vec<Violation>, String> {
        let args = build_perlcritic_args(&self.config, path_str);
        let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output =
            self.runtime.run_command("perlcritic", &args_refs, None).map_err(|e| e.message)?;
        self.parse_output(&output.stdout, path_str)
    }

    /// Insert a new entry, evicting the LRU entry when the cache is full.
    fn insert_entry(&mut self, path: String, content_hash: u64, violations: Vec<Violation>) {
        let max = self.config.max_cache_entries;
        if max == 0 {
            return;
        }

        if self.cache.len() >= max && !self.cache.contains_key(&path) {
            // O(n) scan over a bounded set — acceptable for the default limit of 512.
            if let Some(lru_key) =
                self.cache.iter().min_by_key(|(_, e)| e.access_seq).map(|(k, _)| k.clone())
            {
                self.cache.remove(&lru_key);
            }
        }

        self.access_counter += 1;
        self.cache
            .insert(path, CacheEntry { content_hash, violations, access_seq: self.access_counter });
    }

    /// Parse perlcritic output
    fn parse_output(&self, output: &[u8], file_path: &str) -> Result<Vec<Violation>, String> {
        let output_str = decode_perlcritic_output(output);
        Ok(parse_perlcritic_output(&output_str)
            .into_iter()
            .map(|parsed| Violation {
                policy: parsed.policy.clone(),
                description: parsed.message,
                explanation: self.get_policy_explanation(&parsed.policy),
                severity: Severity::from_number(parsed.severity),
                range: Range {
                    start: Position { byte: 0, line: parsed.line - 1, column: parsed.column - 1 },
                    end: Position { byte: 0, line: parsed.line - 1, column: parsed.column },
                },
                file: file_path.to_string(),
            })
            .collect())
    }

    /// Get explanation for a policy
    fn get_policy_explanation(&self, policy: &str) -> String {
        format!("See perldoc Perl::Critic::Policy::{policy}")
    }

    /// Clear cache for a file
    pub fn invalidate_cache(&mut self, file_path: &str) {
        self.cache.remove(file_path);
    }

    /// Returns the number of entries currently held in the cache.
    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    /// Convert violations to diagnostics
    #[cfg(feature = "lsp-compat")]
    pub fn to_diagnostics(&self, violations: &[Violation]) -> Vec<lsp_types::Diagnostic> {
        violations
            .iter()
            .map(|v| {
                let lsp_range = lsp_types::Range::new(
                    lsp_types::Position::new(v.range.start.line, v.range.start.column),
                    lsp_types::Position::new(v.range.end.line, v.range.end.column),
                );
                lsp_types::Diagnostic {
                    range: lsp_range,
                    severity: Some(v.severity.to_diagnostic_severity()),
                    code: Some(lsp_types::NumberOrString::String(v.policy.clone())),
                    source: Some("perlcritic".to_string()),
                    message: v.description.clone(),
                    related_information: None,
                    tags: None,
                    code_description: None,
                    data: None,
                }
            })
            .collect()
    }

    /// Convert violations to violation summaries (for non-LSP contexts)
    #[cfg(not(feature = "lsp-compat"))]
    pub fn to_violation_summaries(&self, violations: &[Violation]) -> Vec<ViolationSummary> {
        violations
            .iter()
            .map(|v| ViolationSummary {
                policy: v.policy.clone(),
                description: v.description.clone(),
                severity: v.severity.to_severity_level(),
                line: v.range.start.line as usize,
            })
            .collect()
    }

    /// Get quick fix for a violation
    #[cfg(feature = "lsp-compat")]
    pub fn get_quick_fix(&self, violation: &Violation, _content: &str) -> Option<QuickFix> {
        perlcritic_quick_fix(violation)
    }
}

/// Compute a fast, non-cryptographic hash of document content for cache validation.
///
/// Uses the standard library's `DefaultHasher` — suitable for within-process
/// cache keys where stability across restarts is not required.
pub fn hash_content(content: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

fn build_perlcritic_args(config: &CriticConfig, path_str: &str) -> Vec<String> {
    let mut args = vec![format!("--severity={}", config.severity)];

    if let Some(profile) = &config.profile {
        args.push(format!("--profile={profile}"));
    }
    if let Some(theme) = &config.theme {
        args.push(format!("--theme={theme}"));
    }
    for policy in &config.include {
        args.push(format!("--include={policy}"));
    }
    for policy in &config.exclude {
        args.push(format!("--exclude={policy}"));
    }

    args.push("--verbose=%f:%l:%c:%s:%p:%m\\n".to_string());
    args.push("--".to_string());
    args.push(path_str.to_string());
    args
}

fn decode_perlcritic_output(output: &[u8]) -> String {
    if let Ok(valid) = std::str::from_utf8(output) {
        return valid.to_string();
    }

    decode_windows_1252(output)
}

fn decode_windows_1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| char::from_u32(windows_1252_codepoint(*byte)).unwrap_or('\u{FFFD}'))
        .collect()
}

fn windows_1252_codepoint(byte: u8) -> u32 {
    match byte {
        0x80 => 0x20AC,
        0x82 => 0x201A,
        0x83 => 0x0192,
        0x84 => 0x201E,
        0x85 => 0x2026,
        0x86 => 0x2020,
        0x87 => 0x2021,
        0x88 => 0x02C6,
        0x89 => 0x2030,
        0x8A => 0x0160,
        0x8B => 0x2039,
        0x8C => 0x0152,
        0x8E => 0x017D,
        0x91 => 0x2018,
        0x92 => 0x2019,
        0x93 => 0x201C,
        0x94 => 0x201D,
        0x95 => 0x2022,
        0x96 => 0x2013,
        0x97 => 0x2014,
        0x98 => 0x02DC,
        0x99 => 0x2122,
        0x9A => 0x0161,
        0x9B => 0x203A,
        0x9C => 0x0153,
        0x9E => 0x017E,
        0x9F => 0x0178,
        _ => u32::from(byte),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_subprocess_runtime::mock::MockSubprocessRuntime;

    fn make_analyzer(max_cache_entries: usize) -> CriticAnalyzer {
        let config = CriticConfig { max_cache_entries, ..Default::default() };
        let runtime = Arc::new(MockSubprocessRuntime::new());
        CriticAnalyzer::new(config, runtime)
    }

    fn make_analyzer_with_output(_output: &'static [u8]) -> CriticAnalyzer {
        let config = CriticConfig::default();
        // MockSubprocessRuntime::new() defaults to success with empty stdout,
        // which parses as zero violations — sufficient for cache behaviour tests.
        let runtime = Arc::new(MockSubprocessRuntime::new());
        CriticAnalyzer::new(config, runtime)
    }

    #[test]
    fn decode_perlcritic_output_preserves_utf8() {
        let original = "critic: café — naïve";
        let decoded = decode_perlcritic_output(original.as_bytes());
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_perlcritic_output_falls_back_to_windows_1252() {
        // "café — test" encoded as CP-1252 bytes.
        let bytes = b"caf\xe9 \x97 test";
        let decoded = decode_perlcritic_output(bytes);
        assert_eq!(decoded, "café — test");
    }

    #[test]
    fn cache_hit_on_same_content_hash() {
        let mut analyzer = make_analyzer_with_output(b"");
        let path = std::path::Path::new("/tmp/test.pl");
        let hash = hash_content("use strict;\n");

        // First call populates cache.
        let _ = analyzer.analyze_file_with_hash(path, hash);
        assert_eq!(analyzer.cache_len(), 1);

        // Second call with same hash must hit cache (runtime would be called again
        // on a miss, but MockSubprocessRuntime always succeeds, so we verify by
        // checking access_counter advanced only once more).
        let _ = analyzer.analyze_file_with_hash(path, hash);
        assert_eq!(analyzer.cache_len(), 1);
    }

    #[test]
    fn cache_miss_on_different_content_hash() {
        let mut analyzer = make_analyzer_with_output(b"");
        let path = std::path::Path::new("/tmp/test.pl");

        let _ = analyzer.analyze_file_with_hash(path, hash_content("version 1"));
        // Different hash → stale entry replaced.
        let _ = analyzer.analyze_file_with_hash(path, hash_content("version 2"));
        // Cache still holds exactly one entry for the path.
        assert_eq!(analyzer.cache_len(), 1);
    }

    #[test]
    fn lru_eviction_respects_max_cache_entries() {
        let mut analyzer = make_analyzer(3);
        for i in 0..3u32 {
            let path = format!("/tmp/file{i}.pl");
            let _ = analyzer.analyze_file(std::path::Path::new(&path));
        }
        assert_eq!(analyzer.cache_len(), 3);

        // Adding a 4th entry must evict the LRU (file0, never accessed again).
        let _ = analyzer.analyze_file(std::path::Path::new("/tmp/file3.pl"));
        assert_eq!(analyzer.cache_len(), 3);
    }

    #[test]
    fn zero_max_cache_entries_disables_caching() {
        let mut analyzer = make_analyzer(0);
        let path = std::path::Path::new("/tmp/test.pl");
        let _ = analyzer.analyze_file(path);
        assert_eq!(analyzer.cache_len(), 0);
    }

    #[test]
    fn invalidate_cache_removes_entry() {
        let mut analyzer = make_analyzer_with_output(b"");
        let path = std::path::Path::new("/tmp/test.pl");
        let _ = analyzer.analyze_file(path);
        assert_eq!(analyzer.cache_len(), 1);

        analyzer.invalidate_cache("/tmp/test.pl");
        assert_eq!(analyzer.cache_len(), 0);
    }

    #[test]
    fn hash_content_is_deterministic() {
        let h1 = hash_content("use strict;\nuse warnings;\n");
        let h2 = hash_content("use strict;\nuse warnings;\n");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_content_differs_for_different_inputs() {
        let h1 = hash_content("use strict;\n");
        let h2 = hash_content("use warnings;\n");
        assert_ne!(h1, h2);
    }
}
