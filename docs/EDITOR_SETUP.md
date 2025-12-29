# Editor Setup Guide

This guide covers setting up the Perl LSP server with popular editors.

## Table of Contents

- [VS Code](#vs-code)
- [Neovim](#neovim)
- [Emacs](#emacs)
- [Helix](#helix)
- [Sublime Text](#sublime-text)
- [Troubleshooting](#troubleshooting)

---

## VS Code

### Installation

1. **Install the Server**

   ```bash
   cargo install perl-lsp
   ```

   Or download a pre-built binary from the [releases page](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/releases).

2. **Install the VS Code Extension**

   Install a generic LSP client extension, or create a `.vscode/settings.json`:

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

### Configuration

Add to your `settings.json`:

```json
{
  "perl.workspace.includePaths": ["lib", ".", "local/lib/perl5"],
  "perl.workspace.useSystemInc": false,
  "perl.inlayHints.enabled": true,
  "perl.inlayHints.parameterHints": true,
  "perl.limits.workspaceSymbolCap": 200,
  "perl.limits.referencesCap": 500
}
```

---

## Neovim

### Using nvim-lspconfig

1. **Install the Server**

   ```bash
   cargo install perl-lsp
   ```

2. **Configure in `init.lua`**

   ```lua
   -- Add perl-lsp to nvim-lspconfig
   local lspconfig = require('lspconfig')
   local configs = require('lspconfig.configs')

   -- Define perl-lsp if not already defined
   if not configs.perl_lsp then
     configs.perl_lsp = {
       default_config = {
         cmd = { 'perl-lsp', '--stdio' },
         filetypes = { 'perl' },
         root_dir = lspconfig.util.root_pattern('Makefile.PL', 'Build.PL', 'cpanfile', '.git'),
         single_file_support = true,
         settings = {
           perl = {
             workspace = {
               includePaths = { 'lib', '.', 'local/lib/perl5' },
               useSystemInc = false,
             },
             inlayHints = {
               enabled = true,
               parameterHints = true,
             },
             limits = {
               workspaceSymbolCap = 200,
               referencesCap = 500,
             },
           },
         },
       },
     }
   end

   -- Enable the server
   lspconfig.perl_lsp.setup({
     on_attach = function(client, bufnr)
       -- Your on_attach function
     end,
     capabilities = require('cmp_nvim_lsp').default_capabilities(),
   })
   ```

### Using Mason

If you use [mason.nvim](https://github.com/williamboman/mason.nvim):

```lua
-- Not yet available in mason registry - install manually:
-- cargo install perl-lsp

require('lspconfig').perl_lsp.setup({})
```

---

## Emacs

### Using lsp-mode

1. **Install the Server**

   ```bash
   cargo install perl-lsp
   ```

2. **Configure in your Emacs config**

   ```elisp
   (use-package lsp-mode
     :ensure t
     :hook ((cperl-mode . lsp-deferred)
            (perl-mode . lsp-deferred))
     :commands lsp
     :config
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
           (useSystemInc . :json-false))
          (inlayHints
           (enabled . t)
           (parameterHints . t))
          (limits
           (workspaceSymbolCap . 200)
           (referencesCap . 500)))))))
   ```

### Using eglot (Emacs 29+)

```elisp
(use-package eglot
  :ensure t
  :hook ((cperl-mode . eglot-ensure)
         (perl-mode . eglot-ensure))
  :config
  (add-to-list 'eglot-server-programs
               '((cperl-mode perl-mode) . ("perl-lsp" "--stdio"))))
```

---

## Helix

1. **Install the Server**

   ```bash
   cargo install perl-lsp
   ```

2. **Configure in `~/.config/helix/languages.toml`**

   ```toml
   [[language]]
   name = "perl"
   scope = "source.perl"
   injection-regex = "perl"
   file-types = ["pl", "pm", "t"]
   roots = ["Makefile.PL", "Build.PL", "cpanfile", ".git"]
   comment-token = "#"
   indent = { tab-width = 4, unit = "    " }
   language-servers = ["perl-lsp"]

   [language-server.perl-lsp]
   command = "perl-lsp"
   args = ["--stdio"]

   [language-server.perl-lsp.config.perl]
   workspace.includePaths = ["lib", ".", "local/lib/perl5"]
   workspace.useSystemInc = false
   inlayHints.enabled = true
   limits.workspaceSymbolCap = 200
   ```

---

## Sublime Text

1. **Install the Server**

   ```bash
   cargo install perl-lsp
   ```

2. **Install [LSP](https://packagecontrol.io/packages/LSP) package**

3. **Configure in `Preferences > Package Settings > LSP > Settings`**

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
               "useSystemInc": false
             },
             "inlayHints": {
               "enabled": true
             },
             "limits": {
               "workspaceSymbolCap": 200
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

1. **Check Installation**

   ```bash
   which perl-lsp
   perl-lsp --version
   ```

2. **Test Manually**

   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perl-lsp --stdio
   ```

### No Diagnostics

1. **Check File Type** - Ensure your file is recognized as Perl (.pl, .pm, .t)
2. **Check Logs** - Most editors show LSP logs in a dedicated panel
3. **Enable Debug Logging**:
   ```bash
   RUST_LOG=perl_lsp=debug perl-lsp --stdio
   ```

### Slow Performance

1. **Reduce Result Caps** in settings:
   ```json
   {
     "perl.limits.workspaceSymbolCap": 100,
     "perl.limits.referencesCap": 200
   }
   ```

2. **Disable System @INC** if you have slow network filesystems:
   ```json
   {
     "perl.workspace.useSystemInc": false
   }
   ```

### Module Resolution Issues

1. **Configure Include Paths** - Add your project's lib directories
2. **Check Workspace Folders** - Ensure your project root is recognized
3. **Verify Module Exists**:
   ```bash
   perl -e 'use Module::Name;'
   ```

### Connection Issues

If the LSP server disconnects unexpectedly:

1. Check for crash logs in your editor's log panel
2. Run with debug output: `RUST_LOG=debug perl-lsp --stdio`
3. Report issues with reproduction steps on [GitHub](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues)

---

## Next Steps

- See [CONFIG.md](CONFIG.md) for all available configuration options
- See [LSP_FEATURES.md](LSP_FEATURES.md) for supported LSP features
- See [DAP_USER_GUIDE.md](DAP_USER_GUIDE.md) for debugging setup
