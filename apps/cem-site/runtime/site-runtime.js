import { diagnosticsFor, whenRenderSettled } from '@epa-wg/custom-element';
import { componentRuntimeErrors } from '@epa-wg/cem-site/components-runtime';

const errors = [...componentRuntimeErrors];

try {
    const tokenFilter = document.querySelector('[data-token-filter]');
    const tokenStatus = document.querySelector('[data-token-status]');
    const tokenExamples = [...document.querySelectorAll('[data-token-example]')];
    const applyTokenFilter = () => {
        const query = tokenFilter.value.trim().toLowerCase();
        let visible = 0;
        for (const example of tokenExamples) {
            const matches = example.dataset.tokenExample.toLowerCase().includes(query);
            example.hidden = !matches;
            visible += Number(matches);
        }
        tokenStatus.textContent = `${visible} token example${visible === 1 ? '' : 's'}`;
    };
    tokenFilter.addEventListener('input', applyTokenFilter);
    applyTokenFilter();

    const action = await waitForElement('cem-action button');
    const actionHost = action.closest('cem-action');
    const actionCount = document.querySelector('[data-action-count]');
    let count = 0;
    actionHost.addEventListener('click', () => {
        count += 1;
        actionCount.textContent = `Action count: ${count}`;
    });

    await waitForElement('cem-field input');
    const fixture = document.querySelector('[data-cem-fixture-instance]');
    await customElements.whenDefined(fixture.localName);
    await whenRenderSettled(fixture);
    errors.push(
        ...diagnosticsFor(fixture)
            .filter(({ severity }) => severity === 'error' || severity === 'fatal')
            .map(({ code, message }) => `${code}: ${message}`),
    );
    document.querySelector('[data-native-output]').textContent = fixture.innerHTML.trim();
} catch (error) {
    errors.push(error instanceof Error ? error.message : String(error));
}

globalThis.__cemSiteInteractive = {
    done: true,
    errors,
};

async function waitForElement(selector, attempts = 120) {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        const element = document.querySelector(selector);
        if (element) {
            return element;
        }
        await new Promise((resolve) => requestAnimationFrame(resolve));
    }
    throw new Error(`timed out waiting for ${selector}`);
}
