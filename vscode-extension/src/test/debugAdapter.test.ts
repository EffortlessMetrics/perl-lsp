/**
 * Unit tests for Perl debug adapter configuration and descriptor factory.
 */

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import {
  PerlDebugAdapterDescriptorFactory,
  PerlDebugConfigurationProvider,
  buildLaunchJsonContent,
  hasLaunchJson,
  offerDebugConfigOnFirstPerlOpen,
  parseDebugTestLaunchTarget,
  resetDebugConfigPromptFlag,
  rewriteDebugTestLensCommand,
  VSCODE_DEBUG_TEST_COMMAND,
} from '../debugAdapter';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function makeContext(storagePath?: string): any {
  const dir = storagePath ?? fs.mkdtempSync(path.join(os.tmpdir(), 'dap-test-'));
  return {
    globalStorageUri: { fsPath: dir },
    extensionPath: dir,
    subscriptions: [],
  };
}

// ---------------------------------------------------------------------------
// PerlDebugConfigurationProvider
// ---------------------------------------------------------------------------
describe('PerlDebugConfigurationProvider', () => {
  let provider: PerlDebugConfigurationProvider;

  beforeEach(() => {
    provider = new PerlDebugConfigurationProvider();
  });

  describe('resolveDebugConfiguration', () => {
    test('fills in defaults for empty config when active editor is Perl', () => {
      const vscode = require('vscode');
      vscode.window.activeTextEditor = {
        document: { languageId: 'perl', uri: { fsPath: '/test.pl' } },
      };

      const config: any = {};
      provider.resolveDebugConfiguration(undefined, config);

      expect(config.type).toBe('perl');
      expect(config.name).toBe('Launch Perl');
      expect(config.request).toBe('launch');
      expect(config.program).toBe('${file}');

      vscode.window.activeTextEditor = undefined;
    });

    test('does not modify config with existing type/request/name', () => {
      const config: any = {
        type: 'perl',
        request: 'launch',
        name: 'Custom Debug',
        program: '/my/script.pl',
      };
      const result = provider.resolveDebugConfiguration(undefined, config);
      expect(result).toBeDefined();
      expect((result as any).program).toBe('/my/script.pl');
    });

    test('sets attach defaults for TCP mode (no processId)', () => {
      const config: any = {
        type: 'perl',
        request: 'attach',
        name: 'Attach',
      };
      const result = provider.resolveDebugConfiguration(undefined, config);

      expect((result as any).host).toBe('localhost');
      expect((result as any).port).toBe(13603);
    });

    test('preserves user-supplied attach host and port', () => {
      const config: any = {
        type: 'perl',
        request: 'attach',
        name: 'Attach Custom',
        host: '10.0.0.1',
        port: 5000,
      };
      const result = provider.resolveDebugConfiguration(undefined, config);

      expect((result as any).host).toBe('10.0.0.1');
      expect((result as any).port).toBe(5000);
    });

    test('skips TCP defaults when processId is provided', () => {
      const config: any = {
        type: 'perl',
        request: 'attach',
        name: 'Attach PID',
        processId: 42,
      };
      const result = provider.resolveDebugConfiguration(undefined, config);

      expect((result as any).host).toBeUndefined();
      expect((result as any).port).toBeUndefined();
    });

    test('returns undefined when launch has no program', async () => {
      const config: any = {
        type: 'perl',
        request: 'launch',
        name: 'No Program',
      };
      const result = provider.resolveDebugConfiguration(undefined, config);

      if (result && typeof (result as any).then === 'function') {
        const resolved = await result;
        expect(resolved).toBeUndefined();
      }
    });
  });

  describe('provideDebugConfigurations', () => {
    test('provides at least 3 default configurations', () => {
      const configs = provider.provideDebugConfigurations(undefined);
      expect(Array.isArray(configs)).toBe(true);
      expect((configs as any[]).length).toBeGreaterThanOrEqual(3);
    });

    test('includes launch, attach by TCP, and attach by PID templates', () => {
      const configs = provider.provideDebugConfigurations(undefined) as any[];

      const hasLaunch = configs.some(c => c.request === 'launch');
      const hasTCPAttach = configs.some(c => c.request === 'attach' && c.port);
      const hasPIDAttach = configs.some(c => c.request === 'attach' && c.processId);

      expect(hasLaunch).toBe(true);
      expect(hasTCPAttach).toBe(true);
      expect(hasPIDAttach).toBe(true);
    });

    test('all configurations have type "perl"', () => {
      const configs = provider.provideDebugConfigurations(undefined) as any[];
      for (const config of configs) {
        expect(config.type).toBe('perl');
      }
    });
  });
});

