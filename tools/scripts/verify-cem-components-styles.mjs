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
    coupling: ['--cem-coupling-'],
    gap: ['--cem-gap-'],
    inset: ['--cem-inset-'],
    layering: ['--cem-layer-', '--cem-elevation-'],
    navigation: ['--cem-navigation-item-'],
    palette: ['--cem-palette-'],
    progress: ['--cem-progress-'],
    responsive: ['--cem-bp-', '--cem-cq-'],
    select: ['--cem-select-'],
    separator: ['--cem-separator-'],
    slider: ['--cem-slider-'],
    stroke: ['--cem-stroke-'],
    timing: ['--cem-duration-', '--cem-easing-'],
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
const DIVIDER_TAGS = new Set(['cem-divider']);
const EXPANSION_TAGS = new Set(['cem-expansion']);
const PROGRESS_SPINNER_TAGS = new Set(['cem-progress-spinner']);
const SORT_HEADER_TAGS = new Set(['cem-sort-header']);
const PAGINATOR_TAGS = new Set(['cem-paginator']);
const TOOLTIP_TAGS = new Set(['cem-tooltip']);
const CHOICE_POPUP_STACKING_SELECTORS = new Set([
    'cem-autocomplete .cem-autocomplete__popup',
    'cem-select .cem-select__popup',
]);
const CHOICE_POPUP_Z_INDEX_PROPERTY = '--_cem-choice-popup-z-index';
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
const DIVIDER_MARGIN_VALUE =
    'calc(\n        (max(var(--_cem-divider-space), var(--cem-coupling-guard-min)) - var(--cem-stroke-divider)) / 2\n    )';
const DIVIDER_BINDINGS = new Map([
    [
        'cem-divider',
        new Map([
            ['--_cem-divider-space', 'var(--cem-gap-group)'],
            ['display', 'block'],
            ['margin-block', DIVIDER_MARGIN_VALUE],
        ]),
    ],
    ["cem-divider[spacing='related']", new Map([['--_cem-divider-space', 'var(--cem-gap-related)']])],
    ["cem-divider[spacing='block']", new Map([['--_cem-divider-space', 'var(--cem-gap-block)']])],
    ["cem-divider[spacing='section']", new Map([['--_cem-divider-space', 'var(--cem-gap-section)']])],
    [
        "cem-divider[orientation='vertical']",
        new Map([
            ['align-self', 'stretch'],
            ['display', 'inline-flex'],
            ['margin-block', 'var(--cem-stroke-none)'],
            ['margin-inline', DIVIDER_MARGIN_VALUE],
        ]),
    ],
    [
        'cem-divider > .cem-divider',
        new Map([
            ['border-color', 'var(--cem-separator-color)'],
            ['border-style', 'solid'],
            ['border-width', 'var(--cem-stroke-none)'],
            ['box-sizing', 'border-box'],
            ['margin', 'var(--cem-stroke-none)'],
        ]),
    ],
    [
        "cem-divider > .cem-divider[data-orientation='horizontal']",
        new Map([
            ['block-size', 'var(--cem-stroke-none)'],
            ['border-block-start-width', 'var(--cem-stroke-divider)'],
        ]),
    ],
    [
        "cem-divider > .cem-divider[data-orientation='vertical']",
        new Map([
            ['align-self', 'stretch'],
            ['border-inline-start-width', 'var(--cem-stroke-divider)'],
            ['inline-size', 'var(--cem-stroke-none)'],
        ]),
    ],
    [
        "cem-divider[inset] > .cem-divider[data-orientation='horizontal']",
        new Map([['margin-inline-start', 'var(--cem-inset-container)']]),
    ],
    [
        "cem-divider[inset] > .cem-divider[data-orientation='vertical']",
        new Map([['margin-block-start', 'var(--cem-inset-container)']]),
    ],
]);
const EXPANSION_HEADER_SELECTOR =
    'cem-expansion > .cem-expansion > .cem-expansion__heading > .cem-expansion__header';
