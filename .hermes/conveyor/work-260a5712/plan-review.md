# Plan Review Comment — work-260a5712

## Key Issues with the Initial Plan

### 1. `profile` enum is incorrect (Critical)
The plan proposes `perl-lsp.perltidy.profile` with enum values `["default", "pbp", "gnu"]`. This is wrong because:

- `PerlTidyConfig.profile` is a **file path** to `.perltidyrc`, not a profile name
- Built-in profiles (PBP, GNU) are implemented via `extra_args` (e.g., `--perl-best-practices`, `--gnu-style`)
- When `profile` is set, perltidy ignores ALL other options

**Fix**: Add a new `builtinProfile` enum setting that maps to `extra_args` values. Or use `extra_args` directly.

### 2. Settings transformation missing
The plan says to "send via `workspace/didChangeConfiguration`" but doesn't detail the transformation from `perl-lsp.perltidy.*` (VS Code) to `perl.formatting.*` (LSP server).

The perlcritic pattern is:
- `perl-lsp.perlcritic.*` → `perl.perlcritic.*` via `buildPerlCriticConfiguration`

Perltidy needs the same pattern:
- `perl-lsp.perltidy.*` → `perl.formatting.*` via `buildPerlTidyConfiguration`

### 3. `perltidyConfig` precedence not addressed
The plan's Risk #1 is more severe than stated. If a user sets `perltidyConfig` path AND individual settings, the individual settings are silently ignored (perltidy behavior, not our code).

## Recommended Corrections to Plan

1. Replace `profile` enum with `builtinProfile` enum mapping to `extra_args`
2. Add explicit `buildPerlTidyConfiguration` function following `buildPerlCriticConfiguration` pattern
3. Document precedence: `perltidyConfig` path takes absolute precedence, individual settings ignored when path is set
