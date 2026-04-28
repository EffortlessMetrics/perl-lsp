# Neovim Setup Guide for perl-lsp

Use this guide to run `perllsp` in Neovim through Neovim's built-in LSP client.

## Prerequisites

- Neovim 0.11.3 or later; a current stable release is recommended
- `perllsp` installed and available on your `PATH`
- a Perl project opened from the project root

Optional:

- `nvim-lspconfig`, if you already use it for other language-server configs
- `nvim-cmp`, if you prefer cmp-based completion
- `telescope.nvim`, if you want Telescope-backed symbol/reference pickers
- `perltidy`, for formatting
- `perlcritic`, only if Perl::Critic diagnostics are enabled

Verify `perllsp` before changing Neovim configuration:

```bash
perllsp --version
perllsp --health
perllsp --info
```

## Install `perllsp`

### Cargo

```bash
cargo install perllsp
```

### Homebrew

```bash
brew install perl-lsp
```

### From Source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perllsp --locked
```

### Prebuilt Binary

Download the archive for your platform from GitHub Releases, extract it, and put
`perllsp` on your `PATH`.

Release assets use the `perllsp-<version>-<target>` naming pattern. Check the
release page before copying a version number.

## Basic Setup: Neovim 0.11+

Create a custom LSP config file:

```vim
:exe 'edit' stdpath('config') .. '/lsp/perllsp.lua'
```

Add:

```lua
return {
  cmd = { 'perllsp', '--stdio' },
  filetypes = { 'perl' },
  root_markers = {
    '.perl-lsp.toml',
    'Makefile.PL',
    'Build.PL',
    'cpanfile',
    'dist.ini',
    '.git',
  },
  init_options = {
    perl = {
      workspace = {
        includePaths = { 'lib', '.', 'local/lib/perl5' },
        useSystemInc = false,
        resolutionTimeout = 50,
      },
    },
  },
}
```

Then enable it from `init.lua`:

```lua
vim.lsp.enable('perllsp')
```

Restart Neovim, open a Perl file, and run:

```vim
:checkhealth vim.lsp
```

## Optional: Define the Config Inline

Instead of creating `lsp/perllsp.lua`, you can define the config directly in
`init.lua`:

```lua
vim.lsp.config('perllsp', {
  cmd = { 'perllsp', '--stdio' },
  filetypes = { 'perl' },
  root_markers = {
    '.perl-lsp.toml',
    'Makefile.PL',
    'Build.PL',
    'cpanfile',
    'dist.ini',
    '.git',
  },
  init_options = {
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
      limits = {
        workspaceSymbolCap = 200,
        referencesCap = 500,
        completionCap = 100,
      },
    },
  },
})

vim.lsp.enable('perllsp')
```

## Optional: Filetype Detection

Neovim starts the server only when the buffer filetype is `perl`.

Check a buffer with:

```vim
:set filetype?
```

If `.t`, `.psgi`, `.cgi`, or other Perl-bearing files are not detected as Perl
in your environment, add filetype rules before enabling the server:

```lua
vim.filetype.add({
  extension = {
    t = 'perl',
    psgi = 'perl',
    cgi = 'perl',
    fcgi = 'perl',
    PL = 'perl',
  },
})
```

## Optional: Project-Wide perl-lsp Settings

Prefer `.perl-lsp.toml` for settings shared across editors:

```toml
# .perl-lsp.toml
[perl]
include_paths = ["lib", "local/lib/perl5", "vendor/lib"]

