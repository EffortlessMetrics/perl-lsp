$ErrorActionPreference = 'Stop'

# PLACEHOLDER-GUARD: checksum token must be replaced by CI before publishing.
$packageVersion = $env:ChocolateyPackageVersion
if (-not $packageVersion) {
  Write-Error "ChocolateyPackageVersion is not available"
  exit 1
}
$archivePath = "perllsp-${packageVersion}-x86_64-pc-windows-msvc.zip"

$packageArgs = @{
  packageName   = 'perl-lsp'
  fileType      = 'EXE'
  url           = "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v${packageVersion}/${archivePath}"
  checksum      = '__RELEASE_SHA256__'
  checksumType  = 'sha256'
  unzipLocation = $env:ChocolateyPackageFolder
}

Install-ChocolateyZipPackage @packageArgs

# Locate extracted binary in either legacy versioned folder or current root layout.
$installPaths = @(
  Join-Path $env:ChocolateyPackageFolder "perllsp-${packageVersion}-x86_64-pc-windows-msvc",
  $env:ChocolateyPackageFolder
)
$binaryPath = $installPaths |
  ForEach-Object { Join-Path $_ "perllsp.exe" } |
  Where-Object { Test-Path $_ } |
  Select-Object -First 1

if (-not (Test-Path $binaryPath)) {
  Write-Error "Binary not found after extracting ${archivePath}"
  exit 1
}

# Create shims
Install-BinFile -Name "perllsp" -Path $binaryPath

$dapPath = $installPaths |
  ForEach-Object { Join-Path $_ "perl-dap.exe" } |
  Where-Object { Test-Path $_ } |
  Select-Object -First 1

if ($dapPath -and (Test-Path $dapPath)) {
  Install-BinFile -Name "perl-dap" -Path $dapPath
}

Write-Host "perl-lsp has been installed successfully."
Write-Host "To use with your editor, configure it to use 'perllsp --stdio'"
