import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-hex-grid-demo-document';
const DEMO_URL = new URL('../../demo/hex-grid.html', import.meta.url);
const MODULE_URL_DEMO_URL = new URL('../../demo/module-url.html', import.meta.url).href;
const REACT_LOGO_URL = 'https://upload.wikimedia.org/wikipedia/commons/a/a7/React-icon.svg';
const LEGEND = '1. Responsive framework link honeycomb';
const COMPACT_LEGEND = '2. Compact percentage links';
const FIXED_LEGEND = '3. Fixed-length links';
const ALTERNATE_LEGEND = '4. Alternating backgrounds';
const WRAPPING_LEGEND = '5. Wrapping long label';
const FALLBACK_LEGEND = '6. Missing-image fallback';
const WRAPPER_LEGEND = '7. Wrapper DCE theme';
const IMAGE_BUTTON_LEGEND = '8. Image-button presentation';
const EXPECTED_LABELS = [
    'DCE',
    'React',
    'AngularJS',
    'Semantic UI',
    'Open WC',
    'Flutter',
    'Refine',
    'Bootstrap',
    'Vue.js',
    'Lit',
    'Redux',
    'Svelte',
    'SolidJS',
    'Next.js',
] as const;

const meta: Meta = {
    title: 'CEM Elements/Hex Grid Demo',
    tags: ['test'],
};

export default meta;
type Story = StoryObj;

