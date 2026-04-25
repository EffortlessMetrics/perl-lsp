with open('vscode-extension/src/debugAdapter.ts', 'r') as f:
    content = f.read()

new_rewrite = """
const SERVER_DEBUG_TEST_COMMAND = 'perl.debugTest';
export const VSCODE_DEBUG_TEST_COMMAND = 'perl-lsp.debugTest';

const SERVER_RUN_TEST_COMMAND = 'perl.runTest';
export const VSCODE_RUN_TEST_COMMAND = 'perl-lsp.runTests';

const SERVER_TO_EXTENSION_COMMAND_MAP: Record<string, string> = {
    [SERVER_DEBUG_TEST_COMMAND]: VSCODE_DEBUG_TEST_COMMAND,
    [SERVER_RUN_TEST_COMMAND]: VSCODE_RUN_TEST_COMMAND,
};

export function rewriteTestLensCommand<T extends { command?: { command?: string } }>(lens: T): T {
    if (!lens.command || !lens.command.command) {
        return lens;
    }

    const mappedCommand = SERVER_TO_EXTENSION_COMMAND_MAP[lens.command.command];
    if (mappedCommand) {
        return {
            ...lens,
            command: {
                ...lens.command,
                command: mappedCommand,
            },
        };
    }

    return lens;
}"""

old_rewrite = """const SERVER_DEBUG_TEST_COMMAND = 'perl.debugTest';
export const VSCODE_DEBUG_TEST_COMMAND = 'perl-lsp.debugTest';

export interface DebugTestLaunchTarget {
    label: string;
    program: string;
    args: string[];
}

// ---------------------------------------------------------------------------
// Debug configuration wizard helpers (exported for unit testing)
// ---------------------------------------------------------------------------"""

old_rewrite_func = """const SERVER_RUN_TEST_COMMAND = 'perl.runTest';
export const VSCODE_RUN_TEST_COMMAND = 'perl-lsp.runTests';

export function rewriteTestLensCommand<T extends { command?: { command?: string } }>(lens: T): T {
    if (!lens.command) {
        return lens;
    }

    if (lens.command.command === SERVER_DEBUG_TEST_COMMAND) {
        return {
            ...lens,
            command: {
                ...lens.command,
                command: VSCODE_DEBUG_TEST_COMMAND,
            },
        };
    }

    if (lens.command.command === SERVER_RUN_TEST_COMMAND) {
        return {
            ...lens,
            command: {
                ...lens.command,
                command: VSCODE_RUN_TEST_COMMAND,
            },
        };
    }

    return lens;
}"""

content = content.replace(old_rewrite_func, "")
import re
content = re.sub(r"const SERVER_DEBUG_TEST_COMMAND = 'perl\.debugTest';\nexport const VSCODE_DEBUG_TEST_COMMAND = 'perl-lsp\.debugTest';\n", new_rewrite + "\n\nexport interface DebugTestLaunchTarget {\n    label: string;\n    program: string;\n    args: string[];\n}\n", content)

with open('vscode-extension/src/debugAdapter.ts', 'w') as f:
    f.write(content)
