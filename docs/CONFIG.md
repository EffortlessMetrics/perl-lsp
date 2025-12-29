# Configuration Reference

This document describes all available configuration options for the Perl LSP server.

## Table of Contents

- [Configuration Format](#configuration-format)
- [Workspace Settings](#workspace-settings)
- [Inlay Hints](#inlay-hints)
- [Test Runner](#test-runner)
- [Resource Limits](#resource-limits)
- [Environment Variables](#environment-variables)
- [Example Configurations](#example-configurations)

---

## Configuration Format

Settings are provided via LSP `workspace/didChangeConfiguration` or `initializationOptions`.

All settings are under the `perl` namespace:

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

**Type:** `string[]`
**Default:** `["lib", ".", "local/lib/perl5"]`

Directories to search for Perl modules, relative to the workspace root.

```json
{
  "perl.workspace.includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
}
```

### `perl.workspace.useSystemInc`

**Type:** `boolean`
**Default:** `false`

Whether to include system `@INC` paths in module resolution. Disabled by default to avoid blocking on network filesystems.

```json
{
  "perl.workspace.useSystemInc": true
}
```

**Security Note:** The current directory (`.`) is filtered from system `@INC` to prevent injection attacks.

### `perl.workspace.resolutionTimeout`

**Type:** `number` (milliseconds)
**Default:** `50`

Maximum time to spend resolving a module path. Prevents blocking on slow/network filesystems.

```json
{
  "perl.workspace.resolutionTimeout": 100
}
```

---

## Inlay Hints

Configuration for inlay hints displayed in the editor.

### `perl.inlayHints.enabled`

**Type:** `boolean`
**Default:** `true`

Enable or disable all inlay hints.

### `perl.inlayHints.parameterHints`

**Type:** `boolean`
**Default:** `true`

Show parameter name hints in function calls.

```perl
# With parameterHints enabled:
some_function(/* name: */ "value", /* count: */ 42);
```

### `perl.inlayHints.typeHints`

**Type:** `boolean`
**Default:** `true`

Show inferred type hints for variables.

### `perl.inlayHints.chainedHints`

**Type:** `boolean`
**Default:** `false`

Show hints for chained method calls.

### `perl.inlayHints.maxLength`

**Type:** `number`
**Default:** `30`

Maximum length of inlay hint text before truncation.

---

## Test Runner

Configuration for integrated test execution.

### `perl.testRunner.enabled`

**Type:** `boolean`
**Default:** `true`

Enable the integrated test runner.

### `perl.testRunner.command`

**Type:** `string`
**Default:** `"perl"`

Command to run tests.

### `perl.testRunner.args`

**Type:** `string[]`
**Default:** `[]`

Additional arguments to pass to the test command.

```json
{
  "perl.testRunner.command": "prove",
  "perl.testRunner.args": ["-l", "-v"]
}
```

### `perl.testRunner.timeout`

**Type:** `number` (milliseconds)
**Default:** `60000`

Maximum time to wait for test execution.

---

## Resource Limits

Configuration for bounded behavior and performance tuning.

### Result Caps

#### `perl.limits.workspaceSymbolCap`

**Type:** `number`
**Default:** `200`

Maximum number of results from `workspace/symbol` requests.

#### `perl.limits.referencesCap`

**Type:** `number`
**Default:** `500`

Maximum number of results from `textDocument/references` requests.

#### `perl.limits.completionCap`

**Type:** `number`
**Default:** `100`

Maximum number of completion items.

### Cache Limits

#### `perl.limits.astCacheMaxEntries`

**Type:** `number`
**Default:** `100`

Maximum number of parsed ASTs to cache. Reduces memory usage at the cost of re-parsing.

### Index Limits

#### `perl.limits.maxIndexedFiles`

**Type:** `number`
**Default:** `10000`

Maximum number of files to index for workspace features.

#### `perl.limits.maxTotalSymbols`

**Type:** `number`
**Default:** `500000`

Maximum total symbols to store in the workspace index.

### Deadline Settings

#### `perl.limits.workspaceScanDeadlineMs`

**Type:** `number` (milliseconds)
**Default:** `30000`

Maximum time for initial workspace folder scan.

#### `perl.limits.referenceSearchDeadlineMs`

**Type:** `number` (milliseconds)
**Default:** `2000`

Maximum time for reference search operations.

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

## Schema

A JSON Schema for validation is available at:

```
https://raw.githubusercontent.com/EffortlessMetrics/tree-sitter-perl-rs/main/schema/perl-lsp-config.json
```

Use in VS Code:

```json
{
  "json.schemas": [
    {
      "fileMatch": ["settings.json"],
      "url": "https://raw.githubusercontent.com/EffortlessMetrics/tree-sitter-perl-rs/main/schema/perl-lsp-config.json"
    }
  ]
}
```

---

## See Also

- [EDITOR_SETUP.md](EDITOR_SETUP.md) - Editor-specific configuration guides
- [LSP_FEATURES.md](LSP_FEATURES.md) - Supported LSP features
- [THREADING_CONFIGURATION_GUIDE.md](THREADING_CONFIGURATION_GUIDE.md) - Advanced threading options
