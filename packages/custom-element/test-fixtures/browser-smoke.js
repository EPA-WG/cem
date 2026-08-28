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

    const [indexModule, customElementModule, httpRequestModule, localStorageModule, locationModule, moduleUrlModule] =
        await Promise.all([
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

    const helperNames = ['cloneAs', 'deepEqual', 'mergeAttr', 'mix', 'obj2node', 'tagUid', 'xml2dom', 'xmlString'];
    for (const helperName of helperNames) {
        check(`index re-exports ${helperName}`, indexModule[helperName] === customElementModule[helperName]);
    }

    check('deepEqual accepts equal primitives', customElementModule.deepEqual(1, 1));
    check('deepEqual rejects unlike object values', !customElementModule.deepEqual({}, null));
    check('deepEqual rejects objects with different keys', !customElementModule.deepEqual({ a: 1 }, { a: 1, b: 2 }));
    check('deepEqual rejects objects with different values', !customElementModule.deepEqual({ a: 1 }, { a: 2 }));
    check(
        'deepEqual accepts nested objects and arrays',
        customElementModule.deepEqual({ a: 1, b: [2, { c: 3 }] }, { a: 1, b: [2, { c: 3 }] }),
    );

    const mixedTarget = { retained: true };
    check(
        'mix mutates and returns its target',
        customElementModule.mix(mixedTarget, { added: 1 }) === mixedTarget &&
            mixedTarget.retained === true &&
            mixedTarget.added === 1,
    );

    const cloneSource = document.createElement('section');
    cloneSource.setAttribute('data-source', 'clone');
    cloneSource.append('cloned text');
    const clone = customElementModule.cloneAs(cloneSource, 'article');
    check(
        'cloneAs changes the tag while preserving attributes and children',
        clone.localName === 'article' &&
            clone.getAttribute('data-source') === 'clone' &&
            clone.textContent === 'cloned text' &&
            clone !== cloneSource,
    );

    const mergeSource = document.createElement('input');
    mergeSource.setAttribute('id', 'merged');
    mergeSource.setAttribute('readonly', '');
    mergeSource.setAttribute('title', 'source');
    mergeSource.setAttribute('value', 'source value');
    const mergeTarget = document.createElement('input');
    mergeTarget.setAttribute('data-stale', 'remove');
    mergeTarget.value = 'dirty value';
    customElementModule.mergeAttr(mergeSource, mergeTarget);
    check(
        'mergeAttr synchronizes the complete attribute set and value property',
        [...mergeTarget.attributes]
            .map(({ name }) => name)
            .sort()
            .join(',') === 'id,readonly,title,value' &&
            mergeTarget.id === 'merged' &&
            mergeTarget.readOnly &&
            mergeTarget.title === 'source' &&
            mergeTarget.value === 'source value',
    );

    const propertyRetentionTarget = document.createElement('div');
    propertyRetentionTarget.dceExportedAttributes = new Set(['enforced']);
    propertyRetentionTarget.setAttribute('enforced', 'legacy property retention');
    customElementModule.mergeAttr(document.createElement('div'), propertyRetentionTarget);
    check(
        'mergeAttr retires dceExportedAttributes property-based retention',
        !propertyRetentionTarget.hasAttribute('enforced'),
    );

    const attributeRetentionTarget = document.createElement('div');
    attributeRetentionTarget.setAttribute('dce-exported-attributes', 'enforced');
    attributeRetentionTarget.setAttribute('enforced', 'legacy attribute retention');
    customElementModule.mergeAttr(document.createElement('div'), attributeRetentionTarget);
    check(
        'mergeAttr retires dce-exported-attributes content-attribute retention',
        !attributeRetentionTarget.hasAttribute('dce-exported-attributes') &&
            !attributeRetentionTarget.hasAttribute('enforced'),
    );

    const parsedXml = customElementModule.xml2dom('<a/>');
    check('xml2dom parses an XML document', parsedXml.documentElement.localName === 'a');
    check('xmlString serializes an XML document', customElementModule.xmlString(parsedXml).includes('<a'));
    check(
        'obj2node represents a function with an empty element',
        customElementModule.obj2node(() => undefined, 'f', document).outerHTML === '<f></f>',
    );
    check(
        'obj2node represents numeric and string values as text',
        customElementModule.obj2node(9, 'a', document).outerHTML === '<a>9</a>' &&
            customElementModule.obj2node('abc', 's', document).outerHTML === '<s>abc</s>',
    );
    check(
        'obj2node represents primitive properties as attributes and nested objects as elements',
        customElementModule.obj2node({ a: 1, b: { c: 'abc' } }, 's', document).outerHTML ===
            '<s a="1"><b c="abc"></b></s>',
    );

    const uidRoot = document.createElement('div');
    uidRoot.innerHTML = '<span><strong></strong></span>';
    check(
        'tagUid assigns stable descendant identifiers and returns its root',
        customElementModule.tagUid(uidRoot) === uidRoot &&
            uidRoot.querySelector('span')?.getAttribute('data-dce-id') === '1' &&
            uidRoot.querySelector('strong')?.getAttribute('data-dce-id') === '2',
    );

    check('custom-element is registered', customElements.get('custom-element') === CustomElement);
    check('http-request is registered', customElements.get('http-request') === HttpRequestElement);
    check('local-storage is registered', customElements.get('local-storage') === LocalStorageElement);
    check('location-element is registered', customElements.get('location-element') === LocationElement);
    check('module-url is registered', customElements.get('module-url') === ModuleUrl);

    const declareAdapterFixture = async (tag, source) => {
        const declaration = document.createElement('custom-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', tag);
        const template = document.createElement('template');
        // The explicit HTML type selects the substrate's DOM-canonical compiler
        // without triggering the adapter's untyped legacy-template normalization.
        template.setAttribute('type', 'text/html');
        template.innerHTML = source;
        declaration.append(template);
        document.body.append(declaration);
        await customElementModule.whenDeclarationSettled(declaration);
        await customElements.whenDefined(tag);
        check(
            `${tag} declaration compiles without diagnostics`,
            customElementModule.diagnosticsFor(declaration).length === 0,
        );
        return declaration;
    };
    const appendAdapterInstance = async (tag, configure = () => undefined) => {
        const instance = document.createElement(tag);
        configure(instance);
        document.body.append(instance);
        await customElementModule.whenRenderSettled(instance);
        return instance;
    };
    const dispatchAndSettle = async (instance, target, event) => {
        target.dispatchEvent(event);
        await customElementModule.whenRenderSettled(instance);
    };
    const mutateAndSettle = async (instance, mutate) => {
        mutate();
        // Attribute invalidation is observed asynchronously. Yield once so the
        // public settlement helper sees the render scheduled by MutationObserver.
        await Promise.resolve();
        await customElementModule.whenRenderSettled(instance);
    };

    await customElements.whenDefined('fixture-card');
    await new Promise((resolve) => requestAnimationFrame(resolve));
    await new Promise((resolve) => setTimeout(resolve, 0));

    const instance = document.querySelector('fixture-card');
    check('legacy declaration registers produced tag', customElements.get('fixture-card') !== undefined);
    check(
        'untyped legacy declaration uses the browser compatibility selector',
        document.querySelector('custom-element[tag="fixture-card"] > template')?.getAttribute('lang') ===
            'custom-element-v0',
    );
    // The adapter now transpiles the legacy template to CEM-ML and renders it through the cem_ql
    // WASM boundary, which is asynchronous — wait for the rendered output rather than asserting it
    // synchronously (the old DOM-projection bridge rendered synchronously).
    await waitFor(
        'legacy fixture renders host attribute text',
        () => instance?.querySelector('h3')?.textContent?.trim() === 'Smoke',
    );
    await waitFor(
        'legacy fixture projects payload',
        () => instance?.querySelector('p')?.textContent?.trim() === 'Payload',
    );
    check(
        'adapter render uses substrate data island',
        instance?.querySelector('template[data-cem-island="instance"]') !== null,
    );

    const explicitLegacyDeclaration = document.querySelector('custom-element[tag="explicit-legacy-card"]');
    const explicitLegacyInstance = document.querySelector('explicit-legacy-card');
    await customElementModule.whenDeclarationSettled(explicitLegacyDeclaration);
    await customElementModule.whenRenderSettled(explicitLegacyInstance);
    check(
        'explicit deprecated browser compatibility selector remains unchanged',
        explicitLegacyDeclaration?.querySelector(':scope > template')?.getAttribute('lang') === 'custom-element-v0',
    );
    check(
        'explicit deprecated browser compatibility selector renders through the substrate',
        explicitLegacyInstance?.querySelector('[data-role="explicit-legacy"]')?.textContent?.trim() ===
            'Explicit legacy' && explicitLegacyInstance?.querySelector('template[data-cem-island="instance"]') !== null,
    );
    check(
        'explicit deprecated browser compatibility selector compiles without diagnostics',
        customElementModule.diagnosticsFor(explicitLegacyDeclaration).length === 0 &&
            customElementModule.diagnosticsFor(explicitLegacyInstance).length === 0,
    );

    const implicitInstance = document.querySelector('implicit-template-card');
    await waitFor(
        'legacy shorthand declaration renders implicit template content',
        () => implicitInstance?.querySelector('a')?.textContent?.trim() === 'Implicit',
    );
    const implicitDeclaration = document.querySelector('custom-element[tag="implicit-template-card"]');
    check(
        'legacy shorthand declaration is normalized to one inert template',
        implicitDeclaration?.querySelectorAll(':scope > template').length === 1,
    );
    check(
        'legacy shorthand declaration keeps moved content in template',
        implicitDeclaration?.querySelector(':scope > template')?.content.querySelector('a')?.textContent?.trim() ===
            'Implicit',
    );

    const inlineDeclaration = document.querySelector('custom-element.inline-fixture');
    const inlineTag = inlineDeclaration?.getAttribute('tag');
    await waitFor('omitted tag creates an inline produced instance', () =>
        Boolean(inlineTag && inlineDeclaration?.querySelector(inlineTag)?.querySelector('strong')),
    );
    check(
        'inline produced instance renders declaration attributes',
        inlineDeclaration?.querySelector(inlineTag)?.querySelector('strong')?.textContent?.trim() === 'inline-fixture',
    );

    const inlineSrcDeclaration = document.querySelector('custom-element.inline-src-fixture');
    const inlineSrcTag = inlineSrcDeclaration?.getAttribute('tag');
    await waitFor('anonymous src declaration creates an inline produced instance', () =>
        Boolean(inlineSrcTag && inlineSrcDeclaration?.querySelector(inlineSrcTag)?.querySelector('article')),
    );
    check(
        'anonymous src declaration projects template payload',
        inlineSrcDeclaration?.querySelector(inlineSrcTag)?.querySelector('p')?.textContent?.trim() ===
            'Inline src payload',
    );
    check(
        'anonymous src payload template remains inert on declaration',
        inlineSrcDeclaration?.querySelector(':scope > template')?.content.querySelector('span')?.textContent?.trim() ===
            'Inline src payload',
    );

    const externalDocumentDeclaration = document.querySelector('custom-element.inline-external-document-fixture');
    const externalDocumentTag = externalDocumentDeclaration?.getAttribute('tag');
    await waitFor('anonymous external document src creates an inline produced instance', () =>
        Boolean(
            externalDocumentTag &&
            externalDocumentDeclaration?.querySelector(externalDocumentTag)?.querySelector('.external-document'),
        ),
    );
    check(
        'anonymous external document src projects live payload',
        externalDocumentDeclaration
            ?.querySelector(externalDocumentTag)
            ?.querySelector('.external-document p')
            ?.textContent?.trim() === 'External document payload',
    );

    const externalFragmentDeclaration = document.querySelector('custom-element.inline-external-fragment-fixture');
    const externalFragmentTag = externalFragmentDeclaration?.getAttribute('tag');
    await waitFor('anonymous external fragment src creates an inline produced instance', () =>
        Boolean(
            externalFragmentTag &&
            externalFragmentDeclaration?.querySelector(externalFragmentTag)?.querySelector('.external-fragment'),
        ),
    );
    check(
        'anonymous external fragment src renders subtree',
        externalFragmentDeclaration
            ?.querySelector(externalFragmentTag)
            ?.querySelector('.external-fragment strong')
            ?.textContent?.trim() === 'External fragment',
    );

    const externalXhtmlTreeDeclaration = document.querySelector('custom-element.inline-external-xhtml-tree-fixture');
    const externalXhtmlTreeTag = externalXhtmlTreeDeclaration?.getAttribute('tag');
    await waitFor('anonymous external XHTML CEM-ML src renders the recursive produced tree', () =>
        Boolean(
            externalXhtmlTreeTag &&
            externalXhtmlTreeDeclaration
                ?.querySelector(externalXhtmlTreeTag)
                ?.querySelector('.data-island-tree details details details details'),
        ),
    );
    const externalXhtmlTree = externalXhtmlTreeDeclaration?.querySelector(externalXhtmlTreeTag);
    if (!externalXhtmlTree?.querySelector('.data-island-tree details details details details')) {
        const diagnostics = [
            ...customElementModule.diagnosticsFor(externalXhtmlTreeDeclaration),
            ...(externalXhtmlTree ? customElementModule.diagnosticsFor(externalXhtmlTree) : []),
        ];
        errors.push(
            `anonymous external XHTML CEM-ML state: tag=${externalXhtmlTreeTag ?? '<missing>'}; ` +
                `instance=${externalXhtmlTree ? 'present' : 'missing'}; diagnostics=${
                    diagnostics.length > 0
                        ? diagnostics.map((diagnostic) => `${diagnostic.code}: ${diagnostic.message}`).join('; ')
                        : '<none>'
                }`,
        );
    }
    const externalXhtmlTreeText = externalXhtmlTree?.textContent ?? '';
    check(
        'anonymous external XHTML CEM-ML src renders host attributes',
        externalXhtmlTreeText.includes('Anonymous DCE data island') &&
            externalXhtmlTreeText.includes('data-demo=') &&
            externalXhtmlTreeText.includes('custom-element'),
    );
    check(
        'anonymous external XHTML CEM-ML src renders recursive payload attributes',
        externalXhtmlTreeText.includes('data-root=') &&
            externalXhtmlTreeText.includes('custom-element') &&
            externalXhtmlTreeText.includes('data-level=') &&
            externalXhtmlTreeText.includes('3') &&
            externalXhtmlTreeText.includes('code=') &&
            externalXhtmlTreeText.includes('a1'),
    );
    check(
        'anonymous external XHTML CEM-ML src renders recursive payload text',
        externalXhtmlTreeText.includes('Leaf text from custom-element data island'),
    );

    await declareAdapterFixture(
        'adapter-slice-matrix',
        [
            '<slice name="count">0</slice>',
            '<slice name="left"></slice>',
            '<slice name="right"></slice>',
            '<slice name="checked">false</slice>',
            '<slice name="radio"></slice>',
            '<button type="button" data-role="increment" slice="count" slice-event="click tap" slice-value="//count + 1">+</button>',
            '<input data-role="fanout" slice="left|right" slice-event="input" slice-value="$target.value" />',
            '<input data-role="checkbox" type="checkbox" slice="checked" slice-event="change" slice-value="$target.checked" />',
            '<input data-role="radio-one" type="radio" name="adapter-radio" value="one" slice="radio" slice-event="change" slice-value="$target.value" />',
            '<input data-role="radio-two" type="radio" name="adapter-radio" value="two" slice="radio" slice-event="change" slice-value="$target.value" />',
            '<output data-role="count">${$count}</output>',
            '<output data-role="left">${$left}</output>',
            '<output data-role="right">${$right}</output>',
            '<output data-role="checked">${$checked}</output>',
            '<output data-role="radio">${$radio}</output>',
        ].join(''),
    );
    const sliceMatrix = await appendAdapterInstance('adapter-slice-matrix');
    const increment = sliceMatrix.querySelector('[data-role="increment"]');
    increment.click();
    await customElementModule.whenRenderSettled(sliceMatrix);
    check(
        'public adapter handles the first event in a multi-event slice binding',
        sliceMatrix.querySelector('[data-role="count"]')?.textContent === '1',
    );
    await dispatchAndSettle(sliceMatrix, increment, new Event('tap', { bubbles: true }));
    check(
        'public adapter handles the second event in a multi-event slice binding',
        sliceMatrix.querySelector('[data-role="count"]')?.textContent === '2',
    );

    const fanout = sliceMatrix.querySelector('[data-role="fanout"]');
    fanout.value = 'mirrored';
    await dispatchAndSettle(sliceMatrix, fanout, new Event('input', { bubbles: true }));
    check(
        'public adapter writes one event value to multiple slices',
        sliceMatrix.querySelector('[data-role="left"]')?.textContent === 'mirrored' &&
            sliceMatrix.querySelector('[data-role="right"]')?.textContent === 'mirrored',
    );

    const checkbox = sliceMatrix.querySelector('[data-role="checkbox"]');
    checkbox.checked = true;
    await dispatchAndSettle(sliceMatrix, checkbox, new Event('change', { bubbles: true }));
    check(
        'public adapter coerces a checked checkbox to true',
        sliceMatrix.querySelector('[data-role="checked"]')?.textContent === 'true',
    );
    checkbox.checked = false;
    await dispatchAndSettle(sliceMatrix, checkbox, new Event('change', { bubbles: true }));
    check(
        'public adapter coerces an unchecked checkbox to false',
        sliceMatrix.querySelector('[data-role="checked"]')?.textContent === 'false',
    );

    const radioOne = sliceMatrix.querySelector('[data-role="radio-one"]');
    const radioTwo = sliceMatrix.querySelector('[data-role="radio-two"]');
    radioOne.checked = true;
    await dispatchAndSettle(sliceMatrix, radioOne, new Event('change', { bubbles: true }));
    check(
        'public adapter projects the first radio value',
        sliceMatrix.querySelector('[data-role="radio"]')?.textContent === 'one',
    );
    radioTwo.checked = true;
    await dispatchAndSettle(sliceMatrix, radioTwo, new Event('change', { bubbles: true }));
    check(
        'public adapter projects the second radio value',
        sliceMatrix.querySelector('[data-role="radio"]')?.textContent === 'two',
    );

    await declareAdapterFixture(
        'adapter-one-way-attributes',
        [
            '<attribute name="defaulted">Default value</attribute>',
            '<attribute name="selected" select="//source"></attribute>',
            '<slice name="source"></slice>',
            '<input data-role="source" slice="source" slice-event="input" slice-value="$target.value" />',
            '<output data-role="defaulted">${$defaulted}</output>',
            '<output data-role="source-value">${$source}</output>',
        ].join(''),
    );
    const oneWayAttributes = await appendAdapterInstance('adapter-one-way-attributes');
    check(
        'declared defaults stay in render state without propagating to the host',
        oneWayAttributes.querySelector('[data-role="defaulted"]')?.textContent === 'Default value' &&
            !oneWayAttributes.hasAttribute('defaulted'),
    );
    const oneWaySource = oneWayAttributes.querySelector('[data-role="source"]');
    oneWaySource.value = 'runtime value';
    await dispatchAndSettle(oneWayAttributes, oneWaySource, new Event('input', { bubbles: true }));
    check(
        'selected and slice values stay in render state without propagating to the host',
        oneWayAttributes.querySelector('[data-role="source-value"]')?.textContent === 'runtime value' &&
            !oneWayAttributes.hasAttribute('selected') &&
            !oneWayAttributes.hasAttribute('source'),
    );

    await declareAdapterFixture(
        'adapter-form-matrix',
        [
            '<slice name="username"></slice>',
            '<slice name="password"></slice>',
            '<form slice="signin" custom-validity="string-length(/datadom/slice/signin/form-data/username) &gt; 2 and string-length(//form-data/password) &gt; 3 ?? \'enter username and password\'">',
            '<input name="username" required value="{$username}" slice="username" slice-event="input" slice-value="$target.value" />',
            '<input name="password" type="password" required custom-validity="string-length(//form-data/password) &gt; 3 ?? \'password is too short\'" value="{$password}" slice="password" slice-event="input" slice-value="$target.value" />',
            '<output data-role="form-username">${$datadom.formData.signin.username}</output>',
            '<output data-role="mirror-username">${$datadom.slices.signin.formData.username}</output>',
            '<output data-role="form-valid">${$datadom.validationState.signin.valid}</output>',
            '<output data-role="form-message">${$datadom.validationState.signin.validationMessage}</output>',
            '<output data-role="password-valid">${$datadom.validationState.signin.controls.password.valid}</output>',
            '<output data-role="password-message">${$datadom.validationState.signin.controls.password.validationMessage}</output>',
            '</form>',
        ].join(''),
    );
    const formMatrix = await appendAdapterInstance('adapter-form-matrix');
    const username = formMatrix.querySelector('input[name="username"]');
    const password = formMatrix.querySelector('input[name="password"]');
    username.value = 'ada';
    await dispatchAndSettle(formMatrix, username, new Event('input', { bubbles: true }));
    check(
        'public adapter exposes live form data',
        formMatrix.querySelector('[data-role="form-username"]')?.textContent === 'ada',
    );
    check(
        'public adapter mirrors form data into its named slice',
        formMatrix.querySelector('[data-role="mirror-username"]')?.textContent === 'ada',
    );
    check(
        'public adapter exposes invalid form state',
        formMatrix.querySelector('[data-role="form-valid"]')?.textContent === 'false',
    );
    check(
        'public adapter applies a form custom-validity message',
        formMatrix.querySelector('[data-role="form-message"]')?.textContent === 'enter username and password',
    );
    check(
        'public adapter applies a control custom-validity message',
        password.validationMessage === 'password is too short',
    );
    password.value = 'secret';
    await dispatchAndSettle(formMatrix, password, new Event('input', { bubbles: true }));
    check(
        'public adapter exposes valid form state after correction',
        formMatrix.querySelector('[data-role="form-valid"]')?.textContent === 'true',
    );
    check(
        'public adapter clears form custom validity after correction',
        formMatrix.querySelector('[data-role="form-message"]')?.textContent === '',
    );
    check('public adapter clears native control custom validity after correction', password.validationMessage === '');
    check(
        'public adapter exposes valid control state after correction',
        formMatrix.querySelector('[data-role="password-valid"]')?.textContent === 'true',
    );

    const styleDeclaration = await declareAdapterFixture(
        'adapter-style-matrix',
        [
            '<style>:host { --adapter-color: rgb(0, 128, 0); } .adapter-style-target { color: var(--adapter-color); }</style>',
            '<section><slot></slot></section>',
        ].join(''),
    );
    const styledFirst = await appendAdapterInstance('adapter-style-matrix', (element) => {
        element.innerHTML =
            '<template><style>.adapter-style-target { color: rgb(255, 0, 0); }</style><span class="adapter-style-target">first</span></template>';
    });
    const styledSecond = await appendAdapterInstance('adapter-style-matrix', (element) => {
        element.innerHTML = '<span class="adapter-style-target">second</span>';
    });
    const outsideStyled = document.createElement('span');
    outsideStyled.className = 'adapter-style-target';
    outsideStyled.textContent = 'outside';
    document.body.append(outsideStyled);
    const firstStyleTarget = styledFirst.querySelector('.adapter-style-target');
    const secondStyleTarget = styledSecond.querySelector('.adapter-style-target');
    check('public adapter keeps render identity separate from CSS identity', styledFirst.hasAttribute('data-cem-render-scope'));
    check('public adapter emits no private declaration marker', !styledFirst.hasAttribute('scope'));
    check('public adapter emits no instance CSS marker', !styledFirst.hasAttribute('data-cem-instance-scope'));
    const firstStyleColor = getComputedStyle(firstStyleTarget).color;
    const secondStyleColor = getComputedStyle(secondStyleTarget).color;
    check('public adapter applies declaration CSS inside each instance', secondStyleColor === 'rgb(0, 128, 0)');
    check('public adapter lets payload CSS override its own instance', firstStyleColor === 'rgb(255, 0, 0)');
    check(
        'public adapter contains declaration and payload CSS outside the instances',
        !['rgb(0, 128, 0)', 'rgb(255, 0, 0)'].includes(getComputedStyle(outsideStyled).color),
    );
    check(
        'public adapter emits native declaration and implicit instance scopes',
        styleDeclaration
            .querySelector(':scope > style[data-cem-declaration-style="private"]')
            ?.textContent?.includes('@scope (\n    adapter-style-matrix') &&
            styledFirst.querySelector(':scope > style')?.textContent?.includes('@scope to ('),
    );

    await declareAdapterFixture(
        'adapter-dom-matrix',
        [
            '<attribute name="label">Initial</attribute>',
            '<slice name="value">retained</slice>',
            '<label><span data-role="label">${$label}</span><input data-role="control" aria-label="{$label}" value="{$value}" slice="value" slice-event="input" slice-value="$target.value" /></label>',
            '<output data-role="value">${$value}</output>',
        ].join(''),
    );
    const domMatrix = await appendAdapterInstance('adapter-dom-matrix');
    const retainedControl = domMatrix.querySelector('[data-role="control"]');
    retainedControl.focus();
    retainedControl.value = 'retained value';
    retainedControl.setSelectionRange(2, 7);
    await dispatchAndSettle(domMatrix, retainedControl, new Event('input', { bubbles: true }));
    const afterSliceRender = domMatrix.querySelector('[data-role="control"]');
    check('public adapter retains control DOM identity after a slice rerender', afterSliceRender === retainedControl);
    check('public adapter retains focus after a slice rerender', document.activeElement === retainedControl);
    check(
        'public adapter retains selection after a slice rerender',
        retainedControl.selectionStart === 2 && retainedControl.selectionEnd === 7,
    );
    check(
        'public adapter updates output inside the retained DOM',
        domMatrix.querySelector('[data-role="value"]')?.textContent === 'retained value',
    );
    await mutateAndSettle(domMatrix, () => domMatrix.setAttribute('label', 'Updated'));
    check(
        'public adapter retains control DOM identity after an attribute rerender',
        domMatrix.querySelector('[data-role="control"]') === retainedControl,
    );
    check(
        'public adapter updates host attributes through the retained DOM',
        domMatrix.querySelector('[data-role="label"]')?.textContent === 'Updated' &&
            retainedControl.getAttribute('aria-label') === 'Updated',
    );
    check(
        'public adapter retains focus and selection after an attribute rerender',
        document.activeElement === retainedControl &&
            retainedControl.selectionStart === 2 &&
            retainedControl.selectionEnd === 7,
    );

    const request = document.createElement('http-request');
    request.setAttribute('url', './http-data.json');
    request.setAttribute('method', 'GET');
    request.setAttribute('header-accept', 'application/json');
    document.body.appendChild(request);
    await waitFor('http-request fetches JSON data', () => request.value?.data?.status === 'ok');
    check('http-request records response status', request.value?.response?.status === 200);
    check('http-request forwards request headers', request.value?.request?.headers?.accept === 'application/json');

    const xmlRequest = document.createElement('http-request');
    xmlRequest.setAttribute('url', './http-data.xml');
    xmlRequest.setAttribute('method', 'GET');
    xmlRequest.setAttribute('header-accept', 'application/xml');
    document.body.appendChild(xmlRequest);
    await waitFor('http-request fetches XML data', () => xmlRequest.value?.data?.localName === 'response');
    check('http-request records XML response status', xmlRequest.value?.response?.status === 200);
    check(
        'http-request parses XML payload',
        [...(xmlRequest.value?.data?.querySelectorAll('item') ?? [])].map((item) => item.textContent).join(',') ===
            'alpha,beta',
    );

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
        moduleUrl.value?.endsWith('/test-fixtures/browser-smoke.html'),
    );
    check(
        'module-url writes value attribute',
        moduleUrl.getAttribute('value')?.endsWith('/test-fixtures/browser-smoke.html'),
    );

    return { done: true, errors };
}
