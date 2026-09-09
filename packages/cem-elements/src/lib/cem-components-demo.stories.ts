import { expect, within } from 'storybook/test';

import preview from '../../.storybook/preview.js';

const COMPONENT_PAGE_URL = new URL('../../../cem-components/index.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Authentication form',
    '2. Registration',
    '3. Password reset',
    '4. Profile editor',
    '5. Asset browser',
    '6. Discussion thread',
    '7. Settings',
    '8. Feedback settings compatibility',
] as const;

const meta = preview.meta({
    title: 'CEM Components/Package Demo',
    tags: ['test'],
});

export const WorkflowGallery = meta.story({
    render: () => {
        const frame = document.createElement('iframe');
        frame.title = 'cem-components package demo';
        frame.src = COMPONENT_PAGE_URL.href;
        frame.style.inlineSize = '100%';
        frame.style.blockSize = '60rem';
        return frame;
    },
    play: async ({ canvasElement }) => {
        const frame = canvasElement.querySelector<HTMLIFrameElement>('iframe');
        if (!frame) throw new Error('Expected the cem-components demo iframe');
        const page = await waitForFrameDocument(frame);
        const pageWindow = frame.contentWindow;
        if (!pageWindow) throw new Error('Expected a same-origin cem-components demo iframe');
        await pageWindow.customElements.whenDefined('cem-demo-element');

        const samples = Array.from(
            page.querySelectorAll<HTMLElement>('main > cem-demo-element[legend]')
        );
        await expect(samples.map((sample) => sample.getAttribute('legend'))).toEqual(
            [...EXPECTED_LEGENDS]
        );
        await Promise.all(samples.map((sample) => waitForReady(sample)));

        const auth = samples[0];
        const code = auth.querySelector<HTMLElement>('code.cem-source-code.language-html');
        await expect(code).not.toBeNull();
        await expect(code).toHaveAttribute('data-language', 'html');
        await expect(code).toHaveTextContent('<cem-card label="Sign in">');

        await Promise.all([
            waitForElement(auth, 'cem-card section'),
            waitForElement(auth, 'cem-text-field input'),
            waitForElement(auth, 'cem-action button'),
        ]);
        const authCanvas = within(auth);
        await expect(authCanvas.getByRole('region', { name: 'Sign in' })).toBeVisible();
        await expect(authCanvas.getByRole('textbox', { name: 'Email' })).toBeVisible();
        await expect(authCanvas.getByRole('button', { name: 'Continue' })).toBeVisible();

        const asset = samples[4];
        await waitForElement(asset, 'cem-table [role="table"]');
        const previewImage = await waitForElement(asset, 'cem-media-preview img');
        await expect(within(asset).getByRole('table', { name: 'Asset table' })).toBeVisible();
        await expect(previewImage).toHaveAttribute(
            'src',
            new URL('/policy.png', COMPONENT_PAGE_URL).href
        );
    },
});

async function waitForReady(sample: HTMLElement): Promise<void> {
    const updateComplete = (sample as HTMLElement & { updateComplete?: Promise<void> }).updateComplete;
    await updateComplete;
    for (let attempt = 0; attempt < 180; attempt += 1) {
        if (sample.dataset.state === 'ready' && sample.querySelector('[slot="demo"] > *')) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(`Expected ${sample.getAttribute('legend')} to render`);
}

async function waitForFrameDocument(frame: HTMLIFrameElement): Promise<Document> {
    for (let attempt = 0; attempt < 600; attempt += 1) {
        const page = frame.contentDocument;
        if (page?.querySelector('main > cem-demo-element')) return page;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error('Expected the cem-components demo document to load');
}

async function waitForElement(root: ParentNode, selector: string): Promise<HTMLElement> {
    for (let attempt = 0; attempt < 600; attempt += 1) {
        const element = root.querySelector<HTMLElement>(selector);
        if (element) return element;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(`Expected ${selector} to render`);
}
