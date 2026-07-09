#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptDir, '../..');

const artifactPairs = [
  {
    name: 'RunConfig JSON Schema',
    source: 'packages/cem_ml/schema/cli/run-config.schema.json',
    dist: 'packages/cem_ml/dist/cli/run-config.schema.json',
    requiredText: 'https://cem.dev/schema/cli/run-config.schema.json',
  },
  {
    name: 'CLI report JSON Schema',
    source: 'packages/cem_ml/schema/cli/report.schema.json',
    dist: 'packages/cem_ml/dist/cli/report.schema.json',
    requiredText: 'https://cem.dev/schema/cli/report.schema.json',
  },
  {
    name: 'transform graph config schema',
    source: 'packages/cem_ml/schema/cli/transform-config.md',
    dist: 'packages/cem_ml/dist/cli/transform-config.md',
    requiredText: 'https://cem.dev/ns/cli/transform-config/1',
  },
];

let failures = 0;

for (const artifact of artifactPairs) {
  const sourcePath = path.join(workspaceRoot, artifact.source);
  const distPath = path.join(workspaceRoot, artifact.dist);

  if (!fs.existsSync(distPath)) {
    failures += 1;
    console.error(`${artifact.name} is missing from ${artifact.dist}`);
    continue;
  }

  const source = fs.readFileSync(sourcePath, 'utf8');
  const dist = fs.readFileSync(distPath, 'utf8');

  if (source !== dist) {
    failures += 1;
    console.error(`${artifact.name} dist artifact does not match ${artifact.source}`);
  }

  if (!dist.includes(artifact.requiredText)) {
    failures += 1;
    console.error(`${artifact.name} dist artifact is missing ${artifact.requiredText}`);
  }
}

const runConfigDistPath = path.join(workspaceRoot, 'packages/cem_ml/dist/cli/run-config.schema.json');
if (fs.existsSync(runConfigDistPath)) {
  const runConfig = JSON.parse(fs.readFileSync(runConfigDistPath, 'utf8'));
  if (runConfig.$id !== 'https://cem.dev/schema/cli/run-config.schema.json') {
    failures += 1;
    console.error('RunConfig JSON Schema dist artifact has an unexpected $id');
  }
}

const reportSchemaDistPath = path.join(workspaceRoot, 'packages/cem_ml/dist/cli/report.schema.json');
if (fs.existsSync(reportSchemaDistPath)) {
  const reportSchema = JSON.parse(fs.readFileSync(reportSchemaDistPath, 'utf8'));
  if (reportSchema.$id !== 'https://cem.dev/schema/cli/report.schema.json') {
    failures += 1;
    console.error('CLI report JSON Schema dist artifact has an unexpected $id');
  }
}

const transformXhtmlPath = path.join(workspaceRoot, 'packages/cem_ml/dist/cli/transform-config.xhtml');
if (!fs.existsSync(transformXhtmlPath)) {
  failures += 1;
  console.error('transform graph config XHTML artifact is missing from packages/cem_ml/dist/cli');
} else {
  const transformXhtml = fs.readFileSync(transformXhtmlPath, 'utf8');
  if (!transformXhtml.includes('https://cem.dev/ns/cli/transform-config/1')) {
    failures += 1;
    console.error('transform graph config XHTML artifact is missing the schema identity');
  }
}

if (failures > 0) {
  process.exitCode = 1;
} else {
  console.log('Validated CLI schema dist artifacts.');
}
