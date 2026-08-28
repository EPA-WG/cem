#!/usr/bin/env node

import { createHash } from 'node:crypto';
import {
    existsSync,
    readFileSync,
    readdirSync,
    statSync,
} from 'node:fs';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const packageRoot = join(repoRoot, 'packages/cem-components');
const migrationPath = join(packageRoot, 'declarative-migration.json');
const mvpPath = join(repoRoot, 'docs/component-mvp.md');
const primitivesPath = join(packageRoot, 'src/lib/primitives.ts');
const legacySourceRoot = join(packageRoot, 'src/lib');
const globalStylesPath = join(packageRoot, 'src/styles.css');
const ignoredDirectories = new Set([
    '.vitest-attachments',
    'dist',
    'node_modules',
    'out-tsc',
]);
const codeExtension = /\.(?:[cm]?[jt]sx?)$/;
const componentTag = /^cem-[a-z0-9]+(?:-[a-z0-9]+)*$/;
const requiredThemeModes = [
    'cem-theme-light',
    'cem-theme-dark',
    'cem-theme-contrast-light',
    'cem-theme-contrast-dark',
    'cem-theme-native',
];
const failures = [];

function fail(message) {
    failures.push(message);
}

function readText(path) {
    return readFileSync(path, 'utf8');
}

function sortedUnique(values, label) {
    const sorted = [...values].sort();
    const duplicates = sorted.filter((value, index) => value === sorted[index - 1]);
    if (duplicates.length > 0) {
        fail(`${label} contains duplicate entries: ${[...new Set(duplicates)].join(', ')}`);
    }
    return [...new Set(sorted)];
}

function compareExact(actualValues, expectedValues, label) {
    const actual = sortedUnique(actualValues, `${label} actual inventory`);
    const expected = sortedUnique(expectedValues, `${label} expected inventory`);
    const actualSet = new Set(actual);
    const expectedSet = new Set(expected);
    const added = actual.filter((value) => !expectedSet.has(value));
    const missing = expected.filter((value) => !actualSet.has(value));

    if (added.length > 0) {
        fail(`${label} has forbidden additions: ${added.join(', ')}`);
    }
    if (missing.length > 0) {
        fail(`${label} baseline is stale after removals: ${missing.join(', ')}`);
    }
}

function walkFiles(directory) {
    const files = [];
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
        if (entry.isDirectory() && ignoredDirectories.has(entry.name)) {
            continue;
        }
        const path = join(directory, entry.name);
        if (entry.isDirectory()) {
            files.push(...walkFiles(path));
        } else if (entry.isFile()) {
            files.push(path);
        }
    }
    return files;
}

function slash(path) {
    return path.split(sep).join('/');
}

function parseMvpTags(markdown) {
    const tags = [];
    let inTable = false;
    for (const line of markdown.split(/\r?\n/)) {
        if (line.startsWith('| Category | Component ID | Element name |')) {
            inTable = true;
            continue;
        }
        if (!inTable || line.startsWith('| ---')) {
            continue;
        }
        if (!line.startsWith('|')) {
            break;
        }
        const cells = line.slice(1, -1).split('|').map((cell) => cell.trim());
        const tag = cells[2]?.replaceAll('`', '');
        if (tag) {
            tags.push(tag);
        }
    }
    return tags;
}

function rejectImperativeMarkup(path, source) {
    if (/<script\b/i.test(source)) {
        fail(`${path} contains a forbidden <script> element`);
    }
    if (/\son[a-z0-9-]+\s*=/i.test(source)) {
        fail(`${path} contains a forbidden inline JavaScript event handler`);
    }
    if (/javascript\s*:/i.test(source)) {
        fail(`${path} contains a forbidden javascript: URL`);
    }
    if (/\bbehavior(?:-identity)?\s*=/i.test(source)) {
        fail(`${path} contains a forbidden imperative behavior hook`);
    }
}

function embeddedStyleText(source) {
    const styles = [];
    for (const match of source.matchAll(/<style\b[^>]*>([\s\S]*?)<\/style>/gi)) {
        styles.push(match[1]);
    }
    for (const match of source.matchAll(/\{style(?:\s+[^|{}]*)?\s*\|```([\s\S]*?)```\s*\}/gi)) {
        styles.push(match[1]);
    }
    return styles.join('\n');
}

