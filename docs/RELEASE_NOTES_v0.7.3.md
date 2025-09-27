# Release Notes - v0.7.3

## Bug Fixes

### Parser Improvements
- **Fixed return statement modifier parsing** - The parser now correctly handles `return` statements with modifiers like `return if $cond;` and `return $x or die if $error;`
- **Fixed die with statement modifiers** - Zero-arg builtins like `die` now correctly convert to function calls in expression context when used with statement modifiers

### Build & CI Improvements
- **Fixed CI warnings** - Resolved unused variable warning in LSP integration tests
- **Configured benchmarks for release mode** - Added proper workspace-level profile configuration for optimized benchmark builds
- **Cleaned up example warnings** - Fixed various unused import and variable warnings in example code

## Technical Details

### Parser Changes
The parser now correctly identifies statement modifier keywords (`if`, `unless`, `while`, `until`, `for`, `foreach`) when parsing `return` statements, preventing them from being incorrectly parsed as part of the return value expression.

### Performance Configuration
Added workspace-level Cargo profiles:
- `dev` profile with `opt-level = 1` for better development performance
- `release` profile with full optimization (`opt-level = 3`, `lto = true`, `codegen-units = 1`)
- `bench` profile inheriting from release for maximum benchmark performance

## Compatibility
This is a patch release with no breaking changes. It maintains full backward compatibility with v0.7.2.

## Testing
All existing tests pass, plus new tests added for:
- Return statement with various modifiers
- Zero-arg builtins with statement modifiers
- Complex expressions with statement modifiers