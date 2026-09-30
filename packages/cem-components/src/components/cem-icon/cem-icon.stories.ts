import { expect, within, waitFor } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, storybookCemRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-icon.xhtml?raw';

const meta = preview.meta({
    component: 'cem-icon',
    title: 'CEM Components/cem-icon',
    loaders: [async () => { await loadCemDeclaration('cem-icon', declarationSource); return {}; }],
});

export const SourcesAndLiveAttributes = meta.story({
    render: () => '<cem-icon></cem-icon>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-icon') as HTMLElement;
        await whenCemRendered(host);
        expect(host.querySelector('.material-icons')?.textContent).toBe('circle');
        expect(host.querySelector('[part="icon"]')?.getAttribute('aria-hidden')).toBe('true');
        host.setAttribute('name', 'check');
        await whenCemRendered(host);
        expect(host.querySelector('.material-icons')?.textContent).toBe('check');
        for (const [value, selector] of [['★', '.unicode'], ['😀', '.unicode'], ['fas fa-home', 'i.fa-home'], ['settings', '.material-icons']]) {
            host.setAttribute('image', value);
            await whenCemRendered(host);
            expect(host.querySelector(selector)).not.toBeNull();
        }
        host.setAttribute('image', 'data:image/svg+xml,%3Csvg%20xmlns=%22http://www.w3.org/2000/svg%22%20width=%2224%22%20height=%2224%22/%3E');
        await whenCemRendered(host);
        const image = host.querySelector('img') as HTMLImageElement;
        expect(image.alt).toBe('');
        expect(image.getAttribute('aria-hidden')).toBe('true');
        await waitFor(() => expect(image.naturalWidth).toBe(24));
        host.setAttribute('label', 'Status');
        await whenCemRendered(host);
        expect(within(host).getByRole('img', { name: 'Status' })).toBe(host.querySelector('[part="icon"]'));
        expect(within(host).getAllByRole('img')).toHaveLength(1);
        host.removeAttribute('label');
        await whenCemRendered(host);
        expect(within(host).queryByRole('img')).toBeNull();
        host.setAttribute('image', '');
        await whenCemRendered(host);
        expect(host.querySelector('[part="icon"]')).toBeNull();
        host.removeAttribute('image');
        await whenCemRendered(host);
        expect(host.querySelector('.material-icons')?.textContent).toBe('check');
        host.removeAttribute('name');
        await whenCemRendered(host);
        expect(host.querySelector('.material-icons')?.textContent).toBe('circle');
        expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
    },
});

export const ContentAndVisibility = meta.story({
    render: () => '<cem-icon image="★" label="Favorite"><a href="#details">Details</a></cem-icon>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-icon') as HTMLElement;
        await whenCemRendered(host);
        expect(within(host).getByRole('link', { name: 'Details' }).closest('[role="img"], [aria-hidden="true"]')).toBeNull();
        for (const direction of ['column', 'row']) {
            host.setAttribute('direction', direction);
            await whenCemRendered(host);
            expect(getComputedStyle(host.querySelector('[part="content"]') as HTMLElement).flexDirection).toBe(direction);
            expect(within(host).getByRole('link', { name: 'Details' })).not.toBeNull();
        }
        host.setAttribute('class', 'consumer-icon');
        for (const value of ['', 'false', 'true']) {
            host.setAttribute('hidden', value);
            await whenCemRendered(host);
            expect(getComputedStyle(host).display).toBe('none');
        }
        host.removeAttribute('hidden');
        await whenCemRendered(host);
        expect(getComputedStyle(host).display).toBe('inline-flex');
        expect(host.classList.contains('consumer-icon')).toBe(true);
        expect(host.querySelector('button, input')).toBeNull();
        expect(host.querySelector('style')).toBeNull();
        expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
    },
});

