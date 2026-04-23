/**
 * Smart file creation: auto-populate new .pm and .t files with boilerplate.
 *
 * Exported pure functions are unit-tested independently of the VSCode runtime.
 */

import * as path from 'path';

export const enum FileKind {
    Module = 'module',
    Test = 'test',
}

export interface BoilerplateResult {
    kind: FileKind;
    content: string;
}

/**
 * Infer the Perl package name from a `.pm` file path.
 *
 * Anchors on the last `lib` directory segment found in the path.
 * Returns `null` for non-`.pm` paths.
 */
export function inferPackageName(filePath: string): string | null {
    // Normalise Windows backslashes
    const normalised = filePath.replace(/\\/g, '/');
    const ext = path.extname(normalised);

    if (ext !== '.pm') {
        return null;
    }

    const parts = normalised.split('/');
    const libIdx = parts.lastIndexOf('lib');

    if (libIdx !== -1 && libIdx < parts.length - 1) {
        const relative = parts.slice(libIdx + 1);
        const last = relative[relative.length - 1].replace(/\.pm$/, '');
        relative[relative.length - 1] = last;
        return relative.join('::');
    }

    // Fallback: bare basename without extension
    return path.basename(normalised, '.pm');
}

/**
 * Generate boilerplate content for a newly created Perl file.
 *
 * Returns `null` for file types that do not get boilerplate (.pl, .pod, etc.).
 */
export function generateBoilerplate(filePath: string): BoilerplateResult | null {
    const normalised = filePath.replace(/\\/g, '/');
    const ext = path.extname(normalised);

    if (ext === '.pm') {
        const pkg = inferPackageName(filePath) ?? path.basename(normalised, '.pm');
        const content = `package ${pkg};\nuse strict;\nuse warnings;\n\n\n\n1;\n`;
        return { kind: FileKind.Module, content };
    }

    if (ext === '.t') {
        const content = `use strict;\nuse warnings;\nuse Test::More;\n\n\n\ndone_testing;\n`;
        return { kind: FileKind.Test, content };
    }

    return null;
}
