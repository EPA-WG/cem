#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/cem-ml/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');

const commonInputArgs = [
    'packages/cem_ml/schema-packages/cem-ml/v1/examples/basic.cem',
    '--content-type',
    'application/cem',
    '--schema',
    'https://cem.dev/ns/cem-ml/1',
];

const cases = [
    {
        id: 'basic-validate',
        preview: 'basic-validate.svg',
        title: 'CEM-ML validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic CEM-ML example.',
        terminalTitle: 'validate basic.cem',
        renderer: 'json',
        args: ['validate', '--format', 'json', ...commonInputArgs],
    },
    {
        id: 'basic-tabular-terminal',
        preview: 'basic-tabular-terminal.svg',
        title: 'CEM-ML tabular formatter terminal preview',
        description:
            'Terminal-style preview of colored tabular CEM-ML output for the basic CEM-ML example.',
        terminalTitle: 'tabular + terminal color',
        renderer: 'ansi',
        args: [
            'convert',
            ...commonInputArgs,
            '--to-content-type',
            'application/cem',
            '--to-schema',
            'https://cem.dev/ns/cem-ml/1',
            '--cemt-formatter-profile',
            'tabular',
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
    packageLabel: 'CEM-ML',
    refreshCommand: 'node packages/cem_ml/schema-packages/cem-ml/v1/scripts/verify-previews.mjs --update',
});
