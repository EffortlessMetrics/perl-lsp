import { jest } from '@jest/globals';
import { commands, Disposable, window } from './__mocks__/vscode';

type Progress = { report: (value: unknown) => void };
type Cancellation = { isCancellationRequested: boolean };

describe('VS Code unit-test mock', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  test('dispatches registered command arguments and returns the handler result', async () => {
    const handler = jest.fn((...args: unknown[]) => args[0]);
    const disposable = commands.registerCommand('test.mock.command', handler);

    await expect(commands.executeCommand('test.mock.command', 'value')).resolves.toBe('value');
    expect(handler).toHaveBeenCalledWith('value');

    disposable.dispose();
  });

  test('runs progress tasks with a reporter and cancellation token', async () => {
    const task = jest.fn(async (...args: unknown[]) => {
      const [progress, token] = args as [Progress, Cancellation];
      progress.report({ message: 'working' });
      return token.isCancellationRequested ? 'cancelled' : 'done';
    });

    await expect(window.withProgress({}, task)).resolves.toBe('done');
    expect(task).toHaveBeenCalledTimes(1);
  });

  test('disposable invokes its cleanup callback exactly when disposed', () => {
    const cleanup = jest.fn();
    const disposable = new Disposable(cleanup);

    expect(cleanup).not.toHaveBeenCalled();
    disposable.dispose();
    expect(cleanup).toHaveBeenCalledTimes(1);
  });
});
