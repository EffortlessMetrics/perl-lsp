$ErrorActionPreference = 'Stop'

$packageArgs = @{
  packageName   = 'perl-lsp'
  fileType      = 'EXE'
  url           = 'https://github.com/EffortlessMetrics/perl-lsp/releases/download/v1.0.0/perl-lsp-1.0.0-x86_64-pc-windows-msvc.zip'
  checksum      = 'REPLACE_WITH_ACTUAL_SHA256'
  checksumType  = 'sha256'
  unzipLocation = $env:ChocolateyPackageFolder
}

Install-ChocolateyZipPackage @packageArgs

# Add to PATH
$installPath = Join-Path $env:ChocolateyPackageFolder "perl-lsp-1.0.0-x86_64-pc-windows-msvc"
$binaryPath = Join-Path $installPath "perl-lsp.exe"

if (-not (Test-Path $binaryPath)) {
  Write-Error "Binary not found at $binaryPath"
  exit 1
}

# Create shim
Install-BinFile -Name "perl-lsp" -Path $binaryPath

Write-Host "perl-lsp has been installed successfully."
Write-Host "To use with your editor, configure it to use 'perl-lsp --stdio'"
