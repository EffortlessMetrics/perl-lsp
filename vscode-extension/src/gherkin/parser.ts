/**
 * Gherkin Parser Utility
 *
 * Extracted and enhanced from gherkinProviders.ts.
 * Provides parsing for Gherkin/BDD .feature files into OutlineNode trees,
 * with support for Scenario Outline expansion and Background tracking.
 */

import * as vscode from 'vscode';

// Types
export type OutlineKind = 'feature' | 'rule' | 'background' | 'scenario' | 'examples' | 'step';
export type StepKeyword = 'Given' | 'When' | 'Then' | 'And' | 'But' | '*';

export interface OutlineNode {
    name: string;
    detail: string;
    kind: OutlineKind;
    level: number;
    line: number;
    startCharacter: number;
    endCharacter: number;
    endLine: number;
    children: OutlineNode[];
    // For Scenario Outlines:
    examples?: ExamplesTable;
    // For Background tracking:
    isBackground?: boolean;
}

export interface ExamplesTable {
    headers: string[];
    rows: string[][];
}

// Regex patterns
const HEADER_RE =
    /^\s*(Feature|Rule|Background|Scenario(?: Outline| Template)?|Examples)\s*:(.*)$/;
const STEP_RE = /^\s*(Given|When|Then|And|But|\*)(?:\s+|$)(.*)$/;

/**
 * Build an outline tree from Gherkin feature file text.
 */
export function buildOutline(text: string): OutlineNode[] {
    const lines = text.split(/\r?\n/);
    const roots: OutlineNode[] = [];
    const stack: OutlineNode[] = [];

    for (let lineNumber = 0; lineNumber < lines.length; lineNumber += 1) {
        const line = lines[lineNumber];
        const headerMatch = line.match(HEADER_RE);
        const stepMatch = line.match(STEP_RE);

        let node: OutlineNode | null = null;
        if (headerMatch) {
            node = createHeaderNode(line, lineNumber, headerMatch[1], headerMatch[2].trim());
        } else if (stepMatch) {
            node = createStepNode(line, lineNumber, stepMatch[1], stepMatch[2].trim());
        }

        if (!node) {
            continue;
        }

        // Pop nodes that are at or below the current level
        while (stack.length > 0 && stack[stack.length - 1].level >= node.level) {
            finalizeNode(stack.pop()!, lineNumber - 1);
        }

        if (stack.length > 0) {
            stack[stack.length - 1].children.push(node);
        } else {
            roots.push(node);
        }

        stack.push(node);
    }

    const lastLine = Math.max(lines.length - 1, 0);
    while (stack.length > 0) {
        finalizeNode(stack.pop()!, lastLine);
    }

    // Post-process: parse Examples tables and attach to parent Scenario Outline
    for (const root of roots) {
        parseExamplesTables(root);
    }

    return roots;
}

/**
 * Expand a Scenario Outline node into multiple nodes, one per Examples row.
 */
export function expandScenarioOutline(node: OutlineNode): OutlineNode[] {
    // Only expand if it's a scenario with examples
    if (!node.examples || node.examples.rows.length === 0) {
        return [];
    }

    const expanded: OutlineNode[] = [];

    for (const row of node.examples.rows) {
        // Create a new node for this row
        const expandedNode: OutlineNode = {
            name: `${node.name} (${row.join(', ')})`,
            detail: node.detail,
            kind: node.kind,
            level: node.level,
            line: node.line,
            startCharacter: node.startCharacter,
            endCharacter: node.endCharacter,
            endLine: node.endLine,
            children: node.children.map(child => ({ ...child })),
            // Don't carry examples to expanded nodes
        };

        expanded.push(expandedNode);
    }

    return expanded;
}

/**
 * Get the Background node that applies to a given scenario.
 * Returns null if no Background precedes the scenario in the same Feature.
 */
export function getBackgroundForScenario(scenario: OutlineNode, tree: OutlineNode): OutlineNode | null {
    // Find the parent Feature (or Rule) of this scenario
    const parent = findParentOfNode(scenario, tree);
    if (!parent) {
        return null;
    }

    // Look for a Background among the parent's children that precede the scenario
    const scenarioIndex = parent.children.indexOf(scenario);
    if (scenarioIndex <= 0) {
        return null;
    }

    // Find the most recent Background before the scenario
    for (let i = scenarioIndex - 1; i >= 0; i--) {
        const sibling = parent.children[i];
        if (sibling.kind === 'background') {
            return sibling;
        }
    }

    return null;
}

/**
 * Find the parent node of a given node in the tree.
 */
function findParentOfNode(target: OutlineNode, tree: OutlineNode): OutlineNode | null {
    for (const child of tree.children) {
        if (child === target) {
            return tree;
        }
        const found = findParentOfNode(target, child);
        if (found) {
            return found;
        }
    }
    return null;
}

