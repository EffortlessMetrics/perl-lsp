# perl-dap-eval

Safe expression evaluation validation for Perl Debug Adapter Protocol (DAP).

## Features

- **Operation Detection**: Detects dangerous Perl operations (eval, system, exec, etc.)
- **Mutation Detection**: Detects assignment and increment/decrement operators
- **Shell Execution Detection**: Blocks backticks and qx() for shell execution
- **Context-Aware**: Avoids false positives for sigil-prefixed identifiers ($print, @say)

## Usage

```rust
use perl_dap_eval::{SafeEvaluator, ValidationResult};

let evaluator = SafeEvaluator::new();

// Safe expressions pass validation
assert!(evaluator.validate("$x + $y").is_ok());
assert!(evaluator.validate("length($str)").is_ok());

// Dangerous operations are blocked
let result = evaluator.validate("system('rm -rf /')");
assert!(result.is_err());
```

## Security Model

The safe evaluator blocks the following categories of operations:

### Code Execution
- `eval`, `require`, `do` (file)

### Process Control
- `system`, `exec`, `fork`, `exit`, `kill`, `alarm`, `sleep`, etc.

### I/O Operations
- `print`, `say`, `open`, `close`, `read`, `write`, etc.

### Filesystem
- `mkdir`, `rmdir`, `unlink`, `chmod`, `chown`, etc.

### Network
- `socket`, `connect`, `bind`, `listen`, `accept`, etc.

### State Mutation
- Assignment operators (`=`, `+=`, `-=`, etc.)
- Increment/decrement (`++`, `--`)
- Regex mutation (`s///`, `tr///`, `y///`)

### Tie Mechanism
- `tie`, `untie` (can execute arbitrary code via FETCH/STORE)

## Context-Aware Filtering

The validator avoids false positives for:

- **Sigil-prefixed identifiers**: `$print`, `@say`, `%exit` are allowed (they're variable names)
- **Braced variables**: `${print}` is allowed
- **Package-qualified names**: `Foo::print` is allowed (not `CORE::print`)
- **Single-quoted strings**: `'print this'` is allowed

## License

MIT OR Apache-2.0
