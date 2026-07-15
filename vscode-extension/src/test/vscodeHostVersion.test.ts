import {
  MINIMUM_SUPPORTED_VSCODE_VERSION_REQUEST,
  resolveVSCodeTestVersion,
} from './vscodeHostVersion';
import packageJson from '../../package.json';

describe('VS Code host version contract', () => {
  test('matches the shipped engines.vscode floor', () => {
    expect(packageJson.engines.vscode).toBe('^1.125.0');
    expect(MINIMUM_SUPPORTED_VSCODE_VERSION_REQUEST).toBe('1.125.0');
  });

  test('defaults to current stable', () => {
    expect(resolveVSCodeTestVersion(undefined)).toBe('stable');
    expect(resolveVSCodeTestVersion('')).toBe('stable');
  });

  test('accepts stable, Insiders, and the declared minimum exact release', () => {
    expect(resolveVSCodeTestVersion('stable')).toBe('stable');
    expect(resolveVSCodeTestVersion('insiders')).toBe('insiders');
    expect(resolveVSCodeTestVersion(MINIMUM_SUPPORTED_VSCODE_VERSION_REQUEST)).toBe(
      MINIMUM_SUPPORTED_VSCODE_VERSION_REQUEST,
    );
    expect(resolveVSCodeTestVersion('1.126.2')).toBe('1.126.2');
  });

  test('rejects malformed and below-floor versions', () => {
    expect(() => resolveVSCodeTestVersion('1.125')).toThrow(/exact major\.minor\.patch/);
    expect(() => resolveVSCodeTestVersion('1.124.9')).toThrow(/below the declared minimum/);
    expect(() => resolveVSCodeTestVersion('latest')).toThrow(/exact major\.minor\.patch/);
  });
});