export const ThemeSizes = meta.story({
    render: () => ['light', 'dark', 'contrast-light', 'contrast-dark', 'native'].map(theme =>
        `<section class="cem-theme-${theme}"><cem-icon image="★" label="Favorite"></cem-icon></section>`).join(''),
    play: async ({ canvasElement }) => {
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-icon')) {
            await whenCemRendered(host);
            const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
            for (const [size, scale] of [['small', 1], ['normal', 2], ['large', 3]] as const) {
                host.setAttribute('size', size);
                await whenCemRendered(host);
                await waitFor(() => {
                    const style = getComputedStyle(host.querySelector('[part="icon"]') as HTMLElement);
                    expect(parseFloat(style.fontSize)).toBeCloseTo(rem * scale);
                    expect(parseFloat(style.height)).toBeCloseTo(rem * scale);
                    expect(style.color).toBe(getComputedStyle(host).color);
                });
            }
            host.style.setProperty('--cem-icon-size-large', '4rem');
            await waitFor(() => expect(parseFloat(getComputedStyle(host.querySelector('[part="icon"]') as HTMLElement).fontSize)).toBeCloseTo(rem * 4));
            host.style.removeProperty('--cem-icon-size-large');
            host.removeAttribute('size');
            await whenCemRendered(host);
            expect(parseFloat(getComputedStyle(host.querySelector('[part="icon"]') as HTMLElement).fontSize)).toBeCloseTo(rem * 2);
            expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
        }
    },
});

export const LegacyCollections = meta.story({
    render: () => {
        const unicode = ['»', '🚀', '👁', '🎄', '😭', '🔥', '💀', '🛒', '✨', '😊', '😂', '⭐', '🫶', '🎁', '✅'];
        const material = ['recycling', 'shopping_cart', 'search', 'home', 'menu', 'close', 'check_circle', 'favorite', 'add', 'star', 'chevron_right', 'logout', 'add_circle', 'cancel'];
        const fontawesome = ['fab fa-github', 'fas fa-bookmark', 'fab fa-discord', 'fab fa-android', 'fab fa-apple', 'far fa-user', 'far fa-envelope', 'fas fa-thumbs-up', 'far fa-thumbs-down', 'far fa-star', 'fas fa-star', 'fas fa-location-arrow', 'fas fa-map-marker', 'fas fa-map-marked-alt', 'fas fa-globe', 'fas fa-bone', 'fas fa-heart'];
        return [...unicode.map(image => [image, 'unicode']), ...material.map(image => [image, 'material']), ...fontawesome.map(image => [image, 'fontawesome'])]
            .map(([image, source]) => `<cem-icon image="${image}" data-source="${source}"><span>${image}</span></cem-icon>`).join('');
    },
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-icon')];
        await Promise.all(hosts.map(whenCemRendered));
        for (const host of hosts) {
            const image = host.getAttribute('image') as string;
            const glyph = host.querySelector('[part="glyph"]') as HTMLElement;
            if (host.dataset.source === 'fontawesome') {
                expect(glyph.tagName).toBe('I');
                for (const className of image.split(' ')) expect(glyph.classList.contains(className)).toBe(true);
            } else {
                expect(glyph.classList.contains(host.dataset.source === 'unicode' ? 'unicode' : 'material-icons')).toBe(true);
                expect(glyph.textContent).toBe(image);
            }
            expect(host.querySelector('[part="icon"]')?.getAttribute('aria-hidden')).toBe('true');
            expect(host.querySelector('[part="content"] > span:last-child')?.textContent).toBe(image);
            expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
        }
    },
});

export const LegacyColorInheritance = meta.story({
    render: () => ['light', 'dark', 'contrast-light', 'contrast-dark', 'native'].map(theme =>
        `<section class="cem-theme-${theme}">${['danger', 'calm', 'trust'].map(tone =>
            `<cem-icon image="fas fa-heart" style="color:var(--cem-palette-${tone})"><span>${tone}</span></cem-icon>`).join('')}</section>`).join(''),
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-icon')];
        await Promise.all(hosts.map(whenCemRendered));
        for (const host of hosts) {
            const glyph = host.querySelector('[part="glyph"]') as HTMLElement;
            const text = host.querySelector('[part="content"] > span:last-child') as HTMLElement;
            expect(getComputedStyle(glyph).color).toBe(getComputedStyle(host).color);
            expect(getComputedStyle(text).color).toBe(getComputedStyle(host).color);
            host.style.color = 'var(--cem-palette-trust)';
            expect(getComputedStyle(glyph).color).toBe(getComputedStyle(host).color);
        }
    },
});
