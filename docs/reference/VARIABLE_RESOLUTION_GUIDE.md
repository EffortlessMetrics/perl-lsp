# Enhanced Variable Resolution Guide

## Overview

This guide explains the advanced variable pattern recognition capabilities in tree-sitter-perl's scope analyzer, implemented as part of the enhanced `try_resolve_variable` functionality. These improvements significantly reduce false positives in `use strict` environments and provide better IDE support for complex Perl variable patterns.

## 🎯 **How-to Guide**: Using Enhanced Variable Resolution

### Problem: Complex Variable Patterns Not Recognized

**Before Enhancement:**
```perl
use strict;
use warnings;

my %config = (host => 'localhost', port => 3000);
print $config{host};  # ❌ False warning: Variable '$config' used but not declared
```

**After Enhancement:**
```perl
use strict;
use warnings;

my %config = (host => 'localhost', port => 3000);
print $config{host};  # ✅ Correctly resolves %config, no warning
```

### How to Leverage Advanced Pattern Recognition

#### 1. Hash Access Patterns

**Pattern**: `$hash{key}` resolves to `%hash`

```perl
use strict;
use warnings;

my %data = (
    name => 'John',
    age => 30,
    skills => ['Perl', 'Python', 'Rust']
);

# All these work correctly now:
print $data{name};           # ✅ Resolves %data
print $data{skills}->[0];    # ✅ Resolves %data  
my $key = 'age';
print $data{$key};           # ✅ Resolves %data
```

#### 2. Array Access Patterns

**Pattern**: `$array[index]` resolves to `@array`

```perl
use strict;
use warnings;

my @items = qw(foo bar baz qux);

# All these work correctly:
print $items[0];             # ✅ Resolves @items
print $items[-1];            # ✅ Resolves @items
my $idx = 2;
print $items[$idx];          # ✅ Resolves @items
```

#### 3. Method Call Patterns

**Pattern**: `$object->method` resolves base `$object`

```perl
use strict;
use warnings;

my $dbh = DBI->connect($dsn, $user, $pass);

# Method calls correctly resolve:
$dbh->prepare($sql);         # ✅ Resolves $dbh
$dbh->selectrow_array($sql); # ✅ Resolves $dbh
```

#### 4. Complex Nested Patterns

**Pattern**: Multi-level dereference with fallback resolution

```perl
use strict;
use warnings;

my %complex = (
    users => [
        { name => 'Alice', roles => ['admin', 'user'] },
        { name => 'Bob',   roles => ['user'] }
    ],
    config => { db => { host => 'localhost' } }
);

# Complex patterns work:
print $complex{users}->[0]->{name};           # ✅ Resolves %complex
print $complex{config}->{db}->{host};        # ✅ Resolves %complex
my @admin_roles = @{$complex{users}->[0]->{roles}}; # ✅ Resolves %complex
```

### Hash Key Context Detection

**Problem**: Barewords in hash subscripts triggering false warnings

**Solution**: The enhanced resolver detects hash key contexts:

```perl
use strict;
use warnings;

my %colors = (red => '#FF0000', blue => '#0000FF');

# These barewords are correctly identified as hash keys:
print $colors{red};    # ✅ 'red' recognized as hash key, not bareword
print $colors{blue};   # ✅ 'blue' recognized as hash key, not bareword

# Hash slices also work:
my @rgb = @colors{qw(red green blue)};  # ✅ All keys recognized
```

## 🔧 **Reference**: Technical Implementation Details

### Scope Analyzer Enhancements

The `try_resolve_variable_reference` method implements a recursive resolution system:

```rust
fn try_resolve_variable_reference(&self, node: &Node, scope: &Rc<Scope>) -> Option<String> {
    match &node.kind {
        NodeKind::Variable { sigil, name } => {
            // Direct variable lookup
        }
        NodeKind::Binary { op, left, .. } => {
            match op.as_str() {
                "{}" => {
                    // Hash access: $hash{key} -> try to resolve %hash
                    // Convert scalar reference to hash reference
                }
                "[]" => {
                    // Array access: $array[index] -> try to resolve @array
                    // Convert scalar reference to array reference  
                }
                _ => None,
            }
        }
        _ => None,
    }
}
```

### Hash Context Detection

The `is_in_hash_key_context` method uses ancestor analysis:

```rust
fn is_in_hash_key_context(&self, node: &Node) -> bool {
    // Check if node is within a hash subscript context like $hash{bareword}
    // This helps determine if barewords should be treated as hash keys vs identifiers
    self.find_hash_subscript_ancestor(node).is_some()
}
```

