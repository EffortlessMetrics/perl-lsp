# perl-builtins-phf

Compile-time PHF maps for Perl builtin function signatures.

This crate is intentionally narrow: it provides static lookup tables and helper
functions for identifying builtins and retrieving parameter names/signatures in
O(1) time with zero runtime allocation.

## API

- `BUILTIN_SIGS`
- `BUILTIN_FULL_SIGS`
- `get_param_names(name)`
- `is_builtin(name)`
- `builtin_count()`
