$ErrorActionPreference = 'Stop'

# Remove shims
Uninstall-BinFile -Name "perllsp" -ErrorAction SilentlyContinue
Uninstall-BinFile -Name "perl-dap" -ErrorAction SilentlyContinue

Write-Host "perl-lsp has been uninstalled successfully."
