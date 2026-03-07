# Performance Tuning Guide

This guide helps you optimize the performance of perl-lsp for your specific use case and environment.

## Table of Contents

- [Overview](#overview)
- [Performance Targets](#performance-targets)
- [Common Performance Issues](#common-performance-issues)
- [Tuning Strategies](#tuning-strategies)
  - [Workspace Size](#workspace-size)
  - [Hardware Resources](#hardware-resources)
  - [Network Filesystems](#network-filesystems)
  - [Large Files](#large-files)
  - [Memory Constraints](#memory-constraints)
- [Configuration Tuning](#configuration-tuning)
- [Editor-Specific Tuning](#editor-specific-tuning)
- [Monitoring and Diagnostics](#monitoring-and-diagnostics)
- [Benchmarking](#benchmarking)

---

## Overview

perl-lsp is designed for high performance with sub-millisecond incremental parsing and fast LSP operations. However, performance can vary based on:

- **Workspace size** - Number of files and symbols
- **Hardware resources** - CPU, memory, disk I/O
- **Filesystem type** - Local vs. network filesystems
- **Editor configuration** - LSP client settings
- **Perl code complexity** - Module depth, symbol count

This guide provides strategies to optimize performance for different scenarios.

---

## Performance Targets

### Response Time Targets

| Operation | P50 Target | P95 Target | P99 Target | Hard Limit |
|-----------|------------|------------|------------|------------|
| `textDocument/hover` | 5ms | 20ms | 50ms | 100ms |
| `textDocument/definition` | 10ms | 30ms | 75ms | 150ms |
| `textDocument/completion` | 20ms | 50ms | 100ms | 200ms |
| `textDocument/references` | 50ms | 100ms | 250ms | 500ms |
| `textDocument/documentSymbol` | 10ms | 25ms | 50ms | 100ms |
| `textDocument/semanticTokens` | 20ms | 50ms | 100ms | 200ms |
| `textDocument/formatting` | 200ms | 500ms | 1000ms | 2000ms |
| `textDocument/rename` | 100ms | 250ms | 500ms | 1000ms |
| `workspace/symbol` | 20ms | 50ms | 150ms | 300ms |

### Incremental Parsing

| Operation | Target | Hard Limit |
|-----------|--------|------------|
| Parse update | 1ms | 5ms |
| Syntax tree rebuild | 10ms | 50ms |
| Index update | 5ms | 20ms |

### Resource Usage

| Resource | Typical | Maximum |
|----------|---------|---------|
| Memory | 50-200MB | 500MB |
| CPU | 1-5% idle | 50% peak |
| Disk I/O | Minimal | Moderate (indexing) |

---

## Common Performance Issues

### 1. Slow Workspace Indexing

**Symptoms:**
- Long startup time (>10s)
- High CPU usage during indexing
- Delayed symbol availability

**Causes:**
- Large workspace (>10,000 files)
- Network filesystem (NFS, SMB)
- Slow disk I/O
- Deep module nesting

**Solutions:**
- Reduce `maxIndexedFiles`
- Increase `workspaceScanDeadlineMs`
- Exclude non-Perl directories
- Use local filesystem when possible

### 2. Slow Completion

**Symptoms:**
- Delayed completion suggestions
- UI lag while typing
- High memory usage

**Causes:**
- Too many completion items
- Complex symbol resolution
- Slow module resolution

**Solutions:**
- Reduce `completionCap`
- Disable `useSystemInc`
- Reduce `resolutionTimeout`
- Enable completion caching

### 3. Slow Go-to-Definition

**Symptoms:**
- Delayed navigation
- Timeout errors
- Inconsistent results

**Causes:**
- Large symbol index
- Network filesystem latency
- Complex module resolution

**Solutions:**
- Reduce `referencesCap`
- Pre-index frequently used modules
- Use dual-indexing strategy
- Enable incremental parsing

### 4. High Memory Usage

**Symptoms:**
- Memory growth over time
- Out-of-memory errors
- System slowdown

**Causes:**
- Large AST cache
- Too many indexed files
- Memory leaks (unlikely, but possible)

**Solutions:**
- Reduce `astCacheMaxEntries`
- Reduce `maxIndexedFiles`
- Reduce `maxTotalSymbols`
- Restart LSP server periodically

---

## Tuning Strategies

### Workspace Size

#### Small Workspaces (<100 files)

**Characteristics:**
- Fast indexing
- Low memory usage
- All features available

**Recommended Configuration:**

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 1000,
      "maxTotalSymbols": 100000,
      "astCacheMaxEntries": 100
    },
    "workspace": {
      "useSystemInc": true,
      "resolutionTimeout": 100
    }
  }
}
```

**Performance:** Excellent, all features enabled

---

#### Medium Workspaces (100-10,000 files)

**Characteristics:**
- Moderate indexing time
- Balanced memory usage
- Most features available

**Recommended Configuration:**

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 10000,
      "maxTotalSymbols": 500000,
      "astCacheMaxEntries": 100
    },
    "workspace": {
      "useSystemInc": false,
      "resolutionTimeout": 50
    }
  }
}
```

**Performance:** Good, some features may be limited

---

#### Large Workspaces (10,000-100,000 files)

**Characteristics:**
- Long indexing time
- Higher memory usage
- Some features disabled

**Recommended Configuration:**

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 50000,
      "maxTotalSymbols": 1000000,
      "astCacheMaxEntries": 50,
      "workspaceSymbolCap": 100,
      "referencesCap": 200,
      "completionCap": 50
    },
    "workspace": {
      "useSystemInc": false,
      "resolutionTimeout": 25
    },
    "inlayHints": {
      "enabled": false
    }
  }
}
```

**Performance:** Acceptable, some features disabled

---

#### Very Large Workspaces (>100,000 files)

**Characteristics:**
- Very long indexing time
- High memory usage
- Many features disabled

**Recommended Configuration:**

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 20000,
      "maxTotalSymbols": 500000,
      "astCacheMaxEntries": 25,
      "workspaceSymbolCap": 50,
      "referencesCap": 100,
      "completionCap": 30,
      "workspaceScanDeadlineMs": 60000,
      "referenceSearchDeadlineMs": 3000
    },
    "workspace": {
      "useSystemInc": false,
      "resolutionTimeout": 25,
      "includePaths": ["lib", "src"]
    },
    "inlayHints": {
      "enabled": false
    },
    "testRunner": {
      "enabled": false
    }
  }
}
```

**Performance:** Limited, consider splitting workspace

---

### Hardware Resources

#### Low-End Hardware (2 cores, 4GB RAM)

**Characteristics:**
- Limited CPU resources
- Limited memory
- Slower disk I/O

**Recommended Configuration:**

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 5000,
      "maxTotalSymbols": 250000,
      "astCacheMaxEntries": 25,
      "workspaceSymbolCap": 50,
      "referencesCap": 100,
      "completionCap": 30
    },
    "workspace": {
      "useSystemInc": false,
      "resolutionTimeout": 25
    },
    "inlayHints": {
      "enabled": false
    },
    "testRunner": {
      "enabled": false
    }
  }
}
```

**Performance:** Functional, some features disabled

---

#### Mid-Range Hardware (4 cores, 8GB RAM)

**Characteristics:**
- Adequate CPU resources
- Adequate memory
- Good disk I/O

**Recommended Configuration:**

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 10000,
      "maxTotalSymbols": 500000,
      "astCacheMaxEntries": 100,
      "workspaceSymbolCap": 200,
      "referencesCap": 500,
      "completionCap": 100
    },
    "workspace": {
      "useSystemInc": false,
      "resolutionTimeout": 50
    },
    "inlayHints": {
      "enabled": true
    }
  }
}
```

**Performance:** Good, most features available

---

#### High-End Hardware (8+ cores, 16GB+ RAM)

**Characteristics:**
- Abundant CPU resources
- Abundant memory
- Fast disk I/O (SSD/NVMe)

**Recommended Configuration:**

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 50000,
      "maxTotalSymbols": 2000000,
      "astCacheMaxEntries": 200,
      "workspaceSymbolCap": 500,
      "referencesCap": 1000,
      "completionCap": 200
    },
    "workspace": {
      "useSystemInc": true,
      "resolutionTimeout": 100
    },
    "inlayHints": {
      "enabled": true,
      "chainedHints": true
    },
    "testRunner": {
      "enabled": true
    }
  }
}
```

**Performance:** Excellent, all features enabled

---

### Network Filesystems

#### NFS/SMB

**Characteristics:**
- High latency
- Slow metadata operations
- Inconsistent performance

**Recommended Configuration:**

```json
{
  "perl": {
    "workspace": {
      "useSystemInc": false,
      "resolutionTimeout": 25,
      "includePaths": ["lib", "src"]
    },
    "limits": {
      "maxIndexedFiles": 5000,
      "maxTotalSymbols": 250000,
      "astCacheMaxEntries": 50,
      "workspaceScanDeadlineMs": 60000
    },
    "inlayHints": {
      "enabled": false
    }
  }
}
```

**Additional Recommendations:**
- Use local cache when possible
- Exclude remote directories from indexing
- Increase timeouts
- Disable real-time indexing

---

#### SSHFS

**Characteristics:**
- Very high latency
- Very slow metadata operations
- Unreliable connection

**Recommended Configuration:**

```json
{
  "perl": {
    "workspace": {
      "useSystemInc": false,
      "resolutionTimeout": 25,
      "includePaths": ["lib"]
    },
    "limits": {
      "maxIndexedFiles": 1000,
      "maxTotalSymbols": 100000,
      "astCacheMaxEntries": 25,
      "workspaceScanDeadlineMs": 120000
    },
    "inlayHints": {
      "enabled": false
    },
    "testRunner": {
      "enabled": false
    }
  }
}
```

**Additional Recommendations:**
- Use local workspace when possible
- Disable indexing completely
- Use minimal configuration
- Consider alternative: edit locally, sync remotely

---

### Large Files

#### Files >10,000 lines

**Characteristics:**
- Slow parsing
- High memory usage
- Timeout errors

**Recommended Configuration:**

```json
{
  "perl": {
    "limits": {
      "astCacheMaxEntries": 10,
      "workspaceScanDeadlineMs": 120000
    },
    "workspace": {
      "resolutionTimeout": 100
    }
  }
}
```

**Additional Recommendations:**
- Split large files into smaller modules
- Use incremental parsing
- Exclude large files from indexing
- Consider using `# perl-lsp-disable` comments

---

### Memory Constraints

#### <2GB Available Memory

**Recommended Configuration:**

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 1000,
      "maxTotalSymbols": 50000,
      "astCacheMaxEntries": 10,
      "workspaceSymbolCap": 50,
      "referencesCap": 100,
      "completionCap": 30
    },
    "workspace": {
      "useSystemInc": false,
      "resolutionTimeout": 25
    },
    "inlayHints": {
      "enabled": false
    },
    "testRunner": {
      "enabled": false
    }
  }
}
```

**Additional Recommendations:**
- Restart LSP server periodically
- Disable unnecessary features
- Use minimal configuration
- Monitor memory usage

---

## Configuration Tuning

### Performance-Optimized Configuration

For general use with good performance:

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5"],
      "useSystemInc": false,
      "resolutionTimeout": 50
    },
    "inlayHints": {
      "enabled": true,
      "parameterHints": true,
      "typeHints": true,
      "chainedHints": false,
      "maxLength": 30
    },
    "limits": {
      "workspaceSymbolCap": 200,
      "referencesCap": 500,
      "completionCap": 100,
      "astCacheMaxEntries": 100,
      "maxIndexedFiles": 10000,
      "maxTotalSymbols": 500000,
      "workspaceScanDeadlineMs": 30000,
      "referenceSearchDeadlineMs": 2000
    }
  }
}
```

