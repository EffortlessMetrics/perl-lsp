import * as assert from 'assert';
import { LanguageClientStartupMetrics } from '../languageClientStartupMetrics';

describe('LanguageClientStartupMetrics', function () {
  test('records attributable startup phases and monotonic milestones', function () {
    const metrics = new LanguageClientStartupMetrics();
    metrics.setLifecycleState('resolving');
    metrics.beginBinaryResolution();
    metrics.finishBinaryResolution('ok', 'configured');
    metrics.setLifecycleState('starting');
    metrics.beginServerStart();
    metrics.beginInitialize();
    metrics.finishServerStart('ok');
    metrics.finishInitialize('ok');
    metrics.setLifecycleState('running');
    metrics.setServerVersion('0.17.0');
    metrics.markMilestone('workspace_ready');
    metrics.markMilestone('first_useful_request');
    metrics.markMilestone('warm_request');
    metrics.markMilestone('restart');
    metrics.markMilestone('shutdown');

    const snapshot = metrics.snapshot();
    assert.equal(snapshot.lifecycle_state, 'running');
    assert.equal(snapshot.binary_resolution_status, 'ok');
    assert.equal(snapshot.binary_resolution_source, 'configured');
    assert.equal(snapshot.server_start_status, 'ok');
    assert.equal(snapshot.initialize_status, 'ok');
    assert.equal(snapshot.server_version, '0.17.0');
    assert.ok((snapshot.binary_resolution_ms ?? -1) >= 0);
    assert.ok((snapshot.server_start_ms ?? -1) >= 0);
    assert.ok((snapshot.initialize_ms ?? -1) >= 0);

    const milestones = snapshot.milestones;
    const ordered = [
      'extension_load',
      'binary_resolution_started',
      'binary_resolution_completed',
      'process_started',
      'initialize_completed',
      'workspace_ready',
      'first_useful_request',
      'warm_request',
      'restart',
      'shutdown',
    ] as const;
    for (let index = 1; index < ordered.length; index += 1) {
      assert.ok(
        (milestones[ordered[index]] ?? -1) >= (milestones[ordered[index - 1]] ?? -1),
        `${ordered[index]} must not precede ${ordered[index - 1]}`,
      );
    }
  });

  test('does not invent phase durations when resolution is unavailable', function () {
    const metrics = new LanguageClientStartupMetrics();
    metrics.beginBinaryResolution();
    metrics.finishBinaryResolution('unavailable', 'unavailable');
    metrics.finishInitialize('error');

    const snapshot = metrics.snapshot();
    assert.equal(snapshot.binary_resolution_status, 'unavailable');
    assert.equal(snapshot.binary_resolution_source, 'unavailable');
    assert.equal(snapshot.initialize_status, 'idle');
    assert.equal(snapshot.initialize_ms, null);
    assert.equal(snapshot.server_start_status, 'idle');
    assert.equal(snapshot.server_start_ms, null);
  });

  test('retains the first timestamp when a lifecycle event is observed again', function () {
    const now = jest.spyOn(performance, 'now');
    now.mockReturnValueOnce(1000).mockReturnValueOnce(1000).mockReturnValueOnce(1100);
    try {
      const metrics = new LanguageClientStartupMetrics();
      metrics.markMilestone('workspace_ready');
      const first = metrics.snapshot().milestones.workspace_ready;
      metrics.markMilestone('workspace_ready');
      assert.equal(metrics.snapshot().milestones.workspace_ready, first);
    } finally {
      now.mockRestore();
    }
  });
});
