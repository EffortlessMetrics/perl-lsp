/**
 * Unit tests for Gherkin Parser Utility
 *
 * Tests the parsing functions that will be extracted from gherkinProviders.ts
 * and enhanced with Scenario Outline expansion and Background tracking.
 *
 * These tests define what the parser utility must provide.
 */

import { buildOutline, expandScenarioOutline, getBackgroundForScenario, OutlineNode } from '../gherkin/parser';

describe('Gherkin Parser Utility', () => {
    describe('buildOutline', () => {
        test('parses Feature node correctly', () => {
            const text = 'Feature: Login';
            const outline = buildOutline(text);
            expect(outline).toHaveLength(1);
            expect(outline[0].kind).toBe('feature');
            expect(outline[0].name).toBe('Feature: Login');
        });

        test('parses Background node as child of Feature', () => {
            const text = [
                'Feature: Login',
                '  Background: signed-in user',
                '    Given I am logged in',
            ].join('\n');

            const outline = buildOutline(text);
            expect(outline).toHaveLength(1);
            expect(outline[0].children).toHaveLength(1);
            expect(outline[0].children[0].kind).toBe('background');
            expect(outline[0].children[0].name).toBe('Background: signed-in user');
        });

        test('parses Scenario as child of Feature', () => {
            const text = [
                'Feature: Login',
                '  Scenario: successful login',
                '    Given I am on the login page',
            ].join('\n');

            const outline = buildOutline(text);
            const feature = outline[0];
            expect(feature.children).toHaveLength(1);
            expect(feature.children[0].kind).toBe('scenario');
            expect(feature.children[0].name).toBe('Scenario: successful login');
        });

        test('parses Scenario Outline with Examples', () => {
            const text = [
                'Feature: Login',
                '  Scenario Outline: login with users',
                '    Given I am on the login page',
                '    When I enter username <username>',
                '    Examples: users',
                '      | username |',
                '      | user1   |',
                '      | user2   |',
            ].join('\n');

            const outline = buildOutline(text);
            const scenarioOutline = outline[0].children[0];
            expect(scenarioOutline.kind).toBe('scenario');
            expect(scenarioOutline.name).toBe('Scenario Outline: login with users');

            // Examples should be a child
            const examplesNode = scenarioOutline.children.find(
                (c: OutlineNode) => c.kind === 'examples'
            );
            expect(examplesNode).toBeDefined();
            if (examplesNode) {
                expect(examplesNode.name).toBe('Examples: users');
            }
        });

        test('parses Steps with correct kind', () => {
            const text = [
                'Feature: Login',
                '  Scenario: login',
                '    Given I am on the login page',
                '    When I enter credentials',
                '    Then I should see the dashboard',
                '    And the logout button is visible',
            ].join('\n');

            const outline = buildOutline(text);
            const scenario = outline[0].children[0];
            expect(scenario.children).toHaveLength(4);
            expect(scenario.children[0].kind).toBe('step');
            expect(scenario.children[0].detail).toBe('Given');
            expect(scenario.children[1].detail).toBe('When');
            expect(scenario.children[2].detail).toBe('Then');
            expect(scenario.children[3].detail).toBe('And');
        });

        test('parses Rule block', () => {
            const text = [
                'Feature: Login',
                '  Rule: valid credentials',
                '    Scenario: admin login',
                '      Given I am an admin',
            ].join('\n');

            const outline = buildOutline(text);
            expect(outline[0].children[0].kind).toBe('rule');
            expect(outline[0].children[0].name).toBe('Rule: valid credentials');
        });

        test('records line numbers correctly', () => {
            const text = [
                'Feature: Login',        // line 0
                '  Background: setup',    // line 1
                '    Given I am here',   // line 2
                '  Scenario: login',     // line 3
                '    When I try',        // line 4
            ].join('\n');

            const outline = buildOutline(text);
            expect(outline[0].line).toBe(0);
            expect(outline[0].children[0].line).toBe(1);
            expect(outline[0].children[0].children[0].line).toBe(2);
            expect(outline[0].children[1].line).toBe(3);
            expect(outline[0].children[1].children[0].line).toBe(4);
        });

        test('handles multiple Features', () => {
            const text = [
                'Feature: Login',
                '  Scenario: login',
                '    Given I am here',
                'Feature: Logout',
                '  Scenario: logout',
                '    Given I am logged in',
            ].join('\n');

            const outline = buildOutline(text);
            expect(outline).toHaveLength(2);
            expect(outline[0].name).toBe('Feature: Login');
            expect(outline[1].name).toBe('Feature: Logout');
        });

        test('handles tags on Feature', () => {
            const text = [
                '@smoke @login',
                'Feature: Login',
                '  Scenario: login',
                '    Given I am here',
            ].join('\n');

            const outline = buildOutline(text);
            expect(outline).toHaveLength(1);
            expect(outline[0].name).toBe('Feature: Login');
        });
    });

    describe('expandScenarioOutline', () => {
        test('expands Scenario Outline into one node per Examples row', () => {
            const node: OutlineNode = {
                name: 'Scenario Outline: login with users',
                detail: 'scenario',
                kind: 'scenario',
                level: 2,
                line: 1,
                startCharacter: 2,
                endCharacter: 50,
                endLine: 8,
                children: [
                    {
                        name: 'Given I am on the login page',
                        detail: 'Given',
                        kind: 'step',
                        level: 4,
                        line: 2,
                        startCharacter: 4,
                        endCharacter: 30,
                        endLine: 2,
                        children: [],
                    },
                    {
                        name: 'When I enter username <username>',
                        detail: 'When',
                        kind: 'step',
                        level: 4,
                        line: 3,
                        startCharacter: 4,
                        endCharacter: 40,
                        endLine: 3,
                        children: [],
                    },
                    {
                        name: 'Examples: users',
                        detail: 'examples',
                        kind: 'examples',
                        level: 3,
                        line: 4,
                        startCharacter: 4,
                        endCharacter: 20,
                        endLine: 7,
                        children: [],
                    },
                ],
                examples: {
                    headers: ['username'],
                    rows: [['user1'], ['user2'], ['user3']],
                },
            };

            const expanded = expandScenarioOutline(node);

            expect(expanded).toHaveLength(3);
            expect(expanded[0].examples).toBeUndefined(); // Expanded nodes should not have examples
            expect(expanded[1].examples).toBeUndefined();
            expect(expanded[2].examples).toBeUndefined();
        });

        test('expanded nodes have modified labels with example values', () => {
            const node: OutlineNode = {
                name: 'Scenario Outline: login with users',
                detail: 'scenario',
                kind: 'scenario',
                level: 2,
                line: 1,
                startCharacter: 2,
                endCharacter: 50,
                endLine: 8,
                children: [],
                examples: {
                    headers: ['username'],
                    rows: [['alice'], ['bob']],
                },
            };

            const expanded = expandScenarioOutline(node);

            expect(expanded[0].name).toContain('alice');
            expect(expanded[1].name).toContain('bob');
        });

        test('handles multiple Example columns', () => {
            const node: OutlineNode = {
                name: 'Scenario Outline: login',
                detail: 'scenario',
                kind: 'scenario',
                level: 2,
                line: 1,
                startCharacter: 2,
                endCharacter: 50,
                endLine: 10,
                children: [],
                examples: {
                    headers: ['username', 'role'],
                    rows: [['alice', 'admin'], ['bob', 'user']],
                },
            };

            const expanded = expandScenarioOutline(node);

            expect(expanded).toHaveLength(2);
            expect(expanded[0].name).toContain('alice');
            expect(expanded[0].name).toContain('admin');
            expect(expanded[1].name).toContain('bob');
            expect(expanded[1].name).toContain('user');
        });

        test('returns empty array when Examples has no rows', () => {
            const node: OutlineNode = {
                name: 'Scenario Outline: empty',
                detail: 'scenario',
                kind: 'scenario',
                level: 2,
                line: 1,
                startCharacter: 2,
                endCharacter: 30,
                endLine: 5,
                children: [],
                examples: {
                    headers: ['value'],
                    rows: [],
                },
            };

            const expanded = expandScenarioOutline(node);
            expect(expanded).toHaveLength(0);
        });

        test('preserves Steps in expanded nodes', () => {
            const node: OutlineNode = {
                name: 'Scenario Outline: test',
                detail: 'scenario',
                kind: 'scenario',
                level: 2,
                line: 1,
                startCharacter: 2,
                endCharacter: 50,
                endLine: 6,
                children: [
                    {
                        name: 'Given I am on page <page>',
                        detail: 'Given',
                        kind: 'step',
                        level: 4,
                        line: 2,
                        startCharacter: 4,
                        endCharacter: 25,
                        endLine: 2,
                        children: [],
                    },
                ],
                examples: {
                    headers: ['page'],
                    rows: [['/home'], ['/login']],
                },
            };

            const expanded = expandScenarioOutline(node);

            expect(expanded).toHaveLength(2);
            expect(expanded[0].children.length).toBe(1);
            expect(expanded[0].children[0].name).toBe('Given I am on page <page>');
        });
    });

    describe('getBackgroundForScenario', () => {
        test('returns Background node for scenario in same Feature', () => {
            const tree: OutlineNode = {
                name: 'Feature: Login',
                detail: 'feature',
                kind: 'feature',
                level: 0,
                line: 0,
                startCharacter: 0,
                endCharacter: 20,
                endLine: 6,
                children: [
                    {
                        name: 'Background: signed-in user',
                        detail: 'background',
                        kind: 'background',
                        level: 1,
                        line: 1,
                        startCharacter: 2,
                        endCharacter: 30,
                        endLine: 3,
                        children: [
                            {
                                name: 'Given I am logged in',
                                detail: 'Given',
                                kind: 'step',
                                level: 4,
                                line: 2,
                                startCharacter: 4,
                                endCharacter: 25,
                                endLine: 2,
                                children: [],
                            },
                        ],
                    },
                    {
                        name: 'Scenario: successful login',
                        detail: 'scenario',
                        kind: 'scenario',
                        level: 2,
                        line: 4,
                        startCharacter: 2,
                        endCharacter: 30,
                        endLine: 6,
                        children: [],
                    },
                ],
            };

            const scenario = tree.children[1];
            const background = getBackgroundForScenario(scenario, tree);

            expect(background).not.toBeNull();
            expect(background!.name).toBe('Background: signed-in user');
            expect(background!.kind).toBe('background');
        });

        test('returns null when no Background in Feature', () => {
            const tree: OutlineNode = {
                name: 'Feature: Login',
                detail: 'feature',
                kind: 'feature',
                level: 0,
                line: 0,
                startCharacter: 0,
                endCharacter: 20,
                endLine: 4,
                children: [
                    {
                        name: 'Scenario: login',
                        detail: 'scenario',
                        kind: 'scenario',
                        level: 2,
                        line: 1,
                        startCharacter: 2,
                        endCharacter: 20,
                        endLine: 4,
                        children: [],
                    },
                ],
            };

            const scenario = tree.children[0];
            const background = getBackgroundForScenario(scenario, tree);

            expect(background).toBeNull();
        });

        test('returns Background from correct Feature when multiple Features', () => {
            const tree: OutlineNode = {
                name: 'Feature: Login',
                detail: 'feature',
                kind: 'feature',
                level: 0,
                line: 0,
                startCharacter: 0,
                endCharacter: 20,
                endLine: 10,
                children: [
                    {
                        name: 'Background: login setup',
                        detail: 'background',
                        kind: 'background',
                        level: 1,
                        line: 1,
                        startCharacter: 2,
                        endCharacter: 25,
                        endLine: 2,
                        children: [],
                    },
                    {
                        name: 'Scenario: login scenario',
                        detail: 'scenario',
                        kind: 'scenario',
                        level: 2,
                        line: 3,
                        startCharacter: 2,
                        endCharacter: 25,
                        endLine: 5,
                        children: [],
                    },
                ],
            };

            const scenario = tree.children[1];
            const background = getBackgroundForScenario(scenario, tree);

            expect(background).not.toBeNull();
            expect(background!.name).toBe('Background: login setup');
        });

        test('does not return Background from sibling Feature', () => {
            // This is a flat list of Features as returned by buildOutline
            const trees: OutlineNode[] = [
                {
                    name: 'Feature: Login',
                    detail: 'feature',
                    kind: 'feature',
                    level: 0,
                    line: 0,
                    startCharacter: 0,
                    endCharacter: 20,
                    endLine: 5,
                    children: [
                        {
                            name: 'Background: login bg',
                            detail: 'background',
                            kind: 'background',
                            level: 1,
                            line: 1,
                            startCharacter: 2,
                            endCharacter: 20,
                            endLine: 2,
                            children: [],
                        },
                        {
                            name: 'Scenario: login',
                            detail: 'scenario',
                            kind: 'scenario',
                            level: 2,
                            line: 3,
                            startCharacter: 2,
                            endCharacter: 20,
                            endLine: 5,
                            children: [],
                        },
                    ],
                },
                {
                    name: 'Feature: Checkout',
                    detail: 'feature',
                    kind: 'feature',
                    level: 0,
                    line: 7,
                    startCharacter: 0,
                    endCharacter: 20,
                    endLine: 12,
                    children: [
                        {
                            name: 'Scenario: checkout',
                            detail: 'scenario',
                            kind: 'scenario',
                            level: 2,
                            line: 8,
                            startCharacter: 2,
                            endCharacter: 20,
                            endLine: 12,
                            children: [],
                        },
                    ],
                },
            ];

            // When looking for Background for checkout scenario, should not find login Background
            const checkoutScenario = trees[1].children[0];
            const background = getBackgroundForScenario(checkoutScenario, trees[1]);

            expect(background).toBeNull();
        });

        test('returns most recent Background before scenario', () => {
            const tree: OutlineNode = {
                name: 'Feature: Complex',
                detail: 'feature',
                kind: 'feature',
                level: 0,
                line: 0,
                startCharacter: 0,
                endCharacter: 20,
                endLine: 12,
                children: [
                    {
                        name: 'Background: setup',
                        detail: 'background',
                        kind: 'background',
                        level: 1,
                        line: 1,
                        startCharacter: 2,
                        endCharacter: 20,
                        endLine: 2,
                        children: [],
                    },
                    {
                        name: 'Scenario: first',
                        detail: 'scenario',
                        kind: 'scenario',
                        level: 2,
                        line: 3,
                        startCharacter: 2,
                        endCharacter: 20,
                        endLine: 5,
                        children: [],
                    },
                    {
                        name: 'Scenario: second',
                        detail: 'scenario',
                        kind: 'scenario',
                        level: 2,
                        line: 6,
                        startCharacter: 2,
                        endCharacter: 20,
                        endLine: 8,
                        children: [],
                    },
                ],
            };

            // Both scenarios should get the same Background
            const firstScenario = tree.children[1];
            const secondScenario = tree.children[2];

            const bg1 = getBackgroundForScenario(firstScenario, tree);
            const bg2 = getBackgroundForScenario(secondScenario, tree);

            expect(bg1).not.toBeNull();
            expect(bg2).not.toBeNull();
            expect(bg1!.name).toBe('Background: setup');
            expect(bg2!.name).toBe('Background: setup');
        });
    });

    describe('OutlineNode interface', () => {
        test('has correct structure for Feature node', () => {
            const text = 'Feature: Test';
            const outline = buildOutline(text);
            const feature = outline[0];

            expect(feature.kind).toBeDefined();
            expect(feature.name).toBeDefined();
            expect(feature.detail).toBeDefined();
            expect(feature.level).toBeDefined();
            expect(feature.line).toBeDefined();
            expect(feature.startCharacter).toBeDefined();
            expect(feature.endCharacter).toBeDefined();
            expect(feature.endLine).toBeDefined();
            expect(feature.children).toBeDefined();
            expect(Array.isArray(feature.children)).toBe(true);
        });

        test('has correct structure for Scenario node', () => {
            const text = [
                'Feature: Test',
                '  Scenario: test case',
                '    Given something',
            ].join('\n');

            const outline = buildOutline(text);
            const scenario = outline[0].children[0];

            expect(scenario.kind).toBe('scenario');
            expect(scenario.name).toBe('Scenario: test case');
            expect(scenario.children.length).toBe(1);
        });

        test('has correct structure for Step node', () => {
            const text = [
                'Feature: Test',
                '  Scenario: test',
                '    Given I am here',
            ].join('\n');

            const outline = buildOutline(text);
            const step = outline[0].children[0].children[0];

            expect(step.kind).toBe('step');
            expect(step.detail).toBe('Given');
            expect(step.children.length).toBe(0);
        });
    });
});

