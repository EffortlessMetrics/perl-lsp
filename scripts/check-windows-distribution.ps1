param()

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

function Require-File {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Label
  )

  $fullPath = Join-Path $repoRoot $Path
  if (-not (Test-Path -LiteralPath $fullPath)) {
    throw "Missing ${Label}: $Path"
  }
}

function Require-Pattern {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Pattern,
    [Parameter(Mandatory = $true)][string]$Label
  )

  $fullPath = Join-Path $repoRoot $Path
  if (-not (Select-String -LiteralPath $fullPath -Pattern $Pattern -Quiet)) {
    throw "$Label not found in $Path"
  }
}

Write-Host "Windows distribution verification audit"
Write-Host "Repo: $repoRoot"
Write-Host ""

Require-File '.github/workflows/scoop-bump.yml' 'Scoop bump workflow'
Require-File '.github/workflows/chocolatey-bump.yml' 'Chocolatey bump workflow'
Require-File '.github/workflows/release.yml' 'release workflow'
Require-File 'README.md' 'root README'
Require-File 'docs/how-to/INSTALLATION.md' 'installation guide'
Require-File 'docs/RELEASE_PROCESS.md' 'release process doc'
Require-File 'distribution/windows/update-manifests.ps1' 'shared manifest updater'

Require-Pattern '.github/workflows/scoop-bump.yml' 'update-manifests\.ps1' 'Scoop workflow manifest updater call'
Require-Pattern '.github/workflows/scoop-bump.yml' 'Select-String.*__RELEASE_VERSION__\|__RELEASE_HASH__' 'Scoop placeholder guard'
Require-Pattern '.github/workflows/chocolatey-bump.yml' 'update-manifests\.ps1' 'Chocolatey workflow manifest updater call'
Require-Pattern '.github/workflows/chocolatey-bump.yml' 'Select-String.*__RELEASE_VERSION__\|__RELEASE_SHA256__' 'Chocolatey placeholder guard'
Require-Pattern '.github/workflows/release.yml' 'SHA256SUMS' 'release checksum bundle'
Require-Pattern 'README.md' 'Windows package managers' 'README Windows install section'
Require-Pattern 'README.md' 'scoop install perl-lsp' 'README Scoop command'
Require-Pattern 'README.md' 'choco install perl-lsp' 'README Chocolatey command'
Require-Pattern 'docs/how-to/INSTALLATION.md' 'Verification Boundary' 'installation verification boundary'
Require-Pattern 'docs/how-to/INSTALLATION.md' 'perllsp --health' 'installation health check'
Require-Pattern 'docs/RELEASE_PROCESS.md' 'Windows Package-Manager Verification' 'release verification section'
Require-Pattern 'docs/RELEASE_PROCESS.md' 'powershell -NoLogo -NoProfile -File scripts/check-windows-distribution\.ps1' 'release check command'

Write-Host 'PASS: repo-side Windows distribution story is documented and wired.'
Write-Host ''
Write-Host 'Verified in repo:'
Write-Host '- release workflow publishes the Windows zip plus consolidated SHA256SUMS'
Write-Host '- Scoop and Chocolatey workflows refresh repo-owned manifests from that release asset'
Write-Host '- placeholder guards fail if release tokens remain after the update step'
Write-Host '- README and installation docs point users at Scoop and Chocolatey install commands'
Write-Host ''
Write-Host 'Still manual / external:'
Write-Host '- upstream PR acceptance or merge in ScoopInstaller/Main and Chocolatey packaging repos'
Write-Host '- installing on a Windows machine and checking PATH discovery'
Write-Host '- running perllsp --health after install'
Write-Host '- confirming editor discovery, including VS Code'