### Minimal Configuration

For maximum performance with minimal features:

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
      "workspaceSymbolCap": 50,
      "referencesCap": 100,
      "completionCap": 30,
      "astCacheMaxEntries": 25,
      "maxIndexedFiles": 5000,
      "maxTotalSymbols": 250000,
      "workspaceScanDeadlineMs": 20000,
      "referenceSearchDeadlineMs": 1500
    }
  }
}
```

### Development Configuration

For maximum features with acceptable performance:

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", "t/lib", "local/lib/perl5", "vendor/lib"],
      "useSystemInc": true,
      "resolutionTimeout": 100
    },
    "inlayHints": {
      "enabled": true,
      "parameterHints": true,
      "typeHints": true,
      "chainedHints": true,
      "maxLength": 50
    },
    "testRunner": {
      "enabled": true,
      "command": "prove",
      "args": ["-l", "-v", "--timer"],
      "timeout": 120000
    },
    "limits": {
      "workspaceSymbolCap": 500,
      "referencesCap": 1000,
      "completionCap": 200,
      "astCacheMaxEntries": 200,
      "maxIndexedFiles": 50000,
      "maxTotalSymbols": 2000000,
      "workspaceScanDeadlineMs": 60000,
      "referenceSearchDeadlineMs": 3000
    }
  }
}
```

