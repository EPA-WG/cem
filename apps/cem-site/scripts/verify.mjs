import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { join, relative, resolve } from 'node:path';
import { createProjectGraphAsync } from '@nx/devkit';

const workspaceRoot = resolve(import.meta.dirname, '../../..');
const projectRoot = resolve(workspaceRoot, 'apps/cem-site');
const outputRoot = resolve(workspaceRoot, 'dist/apps/cem-site');
const reportRoot = resolve(workspaceRoot, 'dist/reports/cem-site');
const manifest = JSON.parse(
  await readFile(resolve(projectRoot, 'site.routes.json'), 'utf8'),
);
const publicationGraph = await readFile(resolve(projectRoot, 'site.cem'), 'utf8');
const reportText = await readFile(join(outputRoot, 'site.report.json'), 'utf8');
const projectGraph = await createProjectGraphAsync({ exitOnError: false });
const siteProject = projectGraph.nodes['cem-site']?.data;
const siteBuildTarget = siteProject?.targets?.build;

if (!siteProject || !siteBuildTarget) {
  throw new Error('the resolved Nx graph does not contain cem-site:build');
}

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
const ownersByRoute = new Map();
const canonicalSourceText = new Map();
const evidenceSourceText = new Map();
const buildInputs = new Set(siteBuildTarget.inputs);
const allowedContentRoles = new Set([
  'landing',
  'guide-index',
  'guide',
  'authored-reference',
  'generated-reference',
  'catalog',
  'example-index',
  'release-notes',
]);
const allowedRelativeLinkPolicies = new Set([
  'none',
  'site-routes',
  'canonical-source',
]);
const buildDependencies = new Set(
  siteBuildTarget.dependsOn
    .filter((dependency) => typeof dependency === 'object')
    .map((dependency) => `${dependency.projects[0]}:${dependency.target}`),
);

function markdownLinks(source) {
  return [...source.matchAll(/\]\(([^)]+)\)/g)].map((match) => match[1].trim());
}

function isRelativeRepositoryLink(href) {
  return (
    !href.startsWith('#') &&
    !href.startsWith('/') &&
    !/^[a-z][a-z0-9+.-]*:/i.test(href)
  );
}

function expectedCanonicalSourceBase(source) {
  const separator = source.lastIndexOf('/');
  const directory = separator === -1 ? '' : source.slice(0, separator + 1);
  return `https://github.com/EPA-WG/cem/blob/develop/${directory}`;
}

function canonicalOwner(source) {
  const candidates = Object.values(projectGraph.nodes)
    .map((node) => ({
      name: node.name,
      root: node.data.root.replaceAll('\\', '/').replace(/^\.\/$/, '.'),
    }))
    .filter(({ root }) =>
      root === '.' ? true : source === root || source.startsWith(`${root}/`),
    )
    .sort((left, right) => right.root.length - left.root.length);
  if (candidates.length === 0) {
    throw new Error(`${source} has no owning Nx project root`);
  }
  const deepest = candidates[0].root.length;
  const owners = candidates.filter(({ root }) => root.length === deepest);
  if (owners.length !== 1) {
    throw new Error(
      `${source} has ambiguous Nx owners: ${owners.map(({ name }) => name).join(', ')}`,
    );
  }
  return owners[0];
}

