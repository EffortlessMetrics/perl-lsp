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
    prerelease?: boolean;
    assets: ReleaseAsset[];
}

/**
 * Parse the local version string from `perllsp --version` stdout.
 *
 * The binary prints three lines; only the first is needed:
 *   perllsp 0.12.0
 *   Git tag: v0.12.0
 *   Perl Language Server using perl-parser v3
 *
 * Returns the semver string (e.g. "0.12.0") or null if the format is unexpected.
 */
export function parseLocalVersion(versionOutput: string): string | null {
    const firstLine = versionOutput.split('\n')[0].trim();
    const match = /^(?:perllsp|perl-lsp)\s+(\S+)/.exec(firstLine);
    return match ? match[1] : null;
}

/**
 * Numeric semver comparison. Strips a leading 'v' from either argument.
 * Returns -1 if a < b, 0 if equal, 1 if a > b.
 */
export function compareVersions(a: string, b: string): -1 | 0 | 1 {
    const normalize = (v: string) => v.replace(/^v/, '').split('.').map(n => parseInt(n, 10));
    const [aMaj, aMin, aPat] = normalize(a);
    const [bMaj, bMin, bPat] = normalize(b);
    for (const [x, y] of [[aMaj, bMaj], [aMin, bMin], [aPat, bPat]] as [number, number][]) {
        if (x < y) { return -1; }
        if (x > y) { return 1; }
    }
    return 0;
}

