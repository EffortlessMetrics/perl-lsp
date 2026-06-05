import * as vscode from 'vscode';

type OutlineKind = 'feature' | 'rule' | 'background' | 'scenario' | 'examples' | 'step';
type StepKeyword = 'Given' | 'When' | 'Then' | 'And' | 'But' | '*';
type StepDefinitionKeyword = Exclude<StepKeyword, '*'>;

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

interface GherkinStepReference {
    keyword: StepKeyword;
    effectiveKeyword?: StepDefinitionKeyword;
    text: string;
    originSelectionRange: vscode.Range;
}

export interface StepDefinitionDocument {
    uri: vscode.Uri;
    text: string;
}

interface ParsedStepDefinition {
    keyword: StepDefinitionKeyword;
    matcher: StepMatcher;
    range: vscode.Range;
    uri: vscode.Uri;
    score: number;
}

type StepMatcher =
    | {
        kind: 'exact';
        text: string;
    }
    | {
        kind: 'regex';
        source: string;
        flags: string;
    };

const HEADER_RE =
    /^\s*(Feature|Rule|Background|Scenario(?: Outline| Template)?|Examples)\s*:(.*)$/;
const STEP_RE = /^\s*(Given|When|Then|And|But|\*)(?:\s+|$)(.*)$/;
const STEP_DEFINITION_RE = /^\s*(Given|When|Then|And|But)\b/gm;
const STEP_DEFINITION_FILE_GLOBS = [
    '**/features/step_definitions/**/*.pm',
    '**/step_definitions/**/*.pm',
    '**/*.pm',
    '**/*.pl',
    '**/*.t',
] as const;
const STEP_DEFINITION_EXCLUDE_GLOB = '{**/node_modules/**,**/blib/**,**/.git/**}';
const STEP_DEFINITION_FILE_LIMIT = 1000;
const MAX_MATCH_REGEX_LENGTH = 256;
const MAX_MATCH_STEP_TEXT_LENGTH = 512;
// Catastrophic backtracking (ReDoS) requires a *quantified group that itself
// contains a quantifier* (e.g. `(a+)+`), a backreference, or a lookaround. A
// single character class followed by one quantifier (`[^"]+`, `[0-9]+`) is
// linear-time and safe — flagging it produced false negatives that suppressed
// step-definition links for ordinary `"([^"]+)"` patterns (see #859). Keep this
// in sync with the identical constant in gherkinStepDefinitions.ts.
const POTENTIALLY_EXPENSIVE_REGEX_RE = /(?:\([^)]*[+*][^)]*\))[+*{]|\\[1-9]|\(\?<[=!]|(\(\?[!=])/;
const DELIMITER_PAIRS: Record<string, string> = {
    '{': '}',
    '[': ']',
    '(': ')',
    '<': '>',
};

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

    const definitionProvider: vscode.DefinitionProvider = {
        async provideDefinition(
            document: vscode.TextDocument,
            position: vscode.Position,
            token: vscode.CancellationToken
        ): Promise<vscode.LocationLink[] | undefined> {
            const candidates = await loadStepDefinitionDocuments(token);
            if (token.isCancellationRequested) {
                return undefined;
            }

            const links = provideGherkinStepDefinitionLinks(document.getText(), position, candidates);
            return links.length > 0 ? links : undefined;
        },
    };

    return [
        vscode.languages.registerDocumentSymbolProvider(selector, symbolProvider),
        vscode.languages.registerFoldingRangeProvider(selector, foldingProvider),
        vscode.languages.registerDefinitionProvider(selector, definitionProvider),
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

export function provideGherkinStepDefinitionLinks(
    featureText: string,
    position: vscode.Position,
    documents: readonly StepDefinitionDocument[]
): vscode.LocationLink[] {
    const step = extractStepReference(featureText, position);
    if (!step) {
        return [];
    }

    const matches: ParsedStepDefinition[] = [];
    for (const document of documents) {
        matches.push(...findMatchingStepDefinitions(step, document));
    }

    matches.sort((left, right) => {
        if (right.score !== left.score) {
            return right.score - left.score;
        }

        const leftUri = left.uri.toString();
        const rightUri = right.uri.toString();
        if (leftUri !== rightUri) {
            return leftUri.localeCompare(rightUri);
        }

        return compareRanges(left.range, right.range);
    });

    const deduped = new Map<string, vscode.LocationLink>();
    for (const match of matches) {
        const key = `${match.uri.toString()}:${match.range.start.line}:${match.range.start.character}`;
        if (!deduped.has(key)) {
            deduped.set(key, {
                originSelectionRange: step.originSelectionRange,
                targetUri: match.uri,
                targetRange: match.range,
                targetSelectionRange: match.range,
            });
        }
    }

    return Array.from(deduped.values());
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

async function loadStepDefinitionDocuments(
    token: vscode.CancellationToken
): Promise<StepDefinitionDocument[]> {
    const seen = new Map<string, vscode.Uri>();

    for (const pattern of STEP_DEFINITION_FILE_GLOBS) {
        if (token.isCancellationRequested) {
            return [];
        }

        const uris = await vscode.workspace.findFiles(
            pattern,
            STEP_DEFINITION_EXCLUDE_GLOB,
            STEP_DEFINITION_FILE_LIMIT
        );

        for (const uri of uris) {
            seen.set(uri.toString(), uri);
        }
    }

    const documents: StepDefinitionDocument[] = [];
    for (const uri of seen.values()) {
        if (token.isCancellationRequested) {
            break;
        }

        try {
            const document = await vscode.workspace.openTextDocument(uri);
            documents.push({ uri, text: document.getText() });
        } catch {
            // Ignore unreadable files and continue.
        }
    }

    return documents;
}

function extractStepReference(
    text: string,
    position: vscode.Position
): GherkinStepReference | null {
    const lines = text.split(/\r?\n/);
    const line = lines[position.line];
    if (line === undefined) {
        return null;
    }

    const match = line.match(STEP_RE);
    if (!match) {
        return null;
    }

    const keyword = match[1] as StepKeyword;
    const remainder = match[2].trim();
    if (remainder.length === 0) {
        return null;
    }

    const startCharacter = line.search(/\S|$/);
    if (position.character < startCharacter || position.character > line.length) {
        return null;
    }

    return {
        keyword,
        effectiveKeyword: resolveEffectiveKeyword(lines, position.line, keyword),
        text: remainder,
        originSelectionRange: new vscode.Range(position.line, startCharacter, position.line, line.length),
    };
}

function resolveEffectiveKeyword(
    lines: readonly string[],
    line: number,
    keyword: StepKeyword
): StepDefinitionKeyword | undefined {
    if (keyword === 'Given' || keyword === 'When' || keyword === 'Then') {
        return keyword;
    }

    for (let index = line - 1; index >= 0; index -= 1) {
        const match = lines[index]?.match(STEP_RE);
        if (!match) {
            continue;
        }

        const previousKeyword = match[1] as StepKeyword;
        if (previousKeyword === 'Given' || previousKeyword === 'When' || previousKeyword === 'Then') {
            return previousKeyword;
        }
    }

    return undefined;
}

function findMatchingStepDefinitions(
    step: GherkinStepReference,
    document: StepDefinitionDocument
): ParsedStepDefinition[] {
    const matches: ParsedStepDefinition[] = [];

    for (const definition of parseStepDefinitions(document)) {
        if (!keywordsAreCompatible(step, definition.keyword)) {
            continue;
        }

        if (!stepTextMatches(step.text, definition.matcher)) {
            continue;
        }

        matches.push(definition);
    }

    return matches;
}

function parseStepDefinitions(document: StepDefinitionDocument): ParsedStepDefinition[] {
    const results: ParsedStepDefinition[] = [];
    const lineStarts = computeLineStarts(document.text);

    for (const match of document.text.matchAll(STEP_DEFINITION_RE)) {
        const keyword = match[1] as StepDefinitionKeyword;
        const startOffset = match.index ?? 0;
        let cursor = startOffset + match[0].length;
        cursor = skipWhitespace(document.text, cursor);

        const parsedMatcher = parseStepMatcher(document.text, cursor);
        if (!parsedMatcher) {
            continue;
        }

        const commaOffset = skipWhitespace(document.text, parsedMatcher.endOffset);
        if (document.text[commaOffset] !== ',') {
            continue;
        }

        const rangeStart = startOffset + match[0].search(/\S|$/);
        results.push({
            keyword,
            matcher: parsedMatcher.matcher,
            range: rangeFromOffsets(lineStarts, rangeStart, parsedMatcher.endOffset),
            uri: document.uri,
            score: definitionScore(document.uri, keyword),
        });
    }

    return results;
}

function parseStepMatcher(
    text: string,
    startOffset: number
): { matcher: StepMatcher; endOffset: number } | null {
    if (text.startsWith('qr', startOffset)) {
        return parseRegexStepMatcher(text, startOffset);
    }

    const first = text[startOffset];
    if (first === '\'' || first === '"') {
        return parseQuotedStepMatcher(text, startOffset, first);
    }

    return null;
}

function parseRegexStepMatcher(
    text: string,
    startOffset: number
): { matcher: StepMatcher; endOffset: number } | null {
    let cursor = skipWhitespace(text, startOffset + 2);
    const open = text[cursor];
    if (!open || /\s/.test(open)) {
        return null;
    }

    const close = DELIMITER_PAIRS[open] ?? open;
    const paired = close !== open;
    const patternStart = cursor + 1;
    cursor = patternStart;

    let depth = paired ? 1 : 0;
    while (cursor < text.length) {
        const current = text[cursor];
        if (current === '\\') {
            cursor += 2;
            continue;
        }

        if (paired) {
            if (current === open) {
                depth += 1;
                cursor += 1;
                continue;
            }

            if (current === close) {
                depth -= 1;
                if (depth === 0) {
                    const source = text.slice(patternStart, cursor);
                    cursor += 1;
                    const flagsStart = cursor;
                    while (/[A-Za-z]/.test(text[cursor] ?? '')) {
                        cursor += 1;
                    }
                    return {
                        matcher: {
                            kind: 'regex',
                            source,
                            flags: text.slice(flagsStart, cursor),
                        },
                        endOffset: cursor,
                    };
                }
            }

            cursor += 1;
            continue;
        }

        if (current === close) {
            const source = text.slice(patternStart, cursor);
            cursor += 1;
            const flagsStart = cursor;
            while (/[A-Za-z]/.test(text[cursor] ?? '')) {
                cursor += 1;
            }
            return {
                matcher: {
                    kind: 'regex',
                    source,
                    flags: text.slice(flagsStart, cursor),
                },
                endOffset: cursor,
            };
        }

        cursor += 1;
    }

    return null;
}

function parseQuotedStepMatcher(
    text: string,
    startOffset: number,
    quote: '\'' | '"'
): { matcher: StepMatcher; endOffset: number } | null {
    let cursor = startOffset + 1;
    while (cursor < text.length) {
        const current = text[cursor];
        if (current === '\\') {
            cursor += 2;
            continue;
        }

        if (current === quote) {
            return {
                matcher: {
                    kind: 'exact',
                    text: unescapeQuotedStepText(text.slice(startOffset + 1, cursor), quote),
                },
                endOffset: cursor + 1,
            };
        }

        cursor += 1;
    }

    return null;
}

function unescapeQuotedStepText(text: string, quote: '\'' | '"'): string {
    if (quote === '\'') {
        return text.replace(/\\\\/g, '\\').replace(/\\'/g, '\'');
    }

    return text.replace(/\\\\/g, '\\').replace(/\\"/g, '"');
}

function keywordsAreCompatible(
    step: GherkinStepReference,
    definitionKeyword: StepDefinitionKeyword
): boolean {
    if (step.keyword === '*') {
        return true;
    }

    if (step.keyword === 'And' || step.keyword === 'But') {
        return definitionKeyword === 'And' ||
            definitionKeyword === 'But' ||
            definitionKeyword === step.effectiveKeyword;
    }

    return definitionKeyword === step.keyword ||
        definitionKeyword === 'And' ||
        definitionKeyword === 'But';
}

function stepTextMatches(stepText: string, matcher: StepMatcher): boolean {
    if (matcher.kind === 'exact') {
        return matcher.text === stepText;
    }

    if (!isSafeRegexForStepMatching(matcher.source, stepText)) {
        return false;
    }

    try {
        return new RegExp(matcher.source, normalizeRegexFlags(matcher.flags)).test(stepText);
    } catch {
        return false;
    }
}

function isSafeRegexForStepMatching(source: string, stepText: string): boolean {
    if (source.length > MAX_MATCH_REGEX_LENGTH || stepText.length > MAX_MATCH_STEP_TEXT_LENGTH) {
        return false;
    }

    return !POTENTIALLY_EXPENSIVE_REGEX_RE.test(source);
}

function normalizeRegexFlags(flags: string): string {
    let normalized = '';
    for (const flag of flags.toLowerCase()) {
        if ((flag === 'i' || flag === 'm' || flag === 's') && !normalized.includes(flag)) {
            normalized += flag;
        }
    }
    return normalized;
}

function definitionScore(uri: vscode.Uri, keyword: StepDefinitionKeyword): number {
    let score = 0;
    const normalizedPath = uri.fsPath.replace(/\\/g, '/').toLowerCase();

    if (normalizedPath.includes('/step_definitions/')) {
        score += 20;
    }
    if (normalizedPath.endsWith('.pm')) {
        score += 10;
    } else if (normalizedPath.endsWith('.pl')) {
        score += 5;
    }
    if (keyword === 'Given' || keyword === 'When' || keyword === 'Then') {
        score += 5;
    }

    return score;
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

function skipWhitespace(text: string, offset: number): number {
    let cursor = offset;
    while (cursor < text.length && /\s/.test(text[cursor]!)) {
        cursor += 1;
    }
    return cursor;
}

function computeLineStarts(text: string): number[] {
    const starts = [0];
    for (let index = 0; index < text.length; index += 1) {
        if (text[index] === '\n') {
            starts.push(index + 1);
        }
    }
    return starts;
}

function rangeFromOffsets(
    lineStarts: readonly number[],
    startOffset: number,
    endOffset: number
): vscode.Range {
    const start = offsetToPosition(lineStarts, startOffset);
    const end = offsetToPosition(lineStarts, endOffset);
    return new vscode.Range(start.line, start.character, end.line, end.character);
}

function offsetToPosition(
    lineStarts: readonly number[],
    offset: number
): { line: number; character: number } {
    let low = 0;
    let high = lineStarts.length - 1;

    while (low <= high) {
        const mid = Math.floor((low + high) / 2);
        const lineStart = lineStarts[mid]!;
        const nextLineStart = lineStarts[mid + 1] ?? Number.POSITIVE_INFINITY;

        if (offset < lineStart) {
            high = mid - 1;
        } else if (offset >= nextLineStart) {
            low = mid + 1;
        } else {
            return { line: mid, character: offset - lineStart };
        }
    }

    const lastLine = Math.max(lineStarts.length - 1, 0);
    return { line: lastLine, character: Math.max(offset - (lineStarts[lastLine] ?? 0), 0) };
}

function compareRanges(left: vscode.Range, right: vscode.Range): number {
    if (left.start.line !== right.start.line) {
        return left.start.line - right.start.line;
    }
    return left.start.character - right.start.character;
}
