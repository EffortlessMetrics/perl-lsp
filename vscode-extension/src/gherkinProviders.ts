import * as vscode from 'vscode';

type OutlineKind = 'feature' | 'rule' | 'background' | 'scenario' | 'examples' | 'step';

interface OutlineNode {
    name: string;
    detail: string;
    kind: OutlineKind;
    level: number;
    line: number;
    startCharacter: number;
    endCharacter: number;
    endLine: number;
    children: OutlineNode[];
}

const HEADER_RE =
    /^\s*(Feature|Rule|Background|Scenario(?: Outline| Template)?|Examples)\s*:(.*)$/;
const STEP_RE = /^\s*(Given|When|Then|And|But|\*)\b(.*)$/;

export function registerGherkinProviders(): vscode.Disposable[] {
    const selector: vscode.DocumentSelector = [{ language: 'gherkin' }];

    const symbolProvider: vscode.DocumentSymbolProvider = {
        provideDocumentSymbols(document: vscode.TextDocument): vscode.DocumentSymbol[] {
            return provideGherkinDocumentSymbols(document.getText());
        },
    };

    const foldingProvider: vscode.FoldingRangeProvider = {
        provideFoldingRanges(document: vscode.TextDocument): vscode.FoldingRange[] {
            return provideGherkinFoldingRanges(document.getText());
        },
    };

    return [
        vscode.languages.registerDocumentSymbolProvider(selector, symbolProvider),
        vscode.languages.registerFoldingRangeProvider(selector, foldingProvider),
    ];
}

export function provideGherkinDocumentSymbols(text: string): vscode.DocumentSymbol[] {
    return buildOutline(text).map(toDocumentSymbol);
}

export function provideGherkinFoldingRanges(text: string): vscode.FoldingRange[] {
    const ranges: vscode.FoldingRange[] = [];

    const visit = (node: OutlineNode): void => {
        if (node.kind !== 'step' && node.endLine > node.line) {
            ranges.push({ start: node.line, end: node.endLine } as vscode.FoldingRange);
        }
        for (const child of node.children) {
            visit(child);
        }
    };

    for (const root of buildOutline(text)) {
        visit(root);
    }

    return ranges;
}

function buildOutline(text: string): OutlineNode[] {
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

    return roots;
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

function toDocumentSymbol(node: OutlineNode): vscode.DocumentSymbol {
    return {
        name: node.name,
        detail: node.detail,
        kind: symbolKind(node.kind),
        range: new vscode.Range(node.line, node.startCharacter, node.endLine, node.endCharacter),
        selectionRange: new vscode.Range(
            node.line,
            node.startCharacter,
            node.line,
            node.endCharacter
        ),
        children: node.children.map(toDocumentSymbol),
    } as vscode.DocumentSymbol;
}

function symbolKind(kind: OutlineKind): vscode.SymbolKind {
    switch (kind) {
        case 'feature':
        case 'rule':
            return vscode.SymbolKind.Namespace;
        case 'background':
            return vscode.SymbolKind.Object;
        case 'scenario':
            return vscode.SymbolKind.Method;
        case 'examples':
            return vscode.SymbolKind.Array;
        case 'step':
            return vscode.SymbolKind.String;
    }
}
