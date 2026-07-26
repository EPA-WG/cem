#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/cem-ast-projection/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');

const commonArgs = ['validate', '--format', 'json'];
const schemaArgs = ['--schema', 'https://cem.dev/ns/projection/ast/1'];

const cases = [
    {
        id: 'basic-ast-binary-validate',
        preview: 'basic-ast-binary-validate.svg',
        title: 'CEM AST binary validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic binary CEM AST projection example.',
        terminalTitle: 'validate basic-ast.cem-bin',
        renderer: 'json',
        args: [
            ...commonArgs,
            '--content-type',
            'application/vnd.cem.ast+cem-bin',
            ...schemaArgs,
            'packages/cem_ml/schema-packages/cem-ast-projection/v1/examples/basic-ast.cem-bin',
        ],
    },
    {
        id: 'basic-ast-json-validate',
        preview: 'basic-ast-json-validate.svg',
        title: 'CEM AST JSON validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic CEM AST JSON debug view example.',
        terminalTitle: 'validate basic-ast.ast.json',
        renderer: 'json',
        args: [
            ...commonArgs,
            '--content-type',
            'application/vnd.cem.ast+json',
            ...schemaArgs,
            'packages/cem_ml/schema-packages/cem-ast-projection/v1/examples/basic-ast.ast.json',
        ],
    },
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'CEM AST projection',
    refreshCommand:
        'node packages/cem_ml/schema-packages/cem-ast-projection/v1/scripts/verify-previews.mjs --update',
});
