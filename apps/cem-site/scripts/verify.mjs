import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { join, relative, resolve } from 'node:path';

const workspaceRoot = resolve(import.meta.dirname, '../../..');
const projectRoot = resolve(workspaceRoot, 'apps/cem-site');
const outputRoot = resolve(workspaceRoot, 'dist/apps/cem-site');
const reportRoot = resolve(workspaceRoot, 'dist/reports/cem-site');
const manifest = JSON.parse(
  await readFile(resolve(projectRoot, 'site.routes.json'), 'utf8'),
);
const project = JSON.parse(await readFile(resolve(projectRoot, 'project.json'), 'utf8'));
const publicationGraph = await readFile(resolve(projectRoot, 'site.cem'), 'utf8');
const reportText = await readFile(join(outputRoot, 'site.report.json'), 'utf8');

if (manifest.version !== 1 || !Array.isArray(manifest.entries)) {
  throw new Error('site.routes.json must declare version 1 and an entries array');
}

const requiredExclusions = [
  'docs/archive/**',
  'docs/todo.md',
  'roadmap.md',
  '**/*.tmp.md',
  '**/figma/**',
  '**/cem.tokens.intermediate.json',
  '**/cem.tokens.resolved.json',
].sort();
if (
  JSON.stringify([...manifest.exclusions].sort()) !==
  JSON.stringify(requiredExclusions)
) {
  throw new Error('site publication exclusions drifted from the accepted boundary');
}

const forbiddenSources = [
  /^docs\/archive\//,
  /^docs\/todo\.md$/,
  /^roadmap\.md$/,
  /\.tmp\.md$/,
  /(^|\/)figma(\/|$)/,
  /cem\.tokens\.(intermediate|resolved)\.json$/,
];
const entriesByRoute = new Map();
const entriesByOutput = new Map();
const importIds = new Set();
const exportIds = new Set();
const buildInputs = new Set(project.targets.build.inputs);
const buildDependencies = new Set(
  project.targets.build.dependsOn
    .filter((dependency) => typeof dependency === 'object')
    .map((dependency) => `${dependency.projects[0]}:${dependency.target}`),
);

for (const entry of manifest.entries) {
  if (!['page', 'resource'].includes(entry.kind)) {
    throw new Error(`unsupported site entry kind: ${entry.kind}`);
  }
  if (!entry.route.startsWith('/') || !entry.output || !entry.source) {
    throw new Error(`invalid site entry: ${JSON.stringify(entry)}`);
  }
  const expectedOutput =
    entry.kind === 'page'
      ? entry.route === '/'
        ? 'index.html'
        : `${entry.route.slice(1)}index.html`
      : entry.route.slice(1);
  if (entry.output !== expectedOutput) {
    throw new Error(`${entry.route} must publish to ${expectedOutput}, got ${entry.output}`);
  }
  if (entriesByRoute.has(entry.route) || entriesByOutput.has(entry.output)) {
    throw new Error(`duplicate site route or output: ${entry.route}`);
  }
  if (importIds.has(entry.importId) || exportIds.has(entry.exportId)) {
    throw new Error(`duplicate graph identity for ${entry.route}`);
  }
  if (forbiddenSources.some((pattern) => pattern.test(entry.source))) {
    throw new Error(`${entry.route} publishes excluded source ${entry.source}`);
  }
  if (!entry.owner) {
    throw new Error(`${entry.route} has no canonical Nx owner`);
  }
  if (entry.sourceKind === 'generated') {
    if (!entry.upstreamTarget || !buildDependencies.has(entry.upstreamTarget)) {
      throw new Error(`${entry.route} does not schedule ${entry.upstreamTarget}`);
    }
  } else if (entry.sourceKind === 'authored') {
    if (entry.upstreamTarget !== null) {
      throw new Error(`${entry.route} gives an authored source a generation target`);
    }
    if (
      !entry.source.startsWith('apps/cem-site/') &&
      !buildInputs.has(`{workspaceRoot}/${entry.source}`)
    ) {
      throw new Error(`${entry.route} source is absent from the Nx build hash`);
    }
  } else {
    throw new Error(`${entry.route} has unknown source kind ${entry.sourceKind}`);
  }

  await readFile(resolve(workspaceRoot, entry.source));
  const graphSource = relative(projectRoot, resolve(workspaceRoot, entry.source)).replaceAll(
    '\\',
    '/',
  );
  const graphOutput = `../../dist/apps/cem-site/${entry.output}`;
  for (const token of [
    `@id=${entry.importId}`,
    `@src="${graphSource}"`,
    `@id=${entry.exportId}`,
    `@out="${graphOutput}"`,
  ]) {
    if (!publicationGraph.includes(token)) {
      throw new Error(`${entry.route} is missing graph token ${token}`);
    }
  }
  if (
    !reportText.includes(`${entry.importId}:import`) ||
    !reportText.includes(entry.output)
  ) {
    throw new Error(`${entry.route} is missing from the transform report`);
  }

  entriesByRoute.set(entry.route, entry);
  entriesByOutput.set(entry.output, entry);
  importIds.add(entry.importId);
  exportIds.add(entry.exportId);
}

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
  ...manifest.entries.flatMap((entry) => [entry.output, `${entry.output}.map`]),
  'site.report.json',
].sort();
const actualFiles = await filesUnder(outputRoot);
if (JSON.stringify(actualFiles) !== JSON.stringify(expectedFiles)) {
  throw new Error(
    `CEM Site output is not clean.\nExpected: ${expectedFiles.join(', ')}\nActual: ${actualFiles.join(', ')}`,
  );
}

