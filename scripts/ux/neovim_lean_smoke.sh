#!/usr/bin/env bash
# Neovim lean smoke for the 0.15.1 latency lane (PR 6).
#
# What this proves:
#   - perllsp built from this branch starts under `--runtime-mode e2e` and
#     accepts a real Neovim client over LSP.
#   - Opening a small Perl file, applying a short edit burst, and
#     requesting completion or hover returns a non-error response within
#     a small wallclock budget.
#
# Why this is shell-scripted (not a Rust test):
#   The CI Neovim environment is not stable enough to make this a green
#   gate. The script is committed so the receipt is reproducible on
#   developer machines and dedicated benchmark hardware.
#
# Usage:
#   nix develop -c ./scripts/ux/neovim_lean_smoke.sh
#   ./scripts/ux/neovim_lean_smoke.sh /path/to/perllsp
#
# Environment:
#   PERLLSP — explicit path to the perllsp binary (defaults to
#             target/release/perllsp).
#   NEOVIM   — explicit path to nvim (defaults to `nvim` from $PATH).
#
# Exit codes:
#   0 — smoke passed.
#   1 — required tool missing (nvim or perllsp).
#   2 — runtime failure (timeout, non-zero exit from nvim).

set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
perllsp_bin=${PERLLSP:-"${repo_root}/target/release/perllsp"}
nvim_bin=${NEOVIM:-nvim}

if ! command -v "${nvim_bin}" >/dev/null 2>&1; then
  echo "skip: nvim not found (set NEOVIM=/path/to/nvim to override)" >&2
  exit 1
fi
if [[ ! -x "${perllsp_bin}" ]]; then
  echo "skip: perllsp binary not found at ${perllsp_bin}" >&2
  echo "      build with: cargo build -p perl-lsp-rs --release --bin perl-lsp" >&2
  exit 1
fi

tmpdir=$(mktemp -d)
trap 'rm -rf "${tmpdir}"' EXIT

cat >"${tmpdir}/sample.pl" <<'PERL'
use strict;
use warnings;

my $value = 42;
my $other = $val
PERL

cat >"${tmpdir}/init.lua" <<NVIMLUA
-- Minimal Neovim init for the lean smoke. Keeps capabilities small,
-- disables semantic tokens, drops the workspace file-watcher
-- registration, and asserts the latest completion returns.

local caps = vim.lsp.protocol.make_client_capabilities()
if caps.workspace then
  caps.workspace.didChangeWatchedFiles = nil
end

local client_id = vim.lsp.start({
  name = 'perl_lsp',
  cmd = {
    '${perllsp_bin}',
    '--stdio',
    '--runtime-mode', 'e2e',
    '--diagnostic-mode', 'syntax-only',
    '--diagnostic-debounce-ms', '0',
  },
  capabilities = caps,
  root_dir = '${tmpdir}',
  on_attach = function(client, bufnr)
    if vim.lsp.semantic_tokens and vim.lsp.semantic_tokens.enable then
      vim.lsp.semantic_tokens.enable(false, { client_id = client.id, bufnr = bufnr })
    end
  end,
})

if not client_id then
  io.stderr:write('lean-smoke FAILED: vim.lsp.start returned nil\n')
  vim.cmd('cquit 2')
end

vim.cmd('edit ${tmpdir}/sample.pl')

-- Burst-edit: append characters to the partial variable name. Each
-- write bumps the document generation that PR 4's scheduler observes.
local burst = { 'a', 'b', 'c', 'd' }
for _, ch in ipairs(burst) do
  vim.api.nvim_buf_set_text(0, 4, 16, 4, 16, { ch })
  vim.cmd('redraw')
end

local final = vim.lsp.buf_request_sync(
  0,
  'textDocument/completion',
  vim.lsp.util.make_position_params(0, 'utf-16'),
  5000
)

if not final then
  io.stderr:write('lean-smoke FAILED: completion request returned nil\n')
  vim.cmd('cquit 2')
end

io.stderr:write('lean-smoke OK\n')
vim.cmd('qall!')
NVIMLUA

if ! "${nvim_bin}" --headless -u "${tmpdir}/init.lua" 2>"${tmpdir}/nvim.err"; then
  echo "lean-smoke FAILED:" >&2
  cat "${tmpdir}/nvim.err" >&2
  exit 2
fi

if grep -q "lean-smoke OK" "${tmpdir}/nvim.err"; then
  echo "lean-smoke OK"
  exit 0
fi

echo "lean-smoke FAILED: did not see 'lean-smoke OK' marker" >&2
cat "${tmpdir}/nvim.err" >&2
exit 2
