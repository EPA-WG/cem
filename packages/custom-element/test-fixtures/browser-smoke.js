export async function runCustomElementSmoke(importBase) {
    const errors = [];
    const check = (label, condition) => {
        if (!condition) {
            errors.push(label);
        }
    };
    const waitFor = async (label, condition) => {
        const started = Date.now();
        while (!condition()) {
            if (Date.now() - started > 2000) {
                check(label, false);
                return;
            }
            await new Promise((resolve) => setTimeout(resolve, 10));
        }
        check(label, true);
    };

    const [
        indexModule,
        customElementModule,
        httpRequestModule,
        localStorageModule,
        locationModule,
        moduleUrlModule,
    ] = await Promise.all([
        import(`${importBase}/index.js`),
        import(`${importBase}/custom-element.js`),
        import(`${importBase}/http-request.js`),
        import(`${importBase}/local-storage.js`),
        import(`${importBase}/location-element.js`),
        import(`${importBase}/module-url.js`),
    ]);

    const IndexCustomElement = indexModule.default;
    const CustomElement = customElementModule.default;
    const NamedCustomElement = customElementModule.CustomElement;
    const HttpRequestElement = httpRequestModule.default;
    const NamedHttpRequestElement = httpRequestModule.HttpRequestElement;
    const LocalStorageElement = localStorageModule.default;
    const NamedLocalStorageElement = localStorageModule.LocalStorageElement;
    const LocationElement = locationModule.default;
    const NamedLocationElement = locationModule.LocationElement;
    const ModuleUrl = moduleUrlModule.default;
    const NamedModuleUrl = moduleUrlModule.ModuleUrl;

    check('index default export matches CustomElement', IndexCustomElement === CustomElement);
    check('custom-element named/default exports match', NamedCustomElement === CustomElement);
    check('http-request named/default exports match', NamedHttpRequestElement === HttpRequestElement);
    check('local-storage named/default exports match', NamedLocalStorageElement === LocalStorageElement);
    check('location-element named/default exports match', NamedLocationElement === LocationElement);
    check('module-url named/default exports match', NamedModuleUrl === ModuleUrl);

    check('custom-element is registered', customElements.get('custom-element') === CustomElement);
    check('http-request is registered', customElements.get('http-request') === HttpRequestElement);
    check('local-storage is registered', customElements.get('local-storage') === LocalStorageElement);
    check('location-element is registered', customElements.get('location-element') === LocationElement);
    check('module-url is registered', customElements.get('module-url') === ModuleUrl);

    await customElements.whenDefined('fixture-card');
    await new Promise((resolve) => requestAnimationFrame(resolve));
    await new Promise((resolve) => setTimeout(resolve, 0));

    const instance = document.querySelector('fixture-card');
    check('legacy declaration registers produced tag', customElements.get('fixture-card') !== undefined);
    check('legacy fixture renders host attribute text', instance?.querySelector('h3')?.textContent?.trim() === 'Smoke');
    check('legacy fixture projects payload', instance?.querySelector('p')?.textContent?.trim() === 'Payload');
    check(
        'adapter render uses substrate data island',
        instance?.querySelector('template[data-cem-island="instance"]') !== null
    );

    const inlineDeclaration = document.querySelector('custom-element.inline-fixture');
    const inlineTag = inlineDeclaration?.getAttribute('tag');
    await waitFor(
        'omitted tag creates an inline produced instance',
        () => Boolean(inlineTag && inlineDeclaration?.querySelector(inlineTag)?.querySelector('strong'))
    );
    check(
        'inline produced instance renders declaration attributes',
        inlineDeclaration?.querySelector(inlineTag)?.querySelector('strong')?.textContent?.trim() === 'inline-fixture'
    );

    const request = document.createElement('http-request');
    request.setAttribute('url', './http-data.json');
    request.setAttribute('method', 'GET');
    request.setAttribute('header-accept', 'application/json');
    document.body.appendChild(request);
    await waitFor('http-request fetches JSON data', () => request.value?.data?.status === 'ok');
    check('http-request records response status', request.value?.response?.status === 200);
    check('http-request forwards request headers', request.value?.request?.headers?.accept === 'application/json');

    localStorage.removeItem('fixture-key');
    const storage = document.createElement('local-storage');
    storage.setAttribute('key', 'fixture-key');
    storage.setAttribute('type', 'json');
    storage.setAttribute('live', 'live');
    document.body.appendChild(storage);
    await new Promise((resolve) => setTimeout(resolve, 0));
    localStorage.setItem('fixture-key', JSON.stringify({ answer: 42 }));
    await waitFor('local-storage live updates from storage changes', () => storage.value?.answer === 42);

    const locationElement = document.createElement('location-element');
    locationElement.setAttribute('href', new URL('/fixture-location?x=1&x=2#hash', location.href).href);
    document.body.appendChild(locationElement);
    await waitFor('location-element parses URL values', () => locationElement.value?.hash === '#hash');
    check('location-element preserves repeated params', locationElement.value?.params?.x?.join(',') === '1,2');

    const moduleUrl = document.createElement('module-url');
    moduleUrl.setAttribute('src', './browser-smoke.html');
    document.body.appendChild(moduleUrl);
    await waitFor('module-url resolves relative specifiers', () =>
        moduleUrl.value?.endsWith('/test-fixtures/browser-smoke.html')
    );
    check('module-url writes value attribute', moduleUrl.getAttribute('value')?.endsWith('/test-fixtures/browser-smoke.html'));

    return { done: true, errors };
}
