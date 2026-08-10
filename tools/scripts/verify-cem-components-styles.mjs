#!/usr/bin/env node

import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, extname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const componentMvpPath = join(repoRoot, 'docs/component-mvp.md');
const packageJsonPath = join(repoRoot, 'packages/cem-components/package.json');
const primitivesPath = join(repoRoot, 'packages/cem-components/src/lib/primitives.ts');
const examplesDir = join(repoRoot, 'packages/cem-components/examples');
const workflowFixturesDir = join(repoRoot, 'packages/cem-components/tests/workflows');
const componentRoot = join(repoRoot, 'packages/cem-components');
const componentStylesPath = join(componentRoot, 'src/styles.css');
const themePackageJsonPath = join(repoRoot, 'packages/cem-theme/package.json');
const tokenJsonPath = join(repoRoot, 'packages/cem-theme/dist/lib/tokens/cem.tokens.json');
const tokenCssPath = join(repoRoot, 'packages/cem-theme/dist/lib/css/cem-combined.css');

const TOKEN_FAMILY_PREFIXES = {
    action: ['--cem-action-'],
    bend: ['--cem-bend', '--cem-bend-'],
    control: ['--cem-control-'],
    gap: ['--cem-gap-'],
    inset: ['--cem-inset-'],
    palette: ['--cem-palette-'],
    responsive: ['--cem-bp-', '--cem-cq-'],
    stroke: ['--cem-stroke-'],
    typography: ['--cem-typography-', '--cem-fontography-'],
};

const STYLE_EXTENSIONS = new Set(['.css', '.less', '.pcss', '.scss']);
const CSS_COLOR_LITERAL = /#[0-9a-f]{3,8}\b|\b(?:rgb|rgba|hsl|hsla|hwb|lab|lch|oklab|oklch|color-mix)\s*\(/i;
const CSS_SPACING_PROPERTY =
    /\b(?:margin|padding|gap|inset|top|right|bottom|left|width|height|min-width|max-width|min-height|max-height|border-radius|border-width|outline-width|font-size|line-height)\s*:[^;{}]*/gi;
