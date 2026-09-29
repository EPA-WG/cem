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
