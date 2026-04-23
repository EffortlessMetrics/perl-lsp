# Error Recovery Implementation

The v3 Perl parser now supports **error recovery**, allowing it to continue parsing even when encountering syntax errors. This is essential for IDE scenarios where code is often incomplete or temporarily invalid.

## Features

### 1. Error Nodes in AST
```rust
NodeKind::Error {
    message: String,
    expected: Vec<String>,
    partial: Option<Box<Node>>,
}
```

### 2. Synchronization Points
- Semicolons (`;`) - statement boundaries
- Closing braces (`}`) - block boundaries  
- Keywords (`my`, `if`, `sub`, etc.) - statement starts
- End of file

### 3. Recovery Strategies
- **Skip and recover**: Skip tokens until a sync point
- **Insert missing**: Create error nodes for missing expressions
- **Partial parsing**: Continue parsing even with missing delimiters

## Usage

```rust
use perl_parser::RecoveryParser;

let parser = RecoveryParser::new(source);
let (ast, errors) = parser.parse();

// AST contains Error nodes where parsing failed
// errors contains detailed diagnostics
```

## Example Output

For code with errors:
```perl
my $x = ; my $y = 99;
```

The parser produces:
- AST with an Error node for the missing value after `=`
- Continues to successfully parse `my $y = 99;`
- Reports the error with location and expected tokens

## Benefits

1. **IDE Integration**: Essential for real-time parsing as users type
2. **Better Diagnostics**: Reports all errors, not just the first
3. **Partial ASTs**: Enables features like syntax highlighting even with errors
4. **Graceful Degradation**: Maximum information extraction from broken code

## Architecture

- `error_recovery.rs`: Core error recovery traits and types
- `parser_context.rs`: Token management with error tracking
- `recovery_parser.rs`: Parser implementation with recovery
- Examples demonstrate various error scenarios

The implementation follows industry best practices for error recovery in recursive descent parsers, similar to approaches used in Rust Analyzer and TypeScript.