### Dynamic Delimiter Recovery Enhancements

Enhanced regex patterns support comprehensive variable recognition:

```rust
// Enhanced patterns for delimiter variables
static DELIMITER_ASSIGNMENT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?m)^\s*(?:my|our|local|state)\s+[\$@%](\w+)\s*=\s*["']([^"']+)["']"#).unwrap()
});

static ARRAY_ASSIGNMENT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?m)^\s*(?:my|our|local|state)\s+@(\w+)\s*=\s*\(([^)]+)\)"#).unwrap()
});

static HASH_ASSIGNMENT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?m)^\s*(?:my|our|local|state)\s+%(\w+)\s*=\s*\(([^)]+)\)"#).unwrap()
});
```

### Confidence Scoring System

The system uses intelligent confidence scoring for delimiter detection:

- **High Confidence (0.8)**: Variable names containing delimiter keywords (`delim`, `end`, `eof`, `marker`, `tag`, `term`, `terminator`)
- **Medium Confidence (0.5-0.7)**: Context-based patterns (arrays, hashes with delimiter-like values)
- **Low Confidence (0.3-0.4)**: Generic assignments without clear delimiter indicators

## 🎨 **Tutorial**: Setting Up Your Development Environment

### For LSP Server Users

1. **Install the enhanced perl-lsp server**:
```bash
cargo install perl-lsp
# Or build from source with latest enhancements
cargo build --release -p perl-lsp
```

2. **Configure your editor** to use the enhanced diagnostics:
```json
// VSCode settings.json
{
    "perl-lsp.diagnostics.enhanced": true,
    "perl-lsp.variableResolution.complexPatterns": true
}
```

3. **Test the enhancements** with a sample file:
```perl
use strict;
use warnings;

my %config = (host => 'localhost');
my @items = qw(foo bar baz);
my $obj = SomeClass->new();

# These should not generate warnings:
print $config{host};
print $items[0];
$obj->method();
```

### For Parser Library Users

```rust
use perl_parser::{Parser, scope_analyzer::ScopeAnalyzer};

let code = r#"
use strict;
my %hash = (key => 'value');
print $hash{key};
"#;

let mut parser = Parser::new(code);
let ast = parser.parse().expect("Parse failed");

let analyzer = ScopeAnalyzer::new();
let issues = analyzer.analyze_strict_compliance(&ast, code);

// Should find no issues with the enhanced resolution
assert!(issues.is_empty());
```

## 🚀 **Explanation**: Design Decisions and Benefits

### Why Enhanced Variable Resolution Matters

1. **Reduced False Positives**: Traditional static analysis often fails to recognize that `$hash{key}` uses the `%hash` variable, leading to unnecessary warnings.

2. **Better IDE Experience**: Enhanced resolution provides more accurate diagnostics, reducing developer frustration with false warnings.

3. **Perl-Specific Patterns**: Unlike generic parsers, this implementation understands Perl's sigil system and complex dereference patterns.

### Architecture Benefits

1. **Recursive Resolution**: The system can handle arbitrarily complex nesting patterns.

2. **Fallback Mechanisms**: When direct resolution fails, fallback strategies maintain robustness.

3. **Context Awareness**: Hash key context detection significantly reduces bareword false positives.

4. **Performance**: The enhancements add minimal overhead while providing substantial accuracy improvements.

### Comparison with Other Implementations

| Feature | Generic LSP | perl-lsp (before) | perl-lsp (enhanced) |
|---------|-------------|-------------------|-------------------|
| Hash access resolution | ❌ | ❌ | ✅ |
| Array access resolution | ❌ | ❌ | ✅ |
| Method call resolution | ❌ | ❌ | ✅ |
| Hash key context detection | ❌ | ❌ | ✅ |
| Complex pattern fallback | ❌ | ❌ | ✅ |
| False positive rate | High | Medium | Low |

## See Also

- [LSP_ACTUAL_STATUS.md](../LSP_ACTUAL_STATUS.md) - Current LSP feature status
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Overall system architecture  
- [docs/EDGE_CASES.md](EDGE_CASES.md) - Edge case handling details
- [CLAUDE.md](../CLAUDE.md) - Development guide and feature overview

---

This enhanced variable resolution system represents a significant improvement in Perl code analysis accuracy, bringing tree-sitter-perl's scope analyzer closer to understanding real-world Perl patterns while maintaining the performance and reliability expected from production tooling.