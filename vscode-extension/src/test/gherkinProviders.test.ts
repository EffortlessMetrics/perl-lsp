import * as vscode from 'vscode';
import {
  provideGherkinDocumentSymbols,
  provideGherkinFoldingRanges,
  provideGherkinStepDefinitionLinks,
  registerGherkinProviders,
} from '../gherkinProviders';

describe('gherkin outline providers', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  test('registers document symbol, folding, and definition providers for gherkin', () => {
    registerGherkinProviders();

    expect(vscode.languages.registerDocumentSymbolProvider).toHaveBeenCalledTimes(1);
    expect(vscode.languages.registerFoldingRangeProvider).toHaveBeenCalledTimes(1);
    expect(vscode.languages.registerDefinitionProvider).toHaveBeenCalledTimes(1);
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

  test('finds regex-based Perl step definitions for Given steps', () => {
    const featureText = [
      'Feature: Login',
      '  Scenario: Successful login',
      '    Given a user exists with username "alice"',
    ].join('\n');

    const links = provideGherkinStepDefinitionLinks(
      featureText,
      { line: 2, character: 15 } as vscode.Position,
      [
        {
          uri: vscode.Uri.file('/project/features/step_definitions/user_steps.pm'),
          text: [
            'use Test::BDD::Cucumber::StepFile;',
            '',
            'Given qr/a user exists with username "([^"]+)"/, sub {',
            '    my $username = $1;',
            '};',
          ].join('\n'),
        },
      ]
    );

    expect(links).toHaveLength(1);
    expect(links[0].targetUri.fsPath).toBe('/project/features/step_definitions/user_steps.pm');
    expect(links[0].targetSelectionRange?.start.line).toBe(2);
  });

  test('resolves And steps using the previous Given/When/Then context', () => {
    const featureText = [
      'Feature: Checkout',
      '  Scenario: Purchase',
      '    Given I am authenticated',
      '    And the cart is empty',
    ].join('\n');

    const links = provideGherkinStepDefinitionLinks(
      featureText,
      { line: 3, character: 12 } as vscode.Position,
      [
        {
          uri: vscode.Uri.file('/project/t/steps/cart_steps.pm'),
          text: [
            'use Test::BDD::Cucumber::StepFile;',
            '',
            'Given qr/the cart is empty/, sub {',
            '};',
          ].join('\n'),
        },
      ]
    );

    expect(links).toHaveLength(1);
    expect(links[0].targetSelectionRange?.start.line).toBe(2);
  });

  test('uses the registered definition provider to scan workspace step files', async () => {
    registerGherkinProviders();

    const provider = (vscode.languages.registerDefinitionProvider as jest.Mock).mock.calls[0][1];
    (vscode.workspace.findFiles as jest.Mock)
      .mockResolvedValueOnce([vscode.Uri.file('/project/features/step_definitions/login_steps.pm')])
      .mockResolvedValue([]);
    (vscode.workspace.openTextDocument as jest.Mock).mockResolvedValue({
      uri: vscode.Uri.file('/project/features/step_definitions/login_steps.pm'),
      getText: () => [
        'use Test::BDD::Cucumber::StepFile;',
        '',
        'When qr/the user logs in with valid credentials/, sub {',
        '};',
      ].join('\n'),
    });

    const links = await provider.provideDefinition(
      {
        getText: () => [
          'Feature: Login',
          '  Scenario: Happy path',
          '    When the user logs in with valid credentials',
        ].join('\n'),
      } as vscode.TextDocument,
      { line: 2, character: 12 } as vscode.Position,
      { isCancellationRequested: false } as vscode.CancellationToken
    );

    expect(vscode.workspace.findFiles).toHaveBeenCalled();
    expect(vscode.workspace.openTextDocument).toHaveBeenCalledWith(
      expect.objectContaining({ fsPath: '/project/features/step_definitions/login_steps.pm' })
    );
    expect(links).toHaveLength(1);
    expect(links[0].targetUri.fsPath).toBe('/project/features/step_definitions/login_steps.pm');
  });
});
