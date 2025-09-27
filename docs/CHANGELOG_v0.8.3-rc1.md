# Changelog Entry for v0.8.3-rc.1

## [0.8.3-rc.1] - 2025-08-16

### Added
- `serverInfo` LSP method returning version information
- Built-in function signatures for `getsockopt` and `setsockopt`
- CLI smoke tests for binary verification
- Build system detection for `.git/packed-refs` changes

### Changed
- Marked `dbmopen` and `dbmclose` as deprecated
- Updated 45+ test assertions from tautological to meaningful checks
- Marked 45 aspirational test cases as ignored with clear documentation

### Fixed
- Removed unsafe `option_env!().unwrap()` usage that could panic
- Added `#![deny(clippy::option_env_unwrap)]` to prevent regression
- Fixed test infrastructure to properly validate LSP responses

### Technical Debt
- 45 ignored tests documented for future parser enhancements:
  - 23 for constant pragma advanced forms
  - 18 for advanced LSP features (test runner, profiling)
  - 4 for error classifier updates
- 1 operator precedence edge case marked for future fix