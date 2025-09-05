#!/usr/bin/env node

// Benchmark script for the C implementation.
// The script reads Perl source from TEST_CODE and runs the C parser
// multiple times to measure average parsing time.

const Parser = require('tree-sitter');
const Perl = require('../');

const code = process.env.TEST_CODE || '';
const iterations = parseInt(process.env.ITERATIONS || '100', 10);

const parser = new Parser();
parser.setLanguage(Perl);

const start = Date.now();
for (let i = 0; i < iterations; i++) {
  parser.parse(code);
}
const duration = Date.now() - start;

console.log(
  JSON.stringify({
    duration,
    iterations,
    average: duration / iterations,
  })
);

