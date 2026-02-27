$ErrorActionPreference = 'Stop'

# Remove shim
Uninstall-BinFile -Name "perl-lsp" -ErrorAction SilentlyContinue

Write-Host "perl-lsp has been uninstalled successfully."