export const ResponsiveFrameworkLinks: Story = {
    render: () => sourceLoadedDemo(SOURCE_TAG),
    play: async ({ canvasElement }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG);
        await waitForCondition(
            () =>
                host.querySelectorAll(`cem-demo-element[legend="${LEGEND}"] .hex-link`).length === 14 &&
                host.querySelectorAll(`cem-demo-element[legend="${COMPACT_LEGEND}"] .hex-link`).length === 6 &&
                host.querySelectorAll(`cem-demo-element[legend="${FIXED_LEGEND}"] .hex-link`).length === 6,
            'responsive size samples render from the authored HTML document',
            300,
        );

        const sample = requiredElement(host, `cem-demo-element[legend="${LEGEND}"]`);
        const links = Array.from(sample.querySelectorAll<HTMLAnchorElement>('.hex-link'));
        const images = Array.from(sample.querySelectorAll<HTMLImageElement>('.hex-logo'));
        assertDeepEqual(
            links.map((link) => normalize(link.textContent ?? '')),
            [...EXPECTED_LABELS],
            'labels',
        );
        assertEqual(links[0]?.href, MODULE_URL_DEMO_URL, 'relative DCE destination');
        assertEqual(links[1]?.href, 'https://react.dev/', 'React destination');
        assertEqual(links[13]?.href, 'https://nextjs.org/', 'Next.js destination');
        assertEqual(images[1]?.src, REACT_LOGO_URL, 'full React image URL');

        await waitForCondition(
            () =>
                images.length === 14 &&
                images.filter((_, index) => index !== 1).every((image) => image.complete && image.naturalWidth > 0),
            () => `all relative framework logos load; sources=${JSON.stringify(images.map((image) => image.src))}`,
            300,
        );
        assertIncludes(images[0]?.src ?? '', '/demo/framework-logos/wc-square.svg', 'DCE logo URL');

        const helpers = Array.from(sample.querySelectorAll<HTMLElement>('cem-hex-image-link'));
        assertEqual(helpers[0]?.getAttribute('href'), './module-url.html', 'relative href input');
        assertEqual(helpers[0]?.getAttribute('src'), './framework-logos/wc-square.svg', 'relative src input');
        assertEqual(helpers[1]?.getAttribute('href'), 'https://react.dev/', 'full href input');
        assertEqual(helpers[1]?.getAttribute('src'), REACT_LOGO_URL, 'full src input');

        const defaultCell = requiredElement(sample, '.hex');
        assertApproximately(
            helpers[0].getBoundingClientRect().width / defaultCell.getBoundingClientRect().width,
            0.95,
            0.02,
            'default image-link width ratio',
        );

        const compact = requiredElement(host, `cem-demo-element[legend="${COMPACT_LEGEND}"]`);
        const compactCell = requiredElement(compact, '.hex');
        const compactHelper = requiredElement(compact, 'cem-hex-image-link');
        assertApproximately(
            compactHelper.getBoundingClientRect().width / compactCell.getBoundingClientRect().width,
            0.95,
            0.02,
            'compact-grid image-link width ratio',
        );
        const compactGrid = requiredElement(compact, '.hex-grid');
        const compactNav = requiredElement(compact, 'nav');
        assertApproximately(
            compactGrid.getBoundingClientRect().width / compactNav.getBoundingClientRect().width,
            0.65,
            0.02,
            'percentage honeycomb width ratio',
        );
        assertHoneycombGeometry(compact, 'percentage honeycomb');

        const fixed = requiredElement(host, `cem-demo-element[legend="${FIXED_LEGEND}"]`);
        const fixedGridWidth = requiredElement(fixed, '.hex-grid').getBoundingClientRect().width;
        const fixedNavWidth = requiredElement(fixed, 'nav').getBoundingClientRect().width;
        const thirtyFiveRem = 35 * Number.parseFloat(getComputedStyle(document.documentElement).fontSize);
        assertApproximately(
            fixedGridWidth,
            Math.min(thirtyFiveRem, fixedNavWidth),
            1,
            'fixed honeycomb width with available-space cap',
        );
        assertApproximately(
            requiredElement(fixed, 'cem-hex-image-link').getBoundingClientRect().width /
                requiredElement(fixed, '.hex').getBoundingClientRect().width,
            0.95,
            0.02,
            'fixed-grid image-link width ratio',
        );
        assertHoneycombGeometry(fixed, 'fixed-length honeycomb');

        const list = requiredElement(sample, '.hex-grid');
        const firstHex = requiredElement(sample, '.hex');
        const firstLink = links[0];
        assertEqual(getComputedStyle(list).display, 'flex', 'list layout');
        assertEqual(getComputedStyle(firstHex).position, 'relative', 'hex positioning');
        assertIncludes(getComputedStyle(firstLink).clipPath, 'polygon', 'hexagonal clipping');

        firstLink.focus();
        await waitForCondition(() => document.activeElement === firstLink, 'hex link accepts keyboard focus');
        assertEqual(firstLink.getAttribute('aria-label'), 'DCE', 'focused link accessible name');
        const focusRing = getComputedStyle(firstLink, '::after');
        assertEqual(getComputedStyle(firstLink).outlineStyle, 'none', 'rectangular focus outline is disabled');
        assertEqual(focusRing.opacity, '1', 'hexagonal focus ring is visible');
        assertIncludes(focusRing.clipPath, 'evenodd', 'focus ring follows the hexagon');
        assertIncludes(focusRing.backgroundImage, 'repeating-linear-gradient', 'focus ring uses zebra stripes');

        const gridHost = requiredElement(sample, 'cem-hex-grid');
        gridHost.style.inlineSize = '420px';
        gridHost.style.maxInlineSize = '100%';
        await waitForCondition(
            () => rowCounts(sample, '.hex').slice(0, 3).join('|') === '2|1|2',
            () => `narrow component uses 2–1 staggered rows; rows=${JSON.stringify(rowCounts(sample, '.hex'))}`,
        );
        await assertPresentationModes(host);
    },
};