describe('Unicode and internationalization', () => {
    test('handles Chinese characters in scenario names', () => {
        const text = [
            'Feature: 登录功能',
            '  Scenario: 用户登录',
            '    Given I am on the login page',
        ].join('\n');

        const outline = buildOutline(text);
        expect(outline[0].name).toBe('Feature: 登录功能');
        expect(outline[0].children[0].name).toBe('Scenario: 用户登录');
    });

    test('handles emoji in scenario names', () => {
        const text = [
            'Feature: Notifications',
            '  Scenario: 🚀 launch rocket',
            '    Given the rocket is fueled',
        ].join('\n');

        const outline = buildOutline(text);
        expect(outline[0].children[0].name).toContain('🚀');
    });

    test('handles emoji in step text', () => {
        const text = [
            'Feature: UI',
            '  Scenario: button with emoji',
            '    Given I see a 👍 button',
        ].join('\n');

        const outline = buildOutline(text);
        const step = outline[0].children[0].children[0];
        expect(step.name).toContain('👍');
    });

    test('handles unicode quotes in scenario names', () => {
        const text = [
            'Feature: Quotes',
            '  Scenario: "double quoted"',
            '    Given something',
        ].join('\n');

        const outline = buildOutline(text);
        expect(outline[0].children[0].name).toContain('"double quoted"');
    });
});

