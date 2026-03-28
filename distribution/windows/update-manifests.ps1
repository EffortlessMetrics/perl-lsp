param(
  [Parameter(Mandatory = $true)]
  [string]$Version,

  [Parameter(Mandatory = $true)]
  [string]$ReleaseSha256,

  [string]$RepositoryUrl = 'https://github.com/EffortlessMetrics/perl-lsp',
  [string]$ScoopManifestPath = '',
  [string]$ChocolateyNuspecPath = '',
  [string]$ChocolateyInstallPath = '',
  [string]$WingetManifestPath = ''
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$releaseZipUrl = "$RepositoryUrl/releases/download/v$Version/perl-lsp-$Version-x86_64-pc-windows-msvc.zip"

function Resolve-RepoPath {
  param([Parameter(Mandatory = $true)][string]$RelativePath)

  if ([System.IO.Path]::IsPathRooted($RelativePath)) {
    return $RelativePath
  }

  return (Join-Path $repoRoot $RelativePath)
}

function Update-File {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][scriptblock]$Transform
  )

  $resolvedPath = Resolve-RepoPath $Path
  if (-not (Test-Path $resolvedPath)) {
    throw "Expected file to exist: $resolvedPath"
  }

  $content = Get-Content -LiteralPath $resolvedPath -Raw
  $updated = & $Transform $content
  Set-Content -LiteralPath $resolvedPath -Value $updated -Encoding utf8NoBOM
}

if ($ScoopManifestPath) {
  Update-File -Path $ScoopManifestPath -Transform {
    param($content)

    $content = $content -replace '"version": "__RELEASE_VERSION__"', ('"version": "' + $Version + '"')
    $content = $content -replace 'https://github.com/EffortlessMetrics/perl-lsp/releases/download/v__RELEASE_VERSION__/perl-lsp-__RELEASE_VERSION__-x86_64-pc-windows-msvc.zip', $releaseZipUrl
    $content = $content -replace '"hash": "__RELEASE_HASH__"', ('"hash": "' + $ReleaseSha256 + '"')

    return $content
  }
}

if ($ChocolateyNuspecPath) {
  Update-File -Path $ChocolateyNuspecPath -Transform {
    param($content)

    $content = $content -replace '<version>__RELEASE_VERSION__</version>', ('<version>' + $Version + '</version>')

    return $content
  }
}

if ($ChocolateyInstallPath) {
  Update-File -Path $ChocolateyInstallPath -Transform {
    param($content)

    $content = $content -replace '__RELEASE_SHA256__', $ReleaseSha256

    return $content
  }
}

if ($WingetManifestPath) {
  Update-File -Path $WingetManifestPath -Transform {
    param($content)

    $content = $content -replace 'PackageVersion: __RELEASE_VERSION__', ('PackageVersion: ' + $Version)
    $content = $content -replace 'InstallerUrl: __RELEASE_URL__', ('InstallerUrl: ' + $releaseZipUrl)
    $content = $content -replace 'InstallerSha256: __RELEASE_SHA256__', ('InstallerSha256: ' + $ReleaseSha256)

    return $content
  }
}

Write-Host "Updated Windows package manifests for v$Version"
if ($ScoopManifestPath) {
  Write-Host "  Scoop:        $ScoopManifestPath"
}
if ($ChocolateyNuspecPath -or $ChocolateyInstallPath) {
  Write-Host "  Chocolatey:   $ChocolateyNuspecPath $ChocolateyInstallPath"
}
if ($WingetManifestPath) {
  Write-Host "  Winget:       $WingetManifestPath"
}