// ---------------------------------------------------------------------------
// PerlDebugAdapterDescriptorFactory
// ---------------------------------------------------------------------------
describe('PerlDebugAdapterDescriptorFactory', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dap-factory-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test('returns undefined and shows an actionable warning when perl-dap is not found anywhere', () => {
    const ctx = makeContext(tmpDir);
    const factory = new PerlDebugAdapterDescriptorFactory(ctx);
    const vscode = require('vscode');

    const origPath = process.env.PATH;
    const origHome = process.env.HOME;
    const origCargo = process.env.CARGO_HOME;
    process.env.PATH = tmpDir;
    process.env.HOME = tmpDir;
    process.env.CARGO_HOME = tmpDir;

    try {
      const result = factory.createDebugAdapterDescriptor({} as any, undefined);
      expect(result).toBeUndefined();
      expect(vscode.window.showErrorMessage).toHaveBeenCalledWith(
        expect.stringContaining('perl-dap'),
        'Reinstall',
        'Open Debugging Guide'
      );
    } finally {
      process.env.PATH = origPath;
      process.env.HOME = origHome;
      process.env.CARGO_HOME = origCargo;
    }
  });

  test('finds perl-dap in the auto-download directory', () => {
    const binDir = path.join(tmpDir, 'bin', `${process.platform}-${process.arch}`);
    fs.mkdirSync(binDir, { recursive: true });
    const dapName = process.platform === 'win32' ? 'perl-dap.exe' : 'perl-dap';
    const dapPath = path.join(binDir, dapName);
    fs.writeFileSync(dapPath, '#!/bin/sh\necho ok');
    if (process.platform !== 'win32') {
      fs.chmodSync(dapPath, 0o755);
    }

    const ctx = makeContext(tmpDir);
    const factory = new PerlDebugAdapterDescriptorFactory(ctx);
    const result = factory.createDebugAdapterDescriptor({} as any, undefined) as any;

    expect(result).toBeDefined();
    expect(result.command).toBe(dapPath);
  });

  test('descriptor includes RUST_LOG=debug environment variable', () => {
    const binDir = path.join(tmpDir, 'bin', `${process.platform}-${process.arch}`);
    fs.mkdirSync(binDir, { recursive: true });
    const dapName = process.platform === 'win32' ? 'perl-dap.exe' : 'perl-dap';
    const dapPath = path.join(binDir, dapName);
    fs.writeFileSync(dapPath, '#!/bin/sh\necho ok');
    if (process.platform !== 'win32') {
      fs.chmodSync(dapPath, 0o755);
    }

    const ctx = makeContext(tmpDir);
    const factory = new PerlDebugAdapterDescriptorFactory(ctx);
    const result = factory.createDebugAdapterDescriptor({} as any, undefined) as any;

    expect(result.options.env.RUST_LOG).toBe('debug');
  });
});

