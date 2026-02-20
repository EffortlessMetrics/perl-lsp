import * as vscode from 'vscode';
import * as https from 'https';
import * as http from 'http';
import * as fs from 'fs';
import * as path from 'path';
import * as crypto from 'crypto';
import * as os from 'os';
import { promisify } from 'util';
import * as child_process from 'child_process';
import * as tar from 'tar';
import AdmZip from 'adm-zip';

const execFile = promisify(child_process.execFile);

interface ReleaseAsset {
    name: string;
    browser_download_url: string;
}

interface Release {
    tag_name: string;
    assets: ReleaseAsset[];
}

export class BinaryDownloader {
    private static readonly REPO_OWNER = 'EffortlessMetrics';
    private static readonly REPO_NAME = 'perl-lsp';
    private static readonly BINARY_NAME = 'perl-lsp';
    
    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly outputChannel: vscode.OutputChannel
    ) {}
    
    async ensureBinary(): Promise<string | null> {
        const config = vscode.workspace.getConfiguration('perl-lsp');
        const channel = config.get<string>('channel', 'latest');
        const versionTag = config.get<string>('versionTag', '');
        
        // If channel is 'tag' and versionTag is specified, use that specific version
        if (channel === 'tag' && versionTag) {
            this.outputChannel.appendLine(`Using specific version: ${versionTag}`);
        }
        
        // Check if binary already exists
        const existingPath = this.getLocalBinaryPath();
        if (existingPath && fs.existsSync(existingPath)) {
            this.outputChannel.appendLine(`Using existing binary: ${existingPath}`);
            return existingPath;
        }
        
        // Show status bar while downloading
        const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
        statusBar.text = '$(sync~spin) Perl LSP: downloading binary...';
        statusBar.tooltip = 'Downloading Perl Language Server... Click to show logs';
        statusBar.command = 'perl-lsp.showOutput';
        statusBar.show();
        
        // Download binary
        try {
            return await this.downloadWithProgress();
        } catch (error: any) {
            const errorMsg = error?.message || String(error);
            this.outputChannel.appendLine(`Failed to download binary: ${errorMsg}`);
            
            // Provide helpful error messages
            if (errorMsg.includes('ECONNREFUSED') || errorMsg.includes('ETIMEDOUT') || errorMsg.includes('timeout')) {
                vscode.window.showErrorMessage(
                    'Failed to download perl-lsp: Network connection error. ' +
                    'Please check your internet connection and proxy settings.',
                    'Open Settings'
                ).then(choice => {
                    if (choice === 'Open Settings') {
                        vscode.commands.executeCommand('workbench.action.openSettings', 'http.proxy');
                    }
                });
            } else if (errorMsg.includes('tar') || errorMsg.includes('unzip')) {
                vscode.window.showErrorMessage(
                    'Failed to extract perl-lsp: Archive extraction failed. ' +
                    'Please ensure tar (Linux/macOS) or unzip (Windows) is installed.',
                    'Install Manually'
                ).then(choice => {
                    if (choice === 'Install Manually') {
                        vscode.env.openExternal(vscode.Uri.parse('https://github.com/EffortlessMetrics/perl-lsp#quick-install'));
                    }
                });
            } else {
                vscode.window.showErrorMessage(
                    `Failed to download perl-lsp: ${errorMsg}`,
                    'View Logs'
                ).then(choice => {
                    if (choice === 'View Logs') {
                        this.outputChannel.show();
                    }
                });
            }
            return null;
        } finally {
            statusBar.dispose();
        }
    }
    
    private async downloadWithProgress(): Promise<string> {
        return vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: 'Downloading Perl Language Server',
            cancellable: true
        }, async (progress, token) => {
            // Get latest release info
            progress.report({ increment: 0, message: 'Fetching release information...' });
            const release = await this.getLatestRelease();
            
            if (token.isCancellationRequested) {
                throw new Error('Download cancelled');
            }
            
            // Determine platform and architecture
            const target = this.getPlatformTarget();
            
            // Try multiple naming patterns for our release format
            const ext = process.platform === 'win32' ? '.zip' : '.tar.gz';
            const possibleNames = [
                `perl-lsp-${release.tag_name}-${target}${ext}`,
                `perl-lsp-v${release.tag_name.replace('v', '')}-${target}${ext}`,
                `perl-lsp-${target}${ext}`
            ];
            
            let assetName: string | undefined;
            let asset: ReleaseAsset | undefined;
            
            // Find the first matching asset
            for (const name of possibleNames) {
                asset = release.assets.find(a => a.name === name);
                if (asset) {
                    assetName = name;
                    break;
                }
            }
            
            if (!asset || !assetName) {
                const availableAssets = release.assets.map(a => a.name).join(', ');
                this.outputChannel.appendLine(`Target platform: ${target}`);
                this.outputChannel.appendLine(`Available assets: ${availableAssets}`);
                throw new Error(`No binary found for platform: ${target}. Available assets: ${availableAssets}`);
            }

            // Security check: Validate asset name to prevent path traversal
            if (!/^[a-zA-Z0-9_.-]+$/.test(assetName) || assetName.includes('..')) {
                throw new Error(`Invalid asset name detected: ${assetName}`);
            }
            
            this.outputChannel.appendLine(`Found matching asset: ${assetName}`);
            
            // Find checksum file (SHA256SUMS file contains all checksums)
            const checksumAsset = release.assets.find(a => a.name === 'SHA256SUMS');
            
            // Download to temp directory
            const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-'));
            const archivePath = path.join(tempDir, assetName);
            
            try {
                // Download binary archive
                progress.report({ increment: 10, message: 'Downloading binary...' });
                await this.downloadFile(asset.browser_download_url, archivePath);
                
                if (token.isCancellationRequested) {
                    throw new Error('Download cancelled');
                }
                
                // Download and verify checksum (required for security)
                if (!checksumAsset) {
                    throw new Error('Security check failed: No SHA256SUMS file found in release assets.');
                }

                progress.report({ increment: 40, message: 'Verifying checksum...' });
                const checksumPath = path.join(tempDir, 'SHA256SUMS');
                await this.downloadFile(checksumAsset.browser_download_url, checksumPath);

                // Find the checksum line for our file
                const checksums = fs.readFileSync(checksumPath, 'utf8');
                const lines = checksums.split('\n');
                const checksumLine = lines.find(line => line.includes(assetName!));

                if (!checksumLine) {
                    throw new Error(`Security check failed: Checksum for ${assetName} not found in SHA256SUMS file.`);
                }

                const expectedChecksum = checksumLine.split(/\s+/)[0].toLowerCase();
                const actualChecksum = await this.calculateSHA256(archivePath);

                if (expectedChecksum !== actualChecksum) {
                    throw new Error('Security check failed: Checksum verification failed (file may be corrupted or tampered with).');
                }
                this.outputChannel.appendLine('Checksum verified successfully');
                
                // Extract archive
                progress.report({ increment: 30, message: 'Extracting binary...' });
                const extractDir = path.join(tempDir, 'extracted');
                fs.mkdirSync(extractDir);
                
                // Choose extraction method based on file extension
                if (assetName.endsWith('.tar.gz')) {
                    await tar.x({
                        file: archivePath,
                        cwd: extractDir
                    });
                } else if (assetName.endsWith('.zip')) {
                    await new Promise<void>((resolve, reject) => {
                        const zip = new AdmZip(archivePath);
                        zip.extractAllToAsync(extractDir, true, true, (error) => {
                            if (error) {
                                reject(error);
                            } else {
                                resolve();
                            }
                        });
                    });
                } else if (assetName.endsWith('.tar.xz')) {
                    // Fallback to system tar for .tar.xz (node-tar doesn't support xz)
                    await execFile('tar', ['-xJf', archivePath, '-C', extractDir]);
                } else {
                    throw new Error(`Unsupported archive format: ${assetName}`);
                }
                
                // Find the binary
                const binaryName = process.platform === 'win32' ? 'perl-lsp.exe' : 'perl-lsp';
                const extractedBinary = this.findBinary(extractDir, binaryName);
                
                if (!extractedBinary) {
                    throw new Error('Binary not found in archive');
                }
                
                // Move to final location
                progress.report({ increment: 15, message: 'Installing binary...' });
                const finalPath = this.getLocalBinaryPath();
                const finalDir = path.dirname(finalPath);
                
                if (!fs.existsSync(finalDir)) {
                    fs.mkdirSync(finalDir, { recursive: true });
                }
                
                fs.copyFileSync(extractedBinary, finalPath);
                
                // Make executable on Unix
                if (process.platform !== 'win32') {
                    fs.chmodSync(finalPath, 0o755);
                }
                
                progress.report({ increment: 5, message: 'Complete!' });
                this.outputChannel.appendLine(`Binary installed to: ${finalPath}`);
                
                return finalPath;
                
            } finally {
                // Clean up temp directory
                try {
                    fs.rmSync(tempDir, { recursive: true, force: true });
                } catch (e) {
                    this.outputChannel.appendLine(`Failed to clean up temp dir: ${e}`);
                }
            }
        });
    }
    
    private async getLatestRelease(): Promise<Release> {
        const config = vscode.workspace.getConfiguration('perl-lsp');
        const channel = config.get<string>('channel', 'latest');
        const versionTag = config.get<string>('versionTag', '');
        const downloadBaseUrl = config.get<string>('downloadBaseUrl', '');
        
        // Handle internal base URL hosting
        if (downloadBaseUrl) {
            return this.getInternalRelease(downloadBaseUrl, versionTag || 'latest');
        }
        
        let url: string;
        if (channel === 'tag' && versionTag) {
            // Get specific release by tag
            url = `https://api.github.com/repos/${BinaryDownloader.REPO_OWNER}/${BinaryDownloader.REPO_NAME}/releases/tags/${versionTag}`;
        } else if (channel === 'stable') {
            // Get latest non-prerelease
            url = `https://api.github.com/repos/${BinaryDownloader.REPO_OWNER}/${BinaryDownloader.REPO_NAME}/releases`;
        } else {
            // Get latest release (including prereleases)
            url = `https://api.github.com/repos/${BinaryDownloader.REPO_OWNER}/${BinaryDownloader.REPO_NAME}/releases/latest`;
        }
        
        return new Promise((resolve, reject) => {
            const isHttps = url.startsWith('https:');
            const httpModule = isHttps ? https : http;
            
            httpModule.get(url, { headers: { 'User-Agent': 'vscode-perl-lsp' } }, (res) => {
                let data = '';
                res.on('data', chunk => data += chunk);
                res.on('end', () => {
                    try {
                        const parsed = JSON.parse(data);
                        if (parsed.message && parsed.message.includes('Not Found')) {
                            reject(new Error('No releases found'));
                        } else if (Array.isArray(parsed)) {
                            // For stable channel, find first non-prerelease
                            const stableRelease = parsed.find((r: any) => !r.prerelease);
                            if (stableRelease) {
                                resolve(stableRelease);
                            } else {
                                resolve(parsed[0]); // Fall back to latest
                            }
                        } else {
                            resolve(parsed);
                        }
                    } catch (e) {
                        reject(e);
                    }
                });
            }).on('error', reject);
        });
    }
    
    private async getInternalRelease(baseUrl: string, version: string): Promise<Release> {
        // For internal hosting, create a synthetic release object
        // This assumes the internal server hosts files directly without GitHub API
        const normalizedBaseUrl = baseUrl.endsWith('/') ? baseUrl.slice(0, -1) : baseUrl;
        const target = this.getPlatformTarget();
        const ext = process.platform === 'win32' ? '.zip' : '.tar.gz';
        
        // Try multiple naming patterns that might be used internally
        const possibleFilenames = [
            `perl-lsp-${version}-${target}${ext}`,
            `perl-lsp-v${version.replace('v', '')}-${target}${ext}`,
            `perl-lsp-${target}${ext}`,
            `perl-lsp${ext}` // Simplest case for internal hosting
        ];
        
        // Create synthetic release with all possible asset URLs
        const assets: ReleaseAsset[] = possibleFilenames.map(filename => ({
            name: filename,
            browser_download_url: `${normalizedBaseUrl}/${filename}`
        }));
        
        // Add potential checksum file
        assets.push({
            name: 'SHA256SUMS',
            browser_download_url: `${normalizedBaseUrl}/SHA256SUMS`
        });
        
        return {
            tag_name: version,
            assets
        };
    }
    
    private async downloadFile(url: string, dest: string, timeoutMs = 30000, redirects = 5): Promise<void> {
        return new Promise((resolve, reject) => {
            // Security check: Enforce HTTPS for remote URLs to prevent MITM attacks
            try {
                const parsedUrl = new URL(url);

                // Only allow http: and https: protocols
                if (parsedUrl.protocol !== 'http:' && parsedUrl.protocol !== 'https:') {
                    reject(new Error(`Unsupported protocol: ${parsedUrl.protocol}. Only HTTP and HTTPS are allowed.`));
                    return;
                }

                // Check for local addresses (full IPv4 loopback range 127.0.0.0/8)
                // Note: URL.hostname normalizes IPv6 addresses and never includes brackets
                const ipv4LoopbackRegex = /^127(?:\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)){3}$/;
                const isLocal = ['localhost', '::1'].includes(parsedUrl.hostname)
                    || parsedUrl.hostname.endsWith('.localhost')
                    || ipv4LoopbackRegex.test(parsedUrl.hostname);

                if (parsedUrl.protocol === 'http:' && !isLocal) {
                    reject(new Error(`Security violation: Insecure HTTP download prevented for remote host: ${parsedUrl.hostname}. Use HTTPS or a local server.`));
                    return;
                }
            } catch (e) {
                reject(new Error(`Invalid URL format: ${url}. Error: ${e instanceof Error ? e.message : String(e)}`));
                return;
            }

            const file = fs.createWriteStream(dest);
            let timedOut = false;
            
            // Set timeout
            const timeout = setTimeout(() => {
                timedOut = true;
                file.destroy();
                reject(new Error(`Download timeout after ${timeoutMs / 1000} seconds`));
            }, timeoutMs);
            
            // Honor VS Code proxy settings
            const httpConfig = vscode.workspace.getConfiguration('http');
            const proxyStrictSSL = httpConfig.get<boolean>('proxyStrictSSL', true);
            
            const options = {
                headers: { 'User-Agent': 'vscode-perl-lsp' },
                rejectUnauthorized: proxyStrictSSL
            };
            
            // Use appropriate module based on URL protocol
            const isHttps = url.startsWith('https:');
            const httpModule = isHttps ? https : http;
            
            const request = httpModule.get(url, options, (response) => {
                // Handle redirects
                if (response.statusCode === 301 || response.statusCode === 302) {
                    clearTimeout(timeout);
                    file.destroy();
                    const newUrl = response.headers.location;
                    if (newUrl) {
                        // Check redirect limit
                        if (redirects <= 0) {
                            reject(new Error('Too many redirects'));
                            return;
                        }

                        // Resolve relative URLs
                        let resolvedUrl: string;
                        try {
                            resolvedUrl = new URL(newUrl, url).toString();
                        } catch (e) {
                            reject(new Error(`Invalid redirect URL: ${newUrl}`));
                            return;
                        }

                        // Security check: Prevent downgrade from HTTPS to HTTP
                        if (isHttps && resolvedUrl.toLowerCase().startsWith('http:') && !resolvedUrl.toLowerCase().startsWith('https:')) {
                            reject(new Error('Security violation: Redirect from HTTPS to HTTP prevented'));
                            return;
                        }
                        this.downloadFile(resolvedUrl, dest, timeoutMs, redirects - 1).then(resolve).catch(reject);
                        return;
                    }
                }
                
                if (response.statusCode !== 200) {
                    clearTimeout(timeout);
                    file.destroy();
                    reject(new Error(`Failed to download: HTTP ${response.statusCode}`));
                    return;
                }
                
                response.pipe(file);
                
                file.on('finish', () => {
                    if (!timedOut) {
                        clearTimeout(timeout);
                        file.close();
                        resolve();
                    }
                });
                
                file.on('error', (err) => {
                    clearTimeout(timeout);
                    fs.unlink(dest, () => {});
                    reject(err);
                });
            });
            
            request.on('error', (err) => {
                clearTimeout(timeout);
                file.destroy();
                reject(err);
            });
            
            request.on('timeout', () => {
                request.destroy();
                reject(new Error('Request timeout'));
            });
        });
    }
    
    private async calculateSHA256(filePath: string): Promise<string> {
        return new Promise((resolve, reject) => {
            const hash = crypto.createHash('sha256');
            const stream = fs.createReadStream(filePath);
            
            stream.on('data', data => hash.update(data));
            stream.on('end', () => resolve(hash.digest('hex')));
            stream.on('error', reject);
        });
    }
    
    private getPlatformTarget(): string {
        const platform = process.platform;
        const arch = process.arch;
        
        // Map Node.js platform/arch to exact cargo-dist target triples
        if (platform === 'darwin') {
            return arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin';
        } else if (platform === 'linux') {
            const archPrefix = arch === 'arm64' ? 'aarch64' : 'x86_64';
            const libc = this.detectMusl() ? 'musl' : 'gnu';
            return `${archPrefix}-unknown-linux-${libc}`;
        } else if (platform === 'win32') {
            return arch === 'arm64' ? 'aarch64-pc-windows-msvc' : 'x86_64-pc-windows-msvc';
        }
        
        // Fallback to the old logic
        const platformMap: Record<string, string> = {
            'darwin': 'apple-darwin',
            'linux': 'unknown-linux-gnu',
            'win32': 'pc-windows-msvc'
        };
        
        const archMap: Record<string, string> = {
            'x64': 'x86_64',
            'arm64': 'aarch64'
        };
        
        const rustPlatform = platformMap[platform] || platform;
        const rustArch = archMap[arch] || arch;
        
        return `${rustArch}-${rustPlatform}`;
    }
    
    private detectMusl(): boolean {
        // Check for Alpine or musl
        if (fs.existsSync('/etc/alpine-release')) {
            return true;
        }
        
        // Check for musl libc
        const muslLibs = [
            '/lib/libc.musl-x86_64.so.1',
            '/lib/libc.musl-aarch64.so.1',
            '/lib/ld-musl-x86_64.so.1',
            '/lib/ld-musl-aarch64.so.1'
        ];
        
        return muslLibs.some(lib => fs.existsSync(lib));
    }
    
    private getLocalBinaryPath(): string {
        const platform = process.platform;
        const arch = process.arch;
        const binaryName = platform === 'win32' ? 'perl-lsp.exe' : 'perl-lsp';
        
        return path.join(
            this.context.globalStorageUri.fsPath,
            'bin',
            `${platform}-${arch}`,
            binaryName
        );
    }
    
    private findBinary(dir: string, name: string): string | null {
        const entries = fs.readdirSync(dir, { withFileTypes: true });
        
        for (const entry of entries) {
            const fullPath = path.join(dir, entry.name);
            
            if (entry.isDirectory()) {
                const found = this.findBinary(fullPath, name);
                if (found) return found;
            } else if (entry.name === name) {
                return fullPath;
            }
        }
        
        return null;
    }
}