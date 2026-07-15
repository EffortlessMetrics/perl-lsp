export type FeatureActivationDomain =
  | 'mcp'
  | 'whats_new'
  | 'providers'
  | 'configuration'
  | 'debugger'
  | 'onboarding';

type RegistrationStatus = 'not_started' | 'registered' | 'failed';
type FirstUseStatus = 'not_observed' | 'observed';

export interface FeatureActivationMeasurement {
  domain: FeatureActivationDomain;
  activation_critical: boolean;
  module_evaluation: 'static-import-not-observable';
  registration_status: RegistrationStatus;
  registration_ms: number | null;
  first_use_status: FirstUseStatus;
  first_use_ms: number | null;
  error: string | null;
}

export interface FeatureActivationMetricsSnapshot {
  schema_version: 'feature_activation_metrics.v1';
  activation_id: number;
  measurements: FeatureActivationMeasurement[];
  claim_boundary: string;
}

interface MutableMeasurement extends FeatureActivationMeasurement {
  registration_started_at: number | undefined;
}

export class FeatureActivationMetrics {
  private activationId = 0;
  private origin = performance.now();
  private measurements = new Map<FeatureActivationDomain, MutableMeasurement>();

  public beginActivation(): void {
    this.activationId += 1;
    this.origin = performance.now();
    this.measurements.clear();
  }

  public measure<T>(
    domain: FeatureActivationDomain,
    activationCritical: boolean,
    register: () => T,
  ): T {
    const measurement: MutableMeasurement = {
      domain,
      activation_critical: activationCritical,
      module_evaluation: 'static-import-not-observable',
      registration_status: 'not_started',
      registration_ms: null,
      first_use_status: 'not_observed',
      first_use_ms: null,
      error: null,
      registration_started_at: performance.now(),
    };
    this.measurements.set(domain, measurement);
    try {
      const result = register();
      measurement.registration_status = 'registered';
      measurement.registration_ms = this.elapsedSince(measurement.registration_started_at);
      measurement.registration_started_at = undefined;
      return result;
    } catch (error: unknown) {
      measurement.registration_status = 'failed';
      measurement.registration_ms = this.elapsedSince(measurement.registration_started_at);
      measurement.registration_started_at = undefined;
      measurement.error = error instanceof Error ? error.message : String(error);
      throw error;
    }
  }

  public markFirstUse(domain: FeatureActivationDomain): void {
    const measurement = this.measurements.get(domain);
    if (!measurement || measurement.first_use_status === 'observed') {
      return;
    }
    measurement.first_use_status = 'observed';
    measurement.first_use_ms = Math.max(0, Math.round(performance.now() - this.origin));
  }

  public snapshot(): FeatureActivationMetricsSnapshot {
    return {
      schema_version: 'feature_activation_metrics.v1',
      activation_id: this.activationId,
      measurements: [...this.measurements.values()].map(
        ({ registration_started_at: _registrationStartedAt, ...measurement }) => measurement,
      ),
      claim_boundary:
        'Feature measurements cover explicit registration/construction boundaries and optional first-use marks. Static module evaluation is intentionally reported as not observable, and missing first-use marks are not treated as zero cost. This receipt does not justify dynamic loading or a performance budget.',
    };
  }

  private elapsedSince(startedAt: number | undefined): number | null {
    return startedAt === undefined ? null : Math.max(0, Math.round(performance.now() - startedAt));
  }
}
