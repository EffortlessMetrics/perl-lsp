# Anti-Pattern Detection Strategy for Heredoc Edge Cases

## Philosophy: Understand to Fix

As a code understanding tool, our goal is not to execute problematic code, but to:
1. **Detect** anti-patterns and edge cases
2. **Parse** as much as possible around them
3. **Annotate** the AST with warnings and insights
4. **Suggest** fixes and refactoring opportunities

## Implementation Strategy

### 1. Detection Layer

```rust
#[derive(Debug, Clone)]
pub enum AntiPattern {
    FormatHeredoc {
        location: Location,
        format_name: String,
        heredoc_delimiter: String,
    },
    BeginTimeHeredoc {
        location: Location,
        side_effects: Vec<String>,
    },
    SourceFilterHeredoc {
        location: Location,
        filter_module: String,
    },
    DynamicHeredocDelimiter {
        location: Location,
        expression: String,
    },
    RegexCodeBlockHeredoc {
        location: Location,
        pattern: String,
    },
}

pub struct AntiPatternDetector {
    patterns: Vec<AntiPattern>,
    severity_map: HashMap<AntiPattern, Severity>,
}
```

### 2. Partial Parsing with Recovery

Instead of failing completely, we:
- Parse what we can
- Insert special AST nodes for unparseable sections
- Continue parsing after the problematic construct

```rust
pub enum AstNode {
    // Normal nodes...
    Statement(Statement),
    Expression(Expression),
    
    // Special nodes for problematic code
    UnparseableConstruct {
        kind: AntiPattern,
        raw_text: String,
        partial_parse: Option<Box<AstNode>>,
        diagnostics: Vec<Diagnostic>,
    },
}
```

### 3. Diagnostic Information

Each anti-pattern gets rich diagnostics:

```rust
pub struct Diagnostic {
    severity: Severity,
    message: String,
    explanation: String,
    suggested_fix: Option<String>,
    references: Vec<String>,
}
```

## Specific Anti-Pattern Handlers

### Format Heredocs

```rust
impl FormatHeredocHandler {
    fn detect(&self, code: &str) -> Option<FormatHeredoc> {
        // Look for: format NAME = 
        // followed by heredoc on next line
    }
    
    fn diagnose(&self, pattern: &FormatHeredoc) -> Diagnostic {
        Diagnostic {
            severity: Severity::Warning,
            message: "Format with heredoc detected".to_string(),
            explanation: "Perl formats are deprecated and their interaction with heredocs is problematic".to_string(),
            suggested_fix: Some("Consider using sprintf or a templating system instead".to_string()),
            references: vec!["perldoc perlform".to_string()],
        }
    }
}
```

### BEGIN-time Heredocs

```rust
impl BeginTimeHeredocHandler {
    fn detect(&self, code: &str) -> Option<BeginTimeHeredoc> {
        // Look for heredocs inside BEGIN blocks
        // Track potential side effects
    }
    
    fn analyze_side_effects(&self, begin_block: &str) -> Vec<String> {
        // Identify:
        // - Variable modifications
        // - Subroutine definitions
        // - Module loads
        // - File operations
    }
}
```

### Source Filter Detection

```rust
impl SourceFilterDetector {
    fn detect_filters(&self, code: &str) -> Vec<SourceFilter> {
        // Look for: use Filter::*
        // Track which filters might affect heredocs
    }
    
    fn estimate_impact(&self, filter: &SourceFilter) -> Impact {
        match filter.module.as_str() {
            "Filter::Simple" => Impact::Low,
            "Filter::Util::Call" => Impact::High,
            "Acme::*" => Impact::Unpredictable,
            _ => Impact::Unknown,
        }
    }
}
```

## AST Annotation Strategy

### Level 1: Mark Problematic Regions

```rust
// AST after parsing
Program {
    statements: [
        Statement::Normal(...),
        Statement::WithWarning {
            inner: Statement::Format(...),
            warnings: ["Format with heredoc detected"],
        },
        Statement::Normal(...),
    ]
}
```

### Level 2: Provide Context

```rust
// Enhanced AST with context
Statement::Problematic {
    kind: AntiPattern::FormatHeredoc,
    attempted_parse: PartialParse {
        // What we could parse
    },
    raw_text: "format REPORT = \n<<'END'...",
    context: Context {
        preceding_statements: [...],
        following_statements: [...],
        scope_info: ScopeInfo { ... },
    },
}
```

### Level 3: Suggest Transformations

```rust
pub struct Remediation {
    pattern: AntiPattern,
    original_code: String,
    suggested_code: String,
    transformation_steps: Vec<Step>,
    risk_assessment: Risk,
}
```

## User-Facing Features

### 1. Anti-Pattern Report

```
$ perl-parser analyze --anti-patterns file.pl

Anti-Pattern Analysis Report
============================

Found 3 problematic patterns:

1. Format with heredoc (line 45)
   Severity: Medium
   Issue: Perl formats are deprecated and interact poorly with heredocs
   Suggestion: Replace with sprintf or Text::Template
   
2. BEGIN block with heredoc side effects (line 120)
   Severity: High
   Issue: Heredoc modifies $CONFIG at compile time
   Side effects detected: Global variable modification
   Suggestion: Move to runtime initialization
   
3. Dynamic heredoc delimiter (line 234)
   Severity: High
   Issue: Delimiter computed at runtime: <<${\(get_delimiter())}
   Suggestion: Use static delimiter with variable interpolation
```

### 2. Code Intelligence Annotations

In IDE/editor integration:
- Red underline for high-severity anti-patterns
- Yellow for medium severity
- Hover for detailed explanation
- Quick fixes for automated remediation

### 3. Refactoring Assistant

```perl
# Original problematic code
BEGIN {
    $config = <<'END';
    server = $ENV{SERVER}
END
}

# Suggested refactoring
our $config;
INIT {
    $config = <<"END";
    server = $ENV{SERVER}
END
}
```

## Benefits of This Approach

1. **Complete Understanding**: We parse and understand even problematic code
2. **Educational**: Users learn why certain patterns are problematic
3. **Actionable**: Provides concrete fixes, not just warnings
4. **Gradual Migration**: Can refactor incrementally
5. **Tool Integration**: IDEs can use our analysis for better code intelligence

## Implementation Priority

1. **High Priority** (Common anti-patterns):
   - Format heredocs (legacy but still exists)
   - BEGIN-time heredocs (sneaky bugs)
   - Dynamic delimiters (security issues)

2. **Medium Priority** (Less common):
   - Source filter interactions
   - Tied filehandle heredocs
   - Encoding issues

3. **Low Priority** (Rare):
   - Multi-file heredocs
   - Custom syntax extensions
   - Exotic combinations

## Conclusion

By treating anti-patterns as first-class parsing targets rather than errors, we:
- Provide better code understanding
- Help users identify problematic code
- Suggest concrete improvements
- Make legacy code more maintainable

This transforms our parser from just a syntax analyzer into a true code understanding and improvement tool.