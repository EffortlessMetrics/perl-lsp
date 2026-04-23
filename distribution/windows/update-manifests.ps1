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
$releaseZipUrl = "$RepositoryUrl/releases/download/v$Version/perllsp-$Version-x86_64-pc-windows-msvc.zip"

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

    $content = [regex]::Replace($content, '"version":\s*"[^"]+"', ('"version": "' + $Version + '"'), 1)
    $content = [regex]::Replace(
      $content,
      '"url":\s*"https://github\.com/EffortlessMetrics/perl-lsp/releases/download/v[^"]+/perllsp-[^"]+-x86_64-pc-windows-msvc\.zip"',
      ('"url": "' + $releaseZipUrl + '"'),
      1
    )
    $content = [regex]::Replace($content, '"hash":\s*"[^"]+"', ('"hash": "' + $ReleaseSha256 + '"'), 1)

    return $content
  }
}

if ($ChocolateyNuspecPath) {
  Update-File -Path $ChocolateyNuspecPath -Transform {
    param($content)

    $content = [regex]::Replace($content, '<version>[^<]+</version>', ('<version>' + $Version + '</version>'), 1)

    return $content
  }
}

if ($ChocolateyInstallPath) {
  Update-File -Path $ChocolateyInstallPath -Transform {
    param($content)

    $content = [regex]::Replace($content, "checksum\s*=\s*'[^']+'", ("checksum      = '" + $ReleaseSha256 + "'"), 1)

    return $content
  }
}

if ($WingetManifestPath) {
  Update-File -Path $WingetManifestPath -Transform {
    param($content)

    $content = [regex]::Replace($content, 'PackageVersion:\s*\S+', ('PackageVersion: ' + $Version), 1)
    $content = [regex]::Replace($content, 'InstallerUrl:\s*\S+', ('InstallerUrl: ' + $releaseZipUrl), 1)
    $content = [regex]::Replace($content, 'InstallerSha256:\s*\S+', ('InstallerSha256: ' + $ReleaseSha256), 1)

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
