# Perl LSP installer for Windows
#
# Usage:
#   irm https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.ps1 | iex
#
# Or from a clone / downloaded copy, to pass options:
#   .\install.ps1                                    # latest, default dir
#   .\install.ps1 -Version 0.17.0 -InstallDir C:\tools\bin

param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:USERPROFILE\.local\bin"
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$Repo = "EffortlessMetrics/perl-lsp"
# The release workflow packages the binary as `perllsp` on every platform
# (see .github/workflows/release.yml — NAME="perllsp"), and every editor doc
# / README / POSIX installer (scripts/install.sh) uses `perllsp`. Install the
# Windows binary as `perllsp.exe` so the name matches POSIX and the docs.
$Name = "perllsp"

function Write-Info {
    param([string]$Message)
    Write-Host "→ " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Error {
    param([string]$Message)
    Write-Host "Error: " -ForegroundColor Red -NoNewline
    Write-Host $Message
    exit 1
}

function Write-Warn {
    param([string]$Message)
    Write-Host "⚠ " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "✓ " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

# Detect architecture.
#
# Only x86_64-pc-windows-msvc is published: the release matrix in
# .github/workflows/release.yml has no aarch64-pc-windows-msvc entry. Asking
# for one built a URL that always 404s, and the failure surfaced as a bare
# "Failed to download" with nothing actionable in it.
# See EffortlessMetrics/perl-lsp-swarm#5007.
#
# ARM64 Windows runs x64 binaries under emulation on Windows 11 (build 22000
# or newer), so the x64 asset is the working answer there. Windows 10 ARM64
# cannot run the published x64 asset and must fail before downloading it.
#
# A 32-bit PowerShell host on 64-bit Windows reports "x86" in
# PROCESSOR_ARCHITECTURE and the real architecture in PROCESSOR_ARCHITEW6432,
# so consult the latter first.
$HostArch = if ($env:PROCESSOR_ARCHITEW6432) {
    $env:PROCESSOR_ARCHITEW6432
} else {
    $env:PROCESSOR_ARCHITECTURE
}

$IsArm64Host = $HostArch -eq "ARM64"

function Get-WindowsBuildNumber {
    try {
        $build = [int](Get-ItemPropertyValue -Path "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion" -Name "CurrentBuildNumber")
        if ($build -ge 0) {
            return $build
        }
    } catch {
        # Fall back for restricted registry access or older PowerShell hosts.
    }

    try {
        return [int][System.Environment]::OSVersion.Version.Build
    } catch {
        return -1
    }
}

if ($IsArm64Host) {
    $WindowsBuild = Get-WindowsBuildNumber
    if ($WindowsBuild -lt 22000) {
        $DetectedBuild = if ($WindowsBuild -ge 0) { "build $WindowsBuild" } else { "an unknown Windows build" }
        Write-Error "Windows ARM64 x64 emulation requires Windows 11 (build 22000 or newer); detected $DetectedBuild. Windows 10 ARM64 cannot run the published x86_64 binary. Build from source: https://github.com/EffortlessMetrics/perl-lsp/blob/master/docs/how-to/INSTALLATION.md"
    }
}

# Name the target as a whole literal rather than assembling it from an arch
# variable and a "-pc-windows-msvc" suffix. PowerShell cannot be executed on
# the Linux CI host, so the contract test in
# scripts/tests/test-install-target-selection.sh checks which targets this
# script can request by reading them out of the source. Assembling the triple
# from a variable hides it from that check, which is how the original defect
# ($Arch = "aarch64") stayed invisible.
$Target = if ($IsArm64Host -or $HostArch -eq "AMD64") {
    "x86_64-pc-windows-msvc"
} else {
    Write-Error "Unsupported architecture: $HostArch. Only x86_64 Windows binaries are published. Build from source: https://github.com/EffortlessMetrics/perl-lsp/blob/master/docs/how-to/INSTALLATION.md"
}

if ($IsArm64Host) {
    Write-Info "Detected system: Windows (ARM64) - installing $Target"
    Write-Warn "No native ARM64 Windows build is published. The x64 build runs under the x64 emulation in Windows 11 on ARM."
} else {
    Write-Info "Detected system: Windows ($HostArch) - $Target"
}

# Get version
if ($Version -eq "latest") {
    try {
        $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
        $Tag = $Release.tag_name
        Write-Info "Latest version: $Tag"
    } catch {
        Write-Error "Failed to fetch latest release: $_"
    }
} else {
    $Tag = if ($Version.StartsWith("v")) { $Version } else { "v$Version" }
}

$VersionNum = $Tag.TrimStart("v")

# Construct download URL
$Asset = "$Name-$VersionNum-$Target.zip"
$Url = "https://github.com/$Repo/releases/download/$Tag/$Asset"

Write-Info "Downloading $Name $Tag for $Target"

# Create temp directory
$TempDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }

try {
    # Download binary
    $ZipPath = Join-Path $TempDir $Asset
    Write-Info "Downloading from $Url"
    
    try {
        Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing
    } catch {
        Write-Error "Failed to download from $Url : $_"
    }
    
    # Download and verify checksum (optional)
    $ChecksumUrl = "https://github.com/$Repo/releases/download/$Tag/SHA256SUMS"
    $ChecksumPath = Join-Path $TempDir "SHA256SUMS"
    
    try {
        Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumPath -UseBasicParsing
        
        # Verify checksum
        $ExpectedHash = (Get-Content $ChecksumPath | Select-String $Asset).Line.Split(" ")[0]
        $ActualHash = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash.ToLower()
        
        if ($ExpectedHash -eq $ActualHash) {
            Write-Success "Checksum verified"
        } else {
            Write-Error "Checksum mismatch - expected: $ExpectedHash, got: $ActualHash"
        }
    } catch {
        Write-Warn "Could not download or verify checksums"
    }
    
    # Extract archive
    Write-Info "Extracting archive"
    $ExtractDir = Join-Path $TempDir "extract"
    Expand-Archive -Path $ZipPath -DestinationPath $ExtractDir -Force
    
    # Find the binary
    $ExtractedDir = Join-Path $ExtractDir "$Name-$VersionNum-$Target"
    if (-not (Test-Path $ExtractedDir)) {
        # Try without nested directory
        $ExtractedDir = $ExtractDir
    }
    
    $BinaryPath = Join-Path $ExtractedDir "$Name.exe"
    if (-not (Test-Path $BinaryPath)) {
        Write-Error "Binary not found at $BinaryPath"
    }
    
    # Create install directory
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    
    # Install binary
    $DestPath = Join-Path $InstallDir "$Name.exe"
    Write-Info "Installing $Name to $DestPath"
    
    # Remove old binary if exists
    if (Test-Path $DestPath) {
        Remove-Item $DestPath -Force
    }
    
    # Copy binary
    Copy-Item -Path $BinaryPath -Destination $DestPath -Force
    
    Write-Success "Installed $Name to $DestPath"

    # Verify installation
    try {
        $VersionOutput = & $DestPath --version 2>&1
        Write-Success "Installation verified: $VersionOutput"
    } catch {
        Write-Warn "Could not verify installation"
    }
    
    # Check PATH
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -like "*$InstallDir*") {
        Write-Success "$InstallDir is already in your PATH"
    } else {
        Write-Warn "$InstallDir is not in your PATH"
        Write-Host ""
        Write-Host "To add it to your PATH permanently, run:" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "  [Environment]::SetEnvironmentVariable('Path', `"`$env:Path;$InstallDir`", 'User')" -ForegroundColor White
        Write-Host ""
        Write-Host "Or add it temporarily for this session:" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "  `$env:Path += `";$InstallDir`"" -ForegroundColor White
        Write-Host ""
    }
    
    Write-Host ""
    Write-Host "Installation complete! 🎉" -ForegroundColor Green
    Write-Host ""
    Write-Host "To get started with Perl LSP:"
    Write-Host "  • VS Code: Install the Perl LSP extension from the marketplace"
    Write-Host "  • Other editors: Configure to use '$DestPath --stdio'"
    Write-Host ""
    Write-Host "For more information: https://github.com/$Repo"
    
} finally {
    # Cleanup
    if (Test-Path $TempDir) {
        Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
