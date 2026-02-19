# Packaging + Install Story Specification

**Status**: Draft
**PR Target**: Editor Setup + Config Reference
**Baseline**: Post-PR #245

---

## Goal

Turn "cool project" into "people actually use it" with frictionless installation and setup.

---

## Deliverables

### 1. `docs/EDITOR_SETUP.md`

Complete setup instructions for each major editor.

---

## Editor Setup: VS Code

### Installation

```bash
# Install the language server
cargo install perl-lsp

# Or download pre-built binary
curl -fsSL https://github.com/EffortlessMetrics/perl-lsp/releases/latest/download/perl-lsp-$(uname -s)-$(uname -m) -o ~/.local/bin/perl-lsp
chmod +x ~/.local/bin/perl-lsp
```

### Configuration

Create `.vscode/settings.json`:

```json
{
  "perl.languageServer.enable": true,
  "perl.languageServer.path": "perl-lsp",
  "perl.languageServer.args": ["--stdio"],

  // Optional: Enable workspace indexing for cross-file features
  "perl.languageServer.workspaceIndex": true,

  // Optional: Configure @INC for module resolution
  "perl.languageServer.includePaths": [
    "${workspaceFolder}/lib",
    "${workspaceFolder}/local/lib/perl5"
  ]
}
```

### Generic LSP Client (without extension)

If using a generic LSP client extension:

```json
{
  "languageServerExample.trace.server": "verbose",
  "languageServerExample.serverPath": "perl-lsp",
  "languageServerExample.serverArgs": ["--stdio"],
  "languageServerExample.fileAssociations": ["*.pl", "*.pm", "*.t"]
}
```

---

## Editor Setup: Neovim

### With nvim-lspconfig

```lua
-- ~/.config/nvim/lua/lsp/perl.lua

local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Define perl-lsp if not already defined
if not configs.perl_lsp then
  configs.perl_lsp = {
    default_config = {
      cmd = { 'perl-lsp', '--stdio' },
      filetypes = { 'perl' },
      root_dir = lspconfig.util.root_pattern('.git', 'Makefile.PL', 'cpanfile'),
      settings = {
        perl = {
          includePaths = { 'lib', 'local/lib/perl5' },
        },
      },
    },
  }
end

lspconfig.perl_lsp.setup({
  on_attach = function(client, bufnr)
    -- Enable completion
    vim.bo[bufnr].omnifunc = 'v:lua.vim.lsp.omnifunc'

    -- Keymaps
    local opts = { buffer = bufnr }
    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
    vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
    vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
    vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
  end,
})
```

### With lazy.nvim

```lua
-- ~/.config/nvim/lua/plugins/lsp.lua

return {
  {
    'neovim/nvim-lspconfig',
    config = function()
      -- Perl LSP setup (see above)
    end,
  },
}
```

---

## Editor Setup: Emacs

### With lsp-mode

```elisp
;; ~/.emacs.d/init.el or ~/.config/emacs/init.el

(use-package lsp-mode
  :ensure t
  :hook ((perl-mode . lsp)
         (cperl-mode . lsp))
  :commands lsp)

(use-package lsp-ui
  :ensure t
  :commands lsp-ui-mode)

;; Register perl-lsp
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(perl-mode . "perl"))
  (add-to-list 'lsp-language-id-configuration '(cperl-mode . "perl"))

  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("perl-lsp" "--stdio"))
    :major-modes '(perl-mode cperl-mode)
    :server-id 'perl-lsp
    :priority 1)))
```

### With eglot (built-in, Emacs 29+)

```elisp
(add-to-list 'eglot-server-programs
             '((perl-mode cperl-mode) . ("perl-lsp" "--stdio")))

(add-hook 'perl-mode-hook 'eglot-ensure)
(add-hook 'cperl-mode-hook 'eglot-ensure)
```

---

## Editor Setup: Helix

```toml
# ~/.config/helix/languages.toml

[[language]]
name = "perl"
language-servers = ["perl-lsp"]

[language-server.perl-lsp]
command = "perl-lsp"
args = ["--stdio"]
```

---

## Editor Setup: Sublime Text

### With LSP Package

1. Install "LSP" package via Package Control
2. Add to LSP settings (`Preferences > Package Settings > LSP > Settings`):

```json
{
  "clients": {
    "perl-lsp": {
      "enabled": true,
      "command": ["perl-lsp", "--stdio"],
      "selector": "source.perl",
      "initializationOptions": {
        "includePaths": ["lib", "local/lib/perl5"]
      }
    }
  }
}
```

---

## Configuration Reference

_Note: This section documents the configuration options that should be extracted to `docs/CONFIG.md`._

### Server Settings

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `includePaths` | `string[]` | `["lib"]` | Directories to add to @INC for module resolution |
| `enableWorkspaceIndex` | `boolean` | `true` | Enable cross-file features (definition, references) |
| `maxFiles` | `number` | `10000` | Maximum files to index |
| `enableDiagnostics` | `boolean` | `true` | Enable syntax and lint diagnostics |
| `enableFormatting` | `boolean` | `true` | Enable document formatting |
| `perltidyPath` | `string` | `"perltidy"` | Path to perltidy executable |
| `perlcriticPath` | `string` | `"perlcritic"` | Path to perlcritic executable |
| `perlcriticProfile` | `string` | `""` | Path to .perlcriticrc file |