const CSS_SPACING_LITERAL = /\b\d*\.?\d+(?:px|rem|em|vh|vw|vmin|vmax|ch|ex|%)\b|calc\s*\(/i;
const CSS_VAR_REFERENCE = /var\(\s*(--[^\s,)]+)/g;
const ACTION_TAGS = new Set(['cem-action', 'cem-icon-button', 'cem-menu-item']);
const ACTION_BINDINGS = new Map([
    [
        'cem-action > button',
        new Map([
            ['background-color', 'var(--cem-action-primary-default-background)'],
            ['color', 'var(--cem-action-primary-default-text)'],
        ]),
    ],
    [
        'cem-action > button:enabled:hover',
        new Map([
            ['background-color', 'var(--cem-action-primary-hover-background)'],
            ['color', 'var(--cem-action-primary-hover-text)'],
        ]),
    ],
    [
        'cem-icon-button > button',
        new Map([
            ['background-color', 'var(--cem-action-contextual-default-background)'],
            ['color', 'var(--cem-action-contextual-default-text)'],
        ]),
    ],
    [
        'cem-icon-button > button:enabled:hover',
        new Map([
            ['background-color', 'var(--cem-action-contextual-hover-background)'],
            ['color', 'var(--cem-action-contextual-hover-text)'],
        ]),
    ],
    [
        'cem-menu-item > button',
        new Map([
            ['background-color', 'var(--cem-action-contextual-default-background)'],
            ['color', 'var(--cem-action-contextual-default-text)'],
        ]),
    ],
    [
        'cem-menu-item > button:enabled:hover',
        new Map([
            ['background-color', 'var(--cem-action-contextual-hover-background)'],
            ['color', 'var(--cem-action-contextual-hover-text)'],
        ]),
    ],
]);

const failures = [];

function fail(message) {
    failures.push(message);
}

function readText(path) {
    return readFileSync(path, 'utf8');
}

function readJson(path) {
    return JSON.parse(readText(path));
}

function repoPath(path) {
    return relative(repoRoot, path);
}

function parseMvpComponents(markdown) {
    const components = [];
    let inComponentTable = false;

    for (const line of markdown.split(/\r?\n/)) {
        if (line.startsWith('| Category | Component ID | Element name |')) {
            inComponentTable = true;
            continue;
        }
        if (!inComponentTable) {
            continue;
        }
        if (line.startsWith('| ---')) {
            continue;
        }
        if (!line.startsWith('|')) {
            break;
        }

        const cells = line
            .slice(1, -1)
            .split('|')
            .map((cell) => cell.trim());
        if (cells.length !== 5) {
            fail(`component MVP row must have 5 cells: ${line}`);
            continue;
        }

        const [, idCell, tagCell, , tokenFamiliesCell] = cells;
        const tokenFamilies = tokenFamiliesCell
            .split(',')
            .map((family) => family.trim())
            .filter(Boolean);

        components.push({
            id: stripCode(idCell),
            tag: stripCode(tagCell),
            tokenFamilies,
        });
    }

    return components;
}

function stripCode(value) {
    return value.replace(/^`|`$/g, '');
}

function collectTokenNames(value, names = new Set()) {
    if (!value || typeof value !== 'object') {
        return names;
    }

    const cssName = value?.$extensions?.cem?.cssName;
    if (typeof cssName === 'string' && cssName.startsWith('--cem-')) {
        names.add(cssName);
    }

    for (const child of Object.values(value)) {
        collectTokenNames(child, names);
    }

    return names;
}

function tokenMatchesFamily(tokenName, family) {
    return TOKEN_FAMILY_PREFIXES[family]?.some((prefix) => tokenName === prefix || tokenName.startsWith(prefix));
}

function assertPackageDependsOnTheme() {
    const packageJson = readJson(packageJsonPath);
    if (!packageJson.dependencies?.['@epa-wg/cem-theme']) {
        fail('packages/cem-components/package.json must depend on @epa-wg/cem-theme');
    }
}

function assertThemeStylesheetExport() {
    const packageJson = readJson(themePackageJsonPath);
    if (packageJson.exports?.['./styles.css'] !== './dist/lib/css/cem-combined.css') {
        fail('packages/cem-theme/package.json must export ./styles.css from ./dist/lib/css/cem-combined.css');
    }
}

function assertMvpFamiliesResolveToThemeTokens(components, tokenNames, tokenCss) {
    const usedFamilies = new Set();

    for (const component of components) {
        if (component.tokenFamilies.length === 0) {
            fail(`${component.tag}: must list at least one required token family`);
        }
        for (const family of component.tokenFamilies) {
            usedFamilies.add(family);
            if (!TOKEN_FAMILY_PREFIXES[family]) {
                fail(`${component.tag}: unknown token family \`${family}\``);
            }
        }
    }

    for (const family of [...usedFamilies].sort()) {
        const matchingTokens = [...tokenNames].filter((tokenName) => tokenMatchesFamily(tokenName, family));
        if (matchingTokens.length === 0) {
            fail(`token family \`${family}\` has no generated CEM theme token`);
            continue;
        }

        const cssPrefixes = TOKEN_FAMILY_PREFIXES[family];
        if (!cssPrefixes.some((prefix) => tokenCss.includes(prefix))) {
            fail(`token family \`${family}\` is missing from generated CEM theme CSS`);
        }
    }
}

function assertNoComponentSpecificStyleLiterals() {
    for (const path of styleContractFiles()) {
        const text = readText(path);
        const pathLabel = repoPath(path);

        if (/<style\b/i.test(text)) {
            fail(`${pathLabel}: component fixtures must not include inline <style> blocks`);
        }
        if (/\sstyle\s*=/i.test(text) || /@style\s*=/i.test(text)) {
            fail(`${pathLabel}: component fixtures/declarations must not include inline style attributes`);
        }

        if (STYLE_EXTENSIONS.has(extname(path))) {
            assertCssUsesTokensOnly(pathLabel, text);
        }
    }
}

function assertPublicComponentStyles(components, tokenNames) {
    const pathLabel = repoPath(componentStylesPath);
    const cssText = readText(componentStylesPath);
    const componentTags = new Set(components.map(({ tag }) => tag));
    const rules = parseFlatCssRules(pathLabel, cssText);
    const actionRules = new Map();

    for (const match of cssText.matchAll(CSS_VAR_REFERENCE)) {
        const tokenName = match[1];
        if (!tokenName.startsWith('--cem-')) {
            fail(`${pathLabel}: component styles must prefer generated CEM tokens; found ${tokenName}`);
        } else if (!tokenNames.has(tokenName)) {
            fail(`${pathLabel}: unknown generated CEM theme token ${tokenName}`);
        }
    }

    for (const rule of rules) {
        const tag = rule.selector.match(/^([a-z][a-z0-9-]*)\b/)?.[1];
        if (!tag || !componentTags.has(tag)) {
            fail(`${pathLabel}: selector \`${rule.selector}\` must be scoped by a known CEM component element`);
            continue;
        }

        if (ACTION_TAGS.has(tag)) {
            if (actionRules.has(rule.selector)) {
                fail(`${pathLabel}: duplicate action selector \`${rule.selector}\``);
            }
            actionRules.set(rule.selector, rule.declarations);
        }
    }

    for (const [selector, expectedDeclarations] of ACTION_BINDINGS) {
        const declarations = actionRules.get(selector);
        if (!declarations) {
            fail(`${pathLabel}: missing accepted action binding selector \`${selector}\``);
            continue;
        }

        if (declarations.size !== expectedDeclarations.size) {
            fail(`${pathLabel}: \`${selector}\` must change only background-color and color`);
        }
        for (const [property, expectedValue] of expectedDeclarations) {
            const actualValue = declarations.get(property);
            if (actualValue !== expectedValue) {
                fail(
                    `${pathLabel}: \`${selector}\` must bind ${property} to ${expectedValue}, received ${actualValue}`,
                );
            }
        }
    }

    for (const selector of actionRules.keys()) {
        if (!ACTION_BINDINGS.has(selector)) {
            fail(`${pathLabel}: unexpected action selector \`${selector}\` is outside the accepted hover contract`);
        }
    }
}

function parseFlatCssRules(pathLabel, cssText) {
    const withoutComments = cssText.replace(/\/\*[\s\S]*?\*\//g, '');
    const rules = [];
    const rulePattern = /([^{}]+)\{([^{}]*)\}/g;
    let consumed = '';

    for (const match of withoutComments.matchAll(rulePattern)) {
        const selectorList = match[1].trim();
        const declarations = parseDeclarations(pathLabel, selectorList, match[2]);
        consumed += match[0];

        for (const selector of selectorList.split(',').map((value) => value.trim())) {
            if (!selector) {
                fail(`${pathLabel}: empty component selector`);
                continue;
            }
            rules.push({ declarations, selector });
        }
    }

    const remainder = withoutComments.replace(rulePattern, '').trim();
    if (remainder || (withoutComments.trim() && !consumed)) {
        fail(`${pathLabel}: component stylesheet must contain flat, statically verifiable rules`);
    }

    return rules;
}

function parseDeclarations(pathLabel, selectorList, declarationText) {
    const declarations = new Map();

    for (const declaration of declarationText
        .split(';')
        .map((value) => value.trim())
        .filter(Boolean)) {
        const separator = declaration.indexOf(':');
        if (separator <= 0) {
            fail(`${pathLabel}: invalid declaration \`${declaration}\` in \`${selectorList}\``);
            continue;
        }

        const property = declaration.slice(0, separator).trim();
        const value = declaration.slice(separator + 1).trim();
        if (declarations.has(property)) {
            fail(`${pathLabel}: duplicate ${property} declaration in \`${selectorList}\``);
        }
        declarations.set(property, value);
    }

    return declarations;
}

function styleContractFiles() {
    return [
        primitivesPath,
        ...walkFiles(examplesDir).filter((path) => extname(path) === '.html'),
        ...walkFiles(workflowFixturesDir).filter((path) => extname(path) === '.html'),
        ...walkFiles(componentRoot).filter((path) => STYLE_EXTENSIONS.has(extname(path))),
    ];
}

function walkFiles(path) {
    const entries = readdirSync(path, { withFileTypes: true });
    const files = [];

    for (const entry of entries) {
        const child = join(path, entry.name);
        if (entry.isDirectory()) {
            files.push(...walkFiles(child));
        } else if (entry.isFile()) {
            files.push(child);
        }
    }

    return files;
}

function assertCssUsesTokensOnly(pathLabel, cssText) {
    if (CSS_COLOR_LITERAL.test(cssText)) {
        fail(`${pathLabel}: color literals are forbidden; use CEM theme color tokens`);
    }

    for (const match of cssText.matchAll(CSS_SPACING_PROPERTY)) {
        const declaration = match[0];
        if (CSS_SPACING_LITERAL.test(declaration) && !declaration.includes('var(--cem-')) {
            fail(`${pathLabel}: spacing literal \`${declaration.trim()}\` must resolve through a CEM theme token`);
        }
    }
}

if (!statSync(tokenJsonPath, { throwIfNoEntry: false })) {
    fail(`${repoPath(tokenJsonPath)} is missing; run @epa-wg/cem-theme:build:tokens first`);
}
if (!statSync(tokenCssPath, { throwIfNoEntry: false })) {
    fail(`${repoPath(tokenCssPath)} is missing; run @epa-wg/cem-theme:build:css first`);
}

const components = parseMvpComponents(readText(componentMvpPath));
const tokenNames = statSync(tokenJsonPath, { throwIfNoEntry: false })
    ? collectTokenNames(readJson(tokenJsonPath))
    : new Set();
const tokenCss = statSync(tokenCssPath, { throwIfNoEntry: false }) ? readText(tokenCssPath) : '';

assertPackageDependsOnTheme();
assertThemeStylesheetExport();
assertMvpFamiliesResolveToThemeTokens(components, tokenNames, tokenCss);
assertNoComponentSpecificStyleLiterals();
assertPublicComponentStyles(components, tokenNames);

if (failures.length > 0) {
    for (const failure of failures) {
        console.error(`error: ${failure}`);
    }
    process.exit(1);
}

console.log(
    `cem-components style contract verified (${components.length} primitives, ${tokenNames.size} generated theme tokens).`,
);
