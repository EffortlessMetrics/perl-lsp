# Verification Findings — work-260a5712

## Confidence Assessment

**medium** — The research agent correctly identified the high-level gap (VS Code settings not wired to LSP server), but made critical errors about implementation details. The `PerlTidyConfig` struct and `FormattingProvider` exist as described, but the proposed plan contains fundamental misunderstandings about how `profile` works and what settings the LSP server actually accepts.

---

## Confirmed Findings

### 1. `PerlTidyConfig` struct is fully implemented
**Evidence**: `crates/perl-lsp-perltidy/src/lib.rs` lines 21-46

The struct has all the fields mentioned:
- `maximum_line_length: Option<u32>`
- `indent_columns: Option<u32>`
- `tabs: Option<bool>`
- `opening_brace_on_new_line: Option<bool>`
- `cuddled_else: Option<bool>`
- `space_after_keyword: Option<bool>`
- `add_trailing_commas: Option<bool>`
- `vertical_alignment: Option<bool>`
- `block_comment_indentation: Option<u32>`
- `profile: Option<String>` — path to `.perltidyrc`
- `extra_args: Vec<String>`
- `timeout_secs: u64`

### 2. Built-in profiles `pbp()` and `gnu()` exist
**Evidence**: `crates/perl-lsp-perltidy/src/lib.rs` lines 70-104

- `PerlTidyConfig::pbp()` sets `extra_args: vec!["--perl-best-practices".to_string()]`
- `PerlTidyConfig::gnu()` sets `extra_args: vec!["--gnu-style".to_string()]`

### 3. `FormattingProvider` accepts `PerlTidyConfig` via builder
**Evidence**: `crates/perl-lsp-formatting/src/formatting.rs` lines 78-81

```rust
pub fn with_perltidy_config(mut self, config: PerlTidyConfig) -> Self {
    self.perltidy_config = Some(config);
    self
}
```

### 4. VS Code extension exposes `perl-lsp.perltidyConfig` as path string only
**Evidence**: `vscode-extension/package.json` lines 293-300

```json
"perl-lsp.perltidyConfig": {
  "type": "string",
  "default": "",
  "description": "Path to your .perltidyrc configuration file..."
}
```

### 5. VS Code extension sends perlcritic config via `workspace/didChangeConfiguration`
**Evidence**: `vscode-extension/src/extension.ts` lines 143-159 (buildPerlCriticConfiguration) and lines 178-190 (syncPerlCriticConfiguration)

The extension transforms `perl-lsp.perlcritic.*` settings into `perl.perlcritic.*` and sends via `workspace/didChangeConfiguration`. Pattern established.

---

## Corrected Findings

### 1. CRITICAL: `profile` field is NOT a built-in profile selector

**Research agent said**: The `profile` field in `PerlTidyConfig` is "path to `.perltidyrc`" and the plan proposes adding a VS Code setting `perl-lsp.perltidy.profile` with enum values `["default", "pbp", "gnu"]`.

**Reality**: The `profile` field IS a path (confirmed correct). BUT the plan's proposed enum values `["default", "pbp", "gnu"]` would NOT work because:
- `profile` is literally a file path string, not an enum
- Built-in profiles (PBP, GNU) are NOT accessed via `profile` — they set `extra_args` like `--perl-best-practices` or `--gnu-style`

**Evidence**: `crates/perl-lsp-perltidy/src/lib.rs` lines 108-114:
```rust
pub fn to_args(&self) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(profile) = &self.profile {
        args.push(format!("--profile={profile}"));
        return args;  // <-- If profile is set, ALL other options are ignored!
    }
    // ... rest of args
}
```

If a user sets `profile: Some("/path/to/.perltidyrc")`, perltidy uses ONLY that profile file and ignores all other options (maximum_line_length, indent_columns, etc.).

**Correct approach**: To offer "PBP" and "GNU" as quick-select options, the extension should set `extra_args` (not `profile`):
- PBP: `extra_args: ["--perl-best-practices"]`
- GNU: `extra_args: ["--gnu-style"]`
- Default: `extra_args: []` with individual options

### 2. CRITICAL: LSP server accepts ALL perltidy options via `perl.formatting.*`, not just `profile`

**Research agent said**: The LSP server accepts perltidy config via `FormattingProvider::with_perltidy_config()`.

**Reality**: The LSP server has a comprehensive settings interface under `perl.formatting.*` that already covers ALL perltidy options:

**Evidence**: `crates/perl-lsp/src/runtime/workspace.rs` lines 651-680:
```rust
"perl.formatting.profile" => json!(config.perltidy_profile),
"perl.formatting.maximumLineLength" => json!(config.perltidy_maximum_line_length),
"perl.formatting.indentColumns" => json!(config.perltidy_indent_columns),
"perl.formatting.tabs" => json!(config.perltidy_tabs),
"perl.formatting.openingBraceOnNewLine" => json!(config.perltidy_opening_brace_on_new_line),
"perl.formatting.cuddledElse" => json!(config.perltidy_cuddled_else),
"perl.formatting.spaceAfterKeyword" => json!(config.perltidy_space_after_keyword),
"perl.formatting.addTrailingCommas" => json!(config.perltidy_add_trailing_commas),
"perl.formatting.verticalAlignment" => json!(config.perltidy_vertical_alignment),
"perl.formatting.blockCommentIndentation" => json!(config.perltidy_block_comment_indentation),
"perl.formatting.extraArgs" => json!(config.perltidy_extra_args),
"perl.formatting.timeoutSecs" => json!(config.perltidy_timeout_secs),
```