export class BinaryDownloader {
    private static readonly REPO_OWNER = 'EffortlessMetrics';
    private static readonly REPO_NAME = 'perl-lsp';
    private static readonly BINARY_NAME = 'perllsp';
    
    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly outputChannel: vscode.OutputChannel
    ) {}
    
    async ensureBinary(forceDownload = false): Promise<string | null> {
        const config = vscode.workspace.getConfiguration('perl-lsp');
        const channel = config.get<string>('channel', 'latest');
        const versionTag = config.get<string>('versionTag', '');
        
        // If channel is 'tag' and versionTag is specified, use that specific version
        if (channel === 'tag' && versionTag) {
            this.outputChannel.appendLine(`Using specific version: ${versionTag}`);
        }
        
        // Check if binary already exists
        const existingPath = this.getLocalBinaryPath();
        if (!forceDownload && existingPath && fs.existsSync(existingPath)) {
            this.outputChannel.appendLine(`Using existing binary: ${existingPath}`);
            return existingPath;
        }

        if (forceDownload && existingPath && fs.existsSync(existingPath)) {
            this.outputChannel.appendLine(`Refreshing existing binary: ${existingPath}`);
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
        } catch (error: unknown) {
            const errorMsg = error instanceof Error ? error.message : String(error);
            this.outputChannel.appendLine(`Failed to download binary: ${errorMsg}`);

            const manualInstallUrl = 'https://github.com/EffortlessMetrics/perl-lsp#quick-start';
            const manualInstallNote = 'To use a manually installed binary, set the "perl-lsp.serverPath" setting to its path.';

            let message: string;
            let buttons: string[];

            if (errorMsg.includes('ECONNREFUSED') || errorMsg.includes('ETIMEDOUT') || errorMsg.includes('timeout')) {
                // Network connectivity failure — proxy, VPN, or firewall
                message =
                    'perl-lsp: Binary download failed — network error ' +
                    `(${errorMsg.split('\n')[0]}). ` +
                    'Check your proxy/VPN settings (http.proxy in VS Code settings). ' +
                    manualInstallNote;
                buttons = ['Open Proxy Settings', 'Install Manually'];
            } else if (errorMsg.includes('No binary found for platform')) {
                // Architecture or OS not supported by the release
                const platformMatch = /platform:\s*([^\s.]+)/.exec(errorMsg);
                const platformStr = platformMatch ? platformMatch[1] : 'your platform';
                message =
                    `perl-lsp: No pre-built binary for ${platformStr}. ` +
                    'Build from source or download a compatible binary manually. ' +
                    manualInstallNote;
                buttons = ['Install Manually'];
            } else if (errorMsg.includes('HTTP 403')) {
                // GitHub rate limit or auth failure
                message =
                    'perl-lsp: Download blocked (HTTP 403 — GitHub rate limit). ' +
                    'Wait a few minutes, or set the GITHUB_TOKEN environment variable to increase your rate limit. ' +
                    manualInstallNote;
                buttons = ['Install Manually', 'View Logs'];
            } else if (errorMsg.includes('HTTP 404')) {
                // Release or asset not found
                message =
                    'perl-lsp: Binary not found (HTTP 404). ' +
                    'The release asset may not exist yet for this platform. ' +
                    manualInstallNote;
                buttons = ['Install Manually', 'View Logs'];
            } else if (errorMsg.toLowerCase().includes('checksum') || errorMsg.includes('SHA256SUMS')) {
                // Corrupted or tampered download, or missing checksum file
                message =
                    'perl-lsp: Checksum verification failed — download may be corrupted. ' +
                    'Please retry. If this persists, install manually. ' +
                    manualInstallNote;
                buttons = ['Install Manually', 'View Logs'];
            } else if (errorMsg.includes('tar') || errorMsg.includes('unzip') || errorMsg.includes('extract')) {
                // Archive extraction failure
                message =
                    'perl-lsp: Archive extraction failed. ' +
                    'Ensure tar (Linux/macOS) or the built-in zip support (Windows) is working. ' +
                    manualInstallNote;
                buttons = ['Install Manually', 'View Logs'];
            } else {
                // Generic fallback — always surface the manual install path
                message =
                    `perl-lsp: Binary download failed — ${errorMsg.split('\n')[0]}. ` +
                    manualInstallNote;
                buttons = ['Install Manually', 'View Logs'];
            }

            vscode.window.showErrorMessage(message, ...buttons).then((choice: string | undefined) => {
                if (choice === 'Install Manually') {
                    vscode.env.openExternal(vscode.Uri.parse(manualInstallUrl));
                } else if (choice === 'Open Proxy Settings') {
                    vscode.commands.executeCommand('workbench.action.openSettings', 'http.proxy');
                } else if (choice === 'View Logs') {
                    this.outputChannel.show();
                }
            });

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
                `perllsp-${release.tag_name}-${target}${ext}`,
                `perllsp-v${release.tag_name.replace('v', '')}-${target}${ext}`,
                `perllsp-${target}${ext}`,
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
                const binaryNames = process.platform === 'win32'
                    ? ['perllsp.exe', 'perl-lsp.exe']
                    : ['perllsp', 'perl-lsp'];
                const extractedBinary =
                    binaryNames.map(name => this.findBinary(extractDir, name)).find(Boolean) ?? null;
                
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

                // Best-effort: copy perl-dap if found in archive
                const dapName = process.platform === 'win32' ? 'perl-dap.exe' : 'perl-dap';
                const extractedDap = this.findBinary(extractDir, dapName);
                if (extractedDap) {
                    const dapDest = path.join(finalDir, dapName);
                    try {
                        fs.copyFileSync(extractedDap, dapDest);
                        if (process.platform !== 'win32') {
                            fs.chmodSync(dapDest, 0o755);
                        }
                        this.outputChannel.appendLine(`Debug adapter installed to: ${dapDest}`);
                    } catch (e) {
                        this.outputChannel.appendLine(`Note: could not install perl-dap: ${e}`);
                    }
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
                        const parsed: unknown = JSON.parse(data);
                        if (parsed && typeof parsed === 'object' && !Array.isArray(parsed) && 'message' in parsed) {
                            const msg = parsed as { message: string };
                            if (msg.message.includes('Not Found')) {
                                reject(new Error('No releases found'));
                                return;
                            }
                        }
                        if (Array.isArray(parsed)) {
                            // For stable channel, find first non-prerelease
                            const releases = parsed as Release[];
                            const stableRelease = releases.find(r => !r.prerelease);
                            if (stableRelease) {
                                resolve(stableRelease);
                            } else {
                                resolve(releases[0]); // Fall back to latest
                            }
                        } else {
                            resolve(parsed as Release);
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
            `perllsp-${version}-${target}${ext}`,
            `perllsp-v${version.replace('v', '')}-${target}${ext}`,
            `perllsp-${target}${ext}`,
            `perllsp${ext}`,
            `perl-lsp-${version}-${target}${ext}`,
            `perl-lsp-v${version.replace('v', '')}-${target}${ext}`,
            `perl-lsp-${target}${ext}`,
            `perl-lsp${ext}`
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
    
    private async downloadFile(url: string, dest: string, timeoutMs = 30000): Promise<void> {
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
                        // Security check: Prevent downgrade from HTTPS to HTTP
                        if (isHttps && newUrl.toLowerCase().startsWith('http:') && !newUrl.toLowerCase().startsWith('https:')) {
                            reject(new Error('Security violation: Redirect from HTTPS to HTTP prevented'));
                            return;
                        }
                        this.downloadFile(newUrl, dest, timeoutMs).then(resolve).catch(reject);
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
        const binaryName = platform === 'win32' ? 'perllsp.exe' : 'perllsp';
        
        return path.join(
            this.context.globalStorageUri.fsPath,
            'bin',
            `${platform}-${arch}`,
            binaryName
        );
    }
    
    /**
     * Returns the path where perl-dap would be placed inside the auto-download
     * directory.  Used by debugAdapter.ts to locate the binary without
     * duplicating path logic.
     */
    static getLocalDapPath(context: vscode.ExtensionContext): string {
        const platform = process.platform;
        const arch = process.arch;
        const dapName = platform === 'win32' ? 'perl-dap.exe' : 'perl-dap';
        return path.join(
            context.globalStorageUri.fsPath,
            'bin',
            `${platform}-${arch}`,
            dapName
        );
    }

    /**
     * Silent background update check. Fire-and-forget from activate().
     *
     * No-ops when:
     * - channel === 'tag' (user has pinned a version)
     * - serverPath is user-configured (downloader doesn't own the binary)
     * - binary is bundled (under extensionPath, not globalStorageUri)
     * - updateCheckInterval === 0 (user disabled checks)
     * - not enough time has elapsed since the last check
     * - versions are equal or local is ahead
     *
     * All errors are logged to the output channel; none are shown to the user.
     */
    async checkForUpdateSilent(): Promise<void> {
        const config = vscode.workspace.getConfiguration('perl-lsp');

        // Guard: skip if user pinned a specific version
        const channel = config.get<string>('channel', 'latest');
        if (channel === 'tag') { return; }

        // Guard: skip if user manages their own binary
        const userPath = config.get<string>('serverPath', '');
        if (userPath) { return; }

        // Guard: only applies to auto-downloaded binaries (under globalStorageUri)
        const binaryPath = this.getLocalBinaryPath();
        const storagePath = this.context.globalStorageUri.fsPath;
        if (!binaryPath.startsWith(storagePath)) { return; }
        if (!fs.existsSync(binaryPath)) { return; }

        // Guard: check interval (treat negative values same as 0 — disabled)
        const intervalHours = config.get<number>('updateCheckInterval', 24);
        if (intervalHours <= 0) { return; }
        const lastCheck = this.context.globalState.get<number>('perl-lsp.lastUpdateCheck', 0);
        const elapsedHours = (Date.now() - lastCheck) / (1000 * 60 * 60);
        if (elapsedHours < intervalHours) { return; }

        // Record that we checked (even if the check fails) to avoid hammering
        await this.context.globalState.update('perl-lsp.lastUpdateCheck', Date.now());

        try {
            const localVersion = await this.getLocalVersion(binaryPath);
            if (!localVersion) {
                this.outputChannel.appendLine('[update-check] Could not read local version — skipping');
                return;
            }

            const release = await this.getLatestRelease();
            const remoteVersion = release.tag_name.replace(/^v/, '');

            if (compareVersions(localVersion, remoteVersion) >= 0) {
                this.outputChannel.appendLine(`[update-check] Up to date (${localVersion})`);
                return;
            }

            this.outputChannel.appendLine(`[update-check] New version available: ${remoteVersion} (installed: ${localVersion})`);

            const autoUpdate = config.get<boolean>('autoUpdate', false);
            if (autoUpdate) {
                this.outputChannel.appendLine(`[update-check] Auto-updating to ${remoteVersion}`);
                await this.ensureBinary(true);
                return;
            }

            const choice = await vscode.window.showInformationMessage(
                `perllsp ${remoteVersion} is available (installed: ${localVersion})`,
                'Update',
                'Dismiss',
                "Don't ask again"
            );

            if (choice === 'Update') {
                await this.ensureBinary(true);
            } else if (choice === "Don't ask again") {
                await config.update('updateCheckInterval', 0, vscode.ConfigurationTarget.Global);
            }
            // 'Dismiss' is a no-op — will check again next interval
        } catch (err: unknown) {
            const msg = err instanceof Error ? err.message : String(err);
            this.outputChannel.appendLine(`[update-check] Skipping: ${msg}`);
        }
    }

    private async getLocalVersion(binaryPath: string): Promise<string | null> {
        return new Promise(resolve => {
            child_process.execFile(binaryPath, ['--version'], { timeout: 5000 }, (err, stdout) => {
                if (err) { resolve(null); return; }
                resolve(parseLocalVersion(stdout));
            });
        });
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
