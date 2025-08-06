# Perl Language Server Protocol (LSP) Features

This document provides comprehensive documentation of all LSP features implemented in the perl-lsp server.

## Latest Updates (v0.7.2)

- **Enhanced Signature Help**: Now includes comprehensive signatures for 150+ Perl built-in functions
- **Fixed Parser Issues**: Corrected operator precedence for word operators and division operator parsing
- **Improved Accuracy**: Better handling of Perl's context-sensitive syntax

## Table of Contents

- [Core Features](#core-features)
- [Advanced Refactoring](#advanced-refactoring)
- [Enhanced Features](#enhanced-features)
- [Configuration](#configuration)
- [Editor Integration](#editor-integration)
- [Performance](#performance)

## Core Features

### 1. Real-time Diagnostics

The LSP provides comprehensive diagnostics as you type:

- **Syntax Errors**: Parse errors with exact location and recovery suggestions
- **Semantic Warnings**: Undefined variables, unused declarations, deprecated syntax
- **Best Practices**: Missing strict/warnings, assignment in conditions
- **Context-Aware**: Understands scope and variable declarations

Example diagnostics:
```perl
# Missing declaration
print $undefined;  # Error: Variable '$undefined' is not declared

# Assignment in condition
if ($x = 5) { }   # Warning: Assignment in condition (did you mean ==?)

# Deprecated syntax
defined @array;    # Warning: defined(@array) is deprecated
```

### 2. Code Completion

Context-aware completion with rich documentation:

#### Variable Completion
- Completes scalar (`$`), array (`@`), and hash (`%`) variables
- Filters by sigil and prefix
- Shows variable scope (my, our, local)

#### Function Completion
- Built-in functions with signatures
- User-defined subroutines
- Method names after `->`
- Package-qualified names

#### Keyword Completion
- All Perl keywords with snippets
- Control flow structures expand to full syntax
- Context-sensitive (e.g., `elsif` only after `if`)

#### Special Variables
- Perl special variables (`$_`, `@ARGV`, `%ENV`)
- With documentation and usage examples

Example:
```perl
pri<cursor>  # Suggests: print, printf, private
$arr<cursor> # Suggests: $array, $array_ref
for<cursor>  # Expands to: for (my $i = 0; $i < $count; $i++) { ... }
```

### 3. Go to Definition

Navigate to symbol definitions with a single click/keystroke:

- **Variables**: Jump to declaration (my, our, local)
- **Subroutines**: Navigate to sub definition
- **Packages**: Go to package declaration
- **Methods**: Find method implementation
- **Multi-file**: Works across project files

### 4. Find References

Locate all uses of a symbol throughout your codebase:

- **Variable References**: All uses including interpolation
- **Function Calls**: Direct and indirect calls
- **Method Invocations**: Object and class methods
- **String Interpolation**: Variables in strings
- **Regular Expressions**: Variables in regex

Example:
```perl
my $name = "Alice";
print "Hello, $name";     # Found as reference
s/($name)/Found: $1/;     # Found in regex
```

### 5. Hover Information

Rich hover tooltips with:

- **Variable Type**: Scalar, array, hash, reference
- **Function Signatures**: Parameters and return types
- **Documentation**: Inline POD and comments
- **Value Preview**: For constants and literals
- **Module Info**: Package and version information

### 6. Signature Help (Enhanced in v0.7.2)

Real-time parameter hints while typing function calls:

- **150+ Built-in Functions**: Complete coverage of Perl built-ins
- **User Functions**: Extracted from prototypes and signatures
- **Active Parameter**: Highlights current parameter
- **Optional/Required**: Shows parameter requirements
- **Examples**: Usage examples for complex functions

Example:
```perl
substr($string, |  # Shows: substr(EXPR, OFFSET, [LENGTH], [REPLACEMENT])
                ^-- cursor here, highlights OFFSET parameter
```

### 7. Document Symbols

Hierarchical outline view of document structure:

- **Packages**: With version and exports
- **Subroutines**: Including anonymous subs
- **Variables**: Grouped by type
- **Constants**: use constant declarations
- **POD Sections**: Documentation structure
- **Icons**: Visual differentiation by type

### 8. Rename Symbol

Safe, project-wide renaming:

- **Validation**: Checks for conflicts
- **Scope-Aware**: Respects lexical scope
- **Multi-file**: Updates across all files
- **Preview**: Shows changes before applying
- **Undo Support**: Full undo/redo capability

## Advanced Refactoring

### 1. Extract Variable

Extract complex expressions to named variables:

**Before:**
```perl
my $result = length($string) * 2 + calculate_offset($data);
```

**After:**
```perl
my $len = length($string) * 2;
my $result = $len + calculate_offset($data);
```

Features:
- Smart variable naming based on expression
- Finds optimal insertion point
- Handles all expression types
- Preserves formatting

### 2. Extract Subroutine

Extract code blocks to separate functions:

**Before:**
```perl
# Complex calculation inline
my $x = 10;
my $y = 20;
my $sum = $x + $y;
print "Sum: $sum\n";
```

**After:**
```perl
sub calculate_sum {
    my ($x, $y) = @_;
    my $sum = $x + $y;
    print "Sum: $sum\n";
    return $sum;
}

my $result = calculate_sum(10, 20);
```

### 3. Convert Loop Styles

Modernize old-style loops:

**C-style to foreach:**
```perl
# Before
for (my $i = 0; $i < @array; $i++) {
    print $array[$i];
}

# After
foreach my $item (@array) {
    print $item;
}
```

**Implicit to explicit variable:**
```perl
# Before
for (@items) {
    print;  # Uses $_
}

# After  
foreach my $item (@items) {
    print $item;
}
```

### 4. Add Error Checking

Add error handling to file operations:

**Before:**
```perl
open my $fh, '<', 'file.txt';
print $fh "data";
close $fh;
```

**After:**
```perl
open my $fh, '<', 'file.txt' or die "Failed to open: $!";
print $fh "data" or die "Failed to print: $!";
close $fh or die "Failed to close: $!";
```

### 5. Convert to Postfix

Transform control structures to postfix form:

**Before:**
```perl
if ($debug) {
    print "Debug mode\n";
}
```

**After:**
```perl
print "Debug mode\n" if $debug;
```

Works with:
- `if` → postfix if
- `unless` → postfix unless
- `while` → postfix while
- `until` → postfix until

### 6. Organize Imports

Sort and group use statements:

**Before:**
```perl
use JSON;
use strict;
use lib './lib';
use warnings;
use Data::Dumper;
```

**After:**
```perl
use strict;
use warnings;
use Data::Dumper;
use JSON;
use lib './lib';
```

Grouping order:
1. Pragmas (strict, warnings, feature)
2. Core modules
3. CPAN modules
4. Local modules

### 7. Add Missing Pragmas

Quick fix to add recommended pragmas:

```perl
# Adds at the top of file:
use strict;
use warnings;
use utf8;  # If non-ASCII content detected
```

## Enhanced Features

### Semantic Tokens

Advanced syntax highlighting beyond simple regex:

- **Token Types**: 15+ semantic token types
- **Token Modifiers**: readonly, definition, deprecated
- **Context-Aware**: Different highlighting for same text in different contexts
- **Incremental**: Updates only changed regions

### CodeLens

Inline actions above code:

- **Run Test**: Execute test subroutines
- **Debug**: Start debugging session
- **Coverage**: Show test coverage
- **References**: Count of references
- **Complexity**: Cyclomatic complexity metrics

### Call Hierarchy

Visualize function relationships:

- **Incoming Calls**: What calls this function?
- **Outgoing Calls**: What does this function call?
- **Tree View**: Expandable hierarchy
- **Cross-file**: Works across project

### Inlay Hints

Inline parameter and type hints:

```perl
process($data, 1, true);
        ^^^^^  ^  ^^^^
        data   id verbose  # Inlay hints show parameter names
```

### Workspace Symbols

Search symbols across entire workspace:

- **Fuzzy Search**: Flexible matching
- **Symbol Types**: Functions, variables, packages
- **Filtering**: By type, scope, file
- **Performance**: Indexed for speed

### Folding Ranges

Intelligent code folding:

- **Subroutines**: Fold entire functions
- **Blocks**: if/else, loops, try/catch
- **POD**: Documentation sections
- **Comments**: Multi-line comment blocks
- **Heredocs**: Multi-line strings

## Configuration

### Server Settings

```json
{
  "perl.lsp.diagnostics": true,
  "perl.lsp.completion": {
    "enableSnippets": true,
    "addSemicolon": true,
    "includeBuiltins": true
  },
  "perl.lsp.refactoring": {
    "extractVariable": {
      "nameStyle": "snake_case"
    }
  },
  "perl.lsp.formatting": {
    "provider": "perltidy",
    "options": "-pbp"  // Perl Best Practices
  }
}
```

### Performance Tuning

```json
{
  "perl.lsp.maxFileSize": 1048576,  // 1MB
  "perl.lsp.incrementalParsing": true,
  "perl.lsp.cacheDuration": 300,     // seconds
  "perl.lsp.parallelParsing": true
}
```

## Editor Integration

### Visual Studio Code

Install the Perl LSP extension or configure manually:

```json
// .vscode/settings.json
{
  "perl.languageServer": {
    "enabled": true,
    "path": "perl-lsp",
    "args": ["--stdio"]
  }
}
```

### Neovim

Using nvim-lspconfig:

```lua
require('lspconfig').perl_lsp.setup{
  cmd = {'perl-lsp', '--stdio'},
  settings = {
    perl = {
      lsp = {
        diagnostics = true,
        completion = { enableSnippets = true }
      }
    }
  }
}
```

### Emacs

With lsp-mode or eglot:

```elisp
;; lsp-mode
(lsp-register-client
 (make-lsp-client :new-connection (lsp-stdio-connection "perl-lsp")
                  :major-modes '(perl-mode cperl-mode)
                  :priority 10
                  :server-id 'perl-lsp))

;; eglot
(add-to-list 'eglot-server-programs
             '((perl-mode cperl-mode) . ("perl-lsp" "--stdio")))
```

### Sublime Text

```json
// LSP.sublime-settings
{
  "clients": {
    "perl-lsp": {
      "enabled": true,
      "command": ["perl-lsp", "--stdio"],
      "selector": "source.perl"
    }
  }
}
```

## Performance

### Parsing Performance

- **Initial Parse**: 1-150 µs for typical files
- **Incremental Updates**: <10 µs for small changes
- **Large Files**: Linear scaling, ~7.5 µs/KB
- **Memory Usage**: ~2x file size

### LSP Response Times

| Operation | Response Time |
|-----------|--------------|
| Completion | <50ms |
| Go to Definition | <20ms |
| Find References | <100ms |
| Diagnostics | <100ms |
| Hover | <30ms |
| Signature Help | <20ms |
| Document Symbols | <50ms |
| Rename | <200ms |
| Code Actions | <100ms |

### Optimization Tips

1. **Enable Incremental Parsing**: Dramatically improves performance for large files
2. **Use Workspace Indexing**: Pre-indexes symbols for faster searches
3. **Configure Cache**: Adjust cache duration based on project size
4. **Limit File Size**: Set reasonable limits for very large generated files

## Troubleshooting

### Common Issues

**LSP not starting:**
```bash
# Check if perl-lsp is in PATH
which perl-lsp

# Test standalone
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | perl-lsp --stdio
```

**Slow performance:**
- Check file size limits
- Enable incremental parsing
- Increase cache duration
- Check for recursive includes

**Missing features:**
- Ensure latest version: `perl-lsp --version`
- Check editor LSP client capabilities
- Verify configuration is loaded

### Debug Logging

Enable debug logging for troubleshooting:

```bash
# Command line
perl-lsp --stdio --log-level=debug --log-file=perl-lsp.log

# Environment variable
RUST_LOG=debug perl-lsp --stdio
```

## Contributing

We welcome contributions! Areas for improvement:

1. **Additional Refactorings**: More code transformations
2. **Performance**: Further optimization for large codebases
3. **Cross-file Analysis**: Better multi-file support
4. **Type Inference**: Smarter type detection
5. **Framework Support**: Moose, Catalyst, Dancer integration

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is dual-licensed under MIT and Apache 2.0 licenses.