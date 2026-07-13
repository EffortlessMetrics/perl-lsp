import * as path from 'path';
import Mocha from 'mocha';

export async function run(): Promise<void> {
  const mocha = new Mocha({
    ui: 'tdd',
    color: true,
    timeout: 180_000,
  });

  const currentSourceSmoke = process.env.PERL_LSP_CURRENT_SOURCE_SMOKE === '1';
  const smokeTestPaths = currentSourceSmoke
    ? [path.resolve(__dirname, '../../integration/firstHourReceipt.test.js')]
    : [path.resolve(__dirname, '../managedBinaryPublishedSmoke.test.js')];
  for (const smokeTestPath of smokeTestPaths) {
    mocha.addFile(smokeTestPath);
  }
  await mocha.loadFilesAsync();

  if (mocha.suite.total() === 0) {
    throw new Error(`No published-extension smoke tests loaded from ${smokeTestPaths.join(', ')}`);
  }

  return new Promise((resolve, reject) => {
    const runner = mocha.run((failures) => {
      if (runner.total === 0) {
        reject(new Error('No published-extension smoke tests matched.'));
        return;
      }
      if (failures > 0) {
        reject(new Error(`${failures} published-extension smoke test(s) failed.`));
        return;
      }
      resolve();
    });
  });
}
