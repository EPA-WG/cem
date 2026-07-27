#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/yaml/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');

const basicInputArgs = [
    'packages/cem_ml/schema-packages/yaml/v1/examples/basic-document.yaml',
    '--content-type',
    'application/yaml',
    '--schema',
    'https://cem.dev/ns/data/yaml/1',
];

const nestedInputArgs = [
    'packages/cem_ml/schema-packages/yaml/v1/examples/nested-stream.yml',
    '--content-type',
    'text/yaml',
    '--schema',
    'https://cem.dev/ns/data/yaml/1',
];

const yamlOutputArgs = [
    '--to-content-type',
    'application/yaml',
    '--to-schema',
    'https://cem.dev/ns/data/yaml/1',
];

const cases = [
    {
        id: 'basic-document-validate',
        preview: 'basic-document-validate.svg',
        title: 'YAML validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic YAML document example.',
        terminalTitle: 'validate basic-document.yaml',
        renderer: 'json',
        args: ['validate', '--format', 'json', ...basicInputArgs],
    },
    {
        id: 'basic-document-tabular-terminal',
        preview: 'basic-document-tabular-terminal.svg',
        title: 'YAML tabular formatter terminal preview',
        description:
            'Terminal-style preview of colored tabular YAML output rendered through the package CEMT formatter and colorizer.',
        terminalTitle: 'tabular + terminal color',
        renderer: 'ansi',
        width: 560,
        minHeight: 180,
        args: [
            'convert',
            ...basicInputArgs,
            ...yamlOutputArgs,
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'terminal',
            '--output-color-type',
            'ansi-256',
        ],
    },
    {
        id: 'nested-stream-tabular-terminal',
        preview: 'nested-stream-tabular-terminal.svg',
        title: 'YAML nested stream terminal preview',
        description:
            'Terminal-style preview of a nested YAML stream rendered through the package CEMT tabular path.',
        terminalTitle: 'nested stream + terminal color',
        renderer: 'ansi',
        width: 620,
        minHeight: 220,
        args: [
            'convert',
            ...nestedInputArgs,
            ...yamlOutputArgs,
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'terminal',
            '--output-color-type',
            'ansi-256',
        ],
    },
    {
        id: 'basic-document-html',
        preview: 'basic-document-html.svg',
        title: 'YAML HTML colorizer preview',
        description:
            'HTML span preview of the YAML package colorizer output inside the generated PRE container.',
        terminalTitle: 'tabular + HTML color',
        renderer: 'html',
        width: 560,
        minHeight: 180,
        args: [
            'convert',
            ...basicInputArgs,
            ...yamlOutputArgs,
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'html',
        ],
    },
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'YAML',
    refreshCommand: 'node packages/cem_ml/schema-packages/yaml/v1/scripts/verify-previews.mjs --update',
});
