#!/usr/bin/env node

import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, extname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import postcss from 'postcss';

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
    content: ['--cem-content-interaction-'],
    control: ['--cem-control-', '--cem-list-', '--cem-menu-', '--cem-table-'],
    gap: ['--cem-gap-'],
    inset: ['--cem-inset-'],
    layering: ['--cem-layer-', '--cem-elevation-'],
    navigation: ['--cem-navigation-item-'],
    palette: ['--cem-palette-'],
    responsive: ['--cem-bp-', '--cem-cq-'],
    select: ['--cem-select-'],
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
const CONTENT_INTERACTION_TAGS = new Set(['cem-chip', 'cem-list']);
const FEEDBACK_TAGS = new Set(['cem-dialog', 'cem-dialog-shell', 'cem-sheet']);
const NAVIGATION_TAGS = new Set(['cem-nav', 'cem-tabs']);
const PUBLIC_COMPONENT_ADAPTERS = new Set(['--cem-input-indicator-appearance']);
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
        'cem-action > button:enabled:active',
        new Map([
            ['background-color', 'var(--cem-action-primary-active-background)'],
            ['color', 'var(--cem-action-primary-active-text)'],
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
        'cem-icon-button > button:enabled:active',
        new Map([
            ['background-color', 'var(--cem-action-contextual-active-background)'],
            ['color', 'var(--cem-action-contextual-active-text)'],
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
    [
        'cem-menu-item > button:enabled:active',
        new Map([
            ['background-color', 'var(--cem-action-contextual-active-background)'],
            ['color', 'var(--cem-action-contextual-active-text)'],
        ]),
    ],
]);
const CONTENT_INTERACTION_BINDINGS = new Map([
    ...[
        'cem-list[selectable] > select.cem-list.cem-list--selectable',
        'cem-chip[checkable] > button.cem-chip',
    ].map((selector) => [
        selector,
        colorBinding('--cem-content-interaction-default-background', '--cem-content-interaction-default-text'),
    ]),
    [
        "cem-chip[checkable] > button.cem-chip[aria-pressed='true']",
        colorBinding('--cem-content-interaction-selected-background', '--cem-content-interaction-selected-text'),
    ],
    ...[
        'cem-list[selectable] > select.cem-list.cem-list--selectable:enabled:hover',
        'cem-chip[checkable] > button.cem-chip:enabled:hover',
    ].map((selector) => [
        selector,
        colorBinding('--cem-content-interaction-hover-background', '--cem-content-interaction-hover-text'),
    ]),
    [
        "cem-chip[checkable] > button.cem-chip[aria-pressed='true']:enabled:hover",
        colorBinding(
            '--cem-content-interaction-selected-hover-background',
            '--cem-content-interaction-selected-hover-text',
        ),
    ],
    ...[
        'cem-list[selectable] > select.cem-list.cem-list--selectable:disabled',
        'cem-chip[checkable] > button.cem-chip:disabled',
        "cem-chip[checkable] > button.cem-chip[aria-pressed='true']:disabled",
    ].map((selector) => [
        selector,
        colorBinding('--cem-content-interaction-disabled-background', '--cem-content-interaction-disabled-text'),
    ]),
    ...[
        'cem-list[selectable] > select.cem-list.cem-list--selectable:enabled:focus-visible',
        'cem-chip[checkable] > button.cem-chip:enabled:focus-visible',
    ].map((selector) => [selector, focusBinding()]),
]);
const NAVIGATION_BINDINGS = new Map([
    ...[
        'cem-nav > nav > :is(a[href], button)',
        'cem-nav > nav > .cem-nav__content > :is(a[href], button)',
        "cem-tabs > [role='tablist'] > button[role='tab']",
    ].map((selector) => [
        selector,
        colorBinding('--cem-navigation-item-default-background', '--cem-navigation-item-default-text'),
    ]),
    ...[
        "cem-nav > nav > :is(a[href]:not([aria-disabled='true']), button:enabled:not([aria-disabled='true'])):hover",
        "cem-nav > nav > .cem-nav__content > :is(a[href]:not([aria-disabled='true']), button:enabled:not([aria-disabled='true'])):hover",
        "cem-tabs > [role='tablist'] > button[role='tab']:enabled:not([aria-disabled='true']):hover",
    ].map((selector) => [
        selector,
        colorBinding('--cem-navigation-item-hover-background', '--cem-navigation-item-hover-text'),
    ]),
    ...[
        "cem-nav > nav > :is(a[href]:not([aria-disabled='true']), button:enabled:not([aria-disabled='true'])):active",
        "cem-nav > nav > .cem-nav__content > :is(a[href]:not([aria-disabled='true']), button:enabled:not([aria-disabled='true'])):active",
        "cem-tabs > [role='tablist'] > button[role='tab']:enabled:not([aria-disabled='true']):active",
    ].map((selector) => [
        selector,
        colorBinding('--cem-navigation-item-active-background', '--cem-navigation-item-active-text'),
    ]),
    ...[
        "cem-nav > nav > :is(a[href], button)[aria-current]:not([aria-current='false'])",
        "cem-nav > nav > .cem-nav__content > :is(a[href], button)[aria-current]:not([aria-current='false'])",
        "cem-tabs > [role='tablist'] > button[role='tab'][aria-selected='true']",
    ].map((selector) => [
        selector,
        colorBinding('--cem-navigation-item-current-background', '--cem-navigation-item-current-text'),
    ]),
    ...[
        "cem-nav > nav > :is(a[href]:not([aria-disabled='true']), button:enabled:not([aria-disabled='true']))[aria-current]:not([aria-current='false']):hover",
        "cem-nav > nav > .cem-nav__content > :is(a[href]:not([aria-disabled='true']), button:enabled:not([aria-disabled='true']))[aria-current]:not([aria-current='false']):hover",
        "cem-tabs > [role='tablist'] > button[role='tab'][aria-selected='true']:enabled:not([aria-disabled='true']):hover",
    ].map((selector) => [
        selector,
        colorBinding('--cem-navigation-item-current-hover-background', '--cem-navigation-item-current-hover-text'),
    ]),
    ...[
        "cem-nav > nav > :is(a[href]:not([aria-disabled='true']), button:enabled:not([aria-disabled='true']))[aria-current]:not([aria-current='false']):active",
        "cem-nav > nav > .cem-nav__content > :is(a[href]:not([aria-disabled='true']), button:enabled:not([aria-disabled='true']))[aria-current]:not([aria-current='false']):active",
        "cem-tabs > [role='tablist'] > button[role='tab'][aria-selected='true']:enabled:not([aria-disabled='true']):active",
    ].map((selector) => [
        selector,
        colorBinding(
            '--cem-navigation-item-current-active-background',
            '--cem-navigation-item-current-active-text',
        ),
    ]),
    ...[
        "cem-nav > nav > :is(a[href], button):is(button:disabled, [aria-disabled='true'])",
        "cem-nav > nav > .cem-nav__content > :is(a[href], button):is(button:disabled, [aria-disabled='true'])",
        "cem-tabs > [role='tablist'] > button[role='tab']:is(:disabled, [aria-disabled='true'])",
        "cem-nav > nav > :is(a[href], button)[aria-current]:not([aria-current='false']):is(button:disabled, [aria-disabled='true'])",
        "cem-nav > nav > .cem-nav__content > :is(a[href], button)[aria-current]:not([aria-current='false']):is(button:disabled, [aria-disabled='true'])",
        "cem-tabs > [role='tablist'] > button[role='tab'][aria-selected='true']:is(:disabled, [aria-disabled='true'])",
    ].map((selector) => [
        selector,
        colorBinding('--cem-navigation-item-disabled-background', '--cem-navigation-item-disabled-text'),
    ]),
    ...[
        'cem-nav > nav > :is(a[href], button:enabled):focus-visible',
        'cem-nav > nav > .cem-nav__content > :is(a[href], button:enabled):focus-visible',
        "cem-tabs > [role='tablist'] > button[role='tab']:enabled:focus-visible",
    ].map((selector) => [selector, focusBinding()]),
]);
const FEEDBACK_BINDINGS = new Map([
    ['cem-dialog[transient] > dialog.cem-dialog:focus-visible', focusBinding()],
    ['cem-dialog-shell[transient] > dialog.cem-dialog-shell:focus-visible', focusBinding()],
]);
const FEEDBACK_FORCED_COLOR_BINDINGS = new Map([
    ['cem-dialog[transient] > dialog.cem-dialog:focus-visible', forcedColorFocusBinding()],
    ['cem-dialog-shell[transient] > dialog.cem-dialog-shell:focus-visible', forcedColorFocusBinding()],
]);

const failures = [];

function fail(message) {
    failures.push(message);
}

function colorBinding(backgroundToken, textToken) {
    return new Map([
        ['background-color', `var(${backgroundToken})`],
        ['color', `var(${textToken})`],
    ]);
}

function focusBinding() {
    return new Map([
        ['outline', 'var(--cem-stroke-focus) solid var(--cem-zebra-color-1)'],
        ['outline-offset', 'var(--cem-stroke-indicator-offset)'],
    ]);
}

function forcedColorFocusBinding() {
    return new Map([
        ['outline', 'var(--cem-stroke-focus) solid CanvasText'],
        ['outline-offset', 'var(--cem-stroke-indicator-offset)'],
    ]);
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
    const rules = parseCssRules(pathLabel, cssText);
    const actionRules = new Map();
    const contentInteractionRules = new Map();
    const feedbackRules = new Map();
    const feedbackForcedColorRules = new Map();
    const navigationRules = new Map();
    const privateProperties = new Set(
        rules.flatMap(({ declarations }) => [...declarations.keys()].filter((name) => name.startsWith('--_cem-'))),
    );

    for (const match of cssText.matchAll(CSS_VAR_REFERENCE)) {
        const tokenName = match[1];
        if (tokenName.startsWith('--_cem-')) {
            if (!privateProperties.has(tokenName)) {
                fail(`${pathLabel}: undeclared private component calculation property ${tokenName}`);
            }
        } else if (PUBLIC_COMPONENT_ADAPTERS.has(tokenName)) {
            continue;
        } else if (!tokenName.startsWith('--cem-')) {
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

        if (!rule.media && ACTION_TAGS.has(tag)) {
            if (actionRules.has(rule.selector)) {
                fail(`${pathLabel}: duplicate action selector \`${rule.selector}\``);
            }
            actionRules.set(rule.selector, rule.declarations);
        }
        if (!rule.media && CONTENT_INTERACTION_TAGS.has(tag)) {
            if (contentInteractionRules.has(rule.selector)) {
                fail(`${pathLabel}: duplicate content-interaction selector \`${rule.selector}\``);
            }
            contentInteractionRules.set(rule.selector, rule.declarations);
        }
        if (!rule.media && NAVIGATION_TAGS.has(tag)) {
            if (navigationRules.has(rule.selector)) {
                fail(`${pathLabel}: duplicate navigation selector \`${rule.selector}\``);
            }
            navigationRules.set(rule.selector, rule.declarations);
        }
        if (FEEDBACK_TAGS.has(tag)) {
            const feedbackRuleSet = rule.media ? feedbackForcedColorRules : feedbackRules;
            if (feedbackRuleSet.has(rule.selector)) {
                fail(`${pathLabel}: duplicate feedback selector \`${rule.selector}\``);
            }
            feedbackRuleSet.set(rule.selector, rule.declarations);
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
            fail(
                `${pathLabel}: unexpected action selector \`${selector}\` is outside the accepted action-state contracts`,
            );
        }
    }

    for (const [selector, expectedDeclarations] of CONTENT_INTERACTION_BINDINGS) {
        const declarations = contentInteractionRules.get(selector);
        if (!declarations) {
            fail(`${pathLabel}: missing accepted content-interaction binding selector \`${selector}\``);
            continue;
        }

        if (declarations.size !== expectedDeclarations.size) {
            fail(`${pathLabel}: \`${selector}\` has declarations outside its accepted content-state binding`);
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

    for (const selector of contentInteractionRules.keys()) {
        if (!CONTENT_INTERACTION_BINDINGS.has(selector)) {
            fail(
                `${pathLabel}: unexpected content-interaction selector \`${selector}\` is outside the accepted content-state contract`,
            );
        }
    }

    for (const [selector, expectedDeclarations] of NAVIGATION_BINDINGS) {
        const declarations = navigationRules.get(selector);
        if (!declarations) {
            fail(`${pathLabel}: missing accepted navigation binding selector \`${selector}\``);
            continue;
        }

        if (declarations.size !== expectedDeclarations.size) {
            fail(`${pathLabel}: \`${selector}\` has declarations outside its accepted navigation-state binding`);
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

    for (const selector of navigationRules.keys()) {
        if (!NAVIGATION_BINDINGS.has(selector)) {
            fail(
                `${pathLabel}: unexpected navigation selector \`${selector}\` is outside the accepted navigation-state contract`,
            );
        }
    }

    assertExactStateBindings(pathLabel, 'feedback', feedbackRules, FEEDBACK_BINDINGS);
    assertExactStateBindings(
        pathLabel,
        'forced-colors feedback',
        feedbackForcedColorRules,
        FEEDBACK_FORCED_COLOR_BINDINGS,
    );
}

function assertExactStateBindings(pathLabel, contract, actualRules, expectedRules) {
    for (const [selector, expectedDeclarations] of expectedRules) {
        const declarations = actualRules.get(selector);
        if (!declarations) {
            fail(`${pathLabel}: missing accepted ${contract} binding selector \`${selector}\``);
            continue;
        }

        if (declarations.size !== expectedDeclarations.size) {
            fail(`${pathLabel}: \`${selector}\` has declarations outside its accepted ${contract} binding`);
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

    for (const selector of actualRules.keys()) {
        if (!expectedRules.has(selector)) {
            fail(`${pathLabel}: unexpected ${contract} selector \`${selector}\` is outside the accepted contract`);
        }
    }
}

function parseCssRules(pathLabel, cssText) {
    let root;

    try {
        root = postcss.parse(cssText, { from: pathLabel });
    } catch (error) {
        fail(`${pathLabel}: invalid CSS: ${error instanceof Error ? error.message : String(error)}`);
        return [];
    }

    const rules = [];

    root.walkAtRules((atRule) => {
        if (atRule.name !== 'media' || atRule.params !== '(forced-colors: active)') {
            fail(`${pathLabel}: unsupported component stylesheet at-rule @${atRule.name} ${atRule.params}`);
        }
    });

    root.walkRules((rule) => {
        const declarations = parseDeclarations(pathLabel, rule.selector, rule.nodes);

        for (const selector of postcss.list.comma(rule.selector)) {
            if (!selector) {
                fail(`${pathLabel}: empty component selector`);
                continue;
            }
            rules.push({
                declarations,
                media: rule.parent?.type === 'atrule' ? rule.parent.params : null,
                selector: selector.replace(/\s+/g, ' ').trim(),
            });
        }
    });

    return rules;
}

function parseDeclarations(pathLabel, selectorList, nodes) {
    const declarations = new Map();

    for (const node of nodes ?? []) {
        if (node.type === 'comment') {
            continue;
        }
        if (node.type !== 'decl') {
            fail(`${pathLabel}: unsupported ${node.type} inside \`${selectorList}\``);
            continue;
        }

        const property = node.prop;
        const value = node.value;
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
