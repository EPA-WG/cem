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

function authoredCodeDigest(relativePaths) {
    const aggregate = createHash('sha256');
    for (const relativePath of [...relativePaths].sort()) {
        const contentDigest = createHash('sha256')
            .update(readFileSync(join(packageRoot, relativePath)))
            .digest('hex');
        aggregate.update(`${contentDigest}  packages/cem-components/${relativePath}\n`);
    }
    return aggregate.digest('hex');
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
    .map((path) => slash(relative(packageRoot, path)))
    .sort();
compareExact(authoredCodeFiles, legacyCodeFiles, 'authored JavaScript/TypeScript inventory');

if (authoredCodeFiles.length > 0) {
    const digest = authoredCodeDigest(authoredCodeFiles);
    if (digest !== migration.legacyAuthoredCodeDigest) {
        fail(
            'legacy JavaScript/TypeScript changed. Hard stop: migrate the affected UI to XHTML/CEM-ML or add the missing reusable capability to cem-elements; do not refresh the debt digest for an imperative component change',
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
        const storyPath = join(folder, `${tag}.stories.xhtml`);
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
        if (!existsSync(declarationPath) || !existsSync(storyPath)) {
            continue;
        }

        const declaration = readText(declarationPath);
        const style = embeddedStyleText(declaration);
        const story = readText(storyPath);
        rejectImperativeMarkup(slash(relative(repoRoot, declarationPath)), declaration);
        rejectImperativeMarkup(slash(relative(repoRoot, storyPath)), story);

        const escapedTag = tag.replaceAll('-', '\\-');
        if (!new RegExp(`<cem-element\\b[^>]*\\btag=["']${escapedTag}["']`, 'i').test(declaration)) {
            fail(`${slash(relative(repoRoot, declarationPath))} must declare <cem-element tag="${tag}">`);
        }
        if (!/<template\b[^>]*\btype=["']text\/cem-ml["']/i.test(declaration)) {
            fail(`${slash(relative(repoRoot, declarationPath))} must contain <template type="text/cem-ml">`);
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
        if (!/<cem-story\b/i.test(story) || !/<cem-test\b/i.test(story)) {
            fail(`${slash(relative(repoRoot, storyPath))} must colocate <cem-story> cases and <cem-test> unit assertions`);
        }
    }
}

const adapterEvidencePath = join(repoRoot, migration.cemElementsStorybookAdapterEvidence);
const themeEvidencePath = join(repoRoot, migration.cemElementsStorybookThemeEvidence);
if (declarativeTags.length > 0) {
    if (!existsSync(adapterEvidencePath)) {
        fail(
            `declarative Storybook support is missing at ${migration.cemElementsStorybookAdapterEvidence}. Hard stop: implement the XHTML story indexer/loader in cem-elements before adding or migrating a component`,
        );
    }
    if (!existsSync(themeEvidencePath)) {
        fail(
            `Storybook-owned theme switching is missing at ${migration.cemElementsStorybookThemeEvidence}. Hard stop: implement the production-cem-select five-mode controller, theme CSS loading, and embedded-style rendering in cem-elements before adding or migrating a component`,
        );
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
    `cem-components declarative architecture verified: ${declarativeTags.length} declarative component(s), ${legacyTags.length} frozen legacy component(s), ${authoredCodeFiles.length} frozen authored JavaScript/TypeScript file(s); targets are zero legacy components and zero authored code files.`,
);