async function assertPresentationModes(host: HTMLElement): Promise<void> {
    await waitForCondition(
        () =>
            host.querySelectorAll(`cem-demo-element[legend="${ALTERNATE_LEGEND}"] .hex-link`).length === 3 &&
            host.querySelectorAll(`cem-demo-element[legend="${WRAPPING_LEGEND}"] .hex-link`).length === 1 &&
            host.querySelectorAll(`cem-demo-element[legend="${FALLBACK_LEGEND}"] .hex-link`).length === 1 &&
            host.querySelectorAll(`cem-demo-element[legend="${WRAPPER_LEGEND}"] .hex-link`).length === 3 &&
            host.querySelectorAll(`cem-demo-element[legend="${IMAGE_BUTTON_LEGEND}"] .hex-link`).length === 1,
        'presentation samples render from the authored HTML document',
        300,
    );

    const sample = requiredElement(host, `cem-demo-element[legend="${LEGEND}"]`);
    const defaultBackgrounds = Array.from(
        sample.querySelectorAll<HTMLElement>('.hex-link'),
        (link) => getComputedStyle(link).backgroundImage,
    );
    assertEqual(defaultBackgrounds[0], defaultBackgrounds[1], 'default background remains uniform');

    const alternate = requiredElement(host, `cem-demo-element[legend="${ALTERNATE_LEGEND}"]`);
    const alternateBackgrounds = Array.from(
        alternate.querySelectorAll<HTMLElement>('.hex-link'),
        (link) => getComputedStyle(link).backgroundImage,
    );
    assertNotEqual(alternateBackgrounds[0], alternateBackgrounds[1], 'alternate=true cycles backgrounds');
    assertNotEqual(alternateBackgrounds[1], alternateBackgrounds[2], 'alternate=true uses a third background');

    const loadedImage = requiredElement(sample, '.hex-logo');
    const loadedFallback = requiredElement(sample, '.image-fallback');
    await waitForCondition(
        () =>
            loadedImage.classList.contains('hex-logo-load') && getComputedStyle(loadedFallback).visibility === 'hidden',
        'loaded image replaces its white fallback',
    );

    const fallback = requiredElement(host, `cem-demo-element[legend="${FALLBACK_LEGEND}"]`);
    const unavailableImage = requiredElement(fallback, '.hex-logo');
    const unavailableFallback = requiredElement(fallback, '.image-fallback');
    await waitForCondition(
        () =>
            unavailableImage.classList.contains('hex-logo-error') &&
            getComputedStyle(unavailableImage).opacity === '0' &&
            getComputedStyle(unavailableFallback).visibility === 'visible',
        'unavailable image retains its white fallback',
    );

    const wrapping = requiredElement(host, `cem-demo-element[legend="${WRAPPING_LEGEND}"]`);
    const wrappingLink = requiredElement(wrapping, '.hex-link');
    const wrappingLabel = requiredElement(wrappingLink, '.hex-label');
    wrappingLink.focus();
    await waitForCondition(
        () => textLineCount(wrappingLabel) > 1 && labelFitsRaisedInsideLink(wrappingLabel, wrappingLink),
        () =>
            `long label wraps inside its link; lines=${textLineCount(wrappingLabel)}, label=${rectSummary(wrappingLabel)}, link=${rectSummary(wrappingLink)}`,
    );

    const wrapper = requiredElement(host, `cem-demo-element[legend="${WRAPPER_LEGEND}"]`);
    const wrapperFrame = requiredElement(wrapper, '.theme-frame');
    const wrapperLink = requiredElement(wrapper, '.hex-link');
    const wrapperLabel = requiredElement(wrapper, '.hex-label');
    assertIncludes(getComputedStyle(wrapperLink).backgroundImage, 'rgb(49, 46, 129)', 'wrapper background start');
    assertEqual(getComputedStyle(wrapperLabel).color, 'rgb(30, 27, 75)', 'wrapper label color');
    assertEqual(getComputedStyle(wrapperFrame).backgroundColor, 'rgb(238, 242, 255)', 'wrapper frame background');
    assertEqual(getComputedStyle(wrapperFrame).borderTopColor, 'rgb(99, 102, 241)', 'wrapper frame border');

    const imageButton = requiredElement(host, `cem-demo-element[legend="${IMAGE_BUTTON_LEGEND}"]`);
    const imageButtonLink = requiredElement(imageButton, '.hex-link');
    const imageButtonLabel = requiredElement(imageButtonLink, '.hex-label');
    const imageButtonLogo = requiredElement(imageButtonLink, '.hex-logo');
    await waitForCondition(
        () =>
            getComputedStyle(imageButtonLogo).opacity === '1' &&
            labelFitsRaisedInsideLink(imageButtonLabel, imageButtonLink),
        'image-button presentation shows its image and label together',
    );
    const restingFilter = getComputedStyle(imageButtonLink).filter;
    assertIncludes(restingFilter, 'drop-shadow', 'image-button resting shadow');
    imageButtonLink.focus();
    await waitForCondition(
        () =>
            getComputedStyle(imageButtonLink).filter.includes('saturate(1.15)') &&
            getComputedStyle(imageButtonLink).filter.includes('brightness(1.04)'),
        'image-button focus strengthens its shadow',
    );
    assertIncludes(getComputedStyle(imageButtonLink).filter, 'drop-shadow', 'image-button focus shadow');
}

