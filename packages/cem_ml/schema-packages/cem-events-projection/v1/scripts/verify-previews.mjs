#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(
    workspaceRoot,
    'packages/cem_ml/schema-packages/cem-events-projection/v1',
);
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');

const packageLabel = 'CEM Events Projection Schema Package';

const cases = [
    sourceFallbackCase('basic-events', 'basic-events.cem-bin'),
    sourceFallbackCase('invalid-binary', 'invalid-binary.cem-bin'),
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel,
    refreshCommand:
        'node packages/cem_ml/schema-packages/cem-events-projection/v1/scripts/verify-previews.mjs --update',
});

function sourceFallbackCase(id, file) {
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `source ${file}`,
        renderer: 'text',
        sourcePath: `packages/cem_ml/schema-packages/cem-events-projection/v1/examples/${file}`,
        width: 920,
        minHeight: 190,
    };
}