### Initialization Options

Sent during `initialize` request:

```json
{
  "initializationOptions": {
    "includePaths": ["lib", "local/lib/perl5"],
    "enableWorkspaceIndex": true,
    "diagnostics": {
      "enable": true,
      "perlcritic": {
        "enable": true,
        "severity": 3
      }
    },
    "formatting": {
      "enable": true,
      "perltidyArgs": ["-l=100", "-i=4"]
    }
  }
}
```

### Workspace Configuration

Sent via `workspace/didChangeConfiguration`:

```json
{
  "settings": {
    "perl": {
      "inlayHints": {
        "enabled": true,
        "parameterHints": true,
        "typeHints": false
      },
      "workspace": {
        "includePaths": ["lib", "t/lib"],
        "useSystemInc": false,
        "resolutionTimeout": 5000
      }
    }
  }
}
```

Note: Configuration is nested under `perl.inlayHints.*` and `perl.workspace.*` sections.

### Environment Variables

| Variable | Description |
|----------|-------------|
| `PERL_LSP_LOG` | Log level: `error`, `warn`, `info`, `debug`, `trace` |
| `PERL_LSP_LOG_FILE` | Log to file instead of stderr |
| `PERL5LIB` | Additional include paths (standard Perl) |

### Precedence

1. Initialization options (highest)
2. Workspace configuration
3. Environment variables
4. Defaults (lowest)

---

### 3. Troubleshooting Section

---

## Troubleshooting

### Server Not Starting

**Symptoms**: Editor shows "Language server not found" or similar

**Solutions**:
1. Verify `perl-lsp` is in your PATH: `which perl-lsp`
2. Try running directly: `perl-lsp --version`
3. Check editor logs for error messages
4. On Windows, ensure `.exe` extension if needed

### No Completions

**Symptoms**: Completion popup doesn't appear or is empty

**Solutions**:
1. Verify file is recognized as Perl (check syntax highlighting)
2. Check server is running: look for `perl-lsp` process
3. Try hover on a known symbol first
4. Check server logs: `PERL_LSP_LOG=debug perl-lsp --stdio`

### Missing Cross-File Features

**Symptoms**: Go-to-definition only works in same file

**Solutions**:
1. Ensure `enableWorkspaceIndex: true`
2. Wait for initial indexing to complete (check logs)
3. Verify workspace folder is correctly set
4. Check `maxFiles` limit isn't exceeded

### perltidy/perlcritic Not Working

**Symptoms**: Formatting doesn't work, or no linting diagnostics

**Solutions**:
1. Verify tools are installed: `which perltidy`, `which perlcritic`
2. Check paths in configuration
3. Formatting works without perltidy (built-in fallback)
4. perlcritic requires explicit installation

### Multi-Root Workspace Issues

**Symptoms**: Wrong modules resolved, cross-project pollution

**Solutions**:
1. Each root should have its own `lib/` directory
2. Configure per-folder `includePaths`
3. Consider separate workspaces for unrelated projects

### Windows/WSL Specific

**Symptoms**: Path resolution errors, slow performance

**Solutions**:
1. Use WSL2 for best compatibility
2. Keep project files on Linux filesystem (not `/mnt/c/`)
3. Ensure line endings are LF (not CRLF)
4. Use forward slashes in configuration paths

---

### 4. Optional: `scripts/install.sh`

```bash
#!/bin/bash
set -euo pipefail

# perl-lsp installer
VERSION="${1:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
  x86_64) ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

if [ "$VERSION" = "latest" ]; then
  URL="https://github.com/EffortlessMetrics/perl-lsp/releases/latest/download/perl-lsp-${OS}-${ARCH}"
else
  URL="https://github.com/EffortlessMetrics/perl-lsp/releases/download/${VERSION}/perl-lsp-${OS}-${ARCH}"
fi

echo "Installing perl-lsp to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
curl -fsSL "$URL" -o "$INSTALL_DIR/perl-lsp"
chmod +x "$INSTALL_DIR/perl-lsp"

echo "Installed: $("$INSTALL_DIR/perl-lsp" --version)"
echo ""
echo "Add to your PATH if needed:"
echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
```

---

## Implementation Phases

### Phase 1: Core Docs (Week 1)
- Create `docs/EDITOR_SETUP.md` with VS Code, Neovim, Emacs
- Create `docs/CONFIG.md` with all settings

### Phase 2: Expanded Coverage (Week 2)
- Add Helix, Sublime Text, other editors
- Add troubleshooting section
- Create install script

### Phase 3: Validation (Week 3)
- Test all editor configurations
- Validate on fresh machines
- Get external feedback

---

## Success Criteria

- [ ] New user can install and get working in <5 minutes
- [ ] All major editors documented (VS Code, Neovim, Emacs)
- [ ] Troubleshooting covers common issues
- [ ] Configuration is complete and accurate
- [ ] Install script works on Linux and macOS