[features]
inlay_hints = true
```

Use Neovim `init_options` only for Neovim-specific startup behavior.

## Useful LSP Keymaps

Neovim provides several LSP defaults when LSP is active, including `K` for
hover, `CTRL-X CTRL-O` for omnifunc completion, `gO` for document symbols, `grr`
for references, `grn` for rename, and `gra` for code actions.

If you prefer traditional mappings such as `gd`, add them with `LspAttach`:

```lua
vim.api.nvim_create_autocmd('LspAttach', {
  callback = function(ev)
    local client = vim.lsp.get_client_by_id(ev.data.client_id)
    if not client or client.name ~= 'perllsp' then
      return
    end

    local opts = { buffer = ev.buf, silent = true }

    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
    vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
    vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
    vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
    vim.keymap.set({ 'n', 'v' }, '<leader>ca', vim.lsp.buf.code_action, opts)
    vim.keymap.set('n', '<leader>f', function()
      vim.lsp.buf.format({ async = true })
    end, opts)

    vim.keymap.set('n', '[d', function()
      vim.diagnostic.jump({ count = -1 })
    end, opts)

    vim.keymap.set('n', ']d', function()
      vim.diagnostic.jump({ count = 1 })
    end, opts)

    vim.keymap.set('n', '<leader>e', vim.diagnostic.open_float, opts)
    vim.keymap.set('n', '<leader>q', vim.diagnostic.setloclist, opts)
  end,
})
```

## Optional: Built-in LSP Completion

For Neovim's built-in LSP completion:

```lua
vim.api.nvim_create_autocmd('LspAttach', {
  callback = function(ev)
    local client = vim.lsp.get_client_by_id(ev.data.client_id)
    if not client or client.name ~= 'perllsp' then
      return
    end

    vim.lsp.completion.enable(true, client.id, ev.buf, {
      autotrigger = true,
    })

    vim.keymap.set('i', '<C-Space>', function()
      vim.lsp.completion.get()
    end, { buffer = ev.buf, desc = 'Trigger LSP completion' })
  end,
})
```

If you use `nvim-cmp`, configure cmp capabilities before enabling the server:

```lua
local capabilities = require('cmp_nvim_lsp').default_capabilities()

vim.lsp.config('perllsp', {
  capabilities = capabilities,
  cmd = { 'perllsp', '--stdio' },
  filetypes = { 'perl' },
  root_markers = {
    '.perl-lsp.toml',
    'Makefile.PL',
    'Build.PL',
    'cpanfile',
    'dist.ini',
    '.git',
  },
})
vim.lsp.enable('perllsp')
```

## Optional: Inlay Hints

`perllsp` can provide inlay hints. Enable display on attach:

```lua
vim.api.nvim_create_autocmd('LspAttach', {
  callback = function(ev)
    local client = vim.lsp.get_client_by_id(ev.data.client_id)
    if not client or client.name ~= 'perllsp' then
      return
    end

    if client:supports_method('textDocument/inlayHint') then
      vim.lsp.inlay_hint.enable(true, { bufnr = ev.buf })
    end
  end,
})
```

Toggle inlay hints for the current buffer:

```lua
vim.keymap.set('n', '<leader>ih', function()
  vim.lsp.inlay_hint.enable(
    not vim.lsp.inlay_hint.is_enabled({ bufnr = 0 }),
    { bufnr = 0 }
  )
end, { desc = 'Toggle inlay hints' })
```

## Optional: Telescope Integration

If using `telescope.nvim`:

```lua
local telescope = require('telescope.builtin')