const migration = JSON.parse(readText(migrationPath));
const componentRoot = join(packageRoot, migration.componentRoot);
const legacyTags = sortedUnique(migration.legacyComponentTags, 'legacyComponentTags');
const legacyCodeFiles = sortedUnique(
    migration.legacyAuthoredCodeFiles,
    'legacyAuthoredCodeFiles',
);
const frozenLegacyImplementationDigests = migration.frozenLegacyImplementationDigests ?? {};
const themeContract = migration.storybookThemeContract ?? {};

if (themeContract.switcherComponent !== 'cem-select') {
    fail('Storybook theme switching must use the production cem-select component');
}
if (themeContract.themeStylesheetExport !== '@epa-wg/cem-theme/styles.css') {
    fail('Storybook must load @epa-wg/cem-theme/styles.css exactly once');
}
if (themeContract.defaultMode !== 'cem-theme-native') {
    fail('Storybook theme switching must default to cem-theme-native');
}
compareExact(themeContract.modes ?? [], requiredThemeModes, 'Storybook theme modes');
if ((themeContract.modes ?? []).join('\n') !== requiredThemeModes.join('\n')) {
    fail('Storybook theme modes must retain the canonical order');
}

if (migration.targetLegacyComponentCount !== 0 || migration.targetAuthoredCodeFileCount !== 0) {
    fail('declarative migration targets must remain zero');
}

const authoredCodeFiles = walkFiles(packageRoot)
    .filter((path) => codeExtension.test(path))
    .filter((path) => !/\/src\/components\/[^/]+\/[^/]+\.stories\.ts$/.test(slash(path)))
    .map((path) => slash(relative(packageRoot, path)))
    .sort();
compareExact(authoredCodeFiles, legacyCodeFiles, 'authored JavaScript/TypeScript inventory');

const legacyImplementationFiles = legacyCodeFiles.filter(
    (path) => path.endsWith('-behavior.ts') || path === 'src/lib/choice-options.ts',
);
compareExact(
    Object.keys(frozenLegacyImplementationDigests),
    legacyImplementationFiles,
    'frozen legacy implementation digest inventory',
);
for (const [path, expectedDigest] of Object.entries(frozenLegacyImplementationDigests)) {
    const absolutePath = join(packageRoot, path);
    if (!existsSync(absolutePath)) {
        fail(`frozen legacy implementation is missing at ${path}`);
        continue;
    }
    if (!/^[a-f0-9]{64}$/.test(expectedDigest)) {
        fail(`frozen legacy implementation digest for ${path} must be lowercase SHA-256`);
        continue;
    }
    const actualDigest = createHash('sha256').update(readText(absolutePath)).digest('hex');
    if (actualDigest !== expectedDigest) {
        fail(
            `${path} is frozen migration debt and cannot be modified; migrate it to XHTML/CEM-ML and cem-elements instead`,
        );
    }
}

const primitiveTags = existsSync(primitivesPath)
    ? [...readText(primitivesPath).matchAll(/\btag:\s*'(cem-[a-z0-9-]+)'/g)].map((match) => match[1])
    : [];
compareExact(primitiveTags, legacyTags, 'legacy CEM_COMPONENT_PRIMITIVES registry');

