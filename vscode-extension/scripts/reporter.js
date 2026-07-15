'use strict';

/**
 * @param {string} scope
 * @param {{stdout?: {write: (chunk: string) => void}, stderr?: {write: (chunk: string) => void}}} [streams]
 */
function createReporter(scope, streams = {}) {
  const stdout = streams.stdout ?? process.stdout;
  const stderr = streams.stderr ?? process.stderr;
  const prefix = `[${scope}]`;

  return {
    /** @param {string} message */
    info(message) {
      stdout.write(`${prefix} ${message}\n`);
    },
    /** @param {string} message */
    error(message) {
      stderr.write(`${prefix} ${message}\n`);
    },
  };
}

module.exports = { createReporter };
