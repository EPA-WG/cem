#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/csv/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');

const commonInputArgs = [
    'packages/cem_ml/schema-packages/csv/v1/examples/basic-table.csv',
    '--content-type',
    'text/csv',
    '--schema',
    'https://cem.dev/ns/data/csv/1',
];

const cases = [
    {
        id: 'basic-table-validate',
        preview: 'basic-table-validate.svg',
        title: 'CSV validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic CSV table example.',
        terminalTitle: 'validate basic-table.csv',
        renderer: 'json',
        args: ['validate', '--format', 'json', ...commonInputArgs],
    },
    {
        id: 'basic-table-pretty-terminal',
        preview: 'basic-table-pretty-terminal.svg',
        title: 'CSV pretty formatter terminal preview',
        description:
            'Terminal-style preview of colored pretty CSV output with tab-based near alignment.',
        terminalTitle: 'pretty + terminal color',
        renderer: 'ansi',
        width: 520,
        minHeight: 160,
        args: [
            'convert',
            ...commonInputArgs,
            '--to-content-type',
            'text/csv',
            '--to-schema',
            'https://cem.dev/ns/data/csv/1',
            '--cemt-formatter-profile',
            'pretty',
            '--cemt-color-profile',
            'terminal',
            '--output-color-type',
            'ansi-256',
        ],
    },
    {
        id: 'basic-table-tabular-terminal',
        preview: 'basic-table-tabular-terminal.svg',
        title: 'CSV tabular formatter terminal preview',
        description:
            'Terminal-style preview of colored tabular CSV output with vertically aligned delimiters.',
        terminalTitle: 'tabular + terminal color',
        renderer: 'ansi',
        width: 520,
        minHeight: 160,
        args: [
            'convert',
            ...commonInputArgs,
            '--to-content-type',
            'text/csv',
            '--to-schema',
            'https://cem.dev/ns/data/csv/1',
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-formatter-option',
            'csv.maxFieldWidth=24',
            '--cemt-formatter-option',
            'csv.stringTrim=middle',
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
    packageLabel: 'CSV',
    refreshCommand: 'node packages/cem_ml/schema-packages/csv/v1/scripts/verify-previews.mjs --update',
});
