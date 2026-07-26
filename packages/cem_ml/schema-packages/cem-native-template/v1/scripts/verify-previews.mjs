#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(
    workspaceRoot,
    'packages/cem_ml/schema-packages/cem-native-template/v1',
);
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');

const commonInputArgs = [
    'packages/cem_ml/schema-packages/cem-native-template/v1/examples/basic-template.cem',
    '--content-type',
    'application/vnd.cem.template+cem',
    '--schema',
    'https://cem.dev/ns/template/cem-native/1',
];

const cases = [
    {
        id: 'basic-template-validate',
        preview: 'basic-template-validate.svg',
        title: 'CEM-native template validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic CEM-native template example.',
        terminalTitle: 'validate basic-template.cem',
        renderer: 'json',
        args: ['validate', '--format', 'json', ...commonInputArgs],
    },
    {
        id: 'basic-template-pretty-terminal',
        preview: 'basic-template-pretty-terminal.svg',
        title: 'CEM-native template pretty formatter terminal preview',
        description:
            'Terminal-style preview of colored pretty CEM-native template output for the basic template example.',
        terminalTitle: 'pretty + terminal color',
        renderer: 'ansi',
        args: [
            'convert',
            ...commonInputArgs,
            '--to-content-type',
            'application/vnd.cem.template+cem',
            '--to-schema',
            'https://cem.dev/ns/template/cem-native/1',
            '--cemt-formatter-profile',
            'pretty',
            '--cemt-color-profile',
            'terminal',
            '--output-color-type',
            'ansi-256',
        ],
    },
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'CEM-native template',
    refreshCommand:
        'node packages/cem_ml/schema-packages/cem-native-template/v1/scripts/verify-previews.mjs --update',
});