/**
 * Parse Examples tables and attach them to the parent Scenario Outline.
 */
function parseExamplesTables(node: OutlineNode): void {
    let currentScenarioOutline: OutlineNode | null = null;
    let currentExamplesHeaders: string[] = [];
    let currentExamplesRows: string[][] = [];

    for (const child of node.children) {
        if (child.kind === 'scenario' &&
            (child.name.startsWith('Scenario Outline:') || child.name.startsWith('Scenario Template:'))) {
            // Attach previous examples if any
            if (currentScenarioOutline && currentExamplesHeaders.length > 0) {
                currentScenarioOutline.examples = {
                    headers: currentExamplesHeaders,
                    rows: currentExamplesRows,
                };
            }
            currentScenarioOutline = child;
            currentExamplesHeaders = [];
            currentExamplesRows = [];
        } else if (child.kind === 'examples' && currentScenarioOutline) {
            // Parse the Examples table from children (data table rows)
            parseExamplesFromTable(child, currentScenarioOutline);
        } else if (child.kind !== 'step') {
            // Non-step, non-scenario child resets the context
            if (currentScenarioOutline && currentExamplesHeaders.length > 0) {
                currentScenarioOutline.examples = {
                    headers: currentExamplesHeaders,
                    rows: currentExamplesRows,
                };
            }
            currentScenarioOutline = null;
            currentExamplesHeaders = [];
            currentExamplesRows = [];
        }

        // Recurse into children
        parseExamplesTables(child);
    }

    // Handle last scenario outline's examples
    if (currentScenarioOutline && currentExamplesHeaders.length > 0) {
        currentScenarioOutline.examples = {
            headers: currentExamplesHeaders,
            rows: currentExamplesRows,
        };
    }
}

/**
 * Parse Examples data from a table node's children.
 */
function parseExamplesFromTable(tableNode: OutlineNode, scenarioOutline: OutlineNode): void {
    // The Examples node typically has table rows as children
    // or the table data is in the Examples node's own structure
    const tableRows = tableNode.children.filter(c => c.kind === 'examples');

    if (tableRows.length >= 2) {
        // First row is headers
        const headerRow = tableRows[0];
        // Find the | cells - they're in the name in format "| col1 | col2 |"
        const headerMatch = headerRow.name.match(/\|([^|]+)\|/g);
        if (headerMatch) {
            scenarioOutline.examples = scenarioOutline.examples || { headers: [], rows: [] };
            scenarioOutline.examples.headers = headerMatch.map(h => h.replace(/\|/g, '').trim());
            scenarioOutline.examples.rows = tableRows.slice(1).map(row => {
                const cellMatch = row.name.match(/\|([^|]+)\|/g);
                return cellMatch ? cellMatch.map(c => c.replace(/\|/g, '').trim()) : [];
            });
        }
    }
}

function createHeaderNode(
    line: string,
    lineNumber: number,
    keyword: string,
    title: string
): OutlineNode {
    const trimmed = line.trim();
    const startCharacter = line.search(/\S|$/);
    const displayName = title.length > 0 ? `${keyword}: ${title}` : trimmed;

    return {
        name: displayName,
        detail: keyword,
        kind: outlineKindForHeader(keyword),
        level: outlineLevelForHeader(keyword),
        line: lineNumber,
        startCharacter,
        endCharacter: line.length,
        endLine: lineNumber,
        children: [],
    };
}

function createStepNode(
    line: string,
    lineNumber: number,
    keyword: string,
    remainder: string
): OutlineNode {
    const startCharacter = line.search(/\S|$/);
    const displayName = remainder.length > 0 ? `${keyword} ${remainder}` : keyword;

    return {
        name: displayName,
        detail: keyword,
        kind: 'step',
        level: 4,
        line: lineNumber,
        startCharacter,
        endCharacter: line.length,
        endLine: lineNumber,
        children: [],
    };
}

function outlineKindForHeader(keyword: string): OutlineKind {
    switch (keyword) {
        case 'Feature':
            return 'feature';
        case 'Rule':
            return 'rule';
        case 'Background':
            return 'background';
        case 'Examples':
            return 'examples';
        default:
            return 'scenario';
    }
}

function outlineLevelForHeader(keyword: string): number {
    switch (keyword) {
        case 'Feature':
            return 0;
        case 'Rule':
            return 1;
        case 'Background':
        case 'Scenario':
        case 'Scenario Outline':
        case 'Scenario Template':
            return 2;
        case 'Examples':
            return 3;
        default:
            return 2;
    }
}

function finalizeNode(node: OutlineNode, endLine: number): void {
    node.endLine = Math.max(node.line, endLine);
}
