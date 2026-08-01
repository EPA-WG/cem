#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/css/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const packageLabel = 'CSS';
const schema = 'https://cem.dev/ns/data/css/1';

function cssConvertCase({ id, file, contentType = 'text/css' }) {
    const path = `packages/cem_ml/schema-packages/css/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `CSS schema package v1 ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `css ${file}`,
        renderer: 'html',
        expectedStatus: 'success',
        width: 980,
        minHeight: 190,
        args: [
            'convert',
            '--input-spec',
            `uri=${path},contentType=${contentType},schema=${schema}`,
            '--to-content-type',
            'text/css',
            '--to-schema',
            schema,
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'html',
        ],
    };
}

function cssValidateCase({ id, file, contentType = 'text/css' }) {
    const path = `packages/cem_ml/schema-packages/css/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `CSS schema package v1 ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `css validate ${file}`,
        renderer: 'json',
        expectedStatus: 'success',
        width: 1040,
        minHeight: 520,
        args: [
            'validate',
            '--format',
            'json',
            '--fail-level',
            'parse',
            '--content-type',
            contentType,
            '--schema',
            schema,
            path,
        ],
    };
}

const cases = [
    cssConvertCase({ id: 'basic-stylesheet', file: 'basic-stylesheet.css' }),
    cssConvertCase({
        id: 'scoped-component',
        file: 'scoped-component.css',
        contentType: 'text/css; mode=scoped-style-block',
    }),
    cssConvertCase({
        id: 'style-attribute',
        file: 'style-attribute.css',
        contentType: 'text/css; mode=style-attribute',
    }),
    cssValidateCase({ id: 'invalid-import', file: 'invalid-import.css' }),
    cssValidateCase({ id: 'invalid-url', file: 'invalid-url.css' }),
    cssValidateCase({ id: 'invalid-token', file: 'invalid-token.css' }),
    cssConvertCase({ id: 'invalid-declaration', file: 'invalid-declaration.css' }),
    cssConvertCase({
        id: 'encoding-conflict',
        file: 'encoding-conflict.css',
        contentType: 'text/css; charset=iso-8859-1',
    }),
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel,
    refreshCommand:
        'node packages/cem_ml/schema-packages/css/v1/scripts/verify-previews.mjs --update',
});
