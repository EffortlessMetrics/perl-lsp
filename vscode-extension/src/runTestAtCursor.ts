import * as vscode from 'vscode';

export type CursorTestCommand = {
    command: string;
    arguments?: unknown[];
};

type PositionLike = {
    line: number;
    character: number;
};

type RangeLike = {
    start: PositionLike;
    end: PositionLike;
};

type CodeLensLike = {
    range?: RangeLike;
    command?: CursorTestCommand;
};

const RUN_TEST_COMMANDS = new Set(['perl.runTest', 'perl.runSubtest', 'perl.runTestFile']);

function positionBeforeOrEqual(left: PositionLike, right: PositionLike): boolean {
    return left.line < right.line
        || (left.line === right.line && left.character <= right.character);
}

function positionAfterOrEqual(left: PositionLike, right: PositionLike): boolean {
    return left.line > right.line
        || (left.line === right.line && left.character >= right.character);
}

function rangeContains(range: RangeLike, position: PositionLike): boolean {
    return positionBeforeOrEqual(range.start, position) && positionAfterOrEqual(range.end, position);
}

function rangeSpan(range: RangeLike): number {
    const lineSpan = Math.max(0, range.end.line - range.start.line);
    const charSpan = Math.max(0, range.end.character - range.start.character);
    return lineSpan * 10_000 + charSpan;
}

/**
 * Pick the best test-related code lens that contains the cursor position.
 */
export function selectTestCommandAtPosition(
    lenses: CodeLensLike[],
    position: vscode.Position,
): CursorTestCommand | undefined {
    const cursor = { line: position.line, character: position.character };
    const matches = lenses
        .filter((lens): lens is CodeLensLike & { command: CursorTestCommand } => {
            return !!lens.command
                && !!lens.range
                && RUN_TEST_COMMANDS.has(lens.command.command)
                && rangeContains(lens.range, cursor);
        })
        .sort((left, right) => {
            const leftPriority = left.command!.command === 'perl.runTest'
                ? 0
                : left.command!.command === 'perl.runSubtest'
                    ? 1
                    : 2;
            const rightPriority = right.command!.command === 'perl.runTest'
                ? 0
                : right.command!.command === 'perl.runSubtest'
                    ? 1
                    : 2;
            if (leftPriority !== rightPriority) {
                return leftPriority - rightPriority;
            }

            return rangeSpan(left.range!) - rangeSpan(right.range!);
        });

    return matches[0]?.command;
}