const declarativeTags = [];
if (!existsSync(componentRoot)) {
    fail(`missing declarative component root ${slash(relative(repoRoot, componentRoot))}`);
} else {
    for (const entry of readdirSync(componentRoot, { withFileTypes: true })) {
        if (!entry.isDirectory()) {
            continue;
        }
        const tag = entry.name;
        declarativeTags.push(tag);
        if (!componentTag.test(tag)) {
            fail(`${migration.componentRoot}/${tag} is not a canonical cem-* component folder`);
            continue;
        }

        const folder = join(componentRoot, tag);
        const declarationPath = join(folder, `${tag}.xhtml`);
        const storyPath = join(folder, `${tag}.stories.ts`);
        for (const requiredPath of [declarationPath, storyPath]) {
            if (!existsSync(requiredPath) || !statSync(requiredPath).isFile()) {
                fail(`${slash(relative(repoRoot, requiredPath))} is required`);
            }
        }
        const standaloneStyles = readdirSync(folder, { withFileTypes: true })
            .filter((child) => child.isFile() && child.name.endsWith('.css'))
            .map((child) => child.name);
        if (standaloneStyles.length > 0) {
            fail(
                `${slash(relative(repoRoot, folder))} has forbidden standalone CSS: ${standaloneStyles.join(', ')}; embed component styles in the CEM-ML declaration`,
            );
        }
        const unexpectedFiles = readdirSync(folder, { withFileTypes: true })
            .filter((child) => child.isFile())
            .map((child) => child.name)
            .filter((name) => name !== `${tag}.xhtml` && name !== `${tag}.stories.ts`);
        if (unexpectedFiles.length > 0) {
            fail(
                `${slash(relative(repoRoot, folder))} contains files outside the XHTML + CSF Next component contract: ${unexpectedFiles.join(', ')}`,
            );
        }
        if (!existsSync(declarationPath) || !existsSync(storyPath)) {
            continue;
        }

        const declaration = readText(declarationPath);
        const style = embeddedStyleText(declaration);
        const story = readText(storyPath);
        rejectImperativeMarkup(slash(relative(repoRoot, declarationPath)), declaration);

        const escapedTag = tag.replaceAll('-', '\\-');
        if (!new RegExp(`<cem-element\\b[^>]*\\btag=["']${escapedTag}["']`, 'i').test(declaration)) {
            fail(`${slash(relative(repoRoot, declarationPath))} must declare <cem-element tag="${tag}">`);
        }
        if (!/<template\b[^>]*\btype=["']text\/cem-ml["']/i.test(declaration)) {
            fail(`${slash(relative(repoRoot, declarationPath))} must contain <template type="text/cem-ml">`);
        }
        if (!new RegExp(`<template\\b(?=[^>]*\\bid=["']${escapedTag}["'])(?=[^>]*\\btype=["']text/cem-ml["'])`, 'i').test(declaration)) {
            fail(
                `${slash(relative(repoRoot, declarationPath))} must expose <template id="${tag}" type="text/cem-ml"> for URL-fragment reuse`,
            );
        }
        if (!style.trim()) {
            fail(
                `${slash(relative(repoRoot, declarationPath))} must embed component CSS as a <style> node (CEM-ML {style |\`\`\`...\`\`\`})`,
            );
        }
        if (/\sstyle\s*=/i.test(declaration)) {
            fail(`${slash(relative(repoRoot, declarationPath))} must use its embedded <style> node instead of inline style attributes`);
        }
        if (!/var\(\s*--cem-[a-z0-9-]+/i.test(style)) {
            fail(`${slash(relative(repoRoot, declarationPath))} embedded CSS must consume CEM UI theme tokens through var(--cem-*)`);
        }
        const nonCemVariables = [...style.matchAll(/var\(\s*(--[a-z0-9-]+)/gi)]
            .map((match) => match[1])
            .filter((name) => !name.startsWith('--cem-') && !name.startsWith('--_cem-'));
        if (nonCemVariables.length > 0) {
            fail(
                `${slash(relative(repoRoot, declarationPath))} embedded CSS uses non-CEM custom properties: ${[...new Set(nonCemVariables)].sort().join(', ')}`,
            );
        }
        if (/\@import\b/i.test(style)) {
            fail(`${slash(relative(repoRoot, declarationPath))} embedded CSS must not import theme CSS; Storybook loads it once`);
        }
        if (!/from\s+['"]storybook\/test['"]/.test(story)) {
            fail(`${slash(relative(repoRoot, storyPath))} must import its test utilities from storybook/test`);
        }
        const allowedStoryImports = new Set([
            'storybook/test',
            '../../../../cem-elements/.storybook/preview.js',
            `./${tag}.xhtml?raw`,
        ]);
        const storyImports = [...story.matchAll(/\bfrom\s+['"]([^'"]+)['"]/g)].map((match) => match[1]);
        const forbiddenStoryImports = storyImports.filter((specifier) => !allowedStoryImports.has(specifier));
        if (forbiddenStoryImports.length > 0) {
            fail(
                `${slash(relative(repoRoot, storyPath))} imports outside the Storybook test/setup contract: ${[...new Set(forbiddenStoryImports)].sort().join(', ')}`,
            );
        }
        if (!/\bpreview\.meta\s*\(/.test(story) || !/\bmeta\.story\s*\(/.test(story)) {
            fail(`${slash(relative(repoRoot, storyPath))} must use the CSF Next preview.meta/meta.story API`);
        }
        if (!new RegExp(`from\\s+['"]\\./${escapedTag}\\.xhtml\\?raw['"]`).test(story)) {
            fail(`${slash(relative(repoRoot, storyPath))} must import its own XHTML declaration as raw Storybook source`);
        }
        if (!/\bloadCemDeclaration\s*\(/.test(story) || !/\bloaders\s*:/.test(story)) {
            fail(`${slash(relative(repoRoot, storyPath))} must load its XHTML declaration through a component-level Storybook loader`);
        }
        if (!/\brender\s*:\s*\(\)\s*=>[\s\S]*<cem-[a-z0-9-]+\b/.test(story)) {
            fail(`${slash(relative(repoRoot, storyPath))} must return the story HTML body from render()`);
        }
        if (!new RegExp(`<${escapedTag}\\b`, 'i').test(story)) {
            fail(`${slash(relative(repoRoot, storyPath))} must render <${tag}>`);
        }
        if (!/\bplay\s*:\s*async\s*\(/.test(story)) {
            fail(`${slash(relative(repoRoot, storyPath))} must colocate unit assertions in an async play function`);
        }
    }
}

const storybookMainPath = join(repoRoot, migration.cemElementsStorybookMain);
const storybookPreviewPath = join(repoRoot, migration.cemElementsStorybookPreview);
if (declarativeTags.length > 0) {
    if (!existsSync(storybookMainPath)) {
        fail(
            `declarative Storybook support is missing at ${migration.cemElementsStorybookMain}`,
        );
    } else {
        const main = readText(storybookMainPath);
        if (!/defineMain\s*\(/.test(main) || !/cem-components\/src\/components\/\*\*\/\*\.stories\.ts/.test(main)) {
            fail(`${migration.cemElementsStorybookMain} must index colocated cem-components CSF Next stories`);
        }
    }
    if (!existsSync(storybookPreviewPath)) {
        fail(
            `declarative Storybook preview support is missing at ${migration.cemElementsStorybookPreview}`,
        );
    } else {
        const preview = readText(storybookPreviewPath);
        if (!/definePreview\s*\(/.test(preview)) {
            fail(`${migration.cemElementsStorybookPreview} must use the CSF Next definePreview API`);
        }
        if (!/export\s+function\s+loadCemDeclaration\s*\(/.test(preview)) {
            fail(`${migration.cemElementsStorybookPreview} must expose lazy component XHTML declaration loading`);
        }
        if (!/['"]@epa-wg\/cem-theme\/styles\.css['"]/.test(preview)) {
            fail(`${migration.cemElementsStorybookPreview} must load the public CEM theme stylesheet once`);
        }
    }
}

const legacyTestFiles = existsSync(legacySourceRoot)
    ? walkFiles(legacySourceRoot).filter((path) => /\.(?:browser\.)?(?:spec|test)\.ts$/.test(path))
    : [];
const legacyProductionFiles = existsSync(legacySourceRoot)
    ? walkFiles(legacySourceRoot).filter((path) => path.endsWith('.ts') && !/\.(?:browser\.)?(?:spec|test)\.ts$/.test(path))
    : [];
const globalStyles = existsSync(globalStylesPath) ? readText(globalStylesPath) : '';
for (const tag of declarativeTags) {
    const escaped = tag.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    for (const path of legacyTestFiles) {
        if (new RegExp(`<${escaped}\\b`, 'i').test(readText(path))) {
            fail(`${slash(relative(repoRoot, path))} contains unit coverage for migrated ${tag}; keep component tests in ${tag}.stories.ts`);
        }
    }
    for (const path of legacyProductionFiles) {
        const source = readText(path);
        if (new RegExp(`\\btag\\s*:\\s*['"]${escaped}['"]|\\.${escaped}__`, 'i').test(source)) {
            fail(`${slash(relative(repoRoot, path))} still implements migrated ${tag} in JavaScript/TypeScript`);
        }
    }
    if (new RegExp(`\\b${escaped}(?=[\\s.#:[>+~,{])`, 'im').test(globalStyles)) {
        fail(`packages/cem-components/src/styles.css still owns selectors for migrated ${tag}; embed them in ${tag}.xhtml`);
    }
}

const overlap = declarativeTags.filter((tag) => legacyTags.includes(tag));
if (overlap.length > 0) {
    fail(`components cannot be both legacy and declarative: ${overlap.sort().join(', ')}`);
}

const mvpTags = parseMvpTags(readText(mvpPath));
compareExact([...legacyTags, ...declarativeTags], mvpTags, 'MVP component ownership');

if (failures.length > 0) {
    console.error('cem-components declarative architecture verification failed:\n');
    for (const failure of failures) {
        console.error(`- ${failure}`);
    }
    process.exit(1);
}

console.log(
    `cem-components declarative architecture verified: ${declarativeTags.length} declarative component(s), ${legacyTags.length} legacy component(s), ${authoredCodeFiles.length} legacy authored JavaScript/TypeScript file(s); declarative component implementation is XHTML/CEM-ML with colocated CSF Next play tests.`,
);
