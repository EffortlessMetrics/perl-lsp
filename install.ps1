# Perl LSP installer for Windows
# Usage: irm https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/main/install.ps1 | iex

param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:USERPROFILE\.local\bin"
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$Repo = "EffortlessMetrics/perl-lsp"
$Name = "perl-lsp"

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

# Detect architecture
$Arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
    "aarch64"
} elseif ($env:PROCESSOR_ARCHITECTURE -eq "AMD64") {
    "x86_64"
} else {
    Write-Error "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE"
}

$Target = "$Arch-pc-windows-msvc"
Write-Info "Detected system: Windows ($Arch) - $Target"

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

# Construct download URL
$Asset = "$Name-$Tag-$Target.zip"
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
            Write-Warn "Checksum mismatch - expected: $ExpectedHash, got: $ActualHash"
        }
    } catch {
        Write-Warn "Could not download or verify checksums"
    }
    
    # Extract archive
    Write-Info "Extracting archive"
    $ExtractDir = Join-Path $TempDir "extract"
    Expand-Archive -Path $ZipPath -DestinationPath $ExtractDir -Force
    
    # Find the binary
    $ExtractedDir = Join-Path $ExtractDir "$Name-$Tag-$Target"
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