const verification = {
  entries: [],
  exclusions: manifest.exclusions,
  report: 'site.report.json',
};
for (const entry of manifest.entries) {
  const output = await readFile(join(outputRoot, entry.output), 'utf8');
  const sourceMapText = await readFile(join(outputRoot, `${entry.output}.map`), 'utf8');
  const sourceMap = JSON.parse(sourceMapText);

  if (entry.kind === 'page') {
    if (!output.includes('<nav aria-label="Primary">')) {
      throw new Error(`${entry.output} does not contain the shared primary navigation`);
    }
    if (output.includes('&lt;h1') || output.includes('node_modules')) {
      throw new Error(`${entry.output} contains an escaped HTML bridge or source-only path`);
    }
    if (!Array.isArray(sourceMap.outputSpans) || sourceMap.outputSpans.length === 0) {
      throw new Error(`${entry.output}.map has no native output spans`);
    }
    if (!sourceMapText.includes('InterpreterRender')) {
      throw new Error(`${entry.output}.map does not retain CEMT render provenance`);
    }

    const links = [...output.matchAll(/href="([^"]+)"/g)].map((match) => match[1]);
    for (const href of links) {
      if (href.startsWith('#')) {
        continue;
      }
      const target = new URL(href, `https://cem.invalid${entry.route}`);
      if (target.origin === 'https://cem.invalid' && !entriesByRoute.has(target.pathname)) {
        throw new Error(`${entry.output} links to unpublished route ${target.pathname}`);
      }
    }
    verification.entries.push({
      route: entry.route,
      kind: entry.kind,
      owner: entry.owner,
      upstreamTarget: entry.upstreamTarget,
      links,
      outputSpans: sourceMap.outputSpans.length,
    });
  } else {
    const source = await readFile(resolve(workspaceRoot, entry.source), 'utf8');
    if (output !== source) {
      throw new Error(`${entry.output} is not a byte-stable publication of ${entry.source}`);
    }
    JSON.parse(output);
    verification.entries.push({
      route: entry.route,
      kind: entry.kind,
      owner: entry.owner,
      upstreamTarget: entry.upstreamTarget,
      bytes: Buffer.byteLength(output),
    });
  }
}

const generatedReference = await readFile(
  join(outputRoot, 'reference/cem-ml/transform-config/index.html'),
  'utf8',
);
if (!generatedReference.includes('CEM-ML CLI Transform Config Schema')) {
  throw new Error('generated CEM-ML documentation was not ingested into its stable route');
}

await mkdir(reportRoot, { recursive: true });
const reportPath = join(reportRoot, 'verification.json');
await writeFile(reportPath, `${JSON.stringify(verification, null, 2)}\n`, 'utf8');
console.log(`CEM Site verification passed: ${relative(workspaceRoot, reportPath)}`);
