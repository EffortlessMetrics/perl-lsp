import * as fs from 'fs';
import * as path from 'path';

describe('package manifest Perl language registration', () => {
    test('registers common Perl project files by filename', () => {
        const manifestPath = path.resolve(__dirname, '../../package.json');
        const packageJson = JSON.parse(fs.readFileSync(manifestPath, 'utf8')) as {
            contributes?: {
                languages?: Array<{
                    id?: string;
                    filenames?: string[];
                }>;
            };
        };

        const perlLanguage = packageJson.contributes?.languages?.find(language => language.id === 'perl');
        expect(perlLanguage).toBeDefined();

        const filenames = perlLanguage?.filenames ?? [];
        expect(filenames).toEqual(expect.arrayContaining([
            'Makefile.PL',
            'Build.PL',
            'cpanfile',
            'cpanfile.snapshot',
            'dist.ini',
        ]));
    });
});

describe('package manifest demo project command (#1635)', () => {
    const extRoot = path.resolve(__dirname, '../..');

    test('contributes the perl-lsp.openDemoProject command', () => {
        const packageJson = JSON.parse(fs.readFileSync(path.join(extRoot, 'package.json'), 'utf8')) as {
            contributes?: { commands?: Array<{ command?: string; title?: string; category?: string }> };
        };
        const command = packageJson.contributes?.commands?.find(c => c.command === 'perl-lsp.openDemoProject');
        expect(command).toBeDefined();
        expect(command?.title).toBe('Open Demo Project');
        expect(command?.category).toBe('Perl');
    });

    test('bundles the demo project so the command can open it', () => {
        const demoRoot = path.join(extRoot, 'assets', 'demo-project');
        expect(fs.existsSync(path.join(demoRoot, 'main.pl'))).toBe(true);
        expect(fs.existsSync(path.join(demoRoot, 'lib', 'Utils.pm'))).toBe(true);
        expect(fs.existsSync(path.join(demoRoot, 'lib', 'Database.pm'))).toBe(true);
    });
});
