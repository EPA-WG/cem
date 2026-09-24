import { expect, waitFor } from 'storybook/test';

/** Check real fetched source, coloring and isolation from the live examples. */
export async function verifyExternalFilePreviews(
    root: ParentNode,
    pageUrl: URL,
    files: readonly string[],
): Promise<void> {
    for (const file of files) {
        const response = await fetch(new URL(file, pageUrl));
        expect(response.ok).toBe(true);
        const source = await response.text();
        await waitFor(() => {
            const card = root.querySelector(`cem-demo-element[src="./${file}"]`);
            expect(card).toHaveAttribute('data-state', 'ready');
            expect(card).toHaveAttribute('demo', 'false');
            const code = card?.querySelector('[slot=text] code');
            expect(code?.textContent).toBe(source);
            // This failure fixture is plain prose, with no valid JSON tokens to color.
            if (file === 'http-data-invalid.json') {
                expect(code).toHaveAttribute('data-language', 'js');
            } else {
                const token = code?.querySelector(file.endsWith('.json') ? 'i' : 'b');
                expect(token).not.toBeNull();
                expect(getComputedStyle(token as Element).color).not.toBe(getComputedStyle(code as Element).color);
            }
            expect(card?.querySelector('[slot=demo]')).toBeEmptyDOMElement();
        }, { timeout: 30000 });
    }
}
