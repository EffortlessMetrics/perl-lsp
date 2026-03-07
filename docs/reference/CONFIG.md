# Configuration Reference

This document is the authoritative list of all configuration keys for the Perl LSP server.

## Table of Contents

- [Configuration Format](#configuration-format)
- [Workspace Settings](#workspace-settings)
- [Inlay Hints](#inlay-hints)
- [Test Runner](#test-runner)
- [Resource Limits](#resource-limits)
- [Execute Commands](#execute-commands)
- [VS Code Extension Settings](#vs-code-extension-settings)
- [DAP Settings](#dap-settings)
- [Environment Variables](#environment-variables)
- [Example Configurations](#example-configurations)

---

## Configuration Format

Settings are provided via LSP `workspace/didChangeConfiguration` or `initializationOptions`.

All LSP server settings are under the `perl` namespace:

```json
{
  "perl": {
    "workspace": { ... },
    "inlayHints": { ... },
    "testRunner": { ... },
    "limits": { ... }
  }
}
```

---

## Workspace Settings

Configuration for module resolution and workspace behavior.

### `perl.workspace.includePaths`

| Property | Value |
|----------|-------|
| Type | `string[]` |
| Default | `["lib", ".", "local/lib/perl5"]` |
| Source | `crates/perl-parser/src/lsp/state/config.rs` |

Directories to search for Perl modules, relative to the workspace root.

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
    }
  }
}
```

### `perl.workspace.useSystemInc`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `false` |
| Source | `crates/perl-parser/src/lsp/state/config.rs` |

Whether to include system `@INC` paths in module resolution. Disabled by default to avoid blocking on network filesystems.

**Security Note:** The current directory (`.`) is filtered from system `@INC` to prevent injection attacks.

```json
{
  "perl": {
    "workspace": {
      "useSystemInc": true
    }
  }
}
```

### `perl.workspace.resolutionTimeout`

| Property | Value |
|----------|-------|
| Type | `number` (milliseconds) |
| Default | `50` |
| Source | `crates/perl-parser/src/lsp/state/config.rs` |

Maximum time to spend resolving a module path. Prevents blocking on slow/network filesystems.

```json
{
  "perl": {
    "workspace": {
      "resolutionTimeout": 100
    }
  }
}
```

---

## Inlay Hints

Configuration for inlay hints displayed in the editor.

### `perl.inlayHints.enabled`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |
| Source | `crates/perl-parser/src/lsp/state/config.rs` |

Enable or disable all inlay hints.

### `perl.inlayHints.parameterHints`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |
| Source | `crates/perl-parser/src/lsp/state/config.rs` |

Show parameter name hints in function calls.

```perl
# With parameterHints enabled:
some_function(/* name: */ "value", /* count: */ 42);
```

### `perl.inlayHints.typeHints`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |
| Source | `crates/perl-parser/src/lsp/state/config.rs` |

Show inferred type hints for variables.

### `perl.inlayHints.chainedHints`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `false` |
| Source | `crates/perl-parser/src/lsp/state/config.rs` |

Show hints for chained method calls.

### `perl.inlayHints.maxLength`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `30` |
| Source | `crates/perl-parser/src/lsp/state/config.rs` |

Maximum length of inlay hint text before truncation.

---

## Test Runner

Configuration for integrated test execution.

### `perl.testRunner.enabled`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |
| Source | `crates/perl-parser/src/lsp/state/config.rs` |

Enable the integrated test runner.

### `perl.testRunner.command`

| Property | Value |
|----------|-------|
| Type | `string` |
| Default | `"perl"` |
| Source | `crates/perl-parser/src/lsp/state/config.rs` |

Command to run tests.

### `perl.testRunner.args`

| Property | Value |
|----------|-------|
| Type | `string[]` |
| Default | `[]` |
| Source | `crates/perl-parser/src/lsp/state/config.rs` |

Additional arguments to pass to the test command.

```json
{
  "perl": {
    "testRunner": {
      "command": "prove",
      "args": ["-l", "-v"]
    }
  }
}
```

### `perl.testRunner.timeout`

| Property | Value |
|----------|-------|
| Type | `number` (milliseconds) |
| Default | `60000` |
| Source | `crates/perl-parser/src/lsp/state/config.rs` |

Maximum time to wait for test execution.

---

## Resource Limits

Configuration for bounded behavior and performance tuning.

### Result Caps

#### `perl.limits.workspaceSymbolCap`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `200` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum number of results from `workspace/symbol` requests.

#### `perl.limits.referencesCap`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `500` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum number of results from `textDocument/references` requests.

#### `perl.limits.completionCap`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `100` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum number of completion items.

#### `perl.limits.documentSymbolCap`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `500` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum number of results from `textDocument/documentSymbol` requests.

#### `perl.limits.codeLensCap`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `100` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum number of code lens items per file.

#### `perl.limits.diagnosticsPerFileCap`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `200` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum diagnostics per file.

#### `perl.limits.inlayHintsCap`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `500` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum inlay hints per file.

### Cache Limits

#### `perl.limits.astCacheMaxEntries`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `100` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum number of parsed ASTs to cache. Reduces memory usage at the cost of re-parsing.

#### `perl.limits.astCacheTtlSecs`

| Property | Value |
|----------|-------|
| Type | `number` (seconds) |
| Default | `300` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

AST cache time-to-live in seconds (5 minutes default).

#### `perl.limits.symbolCacheMaxEntries`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `1000` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum symbol cache entries.

### Index Limits

#### `perl.limits.maxIndexedFiles`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `10000` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum number of files to index for workspace features.

#### `perl.limits.maxSymbolsPerFile`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `5000` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum symbols per file.

#### `perl.limits.maxTotalSymbols`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `500000` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum total symbols to store in the workspace index.

#### `perl.limits.parseStormThreshold`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `10` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Pending parses before degradation kicks in.

### Deadline Settings

#### `perl.limits.workspaceScanDeadlineMs`

| Property | Value |
|----------|-------|
| Type | `number` (milliseconds) |
| Default | `30000` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum time for initial workspace folder scan.

#### `perl.limits.fileIndexDeadlineMs`

| Property | Value |
|----------|-------|
| Type | `number` (milliseconds) |
| Default | `5000` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum time for single file indexing.

#### `perl.limits.referenceSearchDeadlineMs`

| Property | Value |
|----------|-------|
| Type | `number` (milliseconds) |
| Default | `2000` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum time for reference search operations.

#### `perl.limits.regexScanDeadlineMs`

| Property | Value |
|----------|-------|
| Type | `number` (milliseconds) |
| Default | `1000` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum time for regex scan operations.

#### `perl.limits.fsOperationDeadlineMs`

| Property | Value |
|----------|-------|
| Type | `number` (milliseconds) |
| Default | `500` |
| Source | `crates/perl-parser/src/lsp/state/limits.rs` |

Maximum time for filesystem operations.

---

## Execute Commands

The following commands are registered with the LSP server via `executeCommandProvider`.

| Command | Description | Source |
|---------|-------------|--------|
| `perl.runTests` | Execute Perl test files with TAP output parsing | `crates/perl-parser/src/execute_command.rs` |
| `perl.runFile` | Execute single Perl file with output capture | `crates/perl-parser/src/execute_command.rs` |
| `perl.runTestSub` | Execute specific test subroutine with isolation | `crates/perl-parser/src/execute_command.rs` |
| `perl.debugTests` | Debug test execution with breakpoint support | `crates/perl-parser/src/execute_command.rs` |
| `perl.runCritic` | Perl::Critic analysis with dual analyzer strategy | `crates/perl-parser/src/execute_command.rs` |

### Invoking Commands

Commands are invoked via `workspace/executeCommand`:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "workspace/executeCommand",
  "params": {
    "command": "perl.runCritic",
    "arguments": ["file:///path/to/script.pl"]
  }
}
```

---

## VS Code Extension Settings

These settings are specific to the VS Code extension (`vscode-extension/package.json`).

### `perl-lsp.serverPath`

| Property | Value |
|----------|-------|
| Type | `string` |
| Default | `""` |

Absolute path to `perl-lsp` binary. Leave empty to auto-download latest release.

### `perl-lsp.autoDownload`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |

Automatically download `perl-lsp` binary if not found locally.

### `perl-lsp.downloadBaseUrl`

| Property | Value |
|----------|-------|
| Type | `string` |
| Default | `""` |

Base URL for internal binary hosting. When set, downloads from this location instead of GitHub releases.

### `perl-lsp.channel`

| Property | Value |
|----------|-------|
| Type | `string` |
| Enum | `"latest"`, `"stable"`, `"tag"` |
| Default | `"latest"` |

Release channel selection.

### `perl-lsp.versionTag`

| Property | Value |
|----------|-------|
| Type | `string` |
| Default | `""` |

Specific release tag (e.g., `v0.8.3`) when channel is set to `tag`.

### `perl-lsp.trace.server`

| Property | Value |
|----------|-------|
| Type | `string` |
| Enum | `"off"`, `"messages"`, `"verbose"` |
| Default | `"off"` |

Trace server communication for debugging.

### `perl-lsp.enableDiagnostics`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |

Enable real-time syntax diagnostics.

### `perl-lsp.enableSemanticTokens`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |

Enable semantic syntax highlighting.

### `perl-lsp.enableFormatting`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |

Enable document formatting.

### `perl-lsp.formatOnSave`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `false` |

Format document on save.

### `perl-lsp.enableRefactoring`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |

Enable advanced refactoring features.

### `perl-lsp.perltidyConfig`

| Property | Value |
|----------|-------|
| Type | `string` |
| Default | `""` |

Path to `.perltidyrc` configuration file.

### `perl-lsp.includePaths`

| Property | Value |
|----------|-------|
| Type | `string[]` |
| Default | `["lib", "local/lib/perl5"]` |

Additional paths to search for Perl modules.

### `perl-lsp.enableTestIntegration`

| Property | Value |
|----------|-------|
| Type | `boolean` |
| Default | `true` |

Enable Test::More and Test2 integration.

---

## DAP Settings

Debug Adapter Protocol settings (from `perl-dap`).

### `perl.dap.evaluateTimeout`

| Property | Value |
|----------|-------|
| Type | `number` (seconds) |
| Default | `5` |

Timeout for evaluate requests during debugging.

### `perl.dap.evaluateMaxDepth`

| Property | Value |
|----------|-------|
| Type | `number` |
| Default | `10` |

Maximum recursion depth for evaluate requests.

### `perl.dap.stepTimeout`

| Property | Value |
|----------|-------|
| Type | `number` (seconds) |
| Default | `30` |

Timeout for step operations.

### `perl.dap.continueTimeout`

| Property | Value |
|----------|-------|
| Type | `number` (seconds) |
| Default | `300` |

Timeout for continue operations (5 minutes).

---

## Environment Variables

### `PERL_LSP_INCREMENTAL`

Enable incremental text synchronization (requires `incremental` feature).

```bash
PERL_LSP_INCREMENTAL=1 perl-lsp --stdio
```

### `RUST_LOG`

Enable debug logging for troubleshooting.

```bash
RUST_LOG=perl_lsp=debug perl-lsp --stdio
RUST_LOG=perl_parser=trace perl-lsp --stdio
```

### `RUST_TEST_THREADS`

Control threading for test execution (useful in CI environments).

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp
```

---

## Example Configurations

### Small Project (Default)

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", "."]
    },
    "inlayHints": {
      "enabled": true
    }
  }
}
```

### Large Codebase (10K+ files)

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5"],
      "useSystemInc": false,
      "resolutionTimeout": 100
    },
    "limits": {
      "workspaceSymbolCap": 300,
      "referencesCap": 1000,
      "maxIndexedFiles": 50000,
      "maxTotalSymbols": 2000000,
      "workspaceScanDeadlineMs": 120000
    }
  }
}
```

### Resource-Constrained Environment

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib"],
      "useSystemInc": false,
      "resolutionTimeout": 25
    },
    "inlayHints": {
      "enabled": false
    },
    "limits": {
      "workspaceSymbolCap": 100,
      "referencesCap": 200,
      "astCacheMaxEntries": 50,
      "maxIndexedFiles": 5000,
      "referenceSearchDeadlineMs": 1000
    }
  }
}
```

### CI/Testing Environment

```json
{
  "perl": {
    "workspace": {
      "useSystemInc": false
    },
    "testRunner": {
      "enabled": true,
      "command": "prove",
      "args": ["-l", "-v", "--timer"],
      "timeout": 300000
    }
  }
}
```

---

## Configuration Validation

You can verify configuration is being applied by enabling trace logging:

```bash
RUST_LOG=perl_parser::lsp::state=debug perl-lsp --stdio
```

---

## See Also

- [EDITOR_SETUP.md](EDITOR_SETUP.md) - Editor-specific configuration guides
- [PERFORMANCE_SLO.md](PERFORMANCE_SLO.md) - Performance targets and limits
- [LSP_FEATURES.md](LSP_FEATURES.md) - Supported LSP features
- [THREADING_CONFIGURATION_GUIDE.md](THREADING_CONFIGURATION_GUIDE.md) - Advanced threading options
