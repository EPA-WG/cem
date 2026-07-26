#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/cem-ql/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');

const commonInputArgs = [
    'packages/cem_ml/schema-packages/cem-ql/v1/examples/basic-query.cemql',
    '--content-type',
    'application/vnd.cem.query+cem-ql',
    '--schema',
    'https://cem.dev/ns/query/cem-ql/1',
];

const cases = [
    {
        id: 'basic-query-validate',
        preview: 'basic-query-validate.svg',
        title: 'CEM-QL validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic CEM-QL query example.',
        terminalTitle: 'validate basic-query.cemql',
        renderer: 'json',
        args: ['validate', '--format', 'json', ...commonInputArgs],
    },
    {
        id: 'basic-query-tabular-terminal',
        preview: 'basic-query-tabular-terminal.svg',
        title: 'CEM-QL tabular formatter terminal preview',
        description:
            'Terminal-style preview of colored tabular CEM-QL output for the basic query example.',
        terminalTitle: 'tabular + terminal color',
        renderer: 'ansi',
        args: [
            'convert',
            ...commonInputArgs,
            '--to-content-type',
            'application/vnd.cem.query+cem-ql',
            '--to-schema',
            'https://cem.dev/ns/query/cem-ql/1',
            '--cemt-formatter',
            'cem-ql.format-tree',
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-colorizer',
            'cem-ql.color-tree',
            '--cemt-color-profile',
            'terminal',
            '--output-color-type',
            'ansi-256',
        ],
    },
    {
        id: 'basic-query-tabular-html',
        preview: 'basic-query-tabular-html.svg',
        title: 'CEM-QL tabular formatter HTML preview',
        description: 'Rendered preview of HTML color output for the basic CEM-QL query example.',
        terminalTitle: 'tabular + HTML color',
        renderer: 'html',
        args: [
            'convert',
            ...commonInputArgs,
            '--to-content-type',
            'text/html',
            '--to-schema',
            'https://cem.dev/ns/data/html/1',
            '--cemt-formatter',
            'cem-ql.format-tree',
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-colorizer',
            'cem-ql.color-tree',
            '--cemt-color-profile',
            'html',
            '--output-color-type',
            'html-css-vars',
        ],
    },
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'CEM-QL',
    refreshCommand: 'node packages/cem_ml/schema-packages/cem-ql/v1/scripts/verify-previews.mjs --update',
});
