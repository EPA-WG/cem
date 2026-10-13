import { componentRuntime, componentRuntimeErrors, mountSiteView, viewErrors } from '@epa-wg/cem-site/components-runtime';

const errors = [...componentRuntimeErrors];
try {
    const root = document.querySelector('main');
    // Snapshot native build output once. This is the bootstrap control-data
    // boundary, not an alternate parser for the source JSON/CEM tree.
    const state = {
        title: root.querySelector('h1').textContent.trim(),
        tokens: [...root.querySelectorAll('[data-token-example]')].map(token => ({
            name: token.getAttribute('data-token-example'),
            label: token.querySelector('strong').textContent.trim(),
            tone: token.getAttribute('data-token-tone'),
        })),
        actionLabel: root.querySelector('[data-component-example="action"]').getAttribute('label'),
        fieldLabel: root.querySelector('[data-component-example="field"]').getAttribute('label'),
        source: root.querySelector('[data-cem-source]').textContent,
        name: root.querySelector('[data-cem-fixture-instance]').getAttribute('name'),
        nativeOutput: '',
    };
    const view = await mountSiteView(root, 'cem-site-examples', new URL('./examples-view.xhtml#cem-site-examples', import.meta.url).href,
        state, {});
    const fixture = view.element.querySelector('[data-cem-fixture-instance]');
    await componentRuntime.whenRenderSettled(fixture);
    // Explicit HTML export of the produced greeting paragraph. Instance hydration
    // metadata is not part of the example output. The shared preview displays
    // these exact browser-serialized bytes as literal text.
    view.update({ state: { ...state, nativeOutput: fixture.querySelector('.cem-site-greeting').outerHTML } });
    await view.whenSettled();
    errors.push(...viewErrors(view));
} catch (error) {
    errors.push(error instanceof Error ? error.message : String(error));
}
globalThis.__cemSiteInteractive = { done: true, errors };
