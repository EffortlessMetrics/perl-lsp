# Neovim Setup Guide for perl-lsp

Use this guide to run `perllsp` in Neovim through Neovim's built-in LSP client.

## Prerequisites

- Neovim 0.11.3 or later (current stable recommended)
- `perllsp` installed and available on your `PATH`
- a Perl project opened from the project root

Optional:

- `nvim-lspconfig`, if you already use it for other language servers
- `nvim-cmp`, if you prefer cmp-based completion
- `telescope.nvim`, for picker-based symbol/reference navigation
- `perltidy`, only if explicit external formatting compatibility is enabled
- `perlcritic`, only if explicit legacy Perl::Critic compatibility is enabled

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
brew install effortlessmetrics/tap/perllsp
```

### From source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perllsp --locked
```

### Prebuilt binary

Download the archive for your platform from GitHub Releases, extract it, and put
`perllsp` on your `PATH`.

Release assets use the `perllsp-<version>-<target>` naming pattern.

## Basic setup (Neovim 0.11+)

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

## Optional: lean latency profile

Use this profile when responsiveness matters more than full semantic,
module-resolution, native critic, and workspace dead-code diagnostics. Normal
mode remains the richer default.

```lua
vim.lsp.config('perllsp', {
  cmd = {
    'perllsp',
    '--stdio',
    '--runtime-mode', 'e2e',
    '--diagnostic-mode', 'syntax-only',
    '--diagnostic-debounce-ms', '0',
    '--eager-workspace-indexing=false',
    '--file-watchers=false',
  },
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

This profile keeps parser diagnostics but bypasses the full semantic/module
diagnostic stack and avoids eager workspace indexing and file watcher
registration. It does not provide incremental AST reuse. Semantic-token delta
support is not advertised until the server implements the result-id/delta path.

## Optional: Define the config inline

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
      },
      inlayHints = {
        enabled = true,
        parameterHints = true,
      },
    },
  },
})

vim.lsp.enable('perllsp')
```

## Optional: Filetype detection

Neovim starts the server only when the buffer filetype is `perl`.

```vim
:set filetype?
```

If `.t`, `.psgi`, `.cgi`, or other Perl-bearing files are not detected as Perl,
add filetype rules before enabling the server:

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

## Project-wide settings

Prefer `.perl-lsp.toml` for settings shared across editors:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5", "vendor/lib"]

[features]
inlay_hints = true
```

Use Neovim `init_options` only for Neovim-specific startup behavior.

## Completion and inlay hints

For built-in completion:

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

For inlay hints:

```lua
vim.api.nvim_create_autocmd('LspAttach', {
  callback = function(ev)
    local client = vim.lsp.get_client_by_id(ev.data.client_id)
    if client and client.name == 'perllsp' and client:supports_method('textDocument/inlayHint') then
      vim.lsp.inlay_hint.enable(true, { bufnr = ev.buf })
    end
  end,
})

vim.keymap.set('n', '<leader>ih', function()
  vim.lsp.inlay_hint.enable(
    not vim.lsp.inlay_hint.is_enabled({ bufnr = 0 }),
    { bufnr = 0 }
  )
end, { desc = 'Toggle inlay hints' })
```

## Verify it is running

1. Restart Neovim.
2. Open a Perl file such as `lib/My/Module.pm` or `t/basic.t`.
3. Confirm the filetype is `perl` with `:set filetype?`.
4. Run `:checkhealth vim.lsp`.
5. Introduce a temporary syntax error and confirm diagnostics appear.

You can also check a file outside Neovim:

```bash
perllsp --check path/to/file.pl
```

## Troubleshooting

### Neovim cannot find `perllsp`

```bash
command -v perllsp
perllsp --version
perllsp --health
perllsp --info
```

Windows PowerShell:

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

If Neovim was launched from a GUI, it may not inherit your shell `PATH`. Use an
absolute binary path if needed.

### `perllsp --stdio` appears to hang

This is expected. In stdio mode, `perllsp` waits for framed LSP JSON-RPC input.
Use these commands for manual checks:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

## Legacy setup: Neovim 0.8-0.10 with `nvim-lspconfig`

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
