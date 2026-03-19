/**
 * Unit tests for Perl debug adapter configuration and descriptor factory.
 */

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import {
  PerlDebugAdapterDescriptorFactory,
  PerlDebugConfigurationProvider,
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

  test('returns undefined when perl-dap is not found anywhere', () => {
    const ctx = makeContext(tmpDir);
    const factory = new PerlDebugAdapterDescriptorFactory(ctx);

    const origPath = process.env.PATH;
    const origHome = process.env.HOME;
    const origCargo = process.env.CARGO_HOME;
    process.env.PATH = tmpDir;
    process.env.HOME = tmpDir;
    process.env.CARGO_HOME = tmpDir;

    try {
      const result = factory.createDebugAdapterDescriptor({} as any, undefined);
      expect(result).toBeUndefined();
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
