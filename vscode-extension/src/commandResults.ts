export type ManagedBinarySource = 'github-release' | 'internal-base-url' | 'existing';

export interface ReinstallCommandResult {
  ok: boolean;
  serverPath: string;
  target: string;
  source: ManagedBinarySource;
  version?: string;
  checksumVerified?: boolean;
  error?: string;
}

export type HealthCheckCommandStatus = 'ok' | 'warning' | 'error';

export interface HealthCheckCommandResult {
  ok: boolean;
  checks: Array<{
    label: string;
    status: HealthCheckCommandStatus;
    detail: string;
  }>;
}
