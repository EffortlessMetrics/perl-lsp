#!/usr/bin/env node

// Benchmark script for the C implementation
// This script is used by xtask to benchmark the C parser implementation.

const Parser = require('tree-sitter');
const Perl = require('../');

const testCode = process.env.TEST_CODE;
if (!testCode) {
    console.error('TEST_CODE environment variable not set');
    process.exit(1);
}

const parser = new Parser();
parser.setLanguage(Perl);

// Parse the test code multiple times to get accurate timing
for (let i = 0; i < 100; i++) {
    const tree = parser.parse(testCode);
    if (!tree) {
        console.error('Failed to parse test code');
        process.exit(1);
    }
} 