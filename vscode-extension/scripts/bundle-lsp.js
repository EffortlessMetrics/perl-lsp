#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const PLATFORMS = [
    { platform: 'darwin', arch: 'x64', rustTarget: 'x86_64-apple-darwin' },
    { platform: 'darwin', arch: 'arm64', rustTarget: 'aarch64-apple-darwin' },
    { platform: 'linux', arch: 'x64', rustTarget: 'x86_64-unknown-linux-gnu' },
    { platform: 'linux', arch: 'arm64', rustTarget: 'aarch64-unknown-linux-gnu' },
    { platform: 'win32', arch: 'x64', rustTarget: 'x86_64-pc-windows-msvc' }
];

const extensionRoot = path.join(__dirname, '..');
const projectRoot = path.join(extensionRoot, '..');
const binDir = path.join(extensionRoot, 'bin');

// Create bin directory
if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
}

// Get current platform
const currentPlatform = process.platform;
const currentArch = process.arch;

console.log(`Building perl-lsp for ${currentPlatform}-${currentArch}...`);

// For development, just build for current platform
const platform = PLATFORMS.find(p => p.platform === currentPlatform && p.arch === currentArch);

if (!platform) {
    console.error(`Unsupported platform: ${currentPlatform}-${currentArch}`);
    process.exit(1);
}

try {
    // Build the binary
    console.log('Building perl-lsp binary...');
    const buildCmd = `cargo build -p perl-parser --bin perl-lsp --release`;
    execSync(buildCmd, { 
        cwd: projectRoot,
        stdio: 'inherit'
    });
    
    // Create platform directory
    const platformDir = path.join(binDir, `${platform.platform}-${platform.arch}`);
    if (!fs.existsSync(platformDir)) {
        fs.mkdirSync(platformDir, { recursive: true });
    }
    
    // Copy binary
    const binaryName = platform.platform === 'win32' ? 'perl-lsp.exe' : 'perl-lsp';
    const sourcePath = path.join(projectRoot, 'target', 'release', binaryName);
    const destPath = path.join(platformDir, binaryName);
    
    if (fs.existsSync(sourcePath)) {
        fs.copyFileSync(sourcePath, destPath);
        console.log(`Copied ${binaryName} to ${platformDir}`);
        
        // Make executable on Unix
        if (platform.platform !== 'win32') {
            fs.chmodSync(destPath, 0o755);
        }
    } else {
        console.error(`Binary not found at ${sourcePath}`);
        process.exit(1);
    }
    
    console.log('Bundle complete!');
} catch (error) {
    console.error('Build failed:', error.message);
    process.exit(1);
}

// For production builds, you would loop through all platforms
// and use cross-compilation or CI/CD to build for each target