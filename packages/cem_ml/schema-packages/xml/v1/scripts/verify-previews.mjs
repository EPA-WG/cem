#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/xml/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const packageLabel = 'XML Resource Schema Package';
const schema = 'https://cem.dev/ns/data/xml/1';

function xmlConvertCase({ id, file, contentType = 'application/xml' }) {
    const path = `packages/cem_ml/schema-packages/xml/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `xml ${file}`,
        renderer: 'html',
        expectedStatus: 'success',
        width: 980,
        minHeight: 190,
        args: [
            'convert',
            '--input-spec',
            `uri=${path},contentType=${contentType},schema=${schema}`,
            '--from-format',
            'xml',
            '--to-content-type',
            contentType,
            '--to-schema',
            schema,
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'html',
        ],
    };
}

function xmlValidateCase({ id, file }) {
    const path = `packages/cem_ml/schema-packages/xml/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `xml validate ${file}`,
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
            'application/xml',
            '--schema',
            schema,
            path,
        ],
    };
}

const cases = [
    xmlConvertCase({ id: 'basic-document', file: 'basic-document.xml' }),
    xmlConvertCase({
        id: 'namespaced-document',
        file: 'namespaced-document.xml',
        contentType: 'text/xml; charset=utf-8',
    }),
    xmlValidateCase({ id: 'invalid-mismatched-tag', file: 'invalid-mismatched-tag.xml' }),
    xmlValidateCase({ id: 'invalid-unbound-prefix', file: 'invalid-unbound-prefix.xml' }),
    xmlValidateCase({ id: 'invalid-doctype', file: 'invalid-doctype.xml' }),
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'XML',
    refreshCommand: 'node packages/cem_ml/schema-packages/xml/v1/scripts/verify-previews.mjs --update',
});