And `crates/perl-lsp/src/runtime/language/formatting.rs` lines 29-45 shows `build_perltidy_config()` reads from `self.config.lock()`.

### 3. VS Code extension does NOT currently wire any perltidy settings to LSP server

**Research agent implied**: The gap is that VS Code settings aren't being sent to `FormattingProvider`.

**Reality**: The VS Code extension sends NOTHING for perltidy. It only sends perlcritic config. There is NO `buildPerlTidyConfiguration` function.

**Evidence**: `vscode-extension/src/extension.ts` lines 143-189 — only `buildPerlCriticConfiguration` exists. Search for `buildPerlTidyConfiguration` returns nothing.

The `perl-lsp.perltidyConfig` setting is only used in error messages (`formattingErrors.ts` line 42: "or set perl-lsp.perltidyConfig to your config path"), not sent to the LSP server.

---

## New Findings

### 1. The VS Code → LSP server settings transformation pattern

The perlcritic settings use this pattern:
- VS Code setting: `perl-lsp.perlcritic.enabled`, `perl-lsp.perlcritic.severity`, `perl-lsp.perlcritic.profile`
- Sent via `didChangeConfiguration` as: `perl.perlcritic.enabled`, `perl.perlcritic.severity`, `perl.perlcritic.profile`

A perltidy implementation would follow the same pattern:
- VS Code setting: `perl-lsp.perltidy.enabled`, `perl-lsp.perltidy.maximumLineLength`, etc.
- Sent via `didChangeConfiguration` as: `perl.formatting.enabled`, `perl.formatting.maximumLineLength`, etc.

### 2. `extra_args` is how built-in profiles are implemented on the Rust side

`PerlTidyConfig::pbp()` does NOT set `profile: Some(...)`. It sets:
```rust
extra_args: vec!["--perl-best-practices".to_string()],
```

The `to_args()` method appends `extra_args` to the perltidy command line.

### 3. Precedence behavior when `profile` is set

When `profile` is set to a `.perltidyrc` path, `to_args()` returns ONLY `--profile=<path>` and immediately returns. ALL other options (maximum_line_length, indent_columns, tabs, etc.) are ignored. This is perltidy's native behavior.

This means:
- If a user specifies a custom `.perltidyrc` path via `profile`, individual VS Code settings for indentation/line-length/etc. will have NO effect
- The plan's Risk #1 ("Settings migration") is even more critical than stated — if `perltidyConfig` path is set AND individual options are also set, the individual options are silently ignored

### 4. `perl-lsp.perltidyConfig` in VS Code is separate from `perl.formatting.profile` in LSP

The VS Code extension's `perl-lsp.perltidyConfig` is read locally (for error messages) but NOT sent to the LSP server. The LSP server's `perl.formatting.profile` must be set via `didChangeConfiguration` if a user wants to specify a profile path through VS Code settings.

---

## Scope Assessment

**Issue title**: "IDE Gap: Missing perltidy profile editor configuration"

**Actual scope**:
- Issue asks for VS Code commands + settings UI for perltidy profiles
- Research agent correctly identified this is a VS Code extension gap
- BUT research agent missed that the LSP server already has ALL perltidy settings wired under `perl.formatting.*`
- The actual work is: (1) add VS Code settings under `perl-lsp.perltidy.*`, (2) create `buildPerlTidyConfiguration` to transform to `perl.formatting.*`, (3) send via `didChangeConfiguration`, (4) add profile-selection command

**Affects files**:
- `vscode-extension/package.json` — add new settings
- `vscode-extension/src/extension.ts` — add `buildPerlTidyConfiguration`, `syncPerlTidyConfiguration`, `selectPerlTidyProfile` command, wire to status menu
- Potentially `perl-lsp-config` if a new "builtin profile" field needs to be added to the config struct (for proper PBP/GNU quick-select)

**NOT in scope (as stated)**:
- Live preview WebView panel — correct
- `.perltidyrc` syntax highlighting — correct
- Rust-side changes to `perl-lsp-perltidy` — correct (already complete)

---

## Verification Methodology

1. **PerlTidyConfig struct**: Read `crates/perl-lsp-perltidy/src/lib.rs` lines 1-180, confirmed all fields and `to_args()` behavior
2. **FormattingProvider**: Read `crates/perl-lsp-formatting/src/formatting.rs` lines 1-217, confirmed builder pattern
3. **VS Code settings**: Read `vscode-extension/package.json` lines 265-325, confirmed only `perl-lsp.perltidyConfig` (path) exists
4. **VS Code extension wiring**: Searched `vscode-extension/src/extension.ts` for `buildPerlTidyConfiguration` and `workspace.didChangeConfiguration` — found only perlcritic wiring, no perltidy wiring
5. **LSP server settings interface**: Read `crates/perl-lsp/src/runtime/workspace.rs` lines 600-760, confirmed all `perl.formatting.*` settings exist
6. **LSP server perltidy config builder**: Read `crates/perl-lsp/src/runtime/language/formatting.rs` lines 29-45, confirmed `build_perltidy_config()` reads from `self.config.lock()`
7. **Config struct definition**: Searched `crates/perl-lsp-config/src/lib.rs` for `perltidy` fields, confirmed all settings with comments clarifying `perltidy_profile` is a path, not a profile name
