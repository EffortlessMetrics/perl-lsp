<#
.SYNOPSIS
Remove a disposable agent worktree and its external Cargo target directory.

.DESCRIPTION
This script is the stop-the-line cleanup gate for per-PR agent work. It removes
only the computed worktree and target paths for the supplied issue/slug, and it
fails instead of guessing when the worktree is dirty, the PR is not merged, or a
path falls outside the configured roots.
#>
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d+$')]
    [string]$Issue,

    [Parameter(Mandatory = $true)]
    [string]$Slug,

    [int]$PrNumber,
    [switch]$Abandoned,
    [string]$Branch,

    [string]$CanonicalRoot = 'H:\Code\Rust3\perl-lsp-swarm',
    [string]$WorktreeRoot = 'H:\Code\Rust3\perl-lsp-swarm-worktrees',
    [string]$TargetRoot = 'H:\Code\Rust3\.cargo-target\perl-lsp-swarm'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Fail {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Get-FullPath {
    param([string]$Path)
    return [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
}

function Assert-ChildPath {
    param(
        [string]$Child,
        [string]$Parent,
        [string]$Description
    )

    $childFull = Get-FullPath $Child
    $parentFull = Get-FullPath $Parent
    $parentPrefix = $parentFull
    if (-not $parentPrefix.EndsWith('\')) {
        $parentPrefix = "$parentPrefix\"
    }

    if (-not $childFull.StartsWith($parentPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        Fail "$Description is outside the required root. Path: $childFull Root: $parentFull"
    }

    return $childFull
}

function Invoke-Git {
    param(
        [string]$Repository,
        [string[]]$GitArgs
    )

    $output = & git -C $Repository @GitArgs 2>&1
    if ($LASTEXITCODE -ne 0) {
        Fail "git $($GitArgs -join ' ') failed in $Repository. Output: $output"
    }

    return @($output)
}

if ($Slug -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]*$' -or $Slug -match '[\\/]') {
    Fail "Slug must be a path-safe token, not '$Slug'."
}

if (-not $Abandoned -and $PrNumber -le 0) {
    Fail 'Provide -PrNumber for merged work, or pass -Abandoned for intentionally abandoned work.'
}

$canonical = Get-FullPath $CanonicalRoot
$worktreeRootFull = Get-FullPath $WorktreeRoot
$targetRootFull = Get-FullPath $TargetRoot
$worktreePath = Assert-ChildPath -Child (Join-Path $worktreeRootFull "$Issue-$Slug") -Parent $worktreeRootFull -Description 'Worktree path'
$targetPath = Assert-ChildPath -Child (Join-Path $targetRootFull "$Issue-$Slug") -Parent $targetRootFull -Description 'Cargo target path'
$verifiedMergedPr = $false

if (-not (Test-Path -Path $canonical -PathType Container)) {
    Fail "Canonical checkout does not exist: $canonical"
}

if ($PrNumber -gt 0) {
    $prJson = & gh pr view $PrNumber --json state,mergedAt,headRefName 2>&1
    if ($LASTEXITCODE -ne 0) {
        Fail "Unable to verify PR #$PrNumber. Output: $prJson"
    }

    $pr = $prJson | ConvertFrom-Json
    if (-not $Abandoned -and [string]::IsNullOrWhiteSpace([string]$pr.mergedAt)) {
        Fail "PR #$PrNumber is not merged. Cleanup is blocked."
    }
    elseif (-not [string]::IsNullOrWhiteSpace([string]$pr.mergedAt)) {
        $verifiedMergedPr = $true
    }

    if ([string]::IsNullOrWhiteSpace($Branch)) {
        $Branch = [string]$pr.headRefName
    }
}

if (Test-Path -Path $worktreePath -PathType Container) {
    $worktreeStatus = @(Invoke-Git -Repository $worktreePath -GitArgs @('status', '--porcelain'))
    if ($worktreeStatus.Count -gt 0) {
        Fail "Worktree is dirty; refusing cleanup: $worktreePath"
    }

    if ([string]::IsNullOrWhiteSpace($Branch)) {
        $Branch = @(Invoke-Git -Repository $worktreePath -GitArgs @('branch', '--show-current'))[0]
    }

    if ($PSCmdlet.ShouldProcess($worktreePath, 'git worktree remove')) {
        Invoke-Git -Repository $canonical -GitArgs @('worktree', 'remove', $worktreePath) | Out-Null
    }
}
else {
    Write-Host "worktree already absent: $worktreePath"
}

if (Test-Path -Path $targetPath) {
    if ($PSCmdlet.ShouldProcess($targetPath, 'remove external Cargo target directory')) {
        Remove-Item -Path $targetPath -Recurse -Force
    }
}
else {
    Write-Host "target already absent: $targetPath"
}

if (-not [string]::IsNullOrWhiteSpace($Branch) -and -not $Abandoned) {
    $localBranches = @(Invoke-Git -Repository $canonical -GitArgs @('branch', '--format', '%(refname:short)', '--list', $Branch))
    if ($localBranches -contains $Branch) {
        $mergedBranches = @(Invoke-Git -Repository $canonical -GitArgs @('branch', '--merged', 'origin/main', '--format', '%(refname:short)'))
        $branchDeleteArgs = @('branch', '-d', $Branch)
        $branchDeleteAction = 'delete merged local branch'

        if ($mergedBranches -notcontains $Branch -and $verifiedMergedPr) {
            $branchDeleteArgs = @('branch', '-D', $Branch)
            $branchDeleteAction = 'delete squash-merged local branch'
        }

        if ($mergedBranches -contains $Branch -or $verifiedMergedPr) {
            if ($PSCmdlet.ShouldProcess($Branch, $branchDeleteAction)) {
                Invoke-Git -Repository $canonical -GitArgs $branchDeleteArgs | Out-Null
            }
        }
        else {
            Write-Host "branch not deleted because it is not merged into origin/main: $Branch"
        }
    }
}

Invoke-Git -Repository $canonical -GitArgs @('worktree', 'prune') | Out-Null

$canonicalTarget = Join-Path $canonical 'target'
if (Test-Path -Path $canonicalTarget) {
    Fail "Repo-local target directory still exists in canonical checkout: $canonicalTarget"
}

$canonicalStatus = @(Invoke-Git -Repository $canonical -GitArgs @('status', '--porcelain'))
if ($canonicalStatus.Count -gt 0) {
    Fail "Canonical checkout is dirty after cleanup: $canonical"
}

$storageDoctor = Join-Path $canonical 'scripts\storage-doctor'
if (Test-Path -Path $storageDoctor) {
    Push-Location $canonical
    try {
        & '.\scripts\storage-doctor'
    }
    finally {
        Pop-Location
    }

    if ($LASTEXITCODE -ne 0) {
        Fail 'storage-doctor failed after cleanup.'
    }
}

Write-Host 'agent cleanup ok'
Write-Host "canonical=$canonical"
Write-Host "removed_worktree=$worktreePath"
Write-Host "removed_target=$targetPath"
if (-not [string]::IsNullOrWhiteSpace($Branch)) {
    Write-Host "branch=$Branch"
}
