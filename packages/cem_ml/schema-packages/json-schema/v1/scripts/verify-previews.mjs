#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/json-schema/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const packageLabel = 'JSON Schema Resource Schema Package';

function jsonSchemaValidateExampleCase({ id, file, minHeight = 520 }) {
    const path = `packages/cem_ml/schema-packages/json-schema/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `json-schema validate ${file}`,
        renderer: 'json',
        expectedStatus: 'success',
        width: 1040,
        minHeight,
        args: [
            'validate',
            '--format',
            'json',
            '--fail-level',
            'parse',
            '--content-type',
            'application/schema+json',
            '--schema',
            'https://cem.dev/ns/data/json-schema/1',
            path,
        ],
    };
}

function jsonSchemaCliExampleCase({ id, file, minHeight = 190 }) {
    const path = `packages/cem_ml/schema-packages/json-schema/v1/examples/${file}`;
    const inputSpec = `uri=${path},contentType=application/schema+json,schema=https://cem.dev/ns/data/json-schema/1`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `json-schema ${file}`,
        renderer: 'html',
        expectedStatus: 'any',
        fallbackSourcePath: path,
        width: 820,
        minHeight,
        args: [
            'convert',
            '--input-spec',
            inputSpec,
            '--to-content-type',
            'application/schema+json',
            '--to-schema',
            'https://cem.dev/ns/data/json-schema/1',
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'html',
        ],
    };
}

const cases = [
    jsonSchemaCliExampleCase({
        id: 'basic-schema',
        file: 'basic-schema.schema.json',
        minHeight: 310,
    }),
    jsonSchemaCliExampleCase({
        id: 'catalog-schema',
        file: 'catalog-schema.schema.json',
        minHeight: 520,
    }),
    jsonSchemaCliExampleCase({
        id: 'nested-data',
        file: 'nested-data.schema.json',
        minHeight: 430,
    }),
    jsonSchemaValidateExampleCase({
        id: 'invalid-unsupported-dialect',
        file: 'invalid-unsupported-dialect.schema.json',
    }),
    jsonSchemaValidateExampleCase({ id: 'invalid-parse', file: 'invalid-parse.schema.json' }),
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'JSON Schema',
    refreshCommand:
        'node packages/cem_ml/schema-packages/json-schema/v1/scripts/verify-previews.mjs --update',
});