const EXPANSION_BINDINGS = new Map([
    ['cem-expansion', new Map([['display', 'block']])],
    [
        'cem-expansion > .cem-expansion',
        colorBinding('--cem-palette-comfort', '--cem-palette-comfort-text'),
    ],
    [
        EXPANSION_HEADER_SELECTOR,
        new Map([
            ['align-items', 'center'],
            ['appearance', 'none'],
            ['background-color', 'var(--cem-action-contextual-default-background)'],
            ['border', 'var(--cem-stroke-none) solid transparent'],
            ['border-radius', 'var(--cem-bend-control)'],
            ['box-sizing', 'border-box'],
            ['color', 'var(--cem-action-contextual-default-text)'],
            ['display', 'flex'],
            ['font-family', 'var(--cem-typography-ui-font-family)'],
            ['font-size', 'var(--cem-typography-ui-font-size)'],
            ['font-weight', 'var(--cem-typography-ui-font-weight)'],
            ['gap', 'var(--cem-gap-related)'],
            ['letter-spacing', 'var(--cem-typography-ui-letter-spacing)'],
            ['line-height', 'var(--cem-typography-ui-line-height)'],
            ['margin', 'var(--cem-stroke-none)'],
            ['min-block-size', 'var(--cem-coupling-zone-min)'],
            ['padding-block', 'var(--cem-control-padding-y)'],
            ['padding-inline', 'var(--cem-control-padding-x)'],
            ['text-align', 'start'],
        ]),
    ],
    [
        `${EXPANSION_HEADER_SELECTOR}:enabled:hover`,
        colorBinding('--cem-action-contextual-hover-background', '--cem-action-contextual-hover-text'),
    ],
    [
        `${EXPANSION_HEADER_SELECTOR}:enabled:active`,
        colorBinding('--cem-action-contextual-active-background', '--cem-action-contextual-active-text'),
    ],
    [
        `${EXPANSION_HEADER_SELECTOR}:disabled`,
        colorBinding('--cem-action-contextual-disabled-background', '--cem-action-contextual-disabled-text'),
    ],
    [`${EXPANSION_HEADER_SELECTOR}:enabled:focus-visible`, focusBinding()],
    ['cem-expansion .cem-expansion__summary', new Map([['flex', '1 1 auto']])],
    [
        'cem-expansion .cem-expansion__indicator',
        new Map([
            ['align-items', 'center'],
            ['block-size', 'var(--cem-icon-button-icon-size)'],
            ['display', 'inline-flex'],
            ['flex', '0 0 var(--cem-icon-button-icon-size)'],
            ['inline-size', 'var(--cem-icon-button-icon-size)'],
            ['justify-content', 'center'],
        ]),
    ],
    [
        'cem-expansion > .cem-expansion > .cem-expansion__panel',
        new Map([
            ['background-color', 'var(--cem-palette-comfort)'],
            ['border-radius', 'var(--cem-bend-surface)'],
            ['color', 'var(--cem-palette-comfort-text)'],
            ['margin-block-start', 'var(--cem-gap-related)'],
            ['padding', 'var(--cem-inset-container)'],
        ]),
    ],
]);
const EXPANSION_FORCED_COLOR_BINDINGS = new Map([
    ...[
        'cem-expansion > .cem-expansion',
        EXPANSION_HEADER_SELECTOR,
        'cem-expansion > .cem-expansion > .cem-expansion__panel',
    ].map((selector) => [selector, new Map([['background-color', 'Canvas'], ['color', 'CanvasText']])]),
    ...[
        `${EXPANSION_HEADER_SELECTOR}:enabled:hover`,
        `${EXPANSION_HEADER_SELECTOR}:enabled:active`,
    ].map((selector) => [selector, new Map([['background-color', 'Highlight'], ['color', 'HighlightText']])]),
    [
        `${EXPANSION_HEADER_SELECTOR}:disabled`,
        new Map([['background-color', 'Canvas'], ['color', 'GrayText']]),
    ],
    [`${EXPANSION_HEADER_SELECTOR}:enabled:focus-visible`, forcedColorFocusBinding()],
]);
const SORT_HEADER_BUTTON_SELECTOR = 'cem-sort-header > .cem-sort-header > .cem-sort-header__button';
const SORT_HEADER_BINDINGS = new Map([
    ['cem-sort-header', new Map([['display', 'block']])],
    [
        'cem-sort-header > .cem-sort-header',
        new Map([
            ['display', 'flex'],
            ['inline-size', '100%'],
        ]),
    ],
    [
        SORT_HEADER_BUTTON_SELECTOR,
        new Map([
            ['align-items', 'center'],
            ['appearance', 'none'],
            ['background-color', 'var(--cem-action-contextual-default-background)'],
            ['border', 'var(--cem-stroke-none) solid transparent'],
            ['border-radius', 'var(--cem-bend-control)'],
            ['box-sizing', 'border-box'],
            ['color', 'var(--cem-action-contextual-default-text)'],
            ['display', 'flex'],
            ['font-family', 'var(--cem-typography-ui-font-family)'],
            ['font-size', 'var(--cem-typography-ui-font-size)'],
            ['font-weight', 'var(--cem-typography-ui-font-weight)'],
            ['gap', 'var(--cem-gap-related)'],
            ['inline-size', '100%'],
            ['letter-spacing', 'var(--cem-typography-ui-letter-spacing)'],
            ['line-height', 'var(--cem-typography-ui-line-height)'],
            ['margin', 'var(--cem-stroke-none)'],
            ['min-block-size', 'max(var(--cem-table-row-height), var(--cem-coupling-zone-min))'],
            ['padding-block', 'var(--cem-control-padding-y)'],
            ['padding-inline', 'var(--cem-control-padding-x)'],
            ['text-align', 'start'],
        ]),
    ],
    [
        `${SORT_HEADER_BUTTON_SELECTOR}:enabled:hover`,
        colorBinding('--cem-action-contextual-hover-background', '--cem-action-contextual-hover-text'),
    ],
    [
        `${SORT_HEADER_BUTTON_SELECTOR}:enabled:active`,
        colorBinding('--cem-action-contextual-active-background', '--cem-action-contextual-active-text'),
    ],
    [
        `${SORT_HEADER_BUTTON_SELECTOR}:disabled`,
        colorBinding('--cem-action-contextual-disabled-background', '--cem-action-contextual-disabled-text'),
    ],
    [`${SORT_HEADER_BUTTON_SELECTOR}:enabled:focus-visible`, focusBinding()],
    ['cem-sort-header .cem-sort-header__label', new Map([['flex', '1 1 auto']])],
    [
        'cem-sort-header .cem-sort-header__indicator',
        new Map([
            ['align-items', 'center'],
            ['block-size', 'var(--cem-icon-button-icon-size)'],
            ['display', 'inline-flex'],
            ['flex', '0 0 var(--cem-icon-button-icon-size)'],
            ['inline-size', 'var(--cem-icon-button-icon-size)'],
            ['justify-content', 'center'],
        ]),
    ],
]);
const SORT_HEADER_FORCED_COLOR_BINDINGS = new Map([
    [SORT_HEADER_BUTTON_SELECTOR, new Map([['background-color', 'Canvas'], ['color', 'CanvasText']])],
    ...[
        `${SORT_HEADER_BUTTON_SELECTOR}:enabled:hover`,
        `${SORT_HEADER_BUTTON_SELECTOR}:enabled:active`,
    ].map((selector) => [selector, new Map([['background-color', 'Highlight'], ['color', 'HighlightText']])]),
    [
        `${SORT_HEADER_BUTTON_SELECTOR}:disabled`,
        new Map([['background-color', 'Canvas'], ['color', 'GrayText']]),
    ],
    [`${SORT_HEADER_BUTTON_SELECTOR}:enabled:focus-visible`, forcedColorFocusBinding()],
]);
const PAGINATOR_ACTION_SELECTOR = 'cem-paginator .cem-paginator__action';
const PAGINATOR_SELECT_SELECTOR = 'cem-paginator .cem-paginator__page-size-control';
const PAGINATOR_BINDINGS = new Map([
    ['cem-paginator', new Map([['display', 'block']])],
    [
        'cem-paginator > .cem-paginator',
        new Map([
            ['align-items', 'center'],
            ['background-color', 'var(--cem-palette-comfort)'],
            ['color', 'var(--cem-palette-comfort-text)'],
            ['display', 'flex'],
            ['flex-wrap', 'wrap'],
            ['font-family', 'var(--cem-typography-ui-font-family)'],
            ['font-size', 'var(--cem-typography-ui-font-size)'],
            ['font-weight', 'var(--cem-typography-ui-font-weight)'],
            ['gap', 'var(--cem-gap-group)'],
            ['justify-content', 'flex-end'],
            ['letter-spacing', 'var(--cem-typography-ui-letter-spacing)'],
            ['line-height', 'var(--cem-typography-ui-line-height)'],
            ['padding', 'var(--cem-inset-container)'],
        ]),
    ],
    ...[
        'cem-paginator .cem-paginator__page-size',
        'cem-paginator .cem-paginator__range-actions',
    ].map((selector) => [
        selector,
        new Map([
            ['align-items', 'center'],
            ['display', 'flex'],
            ['gap', 'var(--cem-gap-related)'],
        ]),
    ]),
    [
        PAGINATOR_SELECT_SELECTOR,
        new Map([
            ['background-color', 'var(--cem-select-popup-background)'],
            ['border', 'var(--cem-stroke-boundary) solid var(--cem-input-indicator-anchor-color)'],
            ['border-radius', 'var(--cem-bend-field)'],
            ['box-sizing', 'border-box'],
            ['color', 'var(--cem-select-popup-text)'],
            ['font-family', 'var(--cem-typography-ui-font-family)'],
            ['font-size', 'var(--cem-typography-ui-font-size)'],
            ['font-weight', 'var(--cem-typography-ui-font-weight)'],
            ['letter-spacing', 'var(--cem-typography-ui-letter-spacing)'],
            ['line-height', 'var(--cem-typography-ui-line-height)'],
            ['min-block-size', 'var(--cem-control-height)'],
            ['padding-block', 'var(--cem-control-padding-y)'],
            ['padding-inline', 'var(--cem-control-padding-x)'],
        ]),
    ],
    [
        `${PAGINATOR_SELECT_SELECTOR}:enabled:hover`,
        new Map([['border-color', 'var(--cem-input-indicator-anchor-hover-color)']]),
    ],
    [
        `${PAGINATOR_SELECT_SELECTOR}:disabled`,
        new Map([
            ['background-color', 'var(--cem-select-option-disabled-background)'],
            ['border-color', 'var(--cem-input-indicator-anchor-disabled-color)'],
            ['color', 'var(--cem-select-option-disabled-text)'],
        ]),
    ],
    [`${PAGINATOR_SELECT_SELECTOR}:focus-visible`, focusBinding()],
    [
        'cem-paginator .cem-paginator__range',
        new Map([
            ['font-family', 'var(--cem-typography-data-font-family)'],
            ['font-size', 'var(--cem-typography-data-font-size)'],
            ['font-variant-numeric', 'var(--cem-typography-data-font-variant-numeric)'],
            ['font-weight', 'var(--cem-typography-data-font-weight)'],
            ['letter-spacing', 'var(--cem-typography-data-letter-spacing)'],
            ['line-height', 'var(--cem-typography-data-line-height)'],
        ]),
    ],
    [
        PAGINATOR_ACTION_SELECTOR,
        new Map([
            ['align-items', 'center'],
            ['appearance', 'none'],
            ['background-color', 'var(--cem-action-contextual-default-background)'],
            ['block-size', 'var(--cem-icon-button-size)'],
            ['border', 'var(--cem-stroke-none) solid transparent'],
            ['border-radius', 'var(--cem-bend-control)'],
            ['box-sizing', 'border-box'],
            ['color', 'var(--cem-action-contextual-default-text)'],
            ['display', 'inline-flex'],
            ['flex', '0 0 var(--cem-icon-button-size)'],
            ['inline-size', 'var(--cem-icon-button-size)'],
            ['justify-content', 'center'],
            ['margin', 'var(--cem-stroke-none)'],
            ['padding', 'var(--cem-stroke-none)'],
        ]),
    ],
    [
        `${PAGINATOR_ACTION_SELECTOR}:enabled:not([aria-disabled='true']):hover`,
        colorBinding('--cem-action-contextual-hover-background', '--cem-action-contextual-hover-text'),
    ],
    [
        `${PAGINATOR_ACTION_SELECTOR}:enabled:not([aria-disabled='true']):active`,
        colorBinding('--cem-action-contextual-active-background', '--cem-action-contextual-active-text'),
    ],
    [
        `${PAGINATOR_ACTION_SELECTOR}:is(:disabled, [aria-disabled='true'])`,
        colorBinding('--cem-action-contextual-disabled-background', '--cem-action-contextual-disabled-text'),
    ],
    [`${PAGINATOR_ACTION_SELECTOR}:focus-visible`, focusBinding()],
    [
        'cem-paginator .cem-paginator__icon',
        new Map([
            ['align-items', 'center'],
            ['block-size', 'var(--cem-icon-button-icon-size)'],
            ['display', 'inline-flex'],
            ['flex', '0 0 var(--cem-icon-button-icon-size)'],
            ['inline-size', 'var(--cem-icon-button-icon-size)'],
            ['justify-content', 'center'],
        ]),
    ],
]);
const PAGINATOR_FORCED_COLOR_BINDINGS = new Map([
    [
        'cem-paginator > .cem-paginator',
        new Map([['background-color', 'Canvas'], ['color', 'CanvasText']]),
    ],
    [
        PAGINATOR_SELECT_SELECTOR,
        new Map([
            ['background-color', 'Canvas'],
            ['border-color', 'CanvasText'],
            ['color', 'CanvasText'],
        ]),
    ],
    [`${PAGINATOR_SELECT_SELECTOR}:enabled:hover`, new Map([['border-color', 'Highlight']])],
    [
        `${PAGINATOR_SELECT_SELECTOR}:disabled`,
        new Map([
            ['background-color', 'Canvas'],
            ['border-color', 'GrayText'],
            ['color', 'GrayText'],
        ]),
    ],
    [PAGINATOR_ACTION_SELECTOR, new Map([['background-color', 'Canvas'], ['color', 'CanvasText']])],
    ...[
        `${PAGINATOR_ACTION_SELECTOR}:enabled:not([aria-disabled='true']):hover`,
        `${PAGINATOR_ACTION_SELECTOR}:enabled:not([aria-disabled='true']):active`,
    ].map((selector) => [
        selector,
        new Map([['background-color', 'Highlight'], ['color', 'HighlightText']]),
    ]),
    [
        `${PAGINATOR_ACTION_SELECTOR}:is(:disabled, [aria-disabled='true'])`,
        new Map([['background-color', 'Canvas'], ['color', 'GrayText']]),
    ],
    [`${PAGINATOR_SELECT_SELECTOR}:focus-visible`, forcedColorFocusBinding()],
    [`${PAGINATOR_ACTION_SELECTOR}:focus-visible`, forcedColorFocusBinding()],
]);
const TOOLTIP_BINDINGS = new Map([
    [
        'cem-tooltip',
        new Map([
            ['anchor-scope', '--_cem-tooltip-anchor'],
            ['display', 'inline-block'],
        ]),
    ],
    ['cem-tooltip > .cem-tooltip', new Map([['display', 'contents']])],
    [
        "cem-tooltip > .cem-tooltip > [slot='trigger']",
        new Map([['anchor-name', '--_cem-tooltip-anchor']]),
    ],
    [
        'cem-tooltip .cem-tooltip__description',
        new Map([
            ['block-size', 'var(--cem-stroke-standard)'],
            ['clip', 'rect(0 0 0 0)'],
            ['clip-path', 'inset(50%)'],
            ['inline-size', 'var(--cem-stroke-standard)'],
            ['overflow', 'hidden'],
            ['position', 'absolute'],
            ['white-space', 'nowrap'],
        ]),
    ],
    [
        'cem-tooltip .cem-tooltip__surface',
        new Map([
            ['background-color', 'var(--cem-palette-comfort-x)'],
            ['border', 'var(--cem-stroke-boundary) solid var(--cem-palette-comfort-text-x)'],
            ['border-radius', 'var(--cem-bend-overlay)'],
            ['box-shadow', 'var(--cem-elevation-3)'],
            ['box-sizing', 'border-box'],
            ['color', 'var(--cem-palette-comfort-text-x)'],
            ['font-family', 'var(--cem-typography-ui-font-family)'],
            ['font-size', 'var(--cem-typography-ui-font-size)'],
            ['font-weight', 'var(--cem-typography-ui-font-weight)'],
            ['inset', 'auto'],
            ['letter-spacing', 'var(--cem-typography-ui-letter-spacing)'],
            ['line-height', 'var(--cem-typography-ui-line-height)'],
            ['margin', 'var(--cem-gap-related)'],
            ['max-inline-size', 'var(--cem-typography-reading-measure-max)'],
            ['padding', 'var(--cem-inset-control)'],
            ['position', 'fixed'],
            ['position-anchor', '--_cem-tooltip-anchor'],
            ['position-area', 'block-end center'],
            ['position-try-fallbacks', 'block-start'],
            ['position-try-order', 'most-height'],
            ['white-space', 'normal'],
        ]),
    ],
    [
        "cem-tooltip > .cem-tooltip[data-position='above'] > .cem-tooltip__surface",
        new Map([
            ['position-area', 'block-start center'],
            ['position-try-fallbacks', 'block-end'],
        ]),
    ],
    [
        "cem-tooltip > .cem-tooltip[data-position='before'] > .cem-tooltip__surface",
        new Map([
            ['position-area', 'inline-start center'],
            ['position-try-fallbacks', 'inline-end'],
            ['position-try-order', 'most-width'],
        ]),
    ],
    [
        "cem-tooltip > .cem-tooltip[data-position='after'] > .cem-tooltip__surface",
        new Map([
            ['position-area', 'inline-end center'],
            ['position-try-fallbacks', 'inline-start'],
            ['position-try-order', 'most-width'],
        ]),
    ],
]);
const TOOLTIP_FORCED_COLOR_BINDINGS = new Map([
    [
        'cem-tooltip .cem-tooltip__surface',
        new Map([
            ['background-color', 'Canvas'],
            ['border-color', 'CanvasText'],
            ['color', 'CanvasText'],
            ['forced-color-adjust', 'auto'],
        ]),
    ],
]);
const PROGRESS_SPINNER_BINDINGS = new Map([
    ['cem-progress-spinner', new Map([['display', 'inline-block']])],
    [
        'cem-progress-spinner > .cem-progress-spinner',
        new Map([
            ['block-size', 'var(--cem-progress-spinner-size)'],
            ['display', 'inline-block'],
            ['inline-size', 'var(--cem-progress-spinner-size)'],
        ]),
    ],
    [
        'cem-progress-spinner .cem-progress-spinner__svg',
        new Map([
            ['block-size', '100%'],
            ['display', 'block'],
            ['inline-size', '100%'],
            ['overflow', 'visible'],
            ['transform', 'rotate(-90deg)'],
        ]),
    ],
    [
        'cem-progress-spinner .cem-progress-spinner__track',
        new Map([
            ['fill', 'none'],
            ['stroke-width', 'var(--cem-progress-track-thickness)'],
            ['vector-effect', 'non-scaling-stroke'],
            ['stroke', 'var(--cem-progress-track-color)'],
        ]),
    ],
    [
        'cem-progress-spinner .cem-progress-spinner__indicator',
        new Map([
            ['fill', 'none'],
            ['stroke-width', 'var(--cem-progress-track-thickness)'],
            ['vector-effect', 'non-scaling-stroke'],
            ['stroke', 'var(--cem-progress-indicator-color)'],
            ['stroke-linecap', 'round'],
            ['transform-box', 'fill-box'],
            ['transform-origin', 'center'],
        ]),
    ],
    [
        "cem-progress-spinner > .cem-progress-spinner[data-mode='indeterminate'] .cem-progress-spinner__indicator",
        new Map([
            [
                'animation',
                'cem-progress-spinner-cycle var(--cem-duration-continuous-cycle) var(--cem-easing-uniform) infinite',
            ],
        ]),
    ],
]);
const PROGRESS_SPINNER_REDUCED_MOTION_BINDINGS = new Map([
    [
        "cem-progress-spinner > .cem-progress-spinner[data-mode='indeterminate'] .cem-progress-spinner__indicator",
        new Map([['animation', 'none']]),
    ],
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
    const dividerRules = new Map();
    const expansionRules = new Map();
    const expansionForcedColorRules = new Map();
    const progressSpinnerRules = new Map();
    const progressSpinnerReducedMotionRules = new Map();
    const progressSpinnerForcedColorRules = new Map();
    const sortHeaderRules = new Map();
    const sortHeaderForcedColorRules = new Map();
    const paginatorRules = new Map();
    const paginatorForcedColorRules = new Map();
    const tooltipRules = new Map();
    const tooltipForcedColorRules = new Map();
    const choicePopupStackingRules = new Map();
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
        if (!rule.media && DIVIDER_TAGS.has(tag)) {
            if (dividerRules.has(rule.selector)) {
                fail(`${pathLabel}: duplicate divider selector \`${rule.selector}\``);
            }
            dividerRules.set(rule.selector, rule.declarations);
        }
        if (EXPANSION_TAGS.has(tag)) {
            const expansionRuleSet = rule.media ? expansionForcedColorRules : expansionRules;
            if (expansionRuleSet.has(rule.selector)) {
                fail(`${pathLabel}: duplicate expansion selector \`${rule.selector}\``);
            }
            expansionRuleSet.set(rule.selector, rule.declarations);
        }
        if (PROGRESS_SPINNER_TAGS.has(tag)) {
            const progressRuleSet =
                rule.media === '(prefers-reduced-motion: reduce)'
                    ? progressSpinnerReducedMotionRules
                    : rule.media === '(forced-colors: active)'
                      ? progressSpinnerForcedColorRules
                      : progressSpinnerRules;
            if (progressRuleSet.has(rule.selector)) {
                fail(`${pathLabel}: duplicate progress-spinner selector \`${rule.selector}\``);
            }
            progressRuleSet.set(rule.selector, rule.declarations);
        }
        if (SORT_HEADER_TAGS.has(tag)) {
            const sortHeaderRuleSet = rule.media ? sortHeaderForcedColorRules : sortHeaderRules;
            if (sortHeaderRuleSet.has(rule.selector)) {
                fail(`${pathLabel}: duplicate sort-header selector \`${rule.selector}\``);
            }
            sortHeaderRuleSet.set(rule.selector, rule.declarations);
        }
        if (PAGINATOR_TAGS.has(tag)) {
            const paginatorRuleSet = rule.media ? paginatorForcedColorRules : paginatorRules;
            if (paginatorRuleSet.has(rule.selector)) {
                fail(`${pathLabel}: duplicate paginator selector \`${rule.selector}\``);
            }
            paginatorRuleSet.set(rule.selector, rule.declarations);
        }
        if (TOOLTIP_TAGS.has(tag)) {
            const tooltipRuleSet = rule.media ? tooltipForcedColorRules : tooltipRules;
            if (tooltipRuleSet.has(rule.selector)) {
                fail(`${pathLabel}: duplicate tooltip selector \`${rule.selector}\``);
            }
            tooltipRuleSet.set(rule.selector, rule.declarations);
        }
        if (FEEDBACK_TAGS.has(tag)) {
            const feedbackRuleSet = rule.media ? feedbackForcedColorRules : feedbackRules;
            if (feedbackRuleSet.has(rule.selector)) {
                fail(`${pathLabel}: duplicate feedback selector \`${rule.selector}\``);
            }
            feedbackRuleSet.set(rule.selector, rule.declarations);
        }
        if (rule.declarations.has('z-index')) {
            if (rule.media || !CHOICE_POPUP_STACKING_SELECTORS.has(rule.selector)) {
                fail(`${pathLabel}: z-index is allowed only on accepted choice-popup selectors`);
            } else {
                choicePopupStackingRules.set(rule.selector, rule.declarations);
            }
        }
    }

    for (const selector of CHOICE_POPUP_STACKING_SELECTORS) {
        const declarations = choicePopupStackingRules.get(selector);
        if (!declarations) {
            fail(`${pathLabel}: missing CEM-CSS-002 physical stacking binding on \`${selector}\``);
            continue;
        }
        if (declarations.get(CHOICE_POPUP_Z_INDEX_PROPERTY) !== '1') {
            fail(`${pathLabel}: \`${selector}\` must declare ${CHOICE_POPUP_Z_INDEX_PROPERTY}: 1`);
        }
        if (declarations.get('z-index') !== `var(${CHOICE_POPUP_Z_INDEX_PROPERTY})`) {
            fail(`${pathLabel}: \`${selector}\` must consume the private CEM-CSS-002 z-index adapter`);
        }
        if (declarations.get('box-shadow') !== 'var(--cem-elevation-3)') {
            fail(`${pathLabel}: \`${selector}\` must retain D4 overlay elevation independently of physical stacking`);
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

    assertExactStateBindings(pathLabel, 'divider', dividerRules, DIVIDER_BINDINGS);

    assertExactStateBindings(pathLabel, 'expansion', expansionRules, EXPANSION_BINDINGS);
    assertExactStateBindings(
        pathLabel,
        'forced-colors expansion',
        expansionForcedColorRules,
        EXPANSION_FORCED_COLOR_BINDINGS,
    );

    assertExactStateBindings(pathLabel, 'progress-spinner', progressSpinnerRules, PROGRESS_SPINNER_BINDINGS);
    assertExactStateBindings(
        pathLabel,
        'reduced-motion progress-spinner',
        progressSpinnerReducedMotionRules,
        PROGRESS_SPINNER_REDUCED_MOTION_BINDINGS,
    );
    assertExactStateBindings(
        pathLabel,
        'forced-colors progress-spinner',
        progressSpinnerForcedColorRules,
        new Map(),
    );
    assertProgressSpinnerKeyframes(pathLabel, cssText);

    assertExactStateBindings(pathLabel, 'sort-header', sortHeaderRules, SORT_HEADER_BINDINGS);
    assertExactStateBindings(
        pathLabel,
        'forced-colors sort-header',
        sortHeaderForcedColorRules,
        SORT_HEADER_FORCED_COLOR_BINDINGS,
    );

    assertExactStateBindings(pathLabel, 'paginator', paginatorRules, PAGINATOR_BINDINGS);
    assertExactStateBindings(
        pathLabel,
        'forced-colors paginator',
        paginatorForcedColorRules,
        PAGINATOR_FORCED_COLOR_BINDINGS,
    );

    assertExactStateBindings(pathLabel, 'tooltip', tooltipRules, TOOLTIP_BINDINGS);
    assertExactStateBindings(
        pathLabel,
        'forced-colors tooltip',
        tooltipForcedColorRules,
        TOOLTIP_FORCED_COLOR_BINDINGS,
    );

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
        if (
            atRule.name === 'media'
            && !['(forced-colors: active)', '(prefers-reduced-motion: reduce)'].includes(atRule.params)
        ) {
            fail(`${pathLabel}: unsupported component stylesheet at-rule @${atRule.name} ${atRule.params}`);
        } else if (atRule.name === 'keyframes' && atRule.params !== 'cem-progress-spinner-cycle') {
            fail(`${pathLabel}: unsupported component stylesheet keyframes @${atRule.name} ${atRule.params}`);
        } else if (!['media', 'keyframes'].includes(atRule.name)) {
            fail(`${pathLabel}: unsupported component stylesheet at-rule @${atRule.name} ${atRule.params}`);
        }
    });

    root.walkRules((rule) => {
        if (rule.parent?.type === 'atrule' && rule.parent.name === 'keyframes') return;
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

function assertProgressSpinnerKeyframes(pathLabel, cssText) {
    let root;
    try {
        root = postcss.parse(cssText, { from: pathLabel });
    } catch {
        return;
    }
    const keyframes = root.nodes.filter(
        (node) => node.type === 'atrule' && node.name === 'keyframes' && node.params === 'cem-progress-spinner-cycle',
    );
    if (keyframes.length !== 1) {
        fail(`${pathLabel}: must contain exactly one @keyframes cem-progress-spinner-cycle block`);
        return;
    }
    const frameRules = keyframes[0].nodes?.filter((node) => node.type === 'rule') ?? [];
    if (frameRules.length !== 1 || frameRules[0].selector !== 'to') {
        fail(`${pathLabel}: progress-spinner keyframes must contain only a \`to\` frame`);
        return;
    }
    const declarations = parseDeclarations(pathLabel, '@keyframes cem-progress-spinner-cycle to', frameRules[0].nodes);
    if (declarations.size !== 1 || declarations.get('transform') !== 'rotate(360deg)') {
        fail(`${pathLabel}: progress-spinner cycle must only transform to rotate(360deg)`);
    }
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
