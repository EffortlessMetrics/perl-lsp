import * as vscode from 'vscode';
import {
  provideGherkinDocumentSymbols,
  provideGherkinFoldingRanges,
  registerGherkinProviders,
} from '../gherkinProviders';

describe('gherkin outline providers', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  test('registers document symbol and folding providers for gherkin', () => {
    registerGherkinProviders();

    expect(vscode.languages.registerDocumentSymbolProvider).toHaveBeenCalledTimes(1);
    expect(vscode.languages.registerFoldingRangeProvider).toHaveBeenCalledTimes(1);
    expect((vscode.languages.registerDocumentSymbolProvider as jest.Mock).mock.calls[0][0]).toEqual([
      { language: 'gherkin' },
    ]);
  });

  test('builds hierarchical document symbols for feature structure', () => {
    const text = [
      '@smoke',
      'Feature: Checkout flow',
      '  Background: signed-in user',
      '    Given I am logged in',
      '  Scenario Outline: buying a product',
      '    When I add <item> to the cart',
      '    Then the total should be <total>',
      '    Examples: happy path',
      '      | item   | total |',
      '      | Widget | 10    |',
    ].join('\n');

    const symbols = provideGherkinDocumentSymbols(text);
    expect(symbols).toHaveLength(1);
    expect(symbols[0].name).toBe('Feature: Checkout flow');
    expect(symbols[0].children.map((child) => child.name)).toEqual([
      'Background: signed-in user',
      'Scenario Outline: buying a product',
    ]);

    const background = symbols[0].children[0];
    expect(background.children.map((child) => child.name)).toEqual(['Given I am logged in']);

    const outline = symbols[0].children[1];
    expect(outline.children.map((child) => child.name)).toEqual([
      'When I add <item> to the cart',
      'Then the total should be <total>',
      'Examples: happy path',
    ]);
  });

  test('returns folding ranges for feature sections', () => {
    const text = [
      'Feature: Checkout flow',
      '  Background: signed-in user',
      '    Given I am logged in',
      '  Scenario: buying a product',
      '    When I add the item to the cart',
      '    Then the total should be 10',
    ].join('\n');

    const ranges = provideGherkinFoldingRanges(text);
    expect(ranges).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ start: 0, end: 5 }),
        expect.objectContaining({ start: 1, end: 2 }),
        expect.objectContaining({ start: 3, end: 5 }),
      ])
    );
  });
});
