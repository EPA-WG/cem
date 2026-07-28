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

function jsonSchemaExampleCase({ id, file, minHeight = 190 }) {
    const path = `packages/cem_ml/schema-packages/json-schema/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `source ${file}`,
        renderer: 'text',
        sourcePath: path,
        width: 920,
        minHeight,
    };
}

const cases = [
    jsonSchemaExampleCase({ id: 'basic-schema', file: 'basic-schema.schema.json', minHeight: 240 }),
    jsonSchemaExampleCase({
        id: 'catalog-schema',
        file: 'catalog-schema.schema.json',
        minHeight: 440,
    }),
    jsonSchemaExampleCase({
        id: 'invalid-unsupported-dialect',
        file: 'invalid-unsupported-dialect.schema.json',
    }),
    jsonSchemaExampleCase({ id: 'invalid-parse', file: 'invalid-parse.schema.json' }),
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
