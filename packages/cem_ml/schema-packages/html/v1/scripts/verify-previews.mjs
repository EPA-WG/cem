#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/html/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const packageLabel = 'HTML';
const schema = 'https://cem.dev/ns/data/html/1';

function htmlConvertCase({ id, file, contentType = 'text/html' }) {
    const path = `packages/cem_ml/schema-packages/html/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `HTML schema package v1 ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `html ${file}`,
        renderer: 'html',
        expectedStatus: 'success',
        width: 980,
        minHeight: 190,
        args: [
            'convert',
            '--input-spec',
            `uri=${path},contentType=${contentType},schema=${schema}`,
            '--to-content-type',
            'text/html',
            '--to-schema',
            schema,
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'html',
        ],
    };
}

function htmlValidateCase({ id, file, contentType = 'text/html' }) {
    const path = `packages/cem_ml/schema-packages/html/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `HTML schema package v1 ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `html validate ${file}`,
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
    htmlConvertCase({ id: 'basic-document', file: 'basic-document.html' }),
    htmlConvertCase({ id: 'fragment', file: 'fragment.html' }),
    htmlConvertCase({ id: 'svg-mathml-islands', file: 'svg-mathml-islands.html' }),
    htmlValidateCase({ id: 'invalid-script', file: 'invalid-script.html' }),
    htmlValidateCase({ id: 'invalid-external-resource', file: 'invalid-external-resource.html' }),
    htmlValidateCase({ id: 'invalid-custom-element', file: 'invalid-custom-element.html' }),
    htmlConvertCase({
        id: 'encoding-conflict',
        file: 'encoding-conflict.html',
        contentType: 'text/html; charset=windows-1252',
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
        'node packages/cem_ml/schema-packages/html/v1/scripts/verify-previews.mjs --update',
});
