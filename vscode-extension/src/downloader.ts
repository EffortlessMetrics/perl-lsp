import * as vscode from 'vscode';
import * as https from 'https';
import * as fs from 'fs';
import * as path from 'path';
import * as crypto from 'crypto';
import * as os from 'os';
import { promisify } from 'util';
import * as child_process from 'child_process';

const exec = promisify(child_process.exec);

interface ReleaseAsset {
    name: string;
    browser_download_url: string;
}

interface Release {
    tag_name: string;
    assets: ReleaseAsset[];
}

export class BinaryDownloader {
    private static readonly REPO_OWNER = 'EffortlessSteven';
    private static readonly REPO_NAME = 'tree-sitter-perl';
    private static readonly BINARY_NAME = 'perl-lsp';
    
    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly outputChannel: vscode.OutputChannel
    ) {}
    
    async ensureBinary(): Promise<string | null> {
        // Check if binary already exists
        const existingPath = this.getLocalBinaryPath();
        if (existingPath && fs.existsSync(existingPath)) {
            this.outputChannel.appendLine(`Using existing binary: ${existingPath}`);
            return existingPath;
        }
        
        // Download binary
        try {
            return await this.downloadWithProgress();
        } catch (error) {
            this.outputChannel.appendLine(`Failed to download binary: ${error}`);
            vscode.window.showErrorMessage(`Failed to download perl-lsp: ${error}`);
            return null;
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
            
            // Try multiple naming patterns that cargo-dist might use
            const possibleNames = [
                `perl-lsp-${release.tag_name}-${target}.tar.xz`,
                `perl-lsp-${release.tag_name}-${target}.tar.gz`,
                `perl-lsp-${release.tag_name}-${target}.zip`,
                `perl-lsp-${target}.tar.xz`,
                `perl-lsp-${target}.tar.gz`,
                `perl-lsp-${target}.zip`
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
            
            this.outputChannel.appendLine(`Found matching asset: ${assetName}`);
            
            // Find checksum file (try both extensions)
            const checksumName = `${assetName}.sha256`;
            const checksumAsset = release.assets.find(a => a.name === checksumName);
            
            // Download to temp directory
            const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-'));
            const archivePath = path.join(tempDir, assetName);
            const checksumPath = checksumAsset ? path.join(tempDir, checksumName) : null;
            
            try {
                // Download binary archive
                progress.report({ increment: 10, message: 'Downloading binary...' });
                await this.downloadFile(asset.browser_download_url, archivePath);
                
                if (token.isCancellationRequested) {
                    throw new Error('Download cancelled');
                }
                
                // Download and verify checksum if available
                if (checksumAsset && checksumPath) {
                    progress.report({ increment: 40, message: 'Verifying checksum...' });
                    await this.downloadFile(checksumAsset.browser_download_url, checksumPath);
                    
                    const expectedChecksum = fs.readFileSync(checksumPath, 'utf8').trim().split(' ')[0];
                    const actualChecksum = await this.calculateSHA256(archivePath);
                    
                    if (expectedChecksum !== actualChecksum) {
                        throw new Error('Checksum verification failed');
                    }
                    this.outputChannel.appendLine('Checksum verified successfully');
                }
                
                // Extract archive
                progress.report({ increment: 30, message: 'Extracting binary...' });
                const extractDir = path.join(tempDir, 'extracted');
                fs.mkdirSync(extractDir);
                
                // Choose extraction command based on file extension
                let extractCmd: string;
                if (assetName.endsWith('.tar.xz')) {
                    extractCmd = `tar -xJf "${archivePath}" -C "${extractDir}"`;
                } else if (assetName.endsWith('.tar.gz')) {
                    extractCmd = `tar -xzf "${archivePath}" -C "${extractDir}"`;
                } else if (assetName.endsWith('.zip')) {
                    extractCmd = `unzip -q "${archivePath}" -d "${extractDir}"`;
                } else {
                    throw new Error(`Unsupported archive format: ${assetName}`);
                }
                
                await exec(extractCmd);
                
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
        const url = `https://api.github.com/repos/${BinaryDownloader.REPO_OWNER}/${BinaryDownloader.REPO_NAME}/releases/latest`;
        
        return new Promise((resolve, reject) => {
            https.get(url, { headers: { 'User-Agent': 'vscode-perl-lsp' } }, (res) => {
                let data = '';
                res.on('data', chunk => data += chunk);
                res.on('end', () => {
                    try {
                        const release = JSON.parse(data);
                        if (release.message && release.message.includes('Not Found')) {
                            reject(new Error('No releases found'));
                        } else {
                            resolve(release);
                        }
                    } catch (e) {
                        reject(e);
                    }
                });
            }).on('error', reject);
        });
    }
    
    private async downloadFile(url: string, dest: string): Promise<void> {
        return new Promise((resolve, reject) => {
            const file = fs.createWriteStream(dest);
            
            https.get(url, { headers: { 'User-Agent': 'vscode-perl-lsp' } }, (response) => {
                // Handle redirects
                if (response.statusCode === 301 || response.statusCode === 302) {
                    const newUrl = response.headers.location;
                    if (newUrl) {
                        this.downloadFile(newUrl, dest).then(resolve).catch(reject);
                        return;
                    }
                }
                
                if (response.statusCode !== 200) {
                    reject(new Error(`Failed to download: ${response.statusCode}`));
                    return;
                }
                
                response.pipe(file);
                
                file.on('finish', () => {
                    file.close();
                    resolve();
                });
                
                file.on('error', (err) => {
                    fs.unlink(dest, () => {});
                    reject(err);
                });
            }).on('error', reject);
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
            return arch === 'arm64' ? 'aarch64-unknown-linux-gnu' : 'x86_64-unknown-linux-gnu';
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