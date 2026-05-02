import * as path from 'path';
import Mocha from 'mocha';

export async function run(): Promise<void> {
  const mocha = new Mocha({
    ui: 'tdd',
    color: true,
    timeout: 180_000,
  });

  const smokeTestPath = path.resolve(__dirname, '../managedBinaryPublishedSmoke.test.js');
  mocha.addFile(smokeTestPath);
  await mocha.loadFilesAsync();

  if (mocha.suite.total() === 0) {
    throw new Error(`No published-extension smoke tests loaded from ${smokeTestPath}`);
  }

  return new Promise((resolve, reject) => {
    const runner = mocha.run(failures => {
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
