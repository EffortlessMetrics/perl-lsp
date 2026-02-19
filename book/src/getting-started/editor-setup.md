# Editor Setup Guide

This guide provides copy/paste ready configurations for setting up the Perl LSP server with popular editors.

## Table of Contents

- [Prerequisites](#prerequisites)
- [VS Code](#vs-code)
- [Neovim](#neovim)
- [Emacs](#emacs)
- [Helix](#helix)
- [Sublime Text](#sublime-text)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Install the Server

```bash
# Option 1: Install from crates.io (recommended)
cargo install perl-lsp

# Option 2: Install from source
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd tree-sitter-perl-rs
cargo install --path crates/perl-lsp

# Option 3: Download pre-built binary
# See https://github.com/EffortlessMetrics/perl-lsp/releases
```

### Verify Installation

```bash
# Check binary is in PATH
which perl-lsp

# Check version
perl-lsp --version

# Quick health check
perl-lsp --health
```

---

## VS Code

### Option 1: Using Generic LSP Extension

Install a generic LSP client extension (e.g., "Generic LSP Client" or "vscode-lsp-wl").

Create or edit `.vscode/settings.json` in your workspace:

```json
{
  "languageServerExtensions.serverConfigurations": [
    {
      "id": "perl-lsp",
      "displayName": "Perl Language Server",
      "command": "perl-lsp",
      "args": ["--stdio"],
      "scope": "workspace",
      "fileEvents": ["**/*.pl", "**/*.pm", "**/*.t"]
    }
  ]
}
```

### Option 2: Using the Official Extension

Install from the VS Code marketplace:

```bash
code --install-extension effortlesssteven.perl-lsp
```

### Recommended Settings

Add to your `settings.json` (Ctrl+Shift+P -> "Preferences: Open Settings (JSON)"):

```json
{
  "perl-lsp.serverPath": "",
  "perl-lsp.autoDownload": true,
  "perl-lsp.trace.server": "off",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.formatOnSave": false,
  "perl-lsp.enableRefactoring": true,
  "perl-lsp.includePaths": ["lib", "local/lib/perl5"],
  "perl-lsp.enableTestIntegration": true
}
```

### Debug Logging

For troubleshooting, enable server tracing:

```json
{
  "perl-lsp.trace.server": "verbose"
}
```

---

## Neovim

### Using nvim-lspconfig

Add to your `init.lua`:

```lua
-- Define perl-lsp in lspconfig
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.perl_lsp then
  configs.perl_lsp = {
    default_config = {
      cmd = { 'perl-lsp', '--stdio' },
      filetypes = { 'perl' },
      root_dir = lspconfig.util.root_pattern(
        'Makefile.PL',
        'Build.PL',
        'cpanfile',
        'dist.ini',
        '.git'
      ),
      single_file_support = true,
      settings = {
        perl = {
          workspace = {
            includePaths = { 'lib', '.', 'local/lib/perl5' },
            useSystemInc = false,
            resolutionTimeout = 50,
          },
          inlayHints = {
            enabled = true,
            parameterHints = true,
            typeHints = true,
            chainedHints = false,
            maxLength = 30,
          },
          testRunner = {
            enabled = true,
            command = 'perl',
            args = {},
            timeout = 60000,
          },
          limits = {
            workspaceSymbolCap = 200,
            referencesCap = 500,
            completionCap = 100,
            astCacheMaxEntries = 100,
            maxIndexedFiles = 10000,
            maxTotalSymbols = 500000,
            workspaceScanDeadlineMs = 30000,
            referenceSearchDeadlineMs = 2000,
          },
        },
      },
    },
  }
end

-- Enable the server
lspconfig.perl_lsp.setup({
  on_attach = function(client, bufnr)
    -- Enable completion triggered by <c-x><c-o>
    vim.bo[bufnr].omnifunc = 'v:lua.vim.lsp.omnifunc'

    -- Buffer local keymaps
    local opts = { buffer = bufnr, noremap = true, silent = true }
    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
    vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
    vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
    vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
    vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, opts)
  end,
  capabilities = vim.lsp.protocol.make_client_capabilities(),
})
```

### Minimal Configuration

For a minimal setup:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.perl_lsp then
  configs.perl_lsp = {
    default_config = {
      cmd = { 'perl-lsp', '--stdio' },
      filetypes = { 'perl' },
      root_dir = lspconfig.util.root_pattern('.git'),
      single_file_support = true,
    },
  }
end

lspconfig.perl_lsp.setup({})
```

### With nvim-cmp Completion

```lua
lspconfig.perl_lsp.setup({
  capabilities = require('cmp_nvim_lsp').default_capabilities(),
})
```

---

## Emacs

### Using lsp-mode

Add to your Emacs configuration:

```elisp
(use-package lsp-mode
  :ensure t
  :hook ((cperl-mode . lsp-deferred)
         (perl-mode . lsp-deferred))
  :commands lsp
  :init
  (setq lsp-keymap-prefix "C-c l")
  :config
  ;; Register perl-lsp
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("perl-lsp" "--stdio"))
    :major-modes '(cperl-mode perl-mode)
    :priority -1
    :server-id 'perl-lsp
    :initialization-options
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
        (completionCap . 100)))))))

;; Optional: Enable lsp-ui for enhanced UI
(use-package lsp-ui
  :ensure t
  :hook (lsp-mode . lsp-ui-mode)
  :config
  (setq lsp-ui-doc-enable t
        lsp-ui-doc-show-with-cursor t
        lsp-ui-sideline-enable t))
