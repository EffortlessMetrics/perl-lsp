import * as vscode from 'vscode';
import {
  classifyConfigurationChange,
  type ConfigurationChangeEventLike,
} from './languageClientConfiguration';

export type WorkspaceEventHandlers = {
  onLiveConfigurationChanged: (event: ConfigurationChangeEventLike) => void | Promise<void>;
  onReconstructConfigurationChanged: (event: ConfigurationChangeEventLike) => void | Promise<void>;
  onRestartRequired: (event: ConfigurationChangeEventLike) => void | Promise<void>;
  onError?: (error: unknown) => void;
};

function invoke(
  callback: (event: ConfigurationChangeEventLike) => void | Promise<void>,
  event: ConfigurationChangeEventLike,
  onError: ((error: unknown) => void) | undefined,
): void {
  const reportError = (error: unknown): void => {
    if (onError) {
      onError(error);
      return;
    }
    process.stderr.write(`[workspace-configuration] handler failed: ${String(error)}\n`);
  };

  try {
    Promise.resolve(callback(event)).catch((error: unknown) => {
      reportError(error);
    });
  } catch (error: unknown) {
    reportError(error);
  }
}

export function registerWorkspaceConfigurationEvents(
  handlers: WorkspaceEventHandlers,
): vscode.Disposable {
  return vscode.workspace.onDidChangeConfiguration((event) => {
    const classes = classifyConfigurationChange(event);
    if (classes.includes('live')) {
      invoke(handlers.onLiveConfigurationChanged, event, handlers.onError);
    }
    if (classes.includes('reconstruct')) {
      invoke(handlers.onReconstructConfigurationChanged, event, handlers.onError);
    }
    if (classes.includes('restart')) {
      invoke(handlers.onRestartRequired, event, handlers.onError);
    }
  });
}
