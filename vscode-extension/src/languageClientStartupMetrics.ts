export type StartupPhaseStatus = 'idle' | 'running' | 'ok' | 'unavailable' | 'error';

export type BinaryResolutionSource =
  | 'unknown'
  | 'configured'
  | 'bundled'
  | 'path'
  | 'downloaded'
  | 'unavailable';

export type LanguageClientStartupMilestone =
  | 'extension_load'
  | 'activate_entered'
  | 'commands_registered'
  | 'activate_returned'
  | 'binary_resolution_started'
  | 'binary_resolution_completed'
  | 'process_started'
  | 'initialize_completed'
  | 'workspace_ready'
  | 'first_useful_request'
  | 'warm_request'
  | 'restart'
  | 'shutdown';

export interface LanguageClientStartupMetricsSnapshot {
  lifecycle_state: string;
  binary_resolution_status: StartupPhaseStatus;
  binary_resolution_source: BinaryResolutionSource;
  binary_resolution_ms: number | null;
  server_start_status: StartupPhaseStatus;
  server_start_ms: number | null;
  initialize_status: StartupPhaseStatus;
  initialize_ms: number | null;
  server_version: string | null;
  milestones: Partial<Record<LanguageClientStartupMilestone, number>>;
}

/**
 * Captures monotonic startup milestones without coupling receipt collection to
 * VS Code APIs or making activation wait on filesystem IO.
 */
export class LanguageClientStartupMetrics {
  private readonly origin = performance.now();
  private milestones: Partial<Record<LanguageClientStartupMilestone, number>> = {};
  private lifecycleState = 'stopped';
  private binaryResolutionStatus: StartupPhaseStatus = 'idle';
  private binaryResolutionSource: BinaryResolutionSource = 'unknown';
  private binaryResolutionStartedAt: number | undefined;
  private binaryResolutionMs: number | null = null;
  private serverStartStatus: StartupPhaseStatus = 'idle';
  private serverStartStartedAt: number | undefined;
  private serverStartMs: number | null = null;
  private initializeStatus: StartupPhaseStatus = 'idle';
  private initializeStartedAt: number | undefined;
  private initializeMs: number | null = null;
  private serverVersion: string | null = null;

  public constructor() {
    this.markMilestone('extension_load');
  }

  public markMilestone(milestone: LanguageClientStartupMilestone): void {
    if (this.milestones[milestone] !== undefined) {
      return;
    }
    this.milestones[milestone] = Math.max(0, Math.round(performance.now() - this.origin));
  }

  public setLifecycleState(state: string): void {
    this.lifecycleState = state;
  }

  public setServerVersion(version: string | undefined): void {
    this.serverVersion = version ?? null;
  }

  public beginBinaryResolution(): void {
    this.markMilestone('binary_resolution_started');
    this.binaryResolutionStatus = 'running';
    this.binaryResolutionSource = 'unknown';
    this.binaryResolutionStartedAt = performance.now();
    this.binaryResolutionMs = null;
  }

  public finishBinaryResolution(
    status: Exclude<StartupPhaseStatus, 'idle' | 'running'>,
    source: BinaryResolutionSource = 'unknown',
  ): void {
    this.markMilestone('binary_resolution_completed');
    this.binaryResolutionStatus = status;
    this.binaryResolutionSource = source;
    this.binaryResolutionMs = this.elapsedSince(this.binaryResolutionStartedAt);
    this.binaryResolutionStartedAt = undefined;
  }

  public beginServerStart(): void {
    this.serverStartStatus = 'running';
    this.serverStartStartedAt = performance.now();
    this.serverStartMs = null;
  }

  public finishServerStart(status: Exclude<StartupPhaseStatus, 'idle' | 'running'>): void {
    if (this.serverStartStatus !== 'running') {
      return;
    }
    this.serverStartStatus = status;
    this.serverStartMs = this.elapsedSince(this.serverStartStartedAt);
    this.serverStartStartedAt = undefined;
    if (status === 'ok') {
      this.markMilestone('process_started');
    }
  }

  public beginInitialize(): void {
    this.initializeStatus = 'running';
    this.initializeStartedAt = performance.now();
    this.initializeMs = null;
  }

  public finishInitialize(status: Exclude<StartupPhaseStatus, 'idle' | 'running'>): void {
    if (this.initializeStatus !== 'running') {
      return;
    }
    this.initializeStatus = status;
    this.initializeMs = this.elapsedSince(this.initializeStartedAt);
    this.initializeStartedAt = undefined;
    if (status === 'ok') {
      this.markMilestone('initialize_completed');
    }
  }

  public snapshot(): LanguageClientStartupMetricsSnapshot {
    return {
      lifecycle_state: this.lifecycleState,
      binary_resolution_status: this.binaryResolutionStatus,
      binary_resolution_source: this.binaryResolutionSource,
      binary_resolution_ms: this.binaryResolutionMs,
      server_start_status: this.serverStartStatus,
      server_start_ms: this.serverStartMs,
      initialize_status: this.initializeStatus,
      initialize_ms: this.initializeMs,
      server_version: this.serverVersion,
      milestones: { ...this.milestones },
    };
  }

  private elapsedSince(startedAt: number | undefined): number | null {
    return startedAt === undefined ? null : Math.max(0, Math.round(performance.now() - startedAt));
  }
}