// ---------------------------------------------------------------------------
// debug test command wiring
// ---------------------------------------------------------------------------
describe('debug test command helpers', () => {
  test('rewrites server debug-test code lenses to the VS Code command', () => {
    const lens = {
      command: {
        title: 'Debug Test',
        command: 'perl.debugTest',
        arguments: ['file:///tmp/basic.t::test_basic'],
      },
    };

    expect(rewriteDebugTestLensCommand(lens).command.command).toBe(VSCODE_DEBUG_TEST_COMMAND);
  });

  test('leaves unrelated code lenses unchanged', () => {
    const lens = {
      command: {
        title: 'Run Test',
        command: 'perl.runTest',
      },
    };

    expect(rewriteDebugTestLensCommand(lens)).toEqual(lens);
  });

  test('parses a code-lens test id into a launch target', () => {
    const fileUri = process.platform === 'win32' ? 'file:///C:/tmp/basic.t' : 'file:///tmp/basic.t';
    const expectedProgram =
      process.platform === 'win32' ? path.normalize('C:/tmp/basic.t') : '/tmp/basic.t';

    expect(parseDebugTestLaunchTarget(`${fileUri}::test_basic`)).toEqual({
      label: 'test_basic',
      program: expectedProgram,
      args: [],
    });
  });

  test('parses a TestItem-like object into a launch target', () => {
    expect(parseDebugTestLaunchTarget({
      label: 'constructor',
      uri: { fsPath: path.normalize('/workspace/t/basic.t') },
      args: ['--verbose'],
    })).toEqual({
      label: 'constructor',
      program: path.normalize('/workspace/t/basic.t'),
      args: ['--verbose'],
    });
  });

  test('returns undefined for an invalid debug target payload', () => {
    expect(parseDebugTestLaunchTarget(null)).toBeUndefined();
    expect(parseDebugTestLaunchTarget({ label: 'missing-uri' })).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// buildLaunchJsonContent
// ---------------------------------------------------------------------------
describe('buildLaunchJsonContent', () => {
  test('launch-script template produces valid JSON with perl type', () => {
    const content = buildLaunchJsonContent('launch-script');
    const parsed = JSON.parse(content);
    expect(parsed.version).toBe('0.2.0');
    expect(Array.isArray(parsed.configurations)).toBe(true);
    const cfg = parsed.configurations[0];
    expect(cfg.type).toBe('perl');
    expect(cfg.request).toBe('launch');
  });

  test('attach-process template produces attach config with host and port', () => {
    const content = buildLaunchJsonContent('attach-process');
    const parsed = JSON.parse(content);
    const cfg = parsed.configurations[0];
    expect(cfg.type).toBe('perl');
    expect(cfg.request).toBe('attach');
    expect(cfg.host).toBe('localhost');
    expect(cfg.port).toBe(13603);
  });

  test('remote-ssh template produces attach config with configurable host', () => {
    const content = buildLaunchJsonContent('remote-ssh');
    const parsed = JSON.parse(content);
    const cfg = parsed.configurations[0];
    expect(cfg.type).toBe('perl');
    expect(cfg.request).toBe('attach');
    expect(typeof cfg.host).toBe('string');
  });

  test('all template produces multiple configurations', () => {
    const content = buildLaunchJsonContent('all');
    const parsed = JSON.parse(content);
    expect(parsed.configurations.length).toBeGreaterThanOrEqual(3);
    const types = parsed.configurations.map((c: any) => c.type);
    expect(types.every((t: string) => t === 'perl')).toBe(true);
  });

  test('unknown template falls back to launch-script', () => {
    const content = buildLaunchJsonContent('unknown-template');
    const parsed = JSON.parse(content);
    const cfg = parsed.configurations[0];
    expect(cfg.type).toBe('perl');
    expect(cfg.request).toBe('launch');
  });
});

// ---------------------------------------------------------------------------
// hasLaunchJson
// ---------------------------------------------------------------------------
describe('hasLaunchJson', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'launch-json-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test('returns false when .vscode/launch.json does not exist', () => {
    expect(hasLaunchJson(tmpDir)).toBe(false);
  });

  test('returns false when .vscode directory is missing', () => {
    expect(hasLaunchJson(path.join(tmpDir, 'nonexistent'))).toBe(false);
  });

  test('returns true when .vscode/launch.json exists', () => {
    const vscodDir = path.join(tmpDir, '.vscode');
    fs.mkdirSync(vscodDir);
    fs.writeFileSync(path.join(vscodDir, 'launch.json'), '{}');
    expect(hasLaunchJson(tmpDir)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// offerDebugConfigOnFirstPerlOpen
// ---------------------------------------------------------------------------
describe('offerDebugConfigOnFirstPerlOpen', () => {
  const vscode = require('vscode');

  beforeEach(() => {
    resetDebugConfigPromptFlag();
    jest.clearAllMocks();
    vscode.workspace.workspaceFolders = undefined;
  });

  afterEach(() => {
    vscode.workspace.workspaceFolders = undefined;
  });

  test('does nothing for non-perl documents', async () => {
    const doc = { languageId: 'javascript' };
    await offerDebugConfigOnFirstPerlOpen(doc as any);
    expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
  });

  test('does nothing when no workspace folders are open', async () => {
    vscode.workspace.workspaceFolders = [];
    const doc = { languageId: 'perl' };
    await offerDebugConfigOnFirstPerlOpen(doc as any);
    expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
  });

  test('shows onboarding prompt for perl document in workspace without launch.json', async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'onboard-test-'));
    try {
      vscode.workspace.workspaceFolders = [{ uri: { fsPath: tmpDir }, name: 'test' }];
      const doc = { languageId: 'perl' };
      await offerDebugConfigOnFirstPerlOpen(doc as any);
      expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
        expect.stringContaining('debug configuration'),
        expect.any(String),
        expect.any(String)
      );
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test('does not show prompt when launch.json already exists', async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'onboard-exists-'));
    try {
      const vscodDir = path.join(tmpDir, '.vscode');
      fs.mkdirSync(vscodDir);
      fs.writeFileSync(path.join(vscodDir, 'launch.json'), '{}');
      vscode.workspace.workspaceFolders = [{ uri: { fsPath: tmpDir }, name: 'test' }];
      const doc = { languageId: 'perl' };
      await offerDebugConfigOnFirstPerlOpen(doc as any);
      expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test('shows prompt only once per session even with multiple perl opens', async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'onboard-once-'));
    try {
      vscode.workspace.workspaceFolders = [{ uri: { fsPath: tmpDir }, name: 'test' }];
      const doc = { languageId: 'perl' };
      await offerDebugConfigOnFirstPerlOpen(doc as any);
      await offerDebugConfigOnFirstPerlOpen(doc as any);
      expect(vscode.window.showInformationMessage).toHaveBeenCalledTimes(1);
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });
});