describe('Gherkin edge cases', () => {
    test('handles Scenario Template (alternative Scenario Outline syntax)', () => {
        const text = [
            'Feature: Test',
            '  Scenario Template: test template',
            '    Given I enter <value>',
            '    Examples: cases',
            '      | value |',
            '      | test  |',
        ].join('\n');

        const outline = buildOutline(text);
        expect(outline[0].children[0].name).toBe('Scenario Template: test template');
    });

    test('handles But keyword in steps', () => {
        const text = [
            'Feature: Test',
            '  Scenario: test',
            '    Given I am logged in',
            '    But I have no permissions',
        ].join('\n');

        const outline = buildOutline(text);
        const steps = outline[0].children[0].children;
        expect(steps[1].detail).toBe('But');
    });

    test('handles star (*) keyword for steps', () => {
        const text = [
            'Feature: Test',
            '  Scenario: test',
            '    * first step',
            '    * second step',
        ].join('\n');

        const outline = buildOutline(text);
        const steps = outline[0].children[0].children;
        expect(steps[0].detail).toBe('*');
        expect(steps[1].detail).toBe('*');
    });

    test('handles doc strings in steps', () => {
        const text = [
            'Feature: Test',
            '  Scenario: test',
            '    Given I have a document:',
            '      """',
            '      This is a doc string',
            '      with multiple lines',
            '      """',
        ].join('\n');

        const outline = buildOutline(text);
        // Doc strings should be handled gracefully
        expect(outline[0].children[0]).toBeDefined();
    });

    test('handles data tables in steps', () => {
        const text = [
            'Feature: Test',
            '  Scenario: test',
            '    Given I have users:',
            '      | name  | age |',
            '      | Alice | 30  |',
            '      | Bob   | 25  |',
        ].join('\n');

        const outline = buildOutline(text);
        expect(outline[0].children[0].children[0].name).toBeDefined();
    });
});
