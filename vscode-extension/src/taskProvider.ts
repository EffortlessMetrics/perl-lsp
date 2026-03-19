import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';

interface PerlTaskDefinition extends vscode.TaskDefinition {
    task: string;
}

/**
 * Detects Perl project type from workspace files and provides
 * auto-detected build/test tasks.
 */
export class PerlTaskProvider implements vscode.TaskProvider {
    static readonly type = 'perl';

    private taskPromise: Thenable<vscode.Task[]> | undefined;
    private fileWatcher: vscode.FileSystemWatcher | undefined;

    constructor() {
        // Invalidate cached tasks when project files change
        this.fileWatcher = vscode.workspace.createFileSystemWatcher(
            '**/{Makefile.PL,Build.PL,cpanfile,dist.ini,Makefile}'
        );
        this.fileWatcher.onDidChange(() => this.taskPromise = undefined);
        this.fileWatcher.onDidCreate(() => this.taskPromise = undefined);
        this.fileWatcher.onDidDelete(() => this.taskPromise = undefined);
    }

    public provideTasks(): Thenable<vscode.Task[]> {
        if (!this.taskPromise) {
            this.taskPromise = detectTasks();
        }
        return this.taskPromise;
    }

    public resolveTask(task: vscode.Task): vscode.Task | undefined {
        const definition = task.definition as PerlTaskDefinition;
        if (definition.task) {
            return createTask(definition, task.scope ?? vscode.TaskScope.Workspace);
        }
        return undefined;
    }

    public dispose(): void {
        this.fileWatcher?.dispose();
    }
}

interface ProjectInfo {
    folder: vscode.WorkspaceFolder;
    hasMakefilePL: boolean;
    hasBuildPL: boolean;
    hasCpanfile: boolean;
    hasDistIni: boolean;
    hasTestDir: boolean;
    hasMakefile: boolean;
}

async function detectProjectInfo(folder: vscode.WorkspaceFolder): Promise<ProjectInfo> {
    const root = folder.uri.fsPath;
    const exists = (name: string) => fs.existsSync(path.join(root, name));

    return {
        folder,
        hasMakefilePL: exists('Makefile.PL'),
        hasBuildPL: exists('Build.PL'),
        hasCpanfile: exists('cpanfile'),
        hasDistIni: exists('dist.ini'),
        hasTestDir: exists('t'),
        hasMakefile: exists('Makefile'),
    };
}

async function detectTasks(): Promise<vscode.Task[]> {
    const folders = vscode.workspace.workspaceFolders;
    if (!folders) {
        return [];
    }

    const tasks: vscode.Task[] = [];

    for (const folder of folders) {
        const info = await detectProjectInfo(folder);
        const folderTasks = generateTasks(info);
        tasks.push(...folderTasks);
    }

    return tasks;
}

function generateTasks(info: ProjectInfo): vscode.Task[] {
    const tasks: vscode.Task[] = [];

    // Run Tests - if t/ directory exists
    if (info.hasTestDir) {
        tasks.push(createTask(
            { type: 'perl', task: 'test' },
            info.folder,
            'Perl: Run Tests',
            'prove -lr t/'
        ));
    }

    // Build - depends on project type
    if (info.hasMakefilePL) {
        if (info.hasMakefile) {
            tasks.push(createTask(
                { type: 'perl', task: 'build' },
                info.folder,
                'Perl: Build',
                'make'
            ));
        } else {
            tasks.push(createTask(
                { type: 'perl', task: 'build' },
                info.folder,
                'Perl: Build',
                'perl Makefile.PL && make'
            ));
        }
    } else if (info.hasBuildPL) {
        tasks.push(createTask(
            { type: 'perl', task: 'build' },
            info.folder,
            'Perl: Build',
            'perl Build.PL && ./Build'
        ));
    } else if (info.hasDistIni) {
        tasks.push(createTask(
            { type: 'perl', task: 'build' },
            info.folder,
            'Perl: Build',
            'dzil build'
        ));
    }

    // Install Dependencies
    if (info.hasCpanfile || info.hasMakefilePL || info.hasBuildPL) {
        tasks.push(createTask(
            { type: 'perl', task: 'install-deps' },
            info.folder,
            'Perl: Install Dependencies',
            'cpanm --installdeps .'
        ));
    }

    // Run Current File - always available
    tasks.push(createTask(
        { type: 'perl', task: 'run-file' },
        info.folder,
        'Perl: Run Current File',
        'perl ${file}'
    ));

    // Check Syntax - always available
    tasks.push(createTask(
        { type: 'perl', task: 'check-syntax' },
        info.folder,
        'Perl: Check Syntax',
        'perl -c ${file}'
    ));

    return tasks;
}

function createTask(
    definition: PerlTaskDefinition,
    scope: vscode.WorkspaceFolder | vscode.TaskScope,
    name?: string,
    command?: string,
): vscode.Task {
    const taskName = name ?? definition.task;
    const shellCmd = command ?? definition.task;

    const task = new vscode.Task(
        definition,
        scope,
        taskName,
        'perl',
        new vscode.ShellExecution(shellCmd),
        ['$perlc']
    );

    // Set group hints
    switch (definition.task) {
        case 'test':
            task.group = vscode.TaskGroup.Test;
            break;
        case 'build':
            task.group = vscode.TaskGroup.Build;
            break;
    }

    return task;
}
