import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

const CREATE_STEP_DEFINITION_COMMAND = 'perl-lsp.createGherkinStepDefinition';
const GHERKIN_STEP_RE = /^\s*(Given|When|Then|And|But)\b\s*(.*)$/;
const STEP_DEFINITION_RE = /^\s*(Given|When|Then|And|But)\s+qr\//;
const QUOTED_CAPTURE_RE = /"[^"\r\n]*"/y;
const OUTLINE_PLACEHOLDER_RE = /<[^>\r\n]+>/y;
const DEFAULT_STEP_DEFINITION_GLOB = '**/*.pm';
const DEFAULT_EXCLUDE_GLOB = '{**/node_modules/**,**/blib/**}';
const MAX_STEP_DEFINITION_FILES = 500;
const MAX_MATCH_REGEX_LENGTH = 256;
const MAX_MATCH_STEP_TEXT_LENGTH = 512;
const POTENTIALLY_EXPENSIVE_REGEX_RE = /(?:\([^)]*[+*][^)]*\)|\[[^\]]+\])[+*{]|\\[1-9]|\(\?<|(\(\?[!=])/;

export type StepKeyword = 'Given' | 'When' | 'Then' | 'And' | 'But';
export type StepDefinitionStatus = 'defined' | 'undefined' | 'ambiguous';

export interface GherkinStepLine {
    keyword: StepKeyword;
    text: string;
    line: number;
    rawLine: string;
}

export interface ExtractedStepDefinition {
    keyword: StepKeyword;
    pattern: string;
}

export interface StepDefinitionScan {
    definitions: ExtractedStepDefinition[];
    ambiguous: boolean;
}

interface CreateStepDefinitionArgs {
    featureUri: string;
    line: number;
}

export function registerGherkinStepDefinitionSupport(): vscode.Disposable[] {
    const selector: vscode.DocumentSelector = [{ language: 'gherkin' }];

    const provider: vscode.CodeActionProvider = {
        async provideCodeActions(document, range) {
            return provideGherkinStepDefinitionActions(document, range);
        },
    };

    return [
        vscode.languages.registerCodeActionsProvider(selector, provider),
        vscode.commands.registerCommand(
            CREATE_STEP_DEFINITION_COMMAND,
            async (args: CreateStepDefinitionArgs) => {
                await createStepDefinitionFromFeature(args);
            }
        ),
    ];
}

export function parseGherkinStepLine(lineText: string, line: number): GherkinStepLine | null {
    const match = lineText.match(GHERKIN_STEP_RE);
    if (!match) {
        return null;
    }

    const text = match[2].trim();
    if (text.length === 0) {
        return null;
    }

    return {
        keyword: match[1] as StepKeyword,
        text,
        line,
        rawLine: lineText,
    };
}

export function buildGeneratedStepPattern(stepText: string): string {
    let pattern = '^';
    let cursor = 0;

    while (cursor < stepText.length) {
        QUOTED_CAPTURE_RE.lastIndex = cursor;
        const quoted = QUOTED_CAPTURE_RE.exec(stepText);
        if (quoted && quoted.index === cursor) {
            pattern += '"([^"]+)"';
            cursor += quoted[0].length;
            continue;
        }

        OUTLINE_PLACEHOLDER_RE.lastIndex = cursor;
        const placeholder = OUTLINE_PLACEHOLDER_RE.exec(stepText);
        if (placeholder && placeholder.index === cursor) {
            pattern += '(.+)';
            cursor += placeholder[0].length;
            continue;
        }

        pattern += escapeRegexLiteral(stepText[cursor]);
        cursor += 1;
    }

    pattern += '$';
    return pattern;
}

export function buildGeneratedStepStub(
    step: GherkinStepLine,
    relativeFeaturePath: string
): string {
    return [
        `# Auto-generated from ${relativeFeaturePath}:${step.line + 1}`,
        `${step.keyword} qr/${buildGeneratedStepPattern(step.text)}/, sub {`,
        '    # TODO: implement step',
        '    return;',
        '};',
    ].join('\n');
}

export function buildStepDefinitionFileContent(
    step: GherkinStepLine,
    relativeFeaturePath: string
): string {
    return [
        'use Test::BDD::Cucumber::StepFile;',
        'use strict;',
        'use warnings;',
        '',
        buildGeneratedStepStub(step, relativeFeaturePath),
        '',
    ].join('\n');
}

export function suggestStepDefinitionPath(featurePath: string, workspaceRoot: string): string {
    const normalisedFeaturePath = path.normalize(featurePath);
    const featureBasename = path.basename(normalisedFeaturePath, '.feature');
    const safeStem = featureBasename
        .replace(/[^A-Za-z0-9]+/g, '_')
        .replace(/^_+|_+$/g, '')
        .toLowerCase();
    const filename = `${safeStem || 'generated'}_steps.pm`;
    const featureParts = normalisedFeaturePath.split(path.sep);
    const featuresIndex = featureParts.lastIndexOf('features');

    if (featuresIndex !== -1) {
        const featuresRoot = featureParts.slice(0, featuresIndex + 1).join(path.sep) || path.sep;
        return path.join(featuresRoot, 'step_definitions', filename);
    }

    return path.join(workspaceRoot, 'features', 'step_definitions', filename);
}

export function scanStepDefinitions(source: string): StepDefinitionScan {
    const definitions: ExtractedStepDefinition[] = [];
    let ambiguous = false;

    for (const line of source.split(/\r?\n/)) {
        const trimmed = line.trim();
        if (!trimmed.match(/^(Given|When|Then|And|But)\b/)) {
            continue;
        }

        if (!STEP_DEFINITION_RE.test(trimmed)) {
            ambiguous = true;
            continue;
        }

        const match = trimmed.match(/^(Given|When|Then|And|But)\s+qr\//);
        if (!match) {
            ambiguous = true;
            continue;
        }

        const pattern = extractSlashDelimitedPattern(trimmed, match[0].length - 1);
        if (!pattern) {
            ambiguous = true;
            continue;
        }

        definitions.push({
            keyword: match[1] as StepKeyword,
            pattern,
        });
    }

    return { definitions, ambiguous };
}

export function classifyStepDefinitionStatus(
    step: GherkinStepLine,
    sources: string[]
): StepDefinitionStatus {
    let ambiguous = false;

    for (const source of sources) {
        const scan = scanStepDefinitions(source);
        ambiguous = ambiguous || scan.ambiguous;

        for (const definition of scan.definitions) {
            const matches = testExtractedDefinition(definition, step.text);
            if (matches === true) {
                return 'defined';
            }
            if (matches === null) {
                ambiguous = true;
            }
        }
    }

    return ambiguous ? 'ambiguous' : 'undefined';
}

async function provideGherkinStepDefinitionActions(
    document: vscode.TextDocument,
    range: vscode.Range
): Promise<vscode.CodeAction[]> {
    const step = parseGherkinStepLine(document.lineAt(range.start.line).text, range.start.line);
    if (!step) {
        return [];
    }

    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (!workspaceFolder) {
        return [];
    }

    const sources = await collectWorkspaceStepDefinitionSources(workspaceFolder);
    const status = classifyStepDefinitionStatus(step, sources);
    if (status !== 'undefined') {
        return [];
    }

    const targetPath = suggestStepDefinitionPath(document.uri.fsPath, workspaceFolder.uri.fsPath);
    const action = {
        title: `Create step definition stub in ${vscode.workspace.asRelativePath(vscode.Uri.file(targetPath))}`,
        kind: vscode.CodeActionKind.QuickFix,
        command: {
            command: CREATE_STEP_DEFINITION_COMMAND,
            title: 'Create step definition stub',
            arguments: [{ featureUri: document.uri.toString(), line: step.line }],
        },
    } as vscode.CodeAction;

    return [action];
}

async function createStepDefinitionFromFeature(args: CreateStepDefinitionArgs): Promise<void> {
    const featureUri = vscode.Uri.parse(args.featureUri);
    const document = await vscode.workspace.openTextDocument(featureUri);
    const step = parseGherkinStepLine(document.lineAt(args.line).text, args.line);
    if (!step) {
        void vscode.window.showWarningMessage('No Gherkin step found on the selected line.');
        return;
    }

    const workspaceFolder = vscode.workspace.getWorkspaceFolder(featureUri);
    if (!workspaceFolder) {
        void vscode.window.showWarningMessage(
            'Step definition generation requires an open workspace folder.'
        );
        return;
    }

    const sources = await collectWorkspaceStepDefinitionSources(workspaceFolder);
    const status = classifyStepDefinitionStatus(step, sources);
    if (status === 'defined') {
        void vscode.window.showInformationMessage(
            `A matching step definition already exists for "${step.text}".`
        );
        return;
    }
    if (status === 'ambiguous') {
        void vscode.window.showWarningMessage(
            'Step definition generation is unavailable because existing step definitions could not be matched confidently.'
        );
        return;
    }

    const targetPath = suggestStepDefinitionPath(featureUri.fsPath, workspaceFolder.uri.fsPath);
    const relativeFeaturePath = vscode.workspace.asRelativePath(featureUri);
    await fs.promises.mkdir(path.dirname(targetPath), { recursive: true });

    if (fs.existsSync(targetPath)) {
        const existing = await fs.promises.readFile(targetPath, 'utf8');
        const stub = buildGeneratedStepStub(step, relativeFeaturePath);
        const separator = existing.trimEnd().length === 0 ? '' : '\n\n';
        await fs.promises.writeFile(
            targetPath,
            `${existing.replace(/\s*$/, '')}${separator}${stub}\n`,
            'utf8'
        );
    } else {
        const content = buildStepDefinitionFileContent(step, relativeFeaturePath);
        await fs.promises.writeFile(targetPath, content, 'utf8');
    }

    const targetDocument = await vscode.workspace.openTextDocument(vscode.Uri.file(targetPath));
    await vscode.window.showTextDocument(targetDocument);
}

async function collectWorkspaceStepDefinitionSources(
    workspaceFolder: vscode.WorkspaceFolder
): Promise<string[]> {
    const files = await vscode.workspace.findFiles(
        DEFAULT_STEP_DEFINITION_GLOB,
        DEFAULT_EXCLUDE_GLOB,
        MAX_STEP_DEFINITION_FILES
    );
    const workspacePrefix = ensureTrailingSeparator(workspaceFolder.uri.fsPath);
    const candidateFiles = files.filter((uri) =>
        ensureTrailingSeparator(uri.fsPath).startsWith(workspacePrefix)
    );

    const sources = await Promise.all(candidateFiles.map(async (uri) => {
        const text = await fs.promises.readFile(uri.fsPath, 'utf8');
        if (!uri.fsPath.includes(`${path.sep}step_definitions${path.sep}`)
            && !text.includes('Test::BDD::Cucumber::StepFile')) {
            return null;
        }
        return text;
    }));

    return sources.filter((value): value is string => typeof value === 'string');
}

function ensureTrailingSeparator(value: string): string {
    return value.endsWith(path.sep) ? value : `${value}${path.sep}`;
}

function escapeRegexLiteral(text: string): string {
    return text.replace(/[|\\{}()[\]^$+*?.]/g, '\\$&').replace(/\//g, '\\/');
}

function extractSlashDelimitedPattern(line: string, delimiterIndex: number): string | null {
    let pattern = '';
    let escaped = false;

    for (let index = delimiterIndex + 1; index < line.length; index += 1) {
        const char = line[index];
        if (escaped) {
            pattern += char;
            escaped = false;
            continue;
        }

        if (char === '\\') {
            pattern += char;
            escaped = true;
            continue;
        }

        if (char === '/') {
            return pattern;
        }

        pattern += char;
    }

    return null;
}

function testExtractedDefinition(
    definition: ExtractedStepDefinition,
    stepText: string
): boolean | null {
    if (!isSafeRegexForStepMatching(definition.pattern, stepText)) {
        return null;
    }

    try {
        return new RegExp(definition.pattern).test(stepText);
    } catch {
        return null;
    }
}

function isSafeRegexForStepMatching(source: string, stepText: string): boolean {
    if (source.length > MAX_MATCH_REGEX_LENGTH || stepText.length > MAX_MATCH_STEP_TEXT_LENGTH) {
        return false;
    }

    return !POTENTIALLY_EXPENSIVE_REGEX_RE.test(source);
}
