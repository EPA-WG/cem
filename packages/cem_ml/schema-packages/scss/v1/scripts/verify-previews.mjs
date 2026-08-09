#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/scss/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update: process.argv.includes('--update'),
    cases: [],
    packageLabel: 'SCSS',
    refreshCommand: 'yarn nx run cem_ml_schema_package_scss_v1:samples2readme',
});
