import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';

const workspaceRoot = resolve(import.meta.dirname, '../../..');
const outputRoot = resolve(workspaceRoot, 'dist/apps/cem-site');
const reportRoot = resolve(workspaceRoot, 'dist/reports/cem-site');
const routeFiles = [
  'index.html',
  'guides/index.html',
  'reference/cem-ml/transform-config/index.html',
];

async function filesUnder(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await filesUnder(path)));
    } else {
      files.push(relative(outputRoot, path).replaceAll('\\', '/'));
    }
  }
  return files.sort();
}

const expectedFiles = [
  ...routeFiles,
  ...routeFiles.map((path) => `${path}.map`),
  'site.report.json',
].sort();
const actualFiles = await filesUnder(outputRoot);
if (JSON.stringify(actualFiles) !== JSON.stringify(expectedFiles)) {
  throw new Error(
    `CEM Site output is not clean.\nExpected: ${expectedFiles.join(', ')}\nActual: ${actualFiles.join(', ')}`,
  );
}

const routeSet = new Set(routeFiles);
const verification = { routes: [], sourceMaps: [], report: 'site.report.json' };
for (const routeFile of routeFiles) {
  const html = await readFile(join(outputRoot, routeFile), 'utf8');
  if (!html.includes('<nav aria-label="Primary">')) {
    throw new Error(`${routeFile} does not contain the shared primary navigation`);
  }
  if (html.includes('&lt;h1') || html.includes('node_modules')) {
    throw new Error(`${routeFile} contains an escaped HTML bridge or source-only path`);
  }

  const links = [...html.matchAll(/href="(\/[^"]*)"/g)].map((match) => match[1]);
  for (const href of links) {
    const target = href === '/' ? 'index.html' : `${href.slice(1)}index.html`;
    if (!routeSet.has(target)) {
      throw new Error(`${routeFile} links to unpublished route ${href}`);
    }
  }
  verification.routes.push({ routeFile, links });

  const sourceMapText = await readFile(join(outputRoot, `${routeFile}.map`), 'utf8');
  const sourceMap = JSON.parse(sourceMapText);
  if (!Array.isArray(sourceMap.outputSpans) || sourceMap.outputSpans.length === 0) {
    throw new Error(`${routeFile}.map has no native output spans`);
  }
  if (!sourceMapText.includes('InterpreterRender')) {
    throw new Error(`${routeFile}.map does not retain CEMT render provenance`);
  }
  verification.sourceMaps.push({
    routeFile,
    outputSpans: sourceMap.outputSpans.length,
    frames: Array.isArray(sourceMap.frames) ? sourceMap.frames.length : 0,
  });
}

const generatedReference = await readFile(
  join(outputRoot, 'reference/cem-ml/transform-config/index.html'),
  'utf8',
);
if (!generatedReference.includes('CEM-ML CLI Transform Config Schema')) {
  throw new Error('generated CEM-ML documentation was not ingested into its stable route');
}

const reportText = await readFile(join(outputRoot, 'site.report.json'), 'utf8');
const publicationGraph = await readFile(
  resolve(workspaceRoot, 'apps/cem-site/site.cem'),
  'utf8',
);
if (
  !publicationGraph.includes('../../packages/cem_ml/dist/cli/transform-config.md') ||
  !reportText.includes('transform-reference-source:import')
) {
  throw new Error('site graph/report do not retain generated-document ownership provenance');
}
if (!reportText.includes('reference/cem-ml/transform-config/index.html')) {
  throw new Error('site report does not retain the generated reference export');
}

await mkdir(reportRoot, { recursive: true });
const reportPath = join(reportRoot, 'verification.json');
await writeFile(reportPath, `${JSON.stringify(verification, null, 2)}\n`, 'utf8');
console.log(`CEM Site verification passed: ${relative(workspaceRoot, reportPath)}`);
