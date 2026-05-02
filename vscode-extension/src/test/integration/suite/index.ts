import * as path from 'path';
import Mocha from 'mocha';

export async function run(): Promise<void> {
  const grep = process.env.VSCODE_TEST_GREP;
  const mocha = new Mocha({
    ui: 'tdd',
    color: true,
    timeout: 180_000,
  });

  const smokeTestPath = path.resolve(__dirname, '../managedBinarySmoke.test.js');
  mocha.addFile(smokeTestPath);
  await mocha.loadFilesAsync();
  if (grep) {
    mocha.grep(new RegExp(grep));
  }

  if (mocha.suite.total() === 0) {
    throw new Error(`No extension-host smoke tests loaded from ${smokeTestPath}`);
  }

  return new Promise((resolve, reject) => {
    const runner = mocha.run(failures => {
      if (runner.total === 0) {
        reject(new Error('No extension-host smoke tests matched the requested filter.'));
        return;
      }
      if (failures > 0) {
        reject(new Error(`${failures} extension-host smoke test(s) failed.`));
        return;
      }
      resolve();
    });
  });
}
