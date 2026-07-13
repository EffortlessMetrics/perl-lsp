import type { execFile as ExecFile } from 'child_process';
import {
  MANAGED_BINARY_HEALTH_TIMEOUT_MS,
  runLanguageServerHealthCheck,
} from '../languageServerHealth';

const mockedExecFile = jest.fn();
const execFileOption = mockedExecFile as unknown as typeof ExecFile;

describe('runLanguageServerHealthCheck', () => {
  afterEach(() => {
    jest.useRealTimers();
    mockedExecFile.mockReset();
  });

  test('accepts the documented ok version response and owns the timeout', async () => {
    let callback!: (error: Error | null, stdout: string, stderr: string) => void;
    const child = { kill: jest.fn(() => true) };
    mockedExecFile.mockImplementation((_path, args, options, processCallback) => {
      expect(args).toEqual(['--health']);
      expect(options.timeout).toBe(MANAGED_BINARY_HEALTH_TIMEOUT_MS);
      callback = processCallback;
      return child;
    });
    const log = { appendLine: jest.fn() };
    jest.useFakeTimers();

    const result = runLanguageServerHealthCheck('/tmp/perllsp', log, {
      execFile: execFileOption,
    });
    expect(jest.getTimerCount()).toBe(1);
    callback(null, '\u001b[32;1mok\u001b[0m \u001b[1m0.17.0\u001b[0m\n', '');

    await expect(result).resolves.toBe(true);
    expect(child.kill).not.toHaveBeenCalled();
    expect(jest.getTimerCount()).toBe(0);
    expect(log.appendLine).not.toHaveBeenCalled();
  });

  test('rejects malformed output instead of accepting an ok prefix', async () => {
    mockedExecFile.mockImplementationOnce((_path, _args, _options, callback) => {
      callback(
        null,
        '\u001b[31mokay but malformed\u001b[0m\n',
        '\u001b[33mdiagnostic stderr\u001b[0m',
      );
    });
    const log = { appendLine: jest.fn() };

    await expect(
      runLanguageServerHealthCheck('/tmp/perllsp', log, { execFile: execFileOption }),
    ).resolves.toBe(false);
    expect(log.appendLine).toHaveBeenCalledWith(
      '[health-check] Unexpected output: okay but malformed',
    );
    expect(log.appendLine).toHaveBeenCalledWith('[health-check] stderr: diagnostic stderr');
  });

  test('treats a zero timeout as no timeout', async () => {
    let callback!: (error: Error | null, stdout: string, stderr: string) => void;
    mockedExecFile.mockImplementation((_path, _args, options, processCallback) => {
      expect(options.timeout).toBe(0);
      callback = processCallback;
      return { kill: jest.fn(() => true) };
    });
    const log = { appendLine: jest.fn() };
    jest.useFakeTimers();

    const result = runLanguageServerHealthCheck('/tmp/perllsp', log, {
      execFile: execFileOption,
      timeoutMs: 0,
    });
    expect(jest.getTimerCount()).toBe(0);
    callback(null, 'ok 0.17.0\n', '');

    await expect(result).resolves.toBe(true);
  });

  test('records process failure and both output streams', async () => {
    mockedExecFile.mockImplementation((_path, _args, _options, callback) => {
      callback(new Error('spawn failed'), 'partial stdout', 'stderr detail');
    });
    const log = { appendLine: jest.fn() };

    await expect(
      runLanguageServerHealthCheck('/tmp/perllsp', log, { execFile: execFileOption }),
    ).resolves.toBe(false);
    expect(log.appendLine).toHaveBeenNthCalledWith(1, '[health-check] Failed: spawn failed');
    expect(log.appendLine).toHaveBeenNthCalledWith(2, '[health-check] stderr: stderr detail');
    expect(log.appendLine).toHaveBeenNthCalledWith(3, '[health-check] stdout: partial stdout');
  });

  test('settles on timeout, terminates the child, and clears its timer', async () => {
    jest.useFakeTimers();
    const child = { kill: jest.fn(() => true) };
    mockedExecFile.mockImplementation(() => child);
    const log = { appendLine: jest.fn() };

    const result = runLanguageServerHealthCheck('/tmp/perllsp', log, {
      execFile: execFileOption,
      timeoutMs: 25,
    });
    expect(jest.getTimerCount()).toBe(1);
    jest.advanceTimersByTime(25);

    await expect(result).resolves.toBe(false);
    expect(child.kill).toHaveBeenCalledTimes(1);
    expect(log.appendLine).toHaveBeenCalledWith('[health-check] Timed out after 25ms');
    expect(jest.getTimerCount()).toBe(0);
  });

  test('logging failures do not prevent a failed result from settling', async () => {
    mockedExecFile.mockImplementation((_path, _args, _options, callback) => {
      callback(null, 'not healthy', '');
    });
    const log = {
      appendLine: jest.fn(() => {
        throw new Error('logging unavailable');
      }),
    };

    await expect(
      runLanguageServerHealthCheck('/tmp/perllsp', log, { execFile: execFileOption }),
    ).resolves.toBe(false);
  });

  test('handles a process runner that throws before creating a child', async () => {
    mockedExecFile.mockImplementation(() => {
      throw new Error('runner unavailable');
    });
    const log = { appendLine: jest.fn() };

    await expect(
      runLanguageServerHealthCheck('/tmp/perllsp', log, { execFile: execFileOption }),
    ).resolves.toBe(false);
    expect(log.appendLine).toHaveBeenCalledWith('[health-check] Failed: runner unavailable');
  });
});