---

## Editor-Specific Tuning

### VS Code

```json
{
  "perl-lsp.trace.server": "off",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.formatOnSave": false,
  "perl-lsp.enableRefactoring": true,
  "perl-lsp.includePaths": ["lib", ".", "local/lib/perl5"],
  "perl-lsp.useSystemInc": false,
  "perl-lsp.resolutionTimeout": 50,
  "perl-lsp.workspaceSymbolCap": 200,
  "perl-lsp.referencesCap": 500,
  "perl-lsp.completionCap": 100
}
```

**Additional VS Code Settings:**

```json
{
  "editor.quickSuggestions": {
    "other": true,
    "comments": false,
    "strings": false
  },
  "editor.suggest.showStatusBar": true,
  "editor.suggestSelection": "first",
  "editor.wordBasedSuggestions": true,
  "editor.formatOnSave": false,
  "editor.codeActionsOnSave": {
    "source.fixAll": false
  }
}
```

---

### Neovim

```lua
lspconfig.perl_lsp.setup({
  settings = {
    perl = {
      workspace = {
        includePaths = { "lib", ".", "local/lib/perl5" },
        useSystemInc = false,
        resolutionTimeout = 50,
      },
      inlayHints = {
        enabled = true,
        parameterHints = true,
        typeHints = true,
      },
      limits = {
        workspaceSymbolCap = 200,
        referencesCap = 500,
        completionCap = 100,
      },
    },
  },
  capabilities = require('cmp_nvim_lsp').default_capabilities(),
})
```