async function loadDeclaredSources(entry, field, destination) {
  if (entry[field] === undefined) {
    return;
  }
  if (!Array.isArray(entry[field]) || entry[field].length === 0) {
    throw new Error(`${entry.route} ${field} must be a non-empty array`);
  }
  if (new Set(entry[field]).size !== entry[field].length) {
    throw new Error(`${entry.route} has duplicate ${field}`);
  }
  for (const source of entry[field]) {
    if (
      typeof source !== 'string' ||
      /[*?[]/.test(source) ||
      forbiddenSources.some((pattern) => pattern.test(source))
    ) {
      throw new Error(`${entry.route} has invalid ${field} entry ${source}`);
    }
    destination.set(source, await readFile(resolve(workspaceRoot, source), 'utf8'));
  }
}

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
  if (!allowedContentRoles.has(entry.contentRole)) {
    throw new Error(`${entry.route} has unknown content role ${entry.contentRole}`);
  }
  if (!allowedRelativeLinkPolicies.has(entry.relativeLinkPolicy)) {
    throw new Error(
      `${entry.route} has unknown relative-link policy ${entry.relativeLinkPolicy}`,
    );
  }
  if (
    entry.route.startsWith('/reference/') &&
    !['authored-reference', 'generated-reference'].includes(entry.contentRole)
  ) {
    throw new Error(`${entry.route} does not declare an explicit reference role`);
  }
  if (
    entry.contentRole === 'authored-reference' &&
    entry.sourceKind !== 'authored'
  ) {
    throw new Error(`${entry.route} presents generated content as authored reference`);
  }
  if (
    entry.contentRole === 'generated-reference' &&
    entry.sourceKind !== 'generated'
  ) {
    throw new Error(`${entry.route} presents authored content as generated reference`);
  }
  if (entry.route.startsWith('/examples/') && entry.contentRole !== 'example-index') {
    throw new Error(`${entry.route} does not declare the example-index role`);
  }
  if (entry.route.startsWith('/releases/') && entry.contentRole !== 'release-notes') {
    throw new Error(`${entry.route} does not declare the release-notes role`);
  }
  await loadDeclaredSources(entry, 'canonicalSources', canonicalSourceText);
  await loadDeclaredSources(entry, 'evidenceSources', evidenceSourceText);
  if (!entry.owner) {
    throw new Error(`${entry.route} has no canonical Nx owner`);
  }
  const resolvedOwner = canonicalOwner(entry.source);
  if (entry.owner !== resolvedOwner.name) {
    throw new Error(
      `${entry.route} declares owner ${entry.owner}, but ${entry.source} belongs to ${resolvedOwner.name}`,
    );
  }
  ownersByRoute.set(entry.route, resolvedOwner);
  if (entry.sourceKind === 'generated') {
    if (!entry.upstreamTarget || !buildDependencies.has(entry.upstreamTarget)) {
      throw new Error(`${entry.route} does not schedule ${entry.upstreamTarget}`);
    }
    const ownerTargetPrefix = `${entry.owner}:`;
    if (!entry.upstreamTarget.startsWith(ownerTargetPrefix)) {
      throw new Error(
        `${entry.route} upstream target ${entry.upstreamTarget} is not owned by ${entry.owner}`,
      );
    }
    const targetName = entry.upstreamTarget.slice(ownerTargetPrefix.length);
    if (!projectGraph.nodes[entry.owner].data.targets?.[targetName]) {
      throw new Error(
        `${entry.route} upstream target ${entry.upstreamTarget} is absent from the resolved Nx graph`,
      );
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

  const sourceText = await readFile(resolve(workspaceRoot, entry.source), 'utf8');
  const relativeSourceLinks = markdownLinks(sourceText).filter(isRelativeRepositoryLink);
  if (entry.relativeLinkPolicy === 'canonical-source') {
    const expectedBase = expectedCanonicalSourceBase(entry.source);
    if (
      entry.sourceKind !== 'authored' ||
      relativeSourceLinks.length === 0 ||
      entry.canonicalSourceBase !== expectedBase
    ) {
      throw new Error(
        `${entry.route} canonical-source policy must map authored relative links to ${expectedBase}`,
      );
    }
  } else {
    if (entry.canonicalSourceBase !== undefined) {
      throw new Error(`${entry.route} declares an unused canonicalSourceBase`);
    }
    if (entry.relativeLinkPolicy === 'none' && relativeSourceLinks.length !== 0) {
      throw new Error(`${entry.route} leaves repository-relative links without a policy`);
    }
    if (
      entry.relativeLinkPolicy === 'site-routes' &&
      !entry.source.startsWith('apps/cem-site/')
    ) {
      throw new Error(`${entry.route} applies site-route policy outside site-owned content`);
    }
  }
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
  if (entry.relativeLinkPolicy === 'canonical-source') {
    for (const token of [
      '@name="canonicalSourceBase"',
      `@value="${entry.canonicalSourceBase}"`,
    ]) {
      if (!publicationGraph.includes(token)) {
        throw new Error(`${entry.route} is missing canonical-source graph token ${token}`);
      }
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
    if (entry.relativeLinkPolicy === 'canonical-source') {
      const source = await readFile(resolve(workspaceRoot, entry.source), 'utf8');
      for (const href of markdownLinks(source).filter(isRelativeRepositoryLink)) {
        const rewritten = `${entry.canonicalSourceBase}${href}`;
        if (!links.includes(rewritten)) {
          throw new Error(`${entry.output} does not canonically rewrite ${href}`);
        }
      }
    }
    for (const href of links) {
      if (href.startsWith('#')) {
        continue;
      }
      const target = new URL(href, `https://cem.invalid${entry.route}`);
      if (target.origin === 'https://cem.invalid' && !entriesByRoute.has(target.pathname)) {
        throw new Error(`${entry.output} links to unpublished route ${target.pathname}`);
      }
    }
    const verificationEntry = {
      route: entry.route,
      kind: entry.kind,
      contentRole: entry.contentRole,
      relativeLinkPolicy: entry.relativeLinkPolicy,
      owner: entry.owner,
      ownerRoot: ownersByRoute.get(entry.route).root,
      upstreamTarget: entry.upstreamTarget,
      links,
      outputSpans: sourceMap.outputSpans.length,
    };

    if (entry.route === '/tokens/') {
      if (
        entry.source !==
        'packages/cem-theme/dist/lib/tokens/cem.tokens.catalog.json'
      ) {
        throw new Error('the token browser must consume the public theme token catalog');
      }
      if (output.includes('<script')) {
        throw new Error('the static token browser must not load JavaScript');
      }

      const catalog = JSON.parse(
        await readFile(resolve(workspaceRoot, entry.source), 'utf8'),
      );
      if (!Array.isArray(catalog.tokens) || catalog.tokens.length === 0) {
        throw new Error('the public theme token catalog has no tokens');
      }
      const tokenNames = new Set(catalog.tokens.map((token) => token.name));
      if (tokenNames.size !== catalog.tokens.length) {
        throw new Error('the public theme token catalog has duplicate token names');
      }
      const renderedRows = [...output.matchAll(/data-token-name="([^"]+)"/g)];
      if (renderedRows.length !== catalog.tokens.length) {
        throw new Error(
          `token browser rendered ${renderedRows.length} of ${catalog.tokens.length} catalog records`,
        );
      }
      for (const token of catalog.tokens) {
        const canonicalSource = entry.canonicalSources.find(
          (source) => source.endsWith(`/${token.spec}.md`),
        );
        if (!canonicalSource) {
          throw new Error(`${token.name} has undeclared canonical spec ${token.spec}`);
        }
        if (
          !canonicalSourceText
            .get(canonicalSource)
            .includes(`###### ${token.sourceTable}`)
        ) {
          throw new Error(
            `${token.name} has unknown source table ${token.spec}#${token.sourceTable}`,
          );
        }
        if (!output.includes(`data-token-name="${token.name}"`)) {
          throw new Error(`${token.name} is absent from the rendered token browser`);
        }
      }

      const canonicalSpecs = entry.canonicalSources.map((source) =>
        source.slice(source.lastIndexOf('/') + 1, -3),
      );
      if (
        JSON.stringify(catalog.$generated?.sourceSpecs) !==
        JSON.stringify(canonicalSpecs)
      ) {
        throw new Error('token catalog source specs drifted from canonicalSources');
      }
      const buckets = [...new Set(catalog.tokens.map((token) => token.bucket))].sort();
      if (JSON.stringify(buckets) !== JSON.stringify(['visual', 'voice'])) {
        throw new Error(`token catalog bucket coverage drifted: ${buckets.join(', ')}`);
      }

      Object.assign(verificationEntry, {
        canonicalSources: entry.canonicalSources,
        tokenCount: catalog.tokens.length,
        buckets,
        javascript: false,
      });
    }

    if (entry.route === '/components/') {
      if (
        entry.source !==
        'packages/cem-components/dist/catalog/cem.components.catalog.json'
      ) {
        throw new Error('the component gallery must consume the public component catalog');
      }
      if (output.includes('<script')) {
        throw new Error('the static component gallery must not load JavaScript');
      }

      const catalogText = await readFile(resolve(workspaceRoot, entry.source), 'utf8');
      const catalog = JSON.parse(catalogText);
      if (/figma/i.test(catalogText)) {
        throw new Error('the Phase 6 component catalog must not consume Figma projections');
      }
      if (!Array.isArray(catalog.components) || catalog.components.length === 0) {
        throw new Error('the public component catalog has no components');
      }
      if (
        catalog.components.some((component) =>
          Object.prototype.hasOwnProperty.call(component, 'cemMl'),
        )
      ) {
        throw new Error('the component catalog must not copy executable CEM-ML fixtures');
      }

      const componentTags = new Set(catalog.components.map((component) => component.tag));
      if (componentTags.size !== catalog.components.length) {
        throw new Error('the public component catalog has duplicate component tags');
      }
      const renderedRows = [...output.matchAll(/data-component-tag="([^"]+)"/g)];
      if (renderedRows.length !== catalog.components.length) {
        throw new Error(
          `component gallery rendered ${renderedRows.length} of ${catalog.components.length} catalog records`,
        );
      }

      const componentMvp = canonicalSourceText.get('docs/component-mvp.md');
      const primitiveSource = canonicalSourceText.get(
        'packages/cem-components/src/lib/primitives.ts',
      );
      for (const component of catalog.components) {
        if (
          !component.tag?.startsWith('cem-') ||
          !Array.isArray(component.tokenFamilies) ||
          component.tokenFamilies.length === 0 ||
          !Array.isArray(component.categoryStates) ||
          component.categoryStates.length === 0
        ) {
          throw new Error(`component catalog record is incomplete: ${component.tag}`);
        }
        if (!componentMvp.includes(`| \`${component.tag}\` |`)) {
          throw new Error(`${component.tag} is absent from canonical component semantics`);
        }
        if (!primitiveSource.includes(`tag: '${component.tag}'`)) {
          throw new Error(`${component.tag} is absent from the executable primitive inventory`);
        }
        if (!output.includes(`data-component-tag="${component.tag}"`)) {
          throw new Error(`${component.tag} is absent from the rendered component gallery`);
        }
        if (!output.includes(`href="${component.documentation.referenceHref}"`)) {
          throw new Error(`${component.tag} is missing its package-owned reference link`);
        }
      }

      if (
        JSON.stringify(catalog.$generated?.canonicalSources) !==
          JSON.stringify(entry.canonicalSources) ||
        JSON.stringify(catalog.$generated?.evidenceSources) !==
          JSON.stringify(entry.evidenceSources)
      ) {
        throw new Error('component catalog provenance drifted from the route allowlist');
      }
      const stateReportSource = entry.evidenceSources.find((source) =>
        source.endsWith('/component-state-matrix.json'),
      );
      const stateReport = JSON.parse(evidenceSourceText.get(stateReportSource));
      if (
        JSON.stringify(catalog.stateCoverage?.summary) !==
        JSON.stringify(stateReport.summary)
      ) {
        throw new Error('component catalog state coverage drifted from its Nx report');
      }

      const storybook = catalog.relatedSurfaces?.storybook;
      if (
        storybook?.owner !== 'cem-elements' ||
        storybook?.availability !== 'local-build' ||
        storybook?.devTarget !== 'cem-elements:storybook' ||
        storybook?.buildTarget !== 'cem-elements:build-storybook' ||
        !output.includes(`href="${storybook.sourceHref}"`)
      ) {
        throw new Error('component gallery Storybook ownership or source link drifted');
      }
      const examples = catalog.relatedSurfaces?.examples;
      if (!Array.isArray(examples) || examples.length === 0) {
        throw new Error('component catalog must link package-owned examples');
      }
      for (const example of examples) {
        if (
          example.owner !== '@epa-wg/cem-components' ||
          !example.source?.startsWith('packages/cem-components/examples/') ||
          !output.includes(`href="${example.sourceHref}"`)
        ) {
          throw new Error(`component example ownership or link drifted: ${example.name}`);
        }
      }

      Object.assign(verificationEntry, {
        canonicalSources: entry.canonicalSources,
        evidenceSources: entry.evidenceSources,
        componentCount: catalog.components.length,
        stateCoverage: catalog.stateCoverage.summary,
        exampleLinks: examples.length,
        storybookAvailability: storybook.availability,
        javascript: false,
      });
    }

    verification.entries.push(verificationEntry);
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
      ownerRoot: ownersByRoute.get(entry.route).root,
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
