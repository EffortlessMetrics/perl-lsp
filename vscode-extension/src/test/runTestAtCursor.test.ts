import * as vscode from 'vscode';
import { selectTestCommandAtPosition } from '../runTestAtCursor';

describe('selectTestCommandAtPosition', () => {
  test('prefers runTest lenses over broader matching lenses', () => {
    const command = selectTestCommandAtPosition(
      [
        {
          range: {
            start: { line: 0, character: 0 },
            end: { line: 30, character: 0 },
          },
          command: {
            command: 'perl.runTestFile',
            arguments: ['/tmp/example.t'],
          },
        },
        {
          range: {
            start: { line: 8, character: 0 },
            end: { line: 12, character: 0 },
          },
          command: {
            command: 'perl.runTest',
            arguments: ['file:///tmp/example.t::test_addition'],
          },
        },
      ],
      { line: 10, character: 2 } as vscode.Position,
    );

    expect(command?.command).toBe('perl.runTest');
    expect(command?.arguments).toEqual(['file:///tmp/example.t::test_addition']);
  });

  test('returns a subtest command when that is the only matching runnable lens', () => {
    const command = selectTestCommandAtPosition(
      [
        {
          range: {
            start: { line: 3, character: 0 },
            end: { line: 18, character: 0 },
          },
          command: {
            command: 'perl.runSubtest',
            arguments: ['basic math'],
          },
        },
      ],
      { line: 5, character: 8 } as vscode.Position,
    );

    expect(command?.command).toBe('perl.runSubtest');
    expect(command?.arguments).toEqual(['basic math']);
  });

  test('falls back to a runTestFile lens when no narrower test lens exists', () => {
    const command = selectTestCommandAtPosition(
      [
        {
          range: {
            start: { line: 0, character: 0 },
            end: { line: 50, character: 0 },
          },
          command: {
            command: 'perl.runTestFile',
            arguments: ['/tmp/example.t'],
          },
        },
        {
          range: {
            start: { line: 1, character: 0 },
            end: { line: 3, character: 0 },
          },
          command: {
            command: 'perl.debugTest',
            arguments: ['file:///tmp/example.t::test_addition'],
          },
        },
      ],
      { line: 25, character: 0 } as vscode.Position,
    );

    expect(command?.command).toBe('perl.runTestFile');
  });

  test('returns undefined when no runnable lens contains the cursor', () => {
    const command = selectTestCommandAtPosition(
      [
        {
          range: {
            start: { line: 0, character: 0 },
            end: { line: 5, character: 0 },
          },
          command: {
            command: 'perl.debugTest',
            arguments: ['file:///tmp/example.t::test_addition'],
          },
        },
      ],
      { line: 9, character: 0 } as vscode.Position,
    );

    expect(command).toBeUndefined();
  });
});
