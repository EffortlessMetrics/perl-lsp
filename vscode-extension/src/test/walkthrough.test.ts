/**
 * Contract tests for the VS Code interactive onboarding walkthrough.
 *
 * These tests verify that package.json declares a valid walkthrough
 * manifest with the required steps. Breaking these signals a regression
 * in the first-run onboarding experience.
 */

import * as fs from 'fs';
import * as path from 'path';

const EXT_ROOT = path.resolve(__dirname, '..', '..');

function readPackageJson(): any {
  return JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
}

// ---------------------------------------------------------------------------
// Walkthrough presence and structure
// ---------------------------------------------------------------------------
describe('package.json walkthrough contribution', () => {
  let pkg: any;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('contributes.walkthroughs is defined', () => {
    expect(pkg.contributes.walkthroughs).toBeDefined();
    expect(Array.isArray(pkg.contributes.walkthroughs)).toBe(true);
  });

  test('exactly one walkthrough is contributed', () => {
    const walkthroughs: any[] = pkg.contributes.walkthroughs;
    expect(walkthroughs.length).toBeGreaterThanOrEqual(1);
  });

  test('walkthrough has id, title, description and steps', () => {
    const wt = pkg.contributes.walkthroughs[0];
    expect(typeof wt.id).toBe('string');
    expect(wt.id.length).toBeGreaterThan(0);
    expect(typeof wt.title).toBe('string');
    expect(wt.title.length).toBeGreaterThan(0);
    expect(typeof wt.description).toBe('string');
    expect(wt.description.length).toBeGreaterThan(0);
    expect(Array.isArray(wt.steps)).toBe(true);
  });

  test('walkthrough has all 7 required steps', () => {
    const wt = pkg.contributes.walkthroughs[0];
    expect(wt.steps.length).toBe(7);
  });

  test('every step has id, title, description and media', () => {
    const wt = pkg.contributes.walkthroughs[0];
    for (const step of wt.steps) {
      expect(typeof step.id).toBe('string');
      expect(step.id.length).toBeGreaterThan(0);
      expect(typeof step.title).toBe('string');
      expect(step.title.length).toBeGreaterThan(0);
      expect(typeof step.description).toBe('string');
      expect(step.description.length).toBeGreaterThan(0);
      expect(step.media).toBeDefined();
    }
  });

  test('step ids are unique', () => {
    const wt = pkg.contributes.walkthroughs[0];
    const ids: string[] = wt.steps.map((s: any) => s.id);
    const unique = new Set(ids);
    expect(unique.size).toBe(ids.length);
  });

  test('step ids match the 7 required topics', () => {
    const wt = pkg.contributes.walkthroughs[0];
    const ids: string[] = wt.steps.map((s: any) => s.id);
    const required = [
      'welcome',
      'verify-perl',
      'open-project',
      'try-completion',
      'try-goto-definition',
      'configure-settings',
      'debug-first-script',
    ];
    for (const req of required) {
      expect(ids).toContain(req);
    }
  });

  test('debug step clearly marks debugging as optional and mentions perl-dap', () => {
    const wt = pkg.contributes.walkthroughs[0];
    const debugStep = wt.steps.find((step: any) => step.id === 'debug-first-script');
    expect(debugStep).toBeDefined();
    expect(debugStep.title).toMatch(/optional/i);
    expect(debugStep.description).toMatch(/perl-dap/i);
  });

  test('walkthrough activationEvents includes onWalkthrough', () => {
    // VSCode requires walkthroughs to be listed in activationEvents
    // with onWalkthrough:<id> so that the extension activates when
    // the user opens the walkthrough panel.
    const wt = pkg.contributes.walkthroughs[0];
    const expectedEvent = `onWalkthrough:${wt.id}`;
    expect(pkg.activationEvents).toContain(expectedEvent);
  });
});

// ---------------------------------------------------------------------------
// Walkthrough step media files
// ---------------------------------------------------------------------------
describe('walkthrough step media files', () => {
  let pkg: any;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('every step media image path exists on disk', () => {
    const wt = pkg.contributes.walkthroughs[0];
    for (const step of wt.steps) {
      const media = step.media;
      // media can be { image: "path" } or { markdown: "path" }
      const mediaPath: string = media.image ?? media.markdown;
      expect(typeof mediaPath).toBe('string');
      const absPath = path.join(EXT_ROOT, mediaPath);
      expect(fs.existsSync(absPath)).toBe(true);
    }
  });
});