```

### Using eglot (Emacs 29+)

```elisp
(use-package eglot
  :ensure t
  :hook ((cperl-mode . eglot-ensure)
         (perl-mode . eglot-ensure))
  :config
  (add-to-list 'eglot-server-programs
               '((cperl-mode perl-mode) . ("perl-lsp" "--stdio")))

  ;; Optional: Configure initialization options
  (setq-default eglot-workspace-configuration
    '((perl
       (workspace
        (includePaths . ["lib" "." "local/lib/perl5"])
        (useSystemInc . :json-false))
       (limits
        (workspaceSymbolCap . 200)
        (referencesCap . 500))))))
```

### Keybindings for Emacs

```elisp
;; Common LSP keybindings for perl-mode
(with-eval-after-load 'perl-mode
  (define-key perl-mode-map (kbd "M-.") 'xref-find-definitions)
  (define-key perl-mode-map (kbd "M-?") 'xref-find-references)
  (define-key perl-mode-map (kbd "C-c r") 'lsp-rename)
  (define-key perl-mode-map (kbd "C-c a") 'lsp-execute-code-action))
```

---

## Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "perl"
scope = "source.perl"
injection-regex = "perl"
file-types = ["pl", "pm", "t", "psgi"]
roots = ["Makefile.PL", "Build.PL", "cpanfile", "dist.ini", ".git"]
comment-token = "#"
indent = { tab-width = 4, unit = "    " }
language-servers = ["perl-lsp"]

[language-server.perl-lsp]
command = "perl-lsp"
args = ["--stdio"]

[language-server.perl-lsp.config.perl]
workspace.includePaths = ["lib", ".", "local/lib/perl5"]
workspace.useSystemInc = false
workspace.resolutionTimeout = 50
inlayHints.enabled = true
inlayHints.parameterHints = true
limits.workspaceSymbolCap = 200
limits.referencesCap = 500
```

---

## Sublime Text

1. Install the [LSP](https://packagecontrol.io/packages/LSP) package via Package Control

2. Open `Preferences > Package Settings > LSP > Settings` and add:

```json
{
  "clients": {
    "perl-lsp": {
      "enabled": true,
      "command": ["perl-lsp", "--stdio"],
      "selector": "source.perl",
      "initializationOptions": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5"],
            "useSystemInc": false,
            "resolutionTimeout": 50
          },
          "inlayHints": {
            "enabled": true,
            "parameterHints": true,
            "typeHints": true
          },
          "limits": {
            "workspaceSymbolCap": 200,
            "referencesCap": 500,
            "completionCap": 100
          }
        }
      }
    }
  }
}
```

---

## Troubleshooting

### Server Not Starting

1. **Verify binary location**:
   ```bash
   which perl-lsp
   # Should output path like: /home/user/.cargo/bin/perl-lsp
   ```

2. **Check binary works**:
   ```bash
   perl-lsp --version
   perl-lsp --health
   ```

3. **Test JSON-RPC communication**:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perl-lsp --stdio
   ```

### No Diagnostics

1. **Check file type**: Ensure your file has a Perl extension (.pl, .pm, .t)

2. **Check logs**: Most editors show LSP logs in a dedicated panel
   - VS Code: View > Output > select "Perl Language Server"
   - Neovim: `:LspLog`
   - Emacs: `*lsp-log*` buffer

3. **Enable debug logging**:
   ```bash
   RUST_LOG=perl_lsp=debug perl-lsp --stdio
   ```

### Slow Performance

1. **Reduce result caps**:
   ```json
   {
     "perl": {
       "limits": {
         "workspaceSymbolCap": 100,
         "referencesCap": 200,
         "maxIndexedFiles": 5000
       }
     }
   }
   ```

2. **Disable system @INC** if you have network filesystems:
   ```json
   {
     "perl": {
       "workspace": {
         "useSystemInc": false
       }
     }
   }
   ```

3. **Reduce resolution timeout**:
   ```json
   {
     "perl": {
       "workspace": {
         "resolutionTimeout": 25
       }
     }
   }
   ```

### Module Resolution Issues

1. **Check include paths**:
   ```json
   {
     "perl": {
       "workspace": {
         "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
       }
     }
   }
   ```

2. **Verify module exists**:
   ```bash
   perl -e 'use Module::Name;'
   ```

3. **Check workspace root**: Ensure your editor opened the correct project root

### Connection Issues

1. Check for crash logs in your editor's log panel

2. Run with logging enabled:
   ```bash
   perl-lsp --stdio --log 2>perl-lsp.log
   ```

3. Report issues with reproduction steps on [GitHub](https://github.com/EffortlessMetrics/perl-lsp/issues)

---

## Command Line Reference

```
perl-lsp [options]

Options:
  --stdio          Use stdio for communication (default)
  --socket         Use TCP socket for communication (not yet implemented)
  --port PORT      Port to listen on (default: 9257)
  --log            Enable logging to stderr
  --health         Quick health check (prints 'ok <version>')
  --version        Show version information
  --features-json  Output features catalog as JSON
  --help           Show help message

Examples:
  # Run in stdio mode (for editors)
  perl-lsp --stdio

  # Run with logging enabled
  perl-lsp --stdio --log

  # Run with debug output
  RUST_LOG=perl_lsp=debug perl-lsp --stdio
```

---

## See Also

- [CONFIG.md](CONFIG.md) - Complete configuration reference
- [PERFORMANCE_SLO.md](PERFORMANCE_SLO.md) - Performance targets and limits
- [LSP_FEATURES.md](LSP_FEATURES.md) - Supported LSP features
- [DAP_USER_GUIDE.md](DAP_USER_GUIDE.md) - Debugging setup with perl-dap