vim.keymap.set('n', '<leader>ss', telescope.lsp_document_symbols)
vim.keymap.set('n', '<leader>sw', telescope.lsp_workspace_symbols)
vim.keymap.set('n', '<leader>sr', telescope.lsp_references)
vim.keymap.set('n', '<leader>sd', telescope.lsp_definitions)
```

## Optional: External Formatting and Linting

`perllsp` can provide LSP formatting through `perltidy`. If you also want
non-LSP formatter/linter orchestration, prefer maintained plugins such as
`conform.nvim`, `nvim-lint`, or `none-ls.nvim`.

## Verify It Is Running

1. Restart Neovim.
2. Open a Perl file such as `lib/My/Module.pm`, `script/app.pl`, or `t/basic.t`.
3. Confirm filetype:

   ```vim
   :set filetype?
   ```

4. Check LSP state:

   ```vim
   :checkhealth vim.lsp
   ```

5. Introduce a temporary syntax error and confirm diagnostics appear.
6. Remove the syntax error after testing.

You can also check a file outside Neovim:

```bash
perllsp --check path/to/file.pl
```

## Troubleshooting

### Neovim cannot find `perllsp`

From a shell:

```bash
command -v perllsp
perllsp --version
perllsp --health
perllsp --info
```

On Windows PowerShell:

```powershell
where perllsp
perllsp --version
perllsp --health
perllsp --info
```

From Neovim:

```vim
:!command -v perllsp
```

If Neovim was launched from a GUI, it may not inherit the same `PATH` as your
terminal. Use an absolute command path if needed:

```lua
vim.lsp.config('perllsp', {
  cmd = { '/absolute/path/to/perllsp', '--stdio' },
})
```

### Server does not attach

Check:

```vim
:set filetype?
:checkhealth vim.lsp
```

Common causes:

- the buffer filetype is not `perl`
- the workspace has no root marker
- `perllsp` is not on `PATH`
- the config name was not enabled with `vim.lsp.enable('perllsp')`

### No diagnostics

Run outside Neovim:

```bash
perllsp --check path/to/file.pl
```

Inside Neovim:

```vim
:lua vim.diagnostic.open_float()
:lua vim.diagnostic.setloclist()
```

### Module resolution issues

Prefer `.perl-lsp.toml` for project-wide include paths:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5", "vendor/lib"]
```

Or pass Neovim-specific startup options:

```lua
vim.lsp.config('perllsp', {
  init_options = {
    perl = {
      workspace = {
        includePaths = { 'lib', '.', 'local/lib/perl5', 'vendor/lib' },
        useSystemInc = false,
      },
    },
  },
})
```

### Slow performance

For large workspaces, reduce caps:

```lua
vim.lsp.config('perllsp', {
  init_options = {
    perl = {
      limits = {
        workspaceSymbolCap = 100,
        referencesCap = 200,
        completionCap = 50,
        astCacheMaxEntries = 50,
        maxIndexedFiles = 5000,
        maxTotalSymbols = 250000,
        workspaceScanDeadlineMs = 20000,
        referenceSearchDeadlineMs = 1500,
      },
      workspace = {
        resolutionTimeout = 25,
      },
    },
  },
})
```

### Formatting does not work

Confirm `perltidy` is available:

```bash
perltidy --version
```

Then run:

```vim
:lua vim.lsp.buf.format({ async = true })
```

### Debug logs

Enable Neovim LSP logs temporarily:

```lua
vim.lsp.log.set_level('debug')
```

Open the LSP log:

```vim
:log lsp
```

For server-side logs, start `perllsp` with `--log`:

```lua
vim.lsp.config('perllsp', {
  cmd = { 'perllsp', '--stdio', '--log' },
})
```

or set:

```bash
PERL_LSP_LOG=1
```

### `perllsp --stdio` appears to hang

That is expected. In stdio mode, `perllsp` waits for framed LSP JSON-RPC input
from Neovim. Use these commands for manual checks instead:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

## Legacy Setup: Neovim 0.8–0.10 with nvim-lspconfig

Use this only if you cannot upgrade to Neovim 0.11+.

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.perllsp then
  configs.perllsp = {
    default_config = {
      cmd = { 'perllsp', '--stdio' },
      filetypes = { 'perl' },
      root_dir = lspconfig.util.root_pattern(
        '.perl-lsp.toml',
        'Makefile.PL',
        'Build.PL',
        'cpanfile',
        'dist.ini',
        '.git'
      ),
      single_file_support = true,
      init_options = {
        perl = {
          workspace = {
            includePaths = { 'lib', '.', 'local/lib/perl5' },
            useSystemInc = false,
          },
        },
      },
    },
  }
end

lspconfig.perllsp.setup({})
```

For server-side behavior and configuration details, see:

- [Configuration Reference](../reference/CONFIG.md)
- [Troubleshooting Guide](../how-to/TROUBLESHOOTING.md)
- [Editor Setup](../how-to/EDITOR_SETUP.md)