function sourceLoadedDemo(sourceTag: string): HTMLElement {
    const root = document.createElement('section');
    root.setAttribute('aria-label', 'source-loaded hex-grid demo coverage');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', sourceTag);
    declaration.setAttribute('src', DEMO_URL.href);
    root.append(declaration, document.createElement(sourceTag));
    return root;
}


function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

async function waitForCondition(
    condition: () => boolean,
    message: string | (() => string),
    attempts = 200,
): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(typeof message === 'string' ? message : message());
}

function assertDeepEqual(actual: readonly string[], expected: readonly string[], label: string): void {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
    if (actual !== expected) throw new Error(`${label}: expected ${String(expected)}, received ${String(actual)}`);
}

function assertNotEqual(actual: unknown, expected: unknown, label: string): void {
    if (actual === expected) throw new Error(`${label}: expected distinct values, received ${String(actual)}`);
}

function assertIncludes(actual: string, expected: string, label: string): void {
    if (!actual.includes(expected)) throw new Error(`${label}: expected ${actual} to include ${expected}`);
}

function assertApproximately(actual: number, expected: number, tolerance: number, label: string): void {
    if (Math.abs(actual - expected) > tolerance) {
        throw new Error(`${label}: expected ${expected} ± ${tolerance}, received ${actual}`);
    }
}

function rowCounts(root: ParentNode, selector: string): number[] {
    const rows = new Map<number, number>();
    for (const element of root.querySelectorAll<HTMLElement>(selector)) {
        const top = Math.round(element.getBoundingClientRect().top);
        rows.set(top, (rows.get(top) ?? 0) + 1);
    }
    return [...rows.values()];
}

function labelFitsRaisedInsideLink(label: HTMLElement, link: HTMLElement): boolean {
    const labelRect = label.getBoundingClientRect();
    const linkRect = link.getBoundingClientRect();
    return (
        label.scrollWidth <= label.clientWidth + 1 &&
        labelRect.left >= linkRect.left &&
        labelRect.right <= linkRect.right &&
        labelRect.bottom <= linkRect.bottom - linkRect.height * 0.1
    );
}

function rectSummary(element: Element): string {
    const { left, right, top, bottom, width, height } = element.getBoundingClientRect();
    return JSON.stringify({ left, right, top, bottom, width, height });
}

function textLineCount(element: Element): number {
    const range = document.createRange();
    range.selectNodeContents(element);
    return range.getClientRects().length;
}

function assertHoneycombGeometry(root: ParentNode, label: string): void {
    const cells = Array.from(root.querySelectorAll<HTMLElement>('.hex'));
    const rows = new Map<number, DOMRect[]>();
    for (const cell of cells) {
        const rect = cell.getBoundingClientRect();
        const top = Math.round(rect.top);
        rows.set(top, [...(rows.get(top) ?? []), rect]);
    }
    const orderedRows = [...rows.values()];
    if (orderedRows.length < 2) throw new Error(`${label}: expected staggered rows`);
    const firstRow = orderedRows[0];
    const secondRow = orderedRows[1];
    for (let index = 1; index < firstRow.length; index += 1) {
        assertApproximately(firstRow[index].left, firstRow[index - 1].right, 1, `${label} adjacent cells`);
    }
    if (secondRow[0].left <= firstRow[0].left + firstRow[0].width * 0.4) {
        throw new Error(`${label}: second row is not staggered by roughly half a cell`);
    }
    cells.forEach((cell, index) => {
        const cellRect = cell.getBoundingClientRect();
        const linkRect = requiredElement(cell, 'cem-hex-image-link').getBoundingClientRect();
        if (
            linkRect.left < cellRect.left - 1 ||
            linkRect.right > cellRect.right + 1 ||
            linkRect.top < cellRect.top - 1 ||
            linkRect.bottom > cellRect.bottom + 1
        ) {
            throw new Error(`${label}: link ${index + 1} exceeds its cell`);
        }
    });
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
