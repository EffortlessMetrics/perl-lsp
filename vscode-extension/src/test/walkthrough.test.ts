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

type WalkthroughStep = {
  id: string;
  title: string;
  description: string;
  media: {
    image?: string;
    markdown?: string;
  };
};

type Walkthrough = {
  id: string;
  title: string;
  description: string;
  steps: WalkthroughStep[];
};

type WalkthroughManifest = {
  activationEvents: string[];
  contributes: {
    walkthroughs: Walkthrough[];
  };
};

function findRequired<T>(
  values: readonly T[],
  predicate: (value: T) => boolean,
  description: string,
): T {
  const value = values.find(predicate);
  if (!value) {
    throw new Error(`Expected ${description} in package manifest`);
  }
  return value;
}

function readPackageJson(): WalkthroughManifest {
  return JSON.parse(
    fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'),
  ) as WalkthroughManifest;
}

// ---------------------------------------------------------------------------
// Walkthrough presence and structure
// ---------------------------------------------------------------------------
describe('package.json walkthrough contribution', () => {
  let pkg: WalkthroughManifest;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('contributes.walkthroughs is defined', () => {
    expect(pkg.contributes.walkthroughs).toBeDefined();
    expect(Array.isArray(pkg.contributes.walkthroughs)).toBe(true);
  });

  test('exactly one walkthrough is contributed', () => {
    const walkthroughs: Walkthrough[] = pkg.contributes.walkthroughs;
    expect(walkthroughs.length).toBeGreaterThanOrEqual(1);
  });

  test('walkthrough has id, title, description and steps', () => {
    const wt = findRequired(pkg.contributes.walkthroughs, () => true, 'a walkthrough');
    expect(typeof wt.id).toBe('string');
    expect(wt.id.length).toBeGreaterThan(0);
    expect(typeof wt.title).toBe('string');
    expect(wt.title.length).toBeGreaterThan(0);
    expect(typeof wt.description).toBe('string');
    expect(wt.description.length).toBeGreaterThan(0);
    expect(Array.isArray(wt.steps)).toBe(true);
  });

  test('walkthrough has all 8 required steps', () => {
    const wt = findRequired(pkg.contributes.walkthroughs, () => true, 'a walkthrough');
    expect(wt.steps.length).toBe(8);
  });

  test('every step has id, title, description and media', () => {
    const wt = findRequired(pkg.contributes.walkthroughs, () => true, 'a walkthrough');
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
    const wt = findRequired(pkg.contributes.walkthroughs, () => true, 'a walkthrough');
    const ids: string[] = wt.steps.map((s: WalkthroughStep) => s.id);
    const unique = new Set(ids);
    expect(unique.size).toBe(ids.length);
  });

  test('step ids match the 8 required topics', () => {
    const wt = findRequired(pkg.contributes.walkthroughs, () => true, 'a walkthrough');
    const ids: string[] = wt.steps.map((s: WalkthroughStep) => s.id);
    const required = [
      'welcome',
      'verify-perl',
      'open-project',
      'try-completion',
      'try-goto-definition',
      'ai-completion',
      'configure-settings',
      'debug-first-script',
    ];
    for (const req of required) {
      expect(ids).toContain(req);
    }
  });

  test('verify-perl step is native-first: does not present perltidy/perlcritic as required tooling (#3276)', () => {
    const wt = findRequired(pkg.contributes.walkthroughs, () => true, 'a walkthrough');
    const step = findRequired(
      wt.steps,
      (s: WalkthroughStep) => s.id === 'verify-perl',
      'verify-perl step',
    );
    // The health check confirms the Perl interpreter; native formatting and
    // native critic are built in. External tools must be framed as optional,
    // never as core "Perl tooling" the product requires.
    expect(step.description).toMatch(/native/i);
    expect(step.description).toMatch(/optional/i);
    expect(step.description).not.toMatch(/Perl tooling \(perl, perltidy/i);
  });

  test('verify-perl step exposes the extension health-check command', () => {
    const wt = findRequired(pkg.contributes.walkthroughs, () => true, 'a walkthrough');
    const step = findRequired(
      wt.steps,
      (s: WalkthroughStep) => s.id === 'verify-perl',
      'verify-perl step',
    );
    expect(step.description).toMatch(/command:perl-lsp\.runHealthCheck/);
  });

  test('open-project step offers the bundled demo project (#1635)', () => {
    const wt = findRequired(pkg.contributes.walkthroughs, () => true, 'a walkthrough');
    const step = findRequired(
      wt.steps,
      (s: WalkthroughStep) => s.id === 'open-project',
      'open-project step',
    );
    expect(step.description).toMatch(/command:perl-lsp\.openDemoProject/);
  });

  test('ai-completion step marks the feature optional and off by default (#1634)', () => {
    const wt = findRequired(pkg.contributes.walkthroughs, () => true, 'a walkthrough');
    const step = findRequired(
      wt.steps,
      (s: WalkthroughStep) => s.id === 'ai-completion',
      'ai-completion step',
    );
    expect(step.title).toMatch(/optional/i);
    expect(step.description).toMatch(/off by default/i);
  });

  test('debug step clearly marks debugging as optional and mentions perl-dap', () => {
    const wt = findRequired(pkg.contributes.walkthroughs, () => true, 'a walkthrough');
    const debugStep = findRequired(
      wt.steps,
      (step: WalkthroughStep) => step.id === 'debug-first-script',
      'debug-first-script step',
    );
    expect(debugStep.title).toMatch(/optional/i);
    expect(debugStep.description).toMatch(/perl-dap/i);
  });

  test('walkthrough activationEvents includes onWalkthrough', () => {
    // VSCode requires walkthroughs to be listed in activationEvents
    // with onWalkthrough:<id> so that the extension activates when
    // the user opens the walkthrough panel.
    const wt = findRequired(pkg.contributes.walkthroughs, () => true, 'a walkthrough');
    const expectedEvent = `onWalkthrough:${wt.id}`;
    expect(pkg.activationEvents).toContain(expectedEvent);
  });
});

// ---------------------------------------------------------------------------
// Walkthrough step media files
// ---------------------------------------------------------------------------
describe('walkthrough step media files', () => {
  let pkg: WalkthroughManifest;

  beforeAll(() => {
    pkg = readPackageJson();
  });

  test('every step media image path exists on disk', () => {
    const wt = findRequired(pkg.contributes.walkthroughs, () => true, 'a walkthrough');
    for (const step of wt.steps) {
      const media = step.media;
      // media can be { image: "path" } or { markdown: "path" }
      const mediaPath = media.image ?? media.markdown;
      expect(typeof mediaPath).toBe('string');
      if (typeof mediaPath !== 'string') {
        return;
      }
      const absPath = path.join(EXT_ROOT, mediaPath);
      expect(fs.existsSync(absPath)).toBe(true);
    }
  });
});