**Additional Neovim Settings:**

```lua
-- Disable unused features
vim.lsp.handlers['textDocument/publishDiagnostics'] = function() end

-- Reduce debounce
vim.o.updatetime = 100

-- Disable semantic tokens if slow
vim.lsp.semantic_tokens.enable = false
```

---

### Emacs

```elisp
(setq-default eglot-workspace-configuration
  '((perl
     (workspace
      (includePaths . ["lib" "." "local/lib/perl5"])
      (useSystemInc . :json-false)
      (resolutionTimeout . 50))
     (inlayHints
      (enabled . t)
      (parameterHints . t)
      (typeHints . t))
     (limits
      (workspaceSymbolCap . 200)
      (referencesCap . 500)
      (completionCap . 100)))))

;; Reduce idle delay
(setq lsp-idle-delay 0.5)
(setq lsp-completion-idle-delay 0.2)
```

---

### Helix

```toml
[language-server.perl-lsp.config.perl]
workspace.includePaths = ["lib", ".", "local/lib/perl5"]
workspace.useSystemInc = false
workspace.resolutionTimeout = 50
inlayHints.enabled = true
inlayHints.parameterHints = true
inlayHints.typeHints = true
limits.workspaceSymbolCap = 200
limits.referencesCap = 500
limits.completionCap = 100
```

---

## Monitoring and Diagnostics

### Enable Logging

```bash
# Enable debug logging
RUST_LOG=perl_lsp=debug perl-lsp --stdio

# Enable trace logging
RUST_LOG=perl_lsp=trace perl-lsp --stdio
```

### Monitor Performance

**VS Code:**
- Open Output panel → "Perl Language Server"
- Look for timing information in logs

**Neovim:**
```vim
:LspInfo
:lua print(vim.inspect(vim.lsp.get_log_path()))
```

**Emacs:**
```elisp
M-x lsp-workspace-show-log
```

**Helix:**
- Check `~/.cache/helix/helix.log`

### Performance Metrics

Key metrics to monitor:

1. **Response times** - P50, P95, P99 percentiles
2. **Memory usage** - Resident set size, heap usage
3. **CPU usage** - Idle vs. peak usage
4. **Disk I/O** - Read/write operations
5. **Network I/O** - For remote filesystems

### Common Bottlenecks

1. **Module resolution** - Slow filesystem, network latency
2. **Symbol indexing** - Large workspaces, deep nesting
3. **Completion** - Too many items, complex resolution
4. **Formatting** - External tool (perltidy) overhead
5. **Diagnostics** - Complex syntax analysis

---

## Benchmarking

### Manual Benchmarking

```bash
# Measure startup time
time perl-lsp --stdio < /dev/null

# Measure parsing time
echo '{"jsonrpc":"2.0","id":1,"method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///test.pl","languageId":"perl","version":1,"text":"my $x = 1;"}}}' | time perl-lsp --stdio
```

### Automated Benchmarking

Use the provided benchmark suite:

```bash
# Run performance benchmarks
cargo bench -p perl-parser

# Compare with previous runs
cargo bench -p perl-parser -- --save-baseline main
cargo bench -p perl-parser -- --baseline main
```

### Benchmarking Checklist

- [ ] Measure cold startup time
- [ ] Measure warm startup time
- [ ] Measure incremental parsing time
- [ ] Measure completion latency
- [ ] Measure go-to-definition latency
- [ ] Measure find-references latency
- [ ] Measure memory usage over time
- [ ] Measure CPU usage during operations
- [ ] Measure disk I/O during indexing
- [ ] Measure network I/O for remote filesystems

---

## See Also

- [Configuration Schema](CONFIGURATION_SCHEMA.md) - Complete configuration reference
- [Performance SLO](PERFORMANCE_SLO.md) - Service level objectives
- [Troubleshooting Guide](TROUBLESHOOTING.md) - Common issues and solutions
- [Editor Setup](EDITOR_SETUP.md) - Editor-specific configuration
