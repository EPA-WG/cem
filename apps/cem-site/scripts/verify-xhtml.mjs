/** Browser fixture for route-relative, native v3 XHTML deployment. */
export async function verifyDeployedXhtml(page) {
    const actionResource = './assets/cem-components/cem-action.xhtml';
    const actionResponse = await page.request.get(new URL(actionResource, page.url()).href);
    if (!actionResponse.ok() || !actionResponse.headers()['content-type']?.startsWith('application/xhtml+xml')) {
        throw new Error('canonical action XHTML resource is missing or has the wrong MIME type');
    }
    const action = await page.evaluate(async () => {
        const { componentRuntime } = await import('./assets/cem-site/components-runtime.js');
        const declaration = document.querySelector('custom-element[tag="cem-action"]');
        const host = document.querySelector('cem-action');
        if (!declaration || !host) throw new Error('route must explicitly load and use canonical action');
        await componentRuntime.whenDeclarationSettled(declaration);
        await componentRuntime.whenRenderSettled(host);
        return {
            source: declaration.getAttribute('src'),
            part: host.querySelector('button')?.getAttribute('part'),
            scoped: declaration.querySelector('style[data-cem-declaration-style]')?.textContent.includes('@scope') === true,
            shadow: !!host.shadowRoot,
        };
    });
    if (action.source !== `${actionResource}#cem-action` || action.part !== 'control' || !action.scoped || action.shadow) {
        throw new Error(`canonical action declaration or CSS failed: ${JSON.stringify(action)}`);
    }
    const resource = './assets/cem-components/cem-select.xhtml';
    const response = await page.request.get(new URL(resource, page.url()).href);
    if (!response.ok() || !response.headers()['content-type']?.startsWith('application/xhtml+xml')) {
        throw new Error('canonical XHTML resource is missing or has the wrong MIME type');
    }
    await page.evaluate(async ({ resource }) => {
        const { componentRuntime } = await import('./assets/cem-site/components-runtime.js');
        const fixture = document.createElement('section');
        fixture.id = 'deployed-xhtml-proof';
        fixture.innerHTML = `<custom-element tag="cem-select" capability="choice-select"
            src="${resource}#cem-select"></custom-element>
            <cem-select label="Deployment proof" value="first"><template>
                <cem-option value="first">First</cem-option>
                <cem-option value="second">Second</cem-option>
            </template></cem-select>`;
        document.body.append(fixture);
        const declaration = fixture.querySelector('custom-element');
        await componentRuntime.whenDeclarationSettled(declaration);
        await componentRuntime.whenRenderSettled(fixture.querySelector('cem-select'));
        const errors = componentRuntime.diagnosticsFor(declaration)
            .filter(({ severity }) => severity === 'error' || severity === 'fatal');
        if (errors.length) throw new Error(JSON.stringify(errors));
    }, { resource });
    const fixture = page.locator('#deployed-xhtml-proof');
    await fixture.getByRole('combobox').click();
    await fixture.getByRole('option', { name: 'Second', exact: true }).click();
    const result = await page.evaluate(async () => {
        const { componentRuntime } = await import('./assets/cem-site/components-runtime.js');
        const fixture = document.querySelector('#deployed-xhtml-proof');
        const select = fixture.querySelector('cem-select');
        await componentRuntime.whenRenderSettled(select);
        const style = fixture.querySelector('custom-element > style[data-cem-declaration-style]');
        const result = { value: select.value, display: getComputedStyle(select.querySelector('[part="root"]')).display,
            scoped: style?.textContent.includes('@scope') === true, shadow: !!select.shadowRoot };
        fixture.remove();
        return result;
    });
    if (result.value !== 'second' || result.display !== 'grid' || !result.scoped || result.shadow) {
        throw new Error(`deployed XHTML behavior or CSS failed: ${JSON.stringify(result)}`);
    }
    return { resource, namedTemplate: 'cem-select', ...result };
}
