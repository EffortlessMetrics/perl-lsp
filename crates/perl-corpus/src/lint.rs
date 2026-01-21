use crate::meta::Section;
use anyhow::{Result, bail};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// Known valid tags (for warnings)
pub const KNOWN_TAGS: &[&str] = &[
    // Core language features
    "regex",
    "regex-code",
    "sets",
    "branch-reset",
    "substitution",
    "transliteration",
    "qw",
    "quote",
    "quote-like",
    "qq",
    "qr",
    "qx",
    "q",
    "heredoc",
    "heredoc-indented",
    "heredoc-backtick",
    // Variables and data types
    "scalar",
    "array",
    "hash",
    "reference",
    "glob",
    "typeglob",
    "my",
    "our",
    "local",
    "state",
    "package",
    "variable",
    "declaration",
    // Control flow
    "if",
    "unless",
    "while",
    "until",
    "for",
    "foreach",
    "given",
    "when",
    "flow",
    "labels",
    "continue",
    "next",
    "last",
    "redo",
    "goto",
    "flipflop",
    "range",
    "ternary",
    // Subroutines and methods
    "sub",
    "subroutine",
    "function",
    "method",
    "signature",
    "prototype",
    "anonymous",
    "closure",
    "return",
    "wantarray",
    // Built-ins and functions
    "builtin",
    "math",
    "string",
    "list",
    "file",
    "filetest",
    "io",
    "pack",
    "unpack",
    "split",
    "join",
    "tr",
    "sort",
    "map",
    "grep",
    "print",
    "say",
    "printf",
    "sprintf",
    "format",
    // Operators
    "operator",
    "arithmetic",
    "comparison",
    "logical",
    "bitwise",
    "assignment",
    "dereference",
    "arrow",
    "smartmatch",
    "binding",
    "range-operator",
    // Pragmas and modules
    "use",
    "require",
    "no",
    "import",
    "pragma",
    "strict",
    "warnings",
    "feature",
    "experimental",
    "version",
    "vstring",
    "constant",
    "bytes",
    "utf8",
    "encoding",
    "charnames",
    "unicode",
    // Object-oriented
    "class",
    "field",
    "method",
    "bless",
    "isa",
    "can",
    "does",
    "overload",
    "constructor",
    "destructor",
    "autoload",
    // Special variables
    "special-var",
    "magic",
    "punctuation-var",
    "english",
    // Modern Perl
    "try",
    "catch",
    "finally",
    "defer",
    "async",
    "await",
    "signatures",
    "postfix",
    "defined-or",
    // I/O and system
    "open",
    "close",
    "pipe",
    "socket",
    "perlio",
    "layers",
    "system",
    "exec",
    "fork",
    "wait",
    "signal",
    "alarm",
    "tie",
    "tied",
    "untie",
    // Miscellaneous
    "pod",
    "comment",
    "annotation",
    "pragma",
    "shebang",
    "do",
    "eval",
    "block",
    "expression",
    "statement",
    "context",
    "list-context",
    "scalar-context",
    "void-context",
    "interpolation",
    "escape",
    "delimiter",
    "error",
    "diagnostic",
    // Testing and debugging
    "test",
    "debug",
    "assertion",
    "invariant",
    // Specific edge cases
    "edge-case",
    "ambiguous",
    "lexer-sensitive",
    "parser-sensitive",
    "error-recovery",
    "incomplete",
    "invalid",
];

/// Known valid flags
pub const KNOWN_FLAGS: &[&str] = &[
    "lexer-sensitive",
    "parser-sensitive",
    "ambiguous",
    "error-node-expected",
    "experimental",
    "version-gated",
    "slow",
    "incomplete",
    "todo",
];

/// Lint configuration
pub struct LintConfig {
    pub max_sections_per_file: usize,
    pub check_unknown_tags: bool,
    pub check_unknown_flags: bool,
    pub require_perl_version: bool,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            max_sections_per_file: 12,
            check_unknown_tags: true,
            check_unknown_flags: true,
            require_perl_version: false,
        }
    }
}

/// Lint result
pub struct LintResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl LintResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Lint corpus sections
pub fn lint(sections: &[Section]) -> Result<()> {
    lint_with_config(sections, &LintConfig::default())
}

/// Lint with custom configuration
pub fn lint_with_config(sections: &[Section], config: &LintConfig) -> Result<()> {
    let result = check_sections(sections, config);

    // Print warnings
    for warning in &result.warnings {
        eprintln!("⚠️  {}", warning);
    }

    // Print errors
    for error in &result.errors {
        eprintln!("❌ {}", error);
    }

    if !result.is_ok() {
        bail!("Linting failed with {} errors", result.errors.len());
    }

    Ok(())
}

/// Check sections and return lint results
pub fn check_sections(sections: &[Section], config: &LintConfig) -> LintResult {
    let mut result = LintResult { errors: Vec::new(), warnings: Vec::new() };

    // Regex for valid ID format
    let id_re = Regex::new(r"^[a-z0-9._-]+$")
        .unwrap_or_else(|_| panic!("ID regex is invalid - this is a bug in the corpus linter"));

    // Track seen IDs for duplicate detection
    let mut seen_ids = BTreeSet::new();

    // Track sections per file
    let mut per_file: BTreeMap<&str, usize> = BTreeMap::new();

    // Convert known tags/flags to sets for fast lookup
    let known_tags: HashSet<&str> = KNOWN_TAGS.iter().copied().collect();
    let known_flags: HashSet<&str> = KNOWN_FLAGS.iter().copied().collect();

    for section in sections {
        // Check ID format
        if section.id.is_empty() {
            result.errors.push(format!("Missing @id in {}: {}", section.file, section.title));
        } else if !id_re.is_match(&section.id) {
            result.errors.push(format!(
                "Invalid @id format '{}' in {}: {} (must match [a-z0-9._-]+)",
                section.id, section.file, section.title
            ));
        }

        // Check for duplicate IDs
        if !section.id.is_empty() && !seen_ids.insert(&section.id) {
            result.errors.push(format!("Duplicate @id '{}' in {}", section.id, section.file));
        }

        // Count sections per file
        *per_file.entry(&section.file).or_default() += 1;

        // Check unknown tags
        if config.check_unknown_tags {
            for tag in &section.tags {
                if !known_tags.contains(tag.as_str()) {
                    result
                        .warnings
                        .push(format!("Unknown tag '{}' in {}: {}", tag, section.file, section.id));
                }
            }
        }

        // Check unknown flags
        if config.check_unknown_flags {
            for flag in &section.flags {
                if !known_flags.contains(flag.as_str()) {
                    result.warnings.push(format!(
                        "Unknown flag '{}' in {}: {}",
                        flag, section.file, section.id
                    ));
                }
            }
        }

        // Check perl version if required
        if config.require_perl_version && section.perl.is_none() {
            result
                .warnings
                .push(format!("Missing @perl version in {}: {}", section.file, section.id));
        }

        // Check for empty body
        if section.body.trim().is_empty() {
            result.warnings.push(format!("Empty body in {}: {}", section.file, section.id));
        }
    }

    // Check sections per file limit
    for (file, count) in per_file {
        if count > config.max_sections_per_file {
            result.warnings.push(format!(
                "File {} has {} sections (exceeds limit of {})",
                file, count, config.max_sections_per_file
            ));
        }
    }

    result
}
