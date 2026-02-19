# Edge Case Implementation Plan

## Priority 1: Dynamic Delimiters & BEGIN Blocks (Most Common)

These represent the most common "unparseable" patterns in real Perl code.

### Dynamic Delimiter Recovery System

```rust
// In enhanced_anti_pattern_detector.rs
pub struct DynamicDelimiterRecovery {
    // Track dynamic delimiter variables
    delimiter_vars: HashMap<String, Vec<PossibleValue>>,
    // Recovery strategies
    recovery_mode: RecoveryMode,
}

pub enum RecoveryMode {
    Conservative,    // Mark as unparseable
    BestGuess,      // Try common patterns
    Interactive,    // Ask user for hint
    Sandbox,        // Execute in container
}

impl DynamicDelimiterRecovery {
    pub fn analyze_dynamic_delimiter(&self, var_name: &str, context: &ParseContext) 
        -> DelimiterAnalysis {
        // 1. Check if variable is assigned a literal nearby
        // 2. Look for common patterns (EOF, END, etc.)
        // 3. Track data flow if possible
        // 4. Return best guess or "unknown"
    }
}
```

### Phase-Aware Parsing for BEGIN/CHECK

```rust
#[derive(Debug, Clone)]
pub enum PerlPhase {
    Compile,      // Normal parsing
    Begin,        // BEGIN block
    Check,        // CHECK block
    Init,         // INIT block
    Runtime,      // Normal runtime
    End,          // END block
}

pub struct PhaseAwareParser {
    current_phase: PerlPhase,
    phase_stack: Vec<PerlPhase>,
    deferred_heredocs: Vec<DeferredHeredoc>,
}

impl PhaseAwareParser {
    pub fn enter_phase(&mut self, phase: PerlPhase) {
        self.phase_stack.push(self.current_phase.clone());
        self.current_phase = phase;
    }
    
    pub fn handle_phase_heredoc(&mut self, heredoc: &HeredocDecl) -> PhaseAction {
        match self.current_phase {
            PerlPhase::Begin => PhaseAction::Defer {
                reason: "BEGIN-time heredoc may have side effects",
                severity: Severity::Warning,
            },
            PerlPhase::Runtime => PhaseAction::Parse,
            _ => PhaseAction::PartialParse,
        }
    }
}
```

## Priority 2: Source Filters & Encoding (Less Common)

### Source Filter Detection

```rust
pub struct SourceFilterDetector {
    known_filters: HashSet<&'static str>,
    filter_patterns: Vec<Regex>,
}

impl SourceFilterDetector {
    pub fn new() -> Self {
        let mut known = HashSet::new();
        known.insert("Filter::Simple");
        known.insert("Filter::Util::Call");
        known.insert("Switch");  // The infamous Switch.pm
        
        Self {
            known_filters: known,
            filter_patterns: vec![
                Regex::new(r"use\s+Filter::").unwrap(),
                Regex::new(r"use\s+(\w+).*\bfilter\b").unwrap(),
            ],
        }
    }
    
    pub fn scan_for_filters(&self, code: &str) -> Vec<FilterWarning> {
        let mut warnings = Vec::new();
        
        for line in code.lines() {
            if self.filter_patterns.iter().any(|p| p.is_match(line)) {
                warnings.push(FilterWarning {
                    message: "Source filter detected - parsing may be unreliable",
                    suggestion: "Consider refactoring to avoid source filters",
                    severity: Severity::Warning,
                });
            }
        }
        
        warnings
    }
}
```

### Encoding-Aware Lexer

```rust
pub struct EncodingContext {
    current_encoding: String,
    encoding_stack: Vec<(String, usize)>, // (encoding, line_number)
}

impl EncodingContext {
    pub fn handle_encoding_pragma(&mut self, pragma: &str, line: usize) {
        if let Some(encoding) = Self::parse_encoding_pragma(pragma) {
            self.encoding_stack.push((self.current_encoding.clone(), line));
            self.current_encoding = encoding;
        }
    }
    
    pub fn normalize_for_delimiter(&self, text: &str) -> Result<String, EncodingError> {
        // Convert from current encoding to UTF-8 for consistent matching
        match self.current_encoding.as_str() {
            "utf8" | "UTF-8" => Ok(text.to_string()),
            "latin1" => self.latin1_to_utf8(text),
            _ => Err(EncodingError::UnsupportedEncoding(self.current_encoding.clone())),
        }
    }
}
```

## Priority 3: Advanced Features (Rare)

### Tied Handle Detection

```rust
pub fn detect_tied_handles(code: &str) -> Vec<TiedHandleWarning> {
    let tie_pattern = Regex::new(r"tie\s*\(\s*\*?(\w+)").unwrap();
    let mut tied_handles = HashSet::new();
    
    for cap in tie_pattern.captures_iter(code) {
        if let Some(handle) = cap.get(1) {
            tied_handles.insert(handle.as_str().to_string());
        }
    }
    
    // Later, when we see heredocs to these handles, warn
    tied_handles.into_iter().map(|h| TiedHandleWarning {
        handle: h,
        message: "Heredoc to tied handle - I/O behavior unknown",
    }).collect()
}
```

## Implementation Order

1. **Week 1-2**: Dynamic delimiter recovery + Phase-aware parsing
2. **Week 3**: Source filter detection + Basic encoding support
3. **Week 4**: Tied handles + Initial warning system
4. **Future**: Regex sub-parser, workspace parsing, plugin architecture

## User Experience Design

### Warning Levels

1. **Error**: Dynamic delimiters without fallback
2. **Warning**: BEGIN heredocs, source filters, encoding changes
3. **Info**: Tied handles, custom syntax hints

### Recovery Options

```rust
pub enum UserAction {
    // Safe by default
    IgnoreAndContinue,
    MarkAsUnparseable,
    
    // Requires user consent
    ProvideHint(String),           // "The delimiter is usually 'EOF'"
    EnableBestGuess,               // Use heuristics
    
    // Requires explicit opt-in
    RunInSandbox,                  // Execute code safely
    LoadPlugin(String),            // Custom parser extension
}
```

## Testing Strategy

1. **Corpus of Edge Cases**: Build test suite from real CPAN modules
2. **Regression Tests**: Ensure we don't break normal heredocs
3. **Performance Tests**: Measure overhead of new features
4. **User Studies**: Get feedback on warning messages

## Documentation

1. **User Guide**: "Understanding Parser Warnings"
2. **Developer Guide**: "Extending the Parser"
3. **Best Practices**: "Writing Parser-Friendly Perl"