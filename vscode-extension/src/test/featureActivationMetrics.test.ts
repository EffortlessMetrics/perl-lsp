import { FeatureActivationMetrics } from '../featureActivationMetrics';

describe('feature activation metrics', () => {
  test('records registration, criticality, and first-use state without inventing module cost', () => {
    const metrics = new FeatureActivationMetrics();
    metrics.beginActivation();
    metrics.measure('providers', true, () => 'registered');
    metrics.markFirstUse('providers');

    expect(metrics.snapshot()).toMatchObject({
      schema_version: 'feature_activation_metrics.v1',
      activation_id: 1,
      measurements: [
        {
          domain: 'providers',
          activation_critical: true,
          module_evaluation: 'static-import-not-observable',
          registration_status: 'registered',
          first_use_status: 'observed',
        },
      ],
    });
    expect(metrics.snapshot().measurements[0]?.registration_ms).not.toBeNull();
    expect(metrics.snapshot().measurements[0]?.first_use_ms).not.toBeNull();
  });

  test('records failures and rethrows them for activation cleanup to handle', () => {
    const metrics = new FeatureActivationMetrics();
    metrics.beginActivation();
    expect(() =>
      metrics.measure('mcp', false, () => {
        throw new Error('registration failed');
      }),
    ).toThrow('registration failed');

    expect(metrics.snapshot().measurements[0]).toMatchObject({
      domain: 'mcp',
      registration_status: 'failed',
      error: 'registration failed',
    });
  });

  test('clears measurements for a new extension activation', () => {
    const metrics = new FeatureActivationMetrics();
    metrics.beginActivation();
    metrics.measure('debugger', true, () => undefined);
    metrics.beginActivation();

    expect(metrics.snapshot()).toMatchObject({
      activation_id: 2,
      measurements: [],
    });
  });
});
