import type { Meta, StoryObj } from '@storybook/web-components-vite';
import {
    CemElementRuntime,
    analyzeDeclarationShape,
    cemElements,
    exportDataIslandSnapshotForEdge,
    isValidCustomElementName,
    type CemElementDiagnostic,
    type DataIslandSnapshot,
} from './cem-elements.js';
import {
    diffRenderPlansToPatchFrames,
    materializeRenderPlan,
    projectTemplate,
    readTemplateSource,
    renderPlanIdentity,
    type PatchFrame,
    type RenderPlan,
    type RenderPlanNode,
    type TemplateSourceNode,
} from './projection.js';
import { renderCemMlTemplate, runtimeVersion } from './internal/runtime-support/cem-ql-render.js';

const meta: Meta = {
    title: 'CEM Elements/Runtime',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const PackageRuntimeSurface: Story = {
    render: () => storyPanel('Runtime surface', cemElements()),
    play: () => {
        assertEqual(cemElements(), '@epa-wg/cem-elements', 'package runtime surface name');
    },
};

export const ProducedTagValidation: Story = {
    render: () =>
        storyPanel(
            'Produced tag validation',
            [
                ['cem-button', isValidCustomElementName('cem-button')],
                ['x-token-field', isValidCustomElementName('x-token-field')],
                ['button', isValidCustomElementName('button')],
                ['Cem-Button', isValidCustomElementName('Cem-Button')],
                ['font-face', isValidCustomElementName('font-face')],
            ]
                .map(([tag, valid]) => `${tag}: ${String(valid)}`)
                .join('\n')
        ),
    play: () => {
        assert(isValidCustomElementName('cem-button'), 'cem-button should be a valid produced tag');
        assert(isValidCustomElementName('x-token-field'), 'x-token-field should be a valid produced tag');
        assert(!isValidCustomElementName('button'), 'button should not be a custom-element tag');
        assert(!isValidCustomElementName('Cem-Button'), 'uppercase custom-element tags are invalid');
        assert(!isValidCustomElementName('font-face'), 'reserved custom-element names are invalid');
    },
};

export const InlineDeclarationShape: Story = {
    render: () => storyPanel('Inline declaration shape', 'one direct-child template, no live content'),
    play: () => {
        const result = analyzeDeclarationShape({
            tag: 'cem-button',
            src: null,
            directTemplateCount: 1,
            directLiveNodeCount: 0,
        });
        assert(result.ok, 'a single inline declaration template should be accepted');
        assertEqual(result.diagnostics.length, 0, 'accepted declarations should not emit diagnostics');
    },
};

export const SrcInlineTemplateConflict: Story = {
    render: () => storyPanel('src conflict', 'src plus inline template is invalid'),
    play: () => {
        const result = analyzeDeclarationShape({
            tag: 'cem-button',
            src: './button.cem#button',
            directTemplateCount: 1,
            directLiveNodeCount: 0,
        });
        assert(!result.ok, 'src plus inline template must be rejected');
        assertDiagnostic(result.diagnostics, 'cem-element.src_inline_template_conflict');
    },
};

export const DeclarationLiveContentRejected: Story = {
    render: () => storyPanel('Live declaration content', 'content outside the template wrapper is invalid'),
    play: () => {
        const result = analyzeDeclarationShape({
            tag: 'cem-button',
            src: null,
            directTemplateCount: 1,
            directLiveNodeCount: 1,
        });
        assert(!result.ok, 'live declaration content must be rejected');
        assertDiagnostic(result.diagnostics, 'cem-element.declaration_live_content');
    },
};

export const MissingInlineTemplateRejected: Story = {
    render: () => storyPanel('Missing inline template', 'inline declarations require exactly one template'),
    play: () => {
        const result = analyzeDeclarationShape({
            tag: 'cem-button',
            src: null,
            directTemplateCount: 0,
            directLiveNodeCount: 0,
        });
        assert(!result.ok, 'inline declarations without a template must be rejected');
        assertDiagnostic(result.diagnostics, 'cem-element.inline_template_count');
    },
};

export const DataIslandCaptureAndRender: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'data island capture story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-capture' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-capture');
        declaration.setAttribute('tag', 'story-capture-button');
        const template = document.createElement('template');
        template.innerHTML = [
            '<attribute name="label">Save</attribute>',
            '<button type="button" aria-label="{ $label }">${$label}</button>',
        ].join('');
        declaration.appendChild(template);
        root.appendChild(declaration);

        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-capture-button');
        instance.setAttribute('label', 'Submit');
        instance.textContent = 'Fallback payload';
        root.appendChild(instance);

        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();

        const instance = requiredElement(canvasElement, 'story-capture-button');
        const island = requiredElement(instance, 'template[data-cem-island="instance"]') as HTMLTemplateElement;
        const button = requiredElement(instance, 'button');

        assertEqual(island.content.textContent, 'Fallback payload', 'fallback payload should move to data island');
        assertEqual(button.textContent, 'Submit', 'rendered button should use host attribute value');
        assertEqual(button.getAttribute('aria-label'), 'Submit', 'attribute interpolation should use host value');
    },
};

export const ProjectionBoundaryPlan: Story = {
    render: () => storyPanel('Projection boundary', 'serializable source + values → render plan (no live DOM)'),
    play: () => {
        const source: TemplateSourceNode[] = [
            {
                kind: 'element',
                namespace: null,
                tag: 'attribute',
                attributes: [{ name: 'name', value: 'label' }],
                children: [{ kind: 'text', text: 'Save' }],
            },
            {
                kind: 'element',
                namespace: null,
                tag: 'button',
                attributes: [
                    { name: 'type', value: 'button' },
                    { name: 'aria-label', value: '{ $label }' },
                    { name: 'disabled', value: '{ $busy }' },
                ],
                children: [{ kind: 'text', text: '${ $label }' }],
            },
        ];

        const snapshot = projectionSnapshot('cem-projection-button', {
            label: 'Submit',
            busy: null,
        });
        const plan = projectTemplate(source, {
            snapshot,
            values: { label: 'Submit', busy: null },
        });

        assertEqual(plan.instanceId, 'story-instance-1', 'projection carries snapshot instance identity');
        assertEqual(plan.dataRevision, '1', 'projection carries snapshot data revision');
        assertEqual(plan.nodes.length, 1, 'top-level `attribute` declaration nodes are dropped from output');
        const [button] = plan.nodes;
        assert(button.kind === 'element', 'projected node should be an element');
        assertEqual(button.tag, 'button', 'element tag should be preserved');
        assertEqual(button.renderNodeId, 'cem-projection-button-1', 'projection assigns deterministic render-node ids');
        const ariaLabel = button.attributes.find((attribute) => attribute.name === 'aria-label');
        assertEqual(ariaLabel?.value, 'Submit', 'whole-expression attribute resolves to the host value');
        assert(
            !button.attributes.some((attribute) => attribute.name === 'disabled'),
            'whole-expression attribute resolving to null is dropped'
        );
        assertEqual(button.children.length, 1, 'text child should be projected');
        assert(button.children[0].kind === 'text', 'child should be a text node');
        assertEqual(button.children[0].text, 'Submit', 'text interpolation resolves against values');
    },
};

export const FormattedDomTemplateProjection: Story = {
    render: () => storyPanel('Formatted DOM template', 'DOM parser source → snapshot projection'),
    play: () => {
        const template = document.createElement('template');
        template.innerHTML = `
            <attribute name="label">Save</attribute>
            <article class="card">
                <h3>\${$label}</h3>
                <button type="button" data-state="{$state}">Toggle</button>
            </article>
        `;

        const source = readTemplateSource(template.content);
        const snapshot = projectionSnapshot('cem-formatted-card', {
            label: 'Tokens',
            state: 'open',
        });
        const plan = projectTemplate(source, {
            snapshot,
            values: { label: 'Tokens', state: 'open' },
        });

        assertEqual(plan.nodes.length, 1, 'top-level declaration and indentation whitespace should not render');
        const [article] = plan.nodes;
        assert(article.kind === 'element', 'formatted template should project the article element');
        assertEqual(article.tag, 'article', 'formatted DOM parser source preserves the render root');
        const heading = article.children.find((child) => child.kind === 'element' && child.tag === 'h3');
        assert(heading?.kind === 'element', 'formatted template should keep nested heading');
        assertEqual(heading.children[0]?.kind === 'text' ? heading.children[0].text.trim() : '', 'Tokens', 'heading text resolves through projection');
        const button = article.children.find((child) => child.kind === 'element' && child.tag === 'button');
        assert(button?.kind === 'element', 'formatted template should keep nested button');
        assertEqual(
            button.attributes.find((attribute) => attribute.name === 'data-state')?.value,
            'open',
            'formatted template attribute interpolation resolves through projection'
        );
    },
};

export const RenderLoopNestedAndDynamic: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'render loop story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-render' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-render');
        declaration.setAttribute('tag', 'story-render-card');
        const template = document.createElement('template');
        template.innerHTML = `
            <attribute name="title">Untitled</attribute>
            <article class="card">
                <h3>\${$title}</h3>
                <button type="button" data-state="{$state}" hidden="{$collapsed}">Toggle</button>
            </article>
        `;
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-render-card');
        instance.setAttribute('title', 'Tokens');
        instance.setAttribute('state', 'open');
        root.appendChild(instance);

        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();

        const instance = requiredElement(canvasElement, 'story-render-card');
        const heading = requiredElement(instance, 'article.card h3');
        const button = requiredElement(instance, 'article.card button') as HTMLButtonElement;

        assertEqual(heading.textContent, 'Tokens', 'nested text interpolation should use host attribute value');
        assertEqual(button.getAttribute('data-state'), 'open', 'AVT attribute should resolve to host value');
        assert(!button.hasAttribute('hidden'), 'whole-expression attribute with absent value should be removed');
        assert(button.hasAttribute('data-cem-render-node-id'), 'rendered nodes carry render-node identity');
        assert(button.hasAttribute('data-cem-template-artifact-id'), 'rendered nodes carry template artifact identity');
        assertEqual(button.getAttribute('data-cem-data-revision'), '1', 'rendered nodes carry data revision');
        assertEqual(button.getAttribute('data-cem-source-fidelity'), 'dom-canonical', 'DOM templates carry source fidelity');
    },
};

export const CanonicalCemMlRenderLoop: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'canonical CEM-ML render story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-cem' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-cem');
        declaration.setAttribute('tag', 'story-cem-button');
        const template = document.createElement('template');
        template.setAttribute('type', 'text/cem-ml');
        template.textContent = `
            {attribute @name="label" | Save}
            {attribute @name="busy"}
            {button @type=button @aria-busy={$busy} | {$label}}
        `;
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-cem-button');
        instance.setAttribute('label', 'Submit');
        instance.setAttribute('busy', '');
        root.appendChild(instance);

        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();

        const instance = requiredElement(canvasElement, 'story-cem-button');
        const button = requiredElement(instance, 'button');

        assertEqual(button.textContent?.trim(), 'Submit', 'canonical CEM-ML text projection should use host value');
        assertEqual(button.getAttribute('type'), 'button', 'canonical CEM-ML bare attribute values should render');
        assertEqual(button.getAttribute('aria-busy'), 'true', 'canonical CEM-ML braced attribute values should render');
        assertEqual(
            button.getAttribute('data-cem-source-fidelity'),
            'author-byte-exact',
            'canonical CEM-ML templates carry source fidelity'
        );
    },
};

// ---------------------------------------------------------------------------
// Runtime slice C2.3 — canonical CEM-ML lowered through the cem_ql WASM render
// boundary (host runtime-support layer).
// ---------------------------------------------------------------------------

export const CemQlWasmRenderBoundary: Story = {
    render: () =>
        storyPanel('cem_ql WASM render boundary', 'canonical CEM-ML source + host bindings → render plan via WASM'),
    play: async () => {
        const result = await renderCemMlTemplate(
            '{button @type=button @class="tone {$tone}" | {$label}}',
            { label: 'Save', tone: 'primary' },
            { renderNodeIdPrefix: 'cem-wasm' }
        );

        assertEqual(runtimeVersion(), '0.1.0', 'cem_ql WASM engine version is exposed once initialized');
        assertEqual(result.diagnostics.length, 0, 'a well-formed canonical template renders without diagnostics');
        assertEqual(result.nodes.length, 1, 'render plan has a single root element');

        const [button] = result.nodes;
        assert(button.kind === 'element', 'root render-plan node is an element');
        assertEqual(button.tag, 'button', 'WASM render preserves the element tag');
        assertEqual(button.renderNodeId, 'cem-wasm-1', 'render-node ids use the supplied prefix in pre-order');
        assertEqual(
            button.attributes.find((attribute) => attribute.name === 'type')?.value,
            'button',
            'bare canonical attribute renders through WASM'
        );
        assertEqual(
            button.attributes.find((attribute) => attribute.name === 'class')?.value,
            'tone primary',
            'AVT attribute interpolation resolves host bindings through WASM'
        );
        const text = button.children
            .map((child) => (child.kind === 'text' ? child.text : ''))
            .join('');
        assertEqual(text, 'Save', 'content expression resolves the host binding through WASM');
        assertEqual(
            button.sourceMapRef?.fidelity,
            'author-byte-exact',
            'WASM render carries author-byte-exact fidelity'
        );
        assertEqual(button.sourceMapRef?.frame, 'cem:0', 'root frame is the source byte offset');

        // Diagnostics flow through the same boundary: an unknown binding compiles to a
        // mapped render diagnostic rather than throwing.
        const missing = await renderCemMlTemplate('{button | {$missing}}', {}, { renderNodeIdPrefix: 'cem-missing' });
        assertDiagnostic(missing.diagnostics, 'cem.ql.render.compile_failed');
    },
};

export const CemQlWasmRenderLoopUpgrade: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'cem_ql WASM render loop story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-wasm' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-wasm');
        declaration.setAttribute('tag', 'story-wasm-button');
        const template = document.createElement('template');
        template.setAttribute('type', 'text/cem-ml');
        template.textContent = '{button @type=button @class="tone {$tone}" | {$label}}';
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-wasm-button');
        instance.setAttribute('label', 'Submit');
        instance.setAttribute('tone', 'primary');
        root.appendChild(instance);

        return root;
    },
    play: async ({ canvasElement }) => {
        const instance = requiredElement(canvasElement, 'story-wasm-button');
        // The canonical render is asynchronous (WASM init + render), so poll until the
        // authoritative cem_ql output commits rather than assuming one frame.
        const button = await waitForElement(instance, 'button');

        assertEqual(button.getAttribute('type'), 'button', 'canonical bare attribute renders through WASM');
        assertEqual(button.getAttribute('class'), 'tone primary', 'AVT attribute resolves host attribute through WASM');
        assertEqual(button.textContent?.trim(), 'Submit', 'content expression resolves the host attribute through WASM');

        assertEqual(
            button.getAttribute('data-cem-render-node-id'),
            'story-wasm-button-1',
            'WASM render-node ids are produced-tag scoped'
        );
        assertEqual(button.getAttribute('data-cem-data-revision'), '1', 'WASM render carries the first data revision');
        assert(button.hasAttribute('data-cem-template-artifact-id'), 'WASM nodes carry template artifact identity');
        assertEqual(
            button.getAttribute('data-cem-source-fidelity'),
            'author-byte-exact',
            'WASM nodes mark author-byte-exact fidelity'
        );
        assertEqual(button.getAttribute('data-cem-source-frame'), 'cem:0', 'WASM root frame is the source byte offset');
    },
};

// ---------------------------------------------------------------------------
// Runtime slice C2.4 — functional /datadom data-document selection + `??`
// coalescing through the cem_ql render boundary (no XPath engine).
// ---------------------------------------------------------------------------

export const CemQlDataDocumentBoundary: Story = {
    render: () =>
        storyPanel('cem_ql data-document boundary', 'functional /datadom selection + `??` default via the WASM boundary'),
    play: async () => {
        // `datadom.attributes.<name>` is the functional-parity equivalent of the legacy
        // `/datadom/attributes/<name>` XPath selection; `??` supplies an absent default.
        const present = await renderCemMlTemplate(
            '{button | {$datadom.attributes.label ?? "Anonymous"}}',
            { label: 'Sasha' },
            { renderNodeIdPrefix: 'cem-dd' }
        );
        assertEqual(present.diagnostics.length, 0, 'present selection renders without diagnostics');
        assertEqual(textOfNodes(present.nodes), 'Sasha', 'datadom.attributes selection resolves the host binding');

        const absent = await renderCemMlTemplate(
            '{button | {$datadom.attributes.label ?? "Anonymous"}}',
            {},
            { renderNodeIdPrefix: 'cem-dd' }
        );
        assertEqual(absent.diagnostics.length, 0, 'absent selection coalesces without diagnostics');
        assertEqual(textOfNodes(absent.nodes), 'Anonymous', 'absent selection falls back through `??`');

        const structured = await renderCemMlTemplate(
            '{button | {$datadom.dataset.variant}-{$datadom.payload.text}-{$datadom.slots.leading}}',
            {
                datadom: {
                    attributes: {},
                    dataset: { variant: 'compact' },
                    payload: {
                        text: 'Payload',
                        childCount: 1,
                        nodes: [],
                        slots: { leading: [{ text: 'Lead' }] },
                        data: [],
                        options: [],
                        dataByValue: {},
                        optionsByValue: {},
                    },
                    slots: { leading: [{ text: 'Lead' }] },
                    slices: {},
                    validationState: {},
                    eventPayloads: {},
                },
            },
            { renderNodeIdPrefix: 'cem-dd-structured' }
        );
        assertEqual(structured.diagnostics.length, 0, 'structured datadom renders without diagnostics');
        assertEqual(
            textOfNodes(structured.nodes),
            'compact-Payload-',
            'structured datadom exposes dataset, payload, and slot buckets'
        );
    },
};

export const CemQlDataDocumentRenderLoop: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'cem_ql data-document render loop story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-datadom' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-datadom');
        declaration.setAttribute('tag', 'story-datadom-button');
        const template = document.createElement('template');
        template.setAttribute('type', 'text/cem-ml');
        // Functional data-document selection, lowered through cem_ql at render time.
        template.textContent =
            '{button @type=button | {$datadom.attributes.label}-{$datadom.dataset.variant}-{$datadom.payload.text}}';
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-datadom-button');
        instance.setAttribute('label', 'Tokens');
        instance.setAttribute('data-variant', 'compact');
        instance.textContent = 'Payload';
        root.appendChild(instance);

        return root;
    },
    play: async ({ canvasElement }) => {
        const instance = requiredElement(canvasElement, 'story-datadom-button');
        const button = await waitForElement(instance, 'button');

        assertEqual(
            button.textContent?.trim(),
            'Tokens-compact-Payload',
            'data-document selection resolves snapshot attributes, dataset, and payload through the runtime'
        );
        assertEqual(button.getAttribute('type'), 'button', 'sibling canonical attributes still render');
    },
};

export const DataOptionPayloadRenderLoop: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'data and option payload story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-choice-payload' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-choice-payload');
        declaration.setAttribute('tag', 'story-choice-payload');
        const template = document.createElement('template');
        template.setAttribute('type', 'text/cem-ml');
        template.textContent =
            '{button @type=button | {$datadom.data.apple.label}/{$datadom.options.date.label}/{$datadom.options.checkbox.group}}';
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-choice-payload');
        instance.innerHTML = [
            '<data value="apple">Apple</data>',
            '<select>',
            '<option value="date">Date</option>',
            '<optgroup label="Other">',
            '<option value="checkbox">Checkbox</option>',
            '</optgroup>',
            '</select>',
        ].join('');
        root.appendChild(instance);

        return root;
    },
    play: async ({ canvasElement }) => {
        const instance = requiredElement(canvasElement, 'story-choice-payload');
        const button = await waitForElement(instance, 'button');

        assertEqual(
            button.textContent?.trim(),
            'Apple/Date/Other',
            '<data> and <option> payloads are exposed under datadom by value'
        );
    },
};

// ---------------------------------------------------------------------------
// Runtime slice C2.6 — declaration-bearing canonical templates (with
// `<attribute>` decls) render through the WASM boundary, which drops declaration
// nodes and applies defaults. The C1.5 render fallback is removed.
// ---------------------------------------------------------------------------

export const DeclaredAttributeWasmRenderLoop: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'declared attribute WASM render story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-decl-attr' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-decl-attr');
        declaration.setAttribute('tag', 'story-decl-attr-button');
        const template = document.createElement('template');
        template.setAttribute('type', 'text/cem-ml');
        // Declares an attribute (with a default) and renders it through canonical
        // `{$label}` — previously C1.5-only because of the `<attribute>` declaration.
        template.textContent = '{attribute @name="label" | Save}{button @type=button | {$label}}';
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const named = document.createElement('story-decl-attr-button');
        named.setAttribute('label', 'Submit');
        const fallbackDefault = document.createElement('story-decl-attr-button');
        root.append(named, fallbackDefault);

        return root;
    },
    play: async ({ canvasElement }) => {
        const instances = canvasElement.querySelectorAll('story-decl-attr-button');
        const named = await waitForElement(instances[0], 'button');
        const def = await waitForElement(instances[1], 'button');

        assertEqual(named.textContent?.trim(), 'Submit', 'declared attribute resolves the host value through WASM');
        assertEqual(
            def.textContent?.trim(),
            'Save',
            'declared attribute default renders when the host attribute is absent'
        );
        assert(instances[0].querySelector('attribute') === null, 'the `<attribute>` declaration is dropped from output');
        assert(
            named.hasAttribute('data-cem-template-artifact-id'),
            'a declaration-bearing template renders through the WASM boundary'
        );
        assertEqual(named.getAttribute('type'), 'button', 'sibling canonical attributes still render');
    },
};

export const AttributeObserverRerendersOnUndeclaredAttribute: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'attribute observer story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-attr-observer' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-attr-observer');
        declaration.setAttribute('tag', 'story-attr-observer');
        const template = document.createElement('template');
        template.setAttribute('type', 'text/cem-ml');
        // `tone` is read from the data document but is NOT a declared `<attribute>`, so the
        // old `observedAttributes` path would never observe it; the per-instance
        // MutationObserver re-renders on any host attribute change.
        template.textContent = '{button @type=button | {$datadom.attributes.tone}}';
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-attr-observer');
        root.appendChild(instance);

        return root;
    },
    play: async ({ canvasElement }) => {
        const instance = requiredElement(canvasElement, 'story-attr-observer');
        const button = await waitForElement(instance, 'button');
        assertEqual(button.textContent?.trim(), '', 'with no `tone` attribute the data selection is empty');

        instance.setAttribute('tone', 'primary');
        await waitForCondition(
            () => requiredElement(instance, 'button').textContent?.trim() === 'primary',
            'changing an undeclared host attribute should re-render through the MutationObserver'
        );
        assertEqual(
            requiredElement(instance, 'button').textContent?.trim(),
            'primary',
            'an undeclared host attribute change re-renders via the per-instance MutationObserver'
        );
    },
};

// ---------------------------------------------------------------------------
// Legacy custom-element parity stories — named coverage for behaviors inventoried
// from /home/suns/aWork/custom-element docs and demos.
// ---------------------------------------------------------------------------

export const LegacyAttributeDefaultsAndHostOverridesParity: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'legacy attribute parity story');

        registerInlineDeclaration({
            declarationTag: 'cem-element-story-legacy-attr',
            producedTag: 'story-legacy-attr',
            innerHTML:
                '<attribute name="label">Default</attribute><button type="button" data-label="{$label}">${$label}</button>',
        });

        const fallback = document.createElement('story-legacy-attr');
        const override = document.createElement('story-legacy-attr');
        override.setAttribute('label', 'Override');
        root.append(fallback, override);
        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();
        const instances = Array.from(canvasElement.querySelectorAll('story-legacy-attr'));
        assertEqual(instances.length, 2, 'legacy attribute parity story renders two instances');

        const fallbackButton = requiredElement(instances[0], 'button');
        const overrideButton = requiredElement(instances[1], 'button');
        assertEqual(fallbackButton.textContent, 'Default', 'declared attribute text is used as the default');
        assertEqual(fallbackButton.getAttribute('data-label'), 'Default', 'default attribute resolves in AVT output');
        assertEqual(overrideButton.textContent, 'Override', 'host attribute overrides the declared default');
        assertEqual(overrideButton.getAttribute('data-label'), 'Override', 'host override resolves in AVT output');
    },
};

export const LegacyDatadomAccessMigrationParity: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'legacy datadom migration story');

        registerInlineDeclaration({
            declarationTag: 'cem-element-story-legacy-datadom',
            producedTag: 'story-legacy-datadom',
            type: 'text/cem-ml',
            text: [
                '{attribute @name="label" | Default}',
                '{button @type=button @data-label={datadom.attributes.label ?? "Default"} | {$datadom.attributes.label ?? "Default"}}',
            ].join(''),
        });

        const instance = document.createElement('story-legacy-datadom');
        instance.setAttribute('label', 'Datadom');
        root.appendChild(instance);
        return root;
    },
    play: async ({ canvasElement }) => {
        const instance = requiredElement(canvasElement, 'story-legacy-datadom');
        const button = await waitForElement(instance, 'button');
        assertEqual(button.textContent?.trim(), 'Datadom', 'cem-ql datadom access replaces legacy XPath attributes');
        assertEqual(button.getAttribute('data-label'), 'Datadom', 'structured datadom resolves in CEM-ML AVT output');
    },
};

export const LegacyNamedSlotPayloadParity: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'legacy slot parity story');

        registerInlineDeclaration({
            declarationTag: 'cem-element-story-legacy-slot',
            producedTag: 'story-legacy-slot',
            innerHTML:
                '<article><h3><slot name="title">Untitled</slot></h3><div class="body"><slot>Empty</slot></div></article>',
        });

        const filled = document.createElement('story-legacy-slot');
        const title = document.createElement('span');
        title.setAttribute('slot', 'title');
        title.textContent = 'Legacy title';
        filled.append(title, document.createTextNode('Body payload'));

        const fallback = document.createElement('story-legacy-slot');
        root.append(filled, fallback);
        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();
        const instances = Array.from(canvasElement.querySelectorAll('story-legacy-slot'));
        assertEqual(instances.length, 2, 'legacy slot parity story renders two instances');

        assertEqual(
            requiredElement(instances[0], 'h3').textContent?.trim(),
            'Legacy title',
            'named slot projects matching payload'
        );
        assertEqual(
            requiredElement(instances[0], '.body').textContent?.trim(),
            'Body payload',
            'default slot projects unslotted payload'
        );
        assertEqual(requiredElement(instances[1], 'h3').textContent?.trim(), 'Untitled', 'named slot fallback renders');
        assertEqual(requiredElement(instances[1], '.body').textContent?.trim(), 'Empty', 'default slot fallback renders');
    },
};

export const LegacySliceInputEventParity: Story = {
    render: () =>
        renderInstanceStory({
            declarationTag: 'cem-element-story-legacy-slice',
            producedTag: 'story-legacy-slice',
            ariaLabel: 'legacy slice event parity story',
            innerHTML:
                '<slice name="typed"></slice><label>Type <input slice="typed" slice-event="input" slice-value="{$target.value}" /></label><output>${$typed}</output>',
        }),
    play: async ({ canvasElement }) => {
        await nextFrame();
        const instance = requiredElement(canvasElement, 'story-legacy-slice');
        const input = requiredElement(instance, 'input') as HTMLInputElement;

        input.value = 'typed value';
        input.dispatchEvent(new Event('input', { bubbles: true }));
        await waitForCondition(
            () => requiredElement(instance, 'output').textContent === 'typed value',
            'legacy slice input event rerenders output'
        );
    },
};

export const ExternalSrcDeclarationLoadingParity: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'external src declaration loading story');

        // The host `loadSrcDocument` resolves + fetches the referenced document (here a
        // fixture); the runtime parses it and resolves the `#fragment` to its template.
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-element-story-ext-src',
            loadSrcDocument: async (path) => {
                assertEqual(path, './remote-button.html', 'the loader receives the src path (fragment stripped)');
                return '<template id="remote-button" type="text/cem-ml">{button @type=button | {$datadom.attributes.label}}</template>';
            },
        });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-ext-src');
        declaration.setAttribute('tag', 'story-ext-src-button');
        declaration.setAttribute('src', './remote-button.html#remote-button');
        root.appendChild(declaration);

        const instance = document.createElement('story-ext-src-button');
        instance.setAttribute('label', 'Remote');
        root.appendChild(instance);

        return root;
    },
    play: async ({ canvasElement }) => {
        const instance = requiredElement(canvasElement, 'story-ext-src-button');
        // The produced tag is defined only after the async fetch + parse completes.
        const button = await waitForElement(instance, 'button');
        assertEqual(
            button.textContent?.trim(),
            'Remote',
            'an external src declaration fetches, parses, and renders the produced element'
        );
        assertEqual(button.getAttribute('type'), 'button', 'the fetched template renders its attributes');
    },
};

export const SrcDeclarationLoadingDiagnostics: Story = {
    render: () => storyPanel('src loading diagnostics', 'missing local target + external load failure'),
    play: async () => {
        // A local `src="#id"` whose same-document target is missing reports synchronously.
        const localRuntime = new CemElementRuntime({ declarationTag: 'cem-element-story-src-missing' });
        const missing = buildDeclaration({ tag: 'story-src-missing', src: '#no-such-template', templates: [] });
        assert(!localRuntime.registerDeclaration(missing), 'a missing local src target does not register');
        assertDiagnostic(localRuntime.diagnosticsFor(missing), 'cem-element.src_local_target_missing');

        // An external `src` whose document fails to load reports asynchronously.
        const failRuntime = new CemElementRuntime({
            declarationTag: 'cem-element-story-src-fail',
            loadSrcDocument: async () => {
                throw new Error('offline');
            },
        });
        const failing = buildDeclaration({ tag: 'story-src-fail', src: './missing.html#x', templates: [] });
        failRuntime.registerDeclaration(failing);
        await failRuntime.whenDeclarationSettled(failing);
        assertDiagnostic(failRuntime.diagnosticsFor(failing), 'cem-element.src_load_failed');
    },
};

export const LocalSrcDeclarationLoadingParity: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'local src declaration loading story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-local-src' });
        runtime.install(window);

        // Legacy pattern: a top-level `<template id>` holds the definition; a separate
        // src-referencing `<cem-element>` registers the produced tag from it.
        const template = document.createElement('template');
        template.id = 'story-local-src-template';
        template.setAttribute('type', 'text/cem-ml');
        template.textContent = '{button @type=button | {$datadom.attributes.label}}';
        root.appendChild(template);

        const declaration = document.createElement('cem-element-story-local-src');
        declaration.setAttribute('tag', 'story-local-src-button');
        declaration.setAttribute('src', '#story-local-src-template');
        root.appendChild(declaration);

        const instance = document.createElement('story-local-src-button');
        instance.setAttribute('label', 'Loaded');
        root.appendChild(instance);

        return root;
    },
    play: async ({ canvasElement }) => {
        const instance = requiredElement(canvasElement, 'story-local-src-button');
        const button = await waitForElement(instance, 'button');
        assertEqual(
            button.textContent?.trim(),
            'Loaded',
            'a same-document src="#id" template registers and renders the produced element'
        );
        assertEqual(button.getAttribute('type'), 'button', 'the loaded template renders its attributes');
    },
};

export const LegacyBridgeTemplateParity: Story = {
    render: () => storyPanel('Legacy bridge template', 'custom-element-v0 bridge renders during migration'),
    play: async ({ canvasElement }) => {
        const root = document.createElement('section');
        canvasElement.appendChild(root);
        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-legacy-bridge' });
        const declaration = buildDeclaration({
            tag: 'story-legacy-bridge',
            templates: [{
                lang: 'custom-element-v0',
                html:
                    '<attribute name="label">Legacy</attribute>' +
                    '<button type="button" title="{title}">{$label} {title}</button>' +
                    '<if test="//smile"><span class="smile">{//smile}</span></if>' +
                    '<slot name="description"><i>fallback</i></slot>',
            }],
        });
        runtime.registerDeclaration(declaration);
        assertEqual(runtime.diagnosticsFor(declaration).length, 0, 'legacy bridge declarations register without diagnostics');

        const instance = document.createElement('story-legacy-bridge');
        instance.setAttribute('title', 'Bridge');
        instance.dataset.smile = 'yes';
        instance.innerHTML = '<p slot="description">projected</p>';
        root.appendChild(instance);

        await runtime.whenRenderSettled(instance);
        const button = await waitForElement(instance, 'button');
        assertEqual(button.textContent?.trim(), 'Legacy Bridge', 'legacy text interpolation resolves defaults and host attributes');
        assertEqual(button.getAttribute('title'), 'Bridge', 'legacy attribute value templates resolve host attributes');
        assertEqual(requiredElement(instance, '.smile').textContent, 'yes', 'legacy if test reads dataset-style //path values');
        assertEqual(requiredElement(instance, 'p[slot="description"]').textContent, 'projected', 'legacy slots project payload');
    },
};

// ---------------------------------------------------------------------------
// Runtime slice C2.5 — conditional constructs (cem:if / cem:choose / cem:when /
// cem:otherwise) lowered through the cem_ql render boundary.
// ---------------------------------------------------------------------------

export const CemQlConditionalRenderLoop: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'cem_ql conditional render story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-cond' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-cond');
        declaration.setAttribute('tag', 'story-cond-card');
        const template = document.createElement('template');
        template.setAttribute('type', 'text/cem-ml');
        // `cem:if` gated on a data-document selection, plus a `cem:choose`/`cem:when`/
        // `cem:otherwise` case branch — both evaluate cem-ql `@test` expressions.
        template.textContent =
            '{div @class=card |' +
            ' {cem:if @test="datadom.attributes.label" | {h3 | {$datadom.attributes.label}}}' +
            ' {cem:choose |' +
            ' {cem:when @test="datadom.attributes.kind" | {span @class=kind | {$datadom.attributes.kind}}}' +
            ' {cem:otherwise | {span @class=kind | default}}}}';
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const full = document.createElement('story-cond-card');
        full.setAttribute('label', 'Card');
        full.setAttribute('kind', 'primary');
        const empty = document.createElement('story-cond-card');
        root.append(full, empty);

        return root;
    },
    play: async ({ canvasElement }) => {
        const instances = canvasElement.querySelectorAll('story-cond-card');
        const fullCard = await waitForElement(instances[0], 'div.card');
        const emptyCard = await waitForElement(instances[1], 'div.card');

        // Truthy `cem:if` test emits its body; matching `cem:when` wins the choose.
        assertEqual(
            requiredElement(fullCard, 'h3').textContent?.trim(),
            'Card',
            'cem:if emits its body when the @test is truthy'
        );
        assertEqual(
            requiredElement(fullCard, 'span.kind').textContent?.trim(),
            'primary',
            'cem:choose selects the matching cem:when branch'
        );

        // Falsey `cem:if` test emits nothing; choose falls back to `cem:otherwise`.
        assert(emptyCard.querySelector('h3') === null, 'cem:if omits its body when the @test is falsey');
        assertEqual(
            requiredElement(emptyCard, 'span.kind').textContent?.trim(),
            'default',
            'cem:choose falls back to cem:otherwise when no cem:when matches'
        );
    },
};

// ---------------------------------------------------------------------------
// Runtime slice C2.5 — declarative slot projection: the produced instance's
// payload is distributed into <slot> positions in the light DOM.
// ---------------------------------------------------------------------------

export const SlotProjectionRenderLoop: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'slot projection story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-slot' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-slot');
        declaration.setAttribute('tag', 'story-slot-card');
        const template = document.createElement('template');
        template.innerHTML = [
            '<div class="card">',
            '<slot name="leading"><em class="fallback">none</em></slot>',
            '<div class="body"><slot></slot></div>',
            '<slot name="trailing"></slot>',
            '</div>',
        ].join('');
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const full = document.createElement('story-slot-card');
        full.innerHTML = '<span slot="leading">L</span>Body text<strong>Body node</strong><span slot="trailing">T</span>';
        const empty = document.createElement('story-slot-card');
        root.append(full, empty);

        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();

        const instances = canvasElement.querySelectorAll('story-slot-card');
        const full = instances[0];
        const empty = instances[1];

        // Every <slot> is resolved away in light DOM (replaced by payload or fallback).
        assert(full.querySelector('slot') === null, 'slots are projected away in light DOM');

        const fullCard = requiredElement(full, 'div.card');
        assertEqual(
            fullCard.querySelector('[slot="leading"]')?.textContent,
            'L',
            'named leading slot receives the matching payload'
        );
        assertEqual(
            requiredElement(fullCard, '.body').textContent?.trim(),
            'Body textBody node',
            'default slot receives unslotted text and element payload in source order'
        );
        assertEqual(
            fullCard.querySelector('[slot="trailing"]')?.textContent,
            'T',
            'named trailing slot receives the matching payload'
        );

        // With no payload, each slot falls back to its own default content.
        const emptyCard = requiredElement(empty, 'div.card');
        assert(emptyCard.querySelector('slot') === null, 'unfilled instance slots are also resolved');
        assertEqual(
            emptyCard.querySelector('.fallback')?.textContent,
            'none',
            'an unfilled named slot shows its fallback content'
        );

        const island = requiredElement(full, 'template[data-cem-island="instance"]') as HTMLTemplateElement;
        const leadingPayload = island.content.querySelector('[slot="leading"]');
        assert(leadingPayload !== null, 'serialized slot source remains in the inert island');
        leadingPayload.textContent = 'L2';
        await nextFrame();
        await nextFrame();
        assertEqual(
            requiredElement(full, 'div.card').querySelector('[slot="leading"]')?.textContent,
            'L2',
            'slot projection rerenders from the serialized payload after island mutation'
        );
    },
};

export const SlotProjectionRepeatedNames: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'repeated slot name story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-slot-dup' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-slot-dup');
        declaration.setAttribute('tag', 'story-slot-dup');
        const template = document.createElement('template');
        // Two slots share the name `a`; a slottable is assigned to the first match only.
        template.innerHTML = [
            '<div class="card">',
            '<slot name="a"><em class="f1">f1</em></slot>',
            '<slot name="a"><em class="f2">f2</em></slot>',
            '</div>',
        ].join('');
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-slot-dup');
        instance.innerHTML = '<span slot="a">X</span>';
        root.appendChild(instance);

        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();

        const card = requiredElement(canvasElement, 'story-slot-dup div.card');
        assert(card.querySelector('slot') === null, 'all repeated slots resolve away');
        assertEqual(
            card.querySelector('[slot="a"]')?.textContent,
            'X',
            'the first matching slot receives the single payload'
        );
        assert(card.querySelector('.f1') === null, 'the first slot drops its fallback once filled');
        assertEqual(
            card.querySelector('.f2')?.textContent,
            'f2',
            'a repeated same-name slot falls back when the payload is already consumed'
        );
    },
};

export const SlotProjectionWasmRenderLoop: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'wasm slot projection story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-slot-wasm' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-slot-wasm');
        declaration.setAttribute('tag', 'story-slot-wasm-card');
        const template = document.createElement('template');
        template.setAttribute('type', 'text/cem-ml');
        template.textContent =
            '{div @class=card | {slot @name=leading | {em @class=fallback | none}}{div @class=body | {slot | empty}}}';
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const full = document.createElement('story-slot-wasm-card');
        full.innerHTML = '<span slot="leading">L</span>Body text';
        const empty = document.createElement('story-slot-wasm-card');
        root.append(full, empty);

        return root;
    },
    play: async ({ canvasElement }) => {
        const instances = canvasElement.querySelectorAll('story-slot-wasm-card');
        const fullCard = await waitForElement(instances[0], 'div.card');
        const emptyCard = await waitForElement(instances[1], 'div.card');

        assert(instances[0].querySelector('slot') === null, 'WASM-rendered slots are projected out of the plan');
        assertEqual(
            fullCard.querySelector('[slot="leading"]')?.textContent,
            'L',
            'WASM path projects named payload from the serialized snapshot'
        );
        assertEqual(
            requiredElement(fullCard, '.body').textContent?.trim(),
            'Body text',
            'WASM path projects default payload from the serialized snapshot'
        );
        assertEqual(
            emptyCard.querySelector('.fallback')?.textContent,
            'none',
            'WASM path keeps slot fallback when no payload is assigned'
        );
    },
};

export const RuntimeDiagnosticsSurface: Story = {
    render: () => storyPanel('Runtime diagnostics', 'declaration and render diagnostics remain queryable'),
    play: async ({ canvasElement }) => {
        const root = document.createElement('section');
        canvasElement.appendChild(root);

        const parserRuntime = new CemElementRuntime({ declarationTag: 'cem-element-story-parser-diagnostic' });
        parserRuntime.install(window);
        const parserDeclaration = document.createElement('cem-element-story-parser-diagnostic');
        parserDeclaration.setAttribute('tag', 'story-parser-diagnostic');
        const parserTemplate = document.createElement('template');
        parserTemplate.setAttribute('type', 'text/cem-ml');
        parserTemplate.textContent = '{p Hello {.name}}';
        parserDeclaration.appendChild(parserTemplate);
        root.appendChild(parserDeclaration);
        parserRuntime.registerDeclaration(parserDeclaration);

        await parserRuntime.whenDeclarationSettled(parserDeclaration);
        assertDiagnostic(parserRuntime.diagnosticsFor(parserDeclaration), 'cem.tokenizer.bare_brace_text');

        const renderRuntime = new CemElementRuntime({ declarationTag: 'cem-element-story-render-diagnostic' });
        renderRuntime.install(window);
        const renderDeclaration = document.createElement('cem-element-story-render-diagnostic');
        renderDeclaration.setAttribute('tag', 'story-render-diagnostic');
        const renderTemplate = document.createElement('template');
        renderTemplate.setAttribute('type', 'text/cem-ml');
        renderTemplate.textContent = '{$ | name}';
        renderDeclaration.appendChild(renderTemplate);
        root.appendChild(renderDeclaration);
        renderRuntime.registerDeclaration(renderDeclaration);

        const instance = document.createElement('story-render-diagnostic');
        root.appendChild(instance);
        await renderRuntime.whenRenderSettled(instance);

        assertDiagnostic(renderRuntime.diagnosticsFor(instance), 'cem.ql.render.compile_failed');
    },
};

export const AttributeInvalidationRerenders: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'attribute invalidation story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-attr' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-attr');
        declaration.setAttribute('tag', 'story-attr-label');
        const template = document.createElement('template');
        template.innerHTML = `
            <attribute name="label">Save</attribute>
            <span>${'${$label}'}</span>
        `;
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-attr-label');
        root.appendChild(instance);

        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();

        const instance = requiredElement(canvasElement, 'story-attr-label');
        assertEqual(requiredElement(instance, 'span').textContent, 'Save', 'default attribute value renders first');

        instance.setAttribute('label', 'Updated');
        await nextFrame();

        assertEqual(
            requiredElement(instance, 'span').textContent,
            'Updated',
            'observed host attribute changes trigger rerender'
        );
    },
};

export const SliceEventInvalidationRerenders: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'slice event invalidation story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-slice' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-slice');
        declaration.setAttribute('tag', 'story-slice-field');
        const template = document.createElement('template');
        template.innerHTML = `
            <slice name="query"></slice>
            <label>
                Query
                <input
                    type="text"
                    value="{$query}"
                    slice="query"
                    slice-event="input"
                    slice-value="{$target.value}"
                />
            </label>
            <output>${'${$query}'}</output>
        `;
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-slice-field');
        root.appendChild(instance);

        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();

        const instance = requiredElement(canvasElement, 'story-slice-field');
        const input = requiredElement(instance, 'input') as HTMLInputElement;
        assert(!input.hasAttribute('slice-event'), 'slice-event binding metadata should not remain visible');

        input.value = 'Tokens';
        input.dispatchEvent(new Event('input', { bubbles: true }));
        await nextFrame();

        assertEqual(
            requiredElement(instance, 'output').textContent,
            'Tokens',
            'slice-event updates data-island state and triggers rerender'
        );
        assertEqual(
            (requiredElement(instance, 'input') as HTMLInputElement).getAttribute('value'),
            'Tokens',
            'rerendered controls receive the updated slice value'
        );
    },
};

// ---------------------------------------------------------------------------
// Runtime slice E — source-map / render identity metadata + diagnostics surface.
// ---------------------------------------------------------------------------

export const RenderMetadataPropagatesToNestedDomNodes: Story = {
    render: () =>
        renderInstanceStory({
            declarationTag: 'cem-element-story-meta-dom',
            producedTag: 'story-meta-card',
            ariaLabel: 'render metadata propagation story',
            innerHTML:
                '<attribute name="label">Hi</attribute>' +
                '<section class="card"><button type="button"><span>${$label}</span></button></section>',
            attributes: { label: 'Tokens' },
        }),
    play: async ({ canvasElement }) => {
        await nextFrame();

        const instance = requiredElement(canvasElement, 'story-meta-card');
        const section = requiredElement(instance, 'section.card');
        const button = requiredElement(instance, 'button');
        const span = requiredElement(instance, 'span');

        const artifactId = section.getAttribute('data-cem-template-artifact-id');
        assert(artifactId !== null && artifactId.length > 0, 'rendered nodes carry a template artifact id');

        assertEqual(
            section.getAttribute('data-cem-render-node-id'),
            'story-meta-card-1',
            'root render-node id is deterministic and producedTag-scoped'
        );
        assertEqual(
            button.getAttribute('data-cem-render-node-id'),
            'story-meta-card-2',
            'nested render-node ids increment in pre-order'
        );
        assertEqual(
            span.getAttribute('data-cem-render-node-id'),
            'story-meta-card-3',
            'deepest render-node id continues the sequence'
        );

        for (const el of [section, button, span]) {
            assertEqual(
                el.getAttribute('data-cem-template-artifact-id'),
                artifactId,
                'every rendered node shares the declaration artifact id'
            );
            assertEqual(el.getAttribute('data-cem-data-revision'), '1', 'every rendered node carries the first data revision');
            assertEqual(
                el.getAttribute('data-cem-source-fidelity'),
                'dom-canonical',
                'DOM parity nodes mark dom-canonical fidelity'
            );
        }

        assertEqual(section.getAttribute('data-cem-source-frame'), 'dom:1', 'root frame follows declaration child order');
        assertEqual(button.getAttribute('data-cem-source-frame'), 'dom:1/0', 'nested frame extends the parent frame');
        assertEqual(span.getAttribute('data-cem-source-frame'), 'dom:1/0/0', 'deepest frame extends the full path');

        assertEqual(span.textContent, 'Tokens', 'interpolated leaf still renders content alongside metadata');
    },
};

export const RenderMetadataAdvancesDataRevisionOnRerender: Story = {
    render: () =>
        renderInstanceStory({
            declarationTag: 'cem-element-story-meta-revision',
            producedTag: 'story-meta-revision',
            ariaLabel: 'render metadata revision story',
            innerHTML: '<attribute name="label">Save</attribute><button type="button">${$label}</button>',
        }),
    play: async ({ canvasElement }) => {
        await nextFrame();

        const instance = requiredElement(canvasElement, 'story-meta-revision');
        const first = requiredElement(instance, 'button');
        const nodeId = first.getAttribute('data-cem-render-node-id');
        const frame = first.getAttribute('data-cem-source-frame');
        assertEqual(first.getAttribute('data-cem-data-revision'), '1', 'first render carries data revision 1');

        instance.setAttribute('label', 'Updated');
        await nextFrame();

        const second = requiredElement(instance, 'button');
        assertEqual(second.getAttribute('data-cem-data-revision'), '2', 'rerender advances the data revision');
        assertEqual(
            second.getAttribute('data-cem-render-node-id'),
            nodeId,
            'render-node identity stays stable across rerenders'
        );
        assertEqual(second.getAttribute('data-cem-source-frame'), frame, 'source frame stays stable across rerenders');

        instance.setAttribute('label', 'Third');
        await nextFrame();
        assertEqual(
            requiredElement(instance, 'button').getAttribute('data-cem-data-revision'),
            '3',
            'each invalidation advances the data revision'
        );
    },
};

export const CemMlRenderMetadataCarriesAuthorByteFrames: Story = {
    render: () =>
        renderInstanceStory({
            declarationTag: 'cem-element-story-meta-cem',
            producedTag: 'story-meta-cem',
            ariaLabel: 'CEM-ML render metadata story',
            type: 'text/cem-ml',
            text: '{section @class=card | {button @type=button | {$label}}}',
            attributes: { label: 'Submit' },
        }),
    play: async ({ canvasElement }) => {
        await nextFrame();

        const instance = requiredElement(canvasElement, 'story-meta-cem');
        const section = requiredElement(instance, 'section');
        const button = requiredElement(instance, 'button');

        for (const el of [section, button]) {
            assert(el.hasAttribute('data-cem-render-node-id'), 'CEM-ML nodes carry render-node identity');
            assert(el.hasAttribute('data-cem-template-artifact-id'), 'CEM-ML nodes carry template artifact identity');
            assertEqual(el.getAttribute('data-cem-data-revision'), '1', 'CEM-ML nodes carry data revision');
            assertEqual(
                el.getAttribute('data-cem-source-fidelity'),
                'author-byte-exact',
                'raw-text CEM-ML subset nodes mark author-byte-exact fidelity'
            );
        }

        assertEqual(section.getAttribute('data-cem-source-frame'), 'cem:0', 'CEM-ML root frame is the source byte offset');
        const buttonFrame = button.getAttribute('data-cem-source-frame') ?? '';
        assert(/^cem:\d+$/.test(buttonFrame), 'CEM-ML nested frame is a source byte offset');
        assert(buttonFrame !== 'cem:0', 'nested CEM-ML frame differs from the root offset');

        assertEqual(
            section.getAttribute('data-cem-render-node-id'),
            'story-meta-cem-1',
            'CEM-ML render-node ids are deterministic'
        );
        assertEqual(
            button.getAttribute('data-cem-render-node-id'),
            'story-meta-cem-2',
            'CEM-ML nested render-node ids increment'
        );
        assertEqual(button.textContent?.trim(), 'Submit', 'CEM-ML leaf interpolation renders alongside metadata');
    },
};

export const TemplateArtifactIdentityIsStablePerDeclaration: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'template artifact identity story');

        registerInlineDeclaration({
            declarationTag: 'cem-element-story-artifact-a',
            producedTag: 'story-artifact-a',
            innerHTML: '<button type="button">A</button>',
        });
        registerInlineDeclaration({
            declarationTag: 'cem-element-story-artifact-b',
            producedTag: 'story-artifact-b',
            innerHTML: '<button type="button">B</button>',
        });

        root.append(
            document.createElement('story-artifact-a'),
            document.createElement('story-artifact-a'),
            document.createElement('story-artifact-b')
        );
        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();

        const aInstances = Array.from(canvasElement.querySelectorAll('story-artifact-a'));
        assertEqual(aInstances.length, 2, 'both instances of the shared declaration mount');
        const a1 = requiredElement(aInstances[0], 'button');
        const a2 = requiredElement(aInstances[1], 'button');
        const b = requiredElement(requiredElement(canvasElement, 'story-artifact-b'), 'button');

        const a1Id = a1.getAttribute('data-cem-template-artifact-id');
        const a2Id = a2.getAttribute('data-cem-template-artifact-id');
        const bId = b.getAttribute('data-cem-template-artifact-id');
        assert(a1Id !== null && a2Id !== null && bId !== null, 'all rendered buttons carry an artifact id');

        assertEqual(a1Id, a2Id, 'instances of one declaration share its template artifact identity');
        assert(a1Id !== bId, 'distinct declarations get distinct template artifact identities');
        assertEqual(
            a1.getAttribute('data-cem-render-node-id'),
            a2.getAttribute('data-cem-render-node-id'),
            'render-node ids are template-scoped and identical across instances'
        );
    },
};

export const RenderPlanMaterializationCarriesSourceMetadata: Story = {
    render: () => storyPanel('Materialize metadata', 'render plan nodes → light DOM with identity attributes'),
    play: () => {
        const plan: RenderPlan = {
            producedTag: 'cem-mat',
            instanceId: 'mat-instance-1',
            templateArtifactId: 'mat-artifact-7',
            dataRevision: '7',
            outputTarget: 'light-dom',
            scopePolicyStamp: 'mat-scope',
            nodes: [
                {
                    kind: 'element',
                    namespace: null,
                    tag: 'button',
                    attributes: [{ name: 'type', value: 'button' }],
                    renderNodeId: 'cem-mat-1',
                    children: [{ kind: 'text', text: 'Save' }],
                    sourceMapRef: { fidelity: 'declaration-only', frame: 'decl:0' },
                },
                {
                    kind: 'element',
                    namespace: null,
                    tag: 'span',
                    attributes: [],
                    renderNodeId: 'cem-mat-2',
                    children: [],
                },
            ],
        };

        const fragment = materializeRenderPlan(plan, document);
        const button = fragment.querySelector('button');
        const span = fragment.querySelector('span');
        assert(button !== null && span !== null, 'plan elements materialize into light DOM');

        assertEqual(button.getAttribute('data-cem-render-node-id'), 'cem-mat-1', 'render-node id is written from the plan');
        assertEqual(
            button.getAttribute('data-cem-template-artifact-id'),
            'mat-artifact-7',
            'template artifact id is written from the plan'
        );
        assertEqual(button.getAttribute('data-cem-data-revision'), '7', 'data revision is written from the plan');
        assertEqual(
            button.getAttribute('data-cem-source-fidelity'),
            'declaration-only',
            'the declaration-only fidelity marker is carried verbatim'
        );
        assertEqual(button.getAttribute('data-cem-source-frame'), 'decl:0', 'source frame is carried verbatim');
        assertEqual(button.getAttribute('type'), 'button', 'authored attributes survive alongside metadata');
        assertEqual(button.textContent, 'Save', 'text children materialize');

        assert(span.hasAttribute('data-cem-render-node-id'), 'nodes without a source map still carry render identity');
        assert(!span.hasAttribute('data-cem-source-fidelity'), 'nodes without a source map omit fidelity metadata');
        assert(!span.hasAttribute('data-cem-source-frame'), 'nodes without a source map omit frame metadata');
    },
};

export const RenderNodeIdentityIsDeterministic: Story = {
    render: () => storyPanel('Deterministic identity', 'repeated projection yields identical render-node ids'),
    play: () => {
        const source: TemplateSourceNode[] = [
            {
                kind: 'element',
                namespace: null,
                tag: 'ul',
                attributes: [],
                children: [
                    { kind: 'element', namespace: null, tag: 'li', attributes: [], children: [] },
                    { kind: 'element', namespace: null, tag: 'li', attributes: [], children: [] },
                ],
            },
        ];
        const snapshot = projectionSnapshot('cem-list', {});
        const first = projectTemplate(source, { snapshot, values: {} });
        const second = projectTemplate(source, { snapshot, values: {} });

        const collectIds = (plan: RenderPlan): string[] => {
            const ids: string[] = [];
            const walk = (node: RenderPlanNode): void => {
                if (node.kind === 'element') {
                    ids.push(node.renderNodeId);
                    node.children.forEach(walk);
                }
            };
            plan.nodes.forEach(walk);
            return ids;
        };

        const firstIds = collectIds(first);
        assertEqual(
            firstIds.join(','),
            'cem-list-1,cem-list-2,cem-list-3',
            'render-node ids follow a deterministic pre-order sequence'
        );
        assertEqual(collectIds(second).join(','), firstIds.join(','), 'identical source projects to identical render-node ids');
        assertEqual(new Set(firstIds).size, firstIds.length, 'render-node ids are unique within a plan');
    },
};

export const SsrHydrationFromSerializedSnapshot: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'SSR hydration fixture');

        const templateHtml =
            '<attribute name="label">Fallback</attribute>' +
            '<article class="ssr-card">' +
            '<h2>${$label}</h2>' +
            '<div class="detail"><slot name="detail"></slot></div>' +
            '</article>';
        const template = document.createElement('template');
        template.innerHTML = templateHtml;
        const source = readTemplateSource(template.content);
        const snapshot = projectionSnapshot('story-ssr-card', { label: 'Server Card' });
        snapshot.instanceId = 'ssr-instance-1';
        snapshot.declarationTag = 'cem-element-story-ssr';
        snapshot.templateArtifactId = 'ssr-template-artifact-1';
        snapshot.dataRevision = '7';
        snapshot.payload = {
            ...emptySerializedPayload(),
            text: 'Server detail',
            childCount: 1,
            nodes: [
                {
                    kind: 'element',
                    key: 'payload-0',
                    tag: 'span',
                    namespace: null,
                    attributes: { slot: 'detail' },
                    slot: 'detail',
                    children: [{ kind: 'text', key: 'payload-0/0', text: 'Server detail' }],
                },
            ],
            slots: {
                detail: [
                    {
                        kind: 'element',
                        key: 'payload-0',
                        tag: 'span',
                        namespace: null,
                        attributes: { slot: 'detail' },
                        slot: 'detail',
                        children: [{ kind: 'text', key: 'payload-0/0', text: 'Server detail' }],
                    },
                ],
            },
        };

        const plan = projectTemplate(source, { snapshot, values: { label: 'Server Card' } });
        const serverFragment = materializeRenderPlan(plan, document);
        const serverNodes = Array.from(serverFragment.childNodes);

        registerInlineDeclaration({
            declarationTag: 'cem-element-story-ssr',
            producedTag: 'story-ssr-card',
            innerHTML: templateHtml,
        });

        const instance = document.createElement('story-ssr-card');
        instance.setAttribute('label', 'Server Card');
        const island = document.createElement('template');
        island.setAttribute('data-cem-island', 'instance');
        island.innerHTML = '<span slot="detail">Server detail</span>';
        const metadata = document.createElement('script');
        metadata.setAttribute('type', 'application/json');
        metadata.setAttribute('data-cem-hydration', 'snapshot');
        metadata.textContent = JSON.stringify(snapshot);
        instance.append(
            island,
            document.createComment('cem-render-start'),
            ...serverNodes,
            document.createComment('cem-render-end'),
            metadata
        );
        root.append(instance);
        return root;
    },
    play: async ({ canvasElement }) => {
        const instance = requiredElement(canvasElement, 'story-ssr-card') as HTMLElement;
        const article = await waitForElement(instance, 'article.ssr-card');
        assertEqual(article.querySelector('h2')?.textContent, 'Server Card', 'SSR HTML renders from the serialized snapshot');
        assertEqual(
            article.getAttribute('data-cem-template-artifact-id'),
            'ssr-template-artifact-1',
            'client hydration preserves the server render-plan artifact identity'
        );
        assertEqual(
            article.getAttribute('data-cem-data-revision'),
            '7',
            'client hydration preserves the server render-plan data revision'
        );
        assertEqual(
            requiredElement(instance, 'script[data-cem-hydration="snapshot"]').textContent?.includes('ssr-instance-1'),
            true,
            'hydration metadata carries the serialized DataIslandSnapshot'
        );
        const island = requiredElement(instance, 'template[data-cem-island="instance"]') as HTMLTemplateElement;
        assertEqual(
            island.content.querySelector('[slot="detail"]')?.textContent,
            'Server detail',
            'client hydration keeps the same instance data island payload'
        );
        assertEqual(
            article.querySelector('.detail')?.textContent?.trim(),
            'Server detail',
            'SSR slot projection is visible after client hydration'
        );

        instance.setAttribute('label', 'Client Card');
        await waitForCondition(
            () => requiredElement(instance, 'article.ssr-card').querySelector('h2')?.textContent === 'Client Card',
            'client-side invalidation takes over after hydration'
        );
    },
};

export const EdgePatchFramesFromSerializedSnapshot: Story = {
    render: () => storyPanel('Edge patch frames', 'serialized snapshot + previous render-plan identity → patch stream'),
    play: () => {
        const templateHtml =
            '<attribute name="label">Fallback</attribute>' +
            '<article class="edge-card" data-kind="summary">' +
            '<h2>${$label}</h2>' +
            '<p class="detail"><slot name="detail"></slot></p>' +
            '</article>';
        const template = document.createElement('template');
        template.innerHTML = templateHtml;
        const source = readTemplateSource(template.content);

        const previousSnapshot = edgeProjectionSnapshot('Edge Before', '11');
        const nextSnapshot = edgeProjectionSnapshot('Edge After', '12');
        const previousPlan = projectTemplate(source, { snapshot: previousSnapshot, values: { label: 'Edge Before' } });
        const nextPlan = projectTemplate(source, { snapshot: nextSnapshot, values: { label: 'Edge After' } });
        const frames = diffRenderPlansToPatchFrames(previousPlan, nextPlan, {
            batchSize: 1,
            transactionId: 'edge-tx-1',
        });

        assertEqual(frames[0].type, 'begin', 'edge stream starts with a begin frame');
        assertEqual(frames[0].transactionId, 'edge-tx-1', 'all frames share the edge transaction id');
        assertEqual(
            frames[0].revision.dataRevision,
            '12',
            'begin frame names the next serialized snapshot revision'
        );

        const ops = opsFromPatchFrames(frames);
        const textPatch = ops.find((op) => op.op === 'setText');
        assert(textPatch?.op === 'setText', 'edge diff emits a text patch without live DOM access');
        assertEqual(textPatch.value, 'Edge After', 'text patch carries the next snapshot value');
        assert(
            !ops.some((op) => op.op === 'replaceScope'),
            'same-template edge diffs use stable render-node patches instead of scope replacement'
        );

        const commit = frames.at(-1);
        assert(commit?.type === 'commit', 'edge stream ends with a commit frame');
        assertEqual(
            JSON.stringify(commit.nextRenderPlan),
            JSON.stringify(renderPlanIdentity(nextPlan)),
            'commit carries the next render-plan identity for edge state storage'
        );
    },
};

export const BrowserToEdgeSnapshotPrivacyPolicy: Story = {
    render: () => storyPanel('Edge snapshot privacy', 'policy-denied data is omitted or redacted before export'),
    play: () => {
        const snapshot = edgeProjectionSnapshot('Sensitive Label', '13');
        snapshot.privacyPolicyStamp = 'browser-local-policy-v1';
        snapshot.hostAttributes = {
            label: 'Allowed Label',
        };
        snapshot.dataset = { analyticsId: 'visitor-42' };
        snapshot.payload = {
            ...snapshot.payload,
            text: 'Sensitive detail',
            data: [
                {
                    kind: 'data',
                    key: 'data-0',
                    value: 'secret',
                    label: 'Secret',
                    text: 'Sensitive data',
                    attributes: { value: 'secret' },
                    group: null,
                },
            ],
            dataByValue: {
                secret: {
                    kind: 'data',
                    key: 'data-0',
                    value: 'secret',
                    label: 'Secret',
                    text: 'Sensitive data',
                    attributes: { value: 'secret' },
                    group: null,
                },
            },
        };
        snapshot.slices = { typed: 'draft input' };
        snapshot.validationState = { valid: false, message: 'private validation detail' };
        snapshot.eventPayloads = { input: { value: 'raw browser event payload' } };

        const defaultExport = exportDataIslandSnapshotForEdge(snapshot);
        assert(!('hostAttributes' in defaultExport), 'default edge export omits host attributes');
        assert(!('payload' in defaultExport), 'default edge export omits payload');
        assert(!('validationState' in defaultExport), 'default edge export omits validation state');

        const exported = exportDataIslandSnapshotForEdge(snapshot, {
            privacyPolicyStamp: 'edge-export-policy-v1',
            fields: {
                hostAttributes: 'allow',
                payload: 'redact',
                validationState: 'redact',
                dataset: 'omit',
                slices: 'omit',
                eventPayloads: 'omit',
            },
        });

        assertEqual(exported.privacyPolicyStamp, 'edge-export-policy-v1', 'export records the effective edge policy');
        assertEqual(exported.hostAttributes?.label, 'Allowed Label', 'allowed host attributes are exported');
        assert(!('dataset' in exported), 'denied dataset fields are omitted before edge transport');
        assert(!('slices' in exported), 'transient slice state is omitted before edge transport');
        assert(!('eventPayloads' in exported), 'raw event payloads are omitted before edge transport');
        assertEqual(exported.payload?.text, '', 'redacted payload text is cleared');
        assertEqual(exported.payload?.childCount, 0, 'redacted payload child count is cleared');
        assertEqual(exported.payload?.data.length, 0, 'redacted data payload choices are cleared');
        assertEqual(
            Object.keys(exported.payload?.dataByValue ?? {}).length,
            0,
            'redacted payload lookup records are cleared'
        );
        assertEqual(
            Object.keys(exported.validationState ?? {}).length,
            0,
            'redacted validation state is present but empty'
        );

        snapshot.hostAttributes.label = 'Mutated After Export';
        assertEqual(
            exported.hostAttributes?.label,
            'Allowed Label',
            'exported edge snapshots are detached from later browser mutation'
        );
    },
};

export const DeclarationDiagnosticsAreExposed: Story = {
    render: () => storyPanel('Declaration diagnostics', 'invalid declaration shapes surface through diagnosticsFor'),
    play: () => {
        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-decl-diagnostic' });

        const invalidTag = buildDeclaration({ tag: 'Bad-Tag', templates: [{ html: '<button>x</button>' }] });
        runtime.registerDeclaration(invalidTag);
        const tagDiagnostic = findDiagnostic(runtime.diagnosticsFor(invalidTag), 'cem-element.tag_invalid');
        assertEqual(tagDiagnostic.source, 'declaration', 'tag diagnostics are declaration-sourced');
        assertEqual(tagDiagnostic.severity, 'error', 'an invalid tag is an error-severity diagnostic');

        const missingTag = buildDeclaration({ templates: [{ html: '<button>x</button>' }] });
        runtime.registerDeclaration(missingTag);
        assertDiagnostic(runtime.diagnosticsFor(missingTag), 'cem-element.tag_missing');

        const conflict = buildDeclaration({
            tag: 'story-decl-conflict',
            src: './x.cem#x',
            templates: [{ html: '<button>x</button>' }],
        });
        runtime.registerDeclaration(conflict);
        assertDiagnostic(runtime.diagnosticsFor(conflict), 'cem-element.src_inline_template_conflict');

        const srcMissing = buildDeclaration({ tag: 'story-decl-src', src: '#no-such-template' });
        runtime.registerDeclaration(srcMissing);
        assertDiagnostic(runtime.diagnosticsFor(srcMissing), 'cem-element.src_local_target_missing');

        const noTemplate = buildDeclaration({ tag: 'story-decl-empty' });
        runtime.registerDeclaration(noTemplate);
        assertDiagnostic(runtime.diagnosticsFor(noTemplate), 'cem-element.inline_template_count');

        const liveContent = buildDeclaration({
            tag: 'story-decl-live',
            templates: [{ html: '<button>x</button>' }],
            liveContent: true,
        });
        runtime.registerDeclaration(liveContent);
        assertDiagnostic(runtime.diagnosticsFor(liveContent), 'cem-element.declaration_live_content');

        const firstDefine = buildDeclaration({
            tag: 'story-decl-duplicate',
            templates: [{ html: '<button>first</button>' }],
        });
        runtime.registerDeclaration(firstDefine);
        assertEqual(runtime.diagnosticsFor(firstDefine).length, 0, 'a valid declaration registers without diagnostics');
        const secondDefine = buildDeclaration({
            tag: 'story-decl-duplicate',
            templates: [{ html: '<button>second</button>' }],
        });
        runtime.registerDeclaration(secondDefine);
        assertDiagnostic(runtime.diagnosticsFor(secondDefine), 'cem-element.tag_already_defined');
    },
};

export const CemMlParseDiagnosticsAreExposed: Story = {
    render: () => storyPanel('CEM-ML parse diagnostics', 'malformed CEM-ML surfaces parser diagnostics'),
    play: async () => {
        const cases: Array<[string, string]> = [
            ['{p Hello {.name}}', 'cem.tokenizer.bare_brace_text'],
            ['{button @type=button | x', 'cem.tokenizer.unterminated_node'],
            ['{button @title={oops', 'cem.tokenizer.unterminated_avt_span'],
        ];

        for (const [index, [template, code]] of cases.entries()) {
            const runtime = new CemElementRuntime({ declarationTag: `cem-element-story-parse-${index}` });
            const declaration = buildDeclaration({
                tag: `story-parse-case-${index}`,
                templates: [{ type: 'text/cem-ml', text: template }],
            });
            runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
            const diagnostic = findDiagnostic(runtime.diagnosticsFor(declaration), code);
            assertEqual(diagnostic.source, 'declaration', 'parse diagnostics are declaration-sourced');
        }
    },
};

export const RenderFailureDiagnosticsAreExposed: Story = {
    render: () => storyPanel('Render diagnostics', 'render-time failures surface through diagnosticsFor'),
    play: async ({ canvasElement }) => {
        const root = document.createElement('section');
        canvasElement.appendChild(root);

        // A healthy render leaves the instance free of diagnostics.
        const cleanRuntime = new CemElementRuntime({ declarationTag: 'cem-element-story-render-clean' });
        const cleanDeclaration = buildDeclaration({
            tag: 'story-render-clean',
            templates: [{ html: '<button type="button">ok</button>' }],
        });
        cleanRuntime.registerDeclaration(cleanDeclaration);
        const cleanInstance = document.createElement('story-render-clean');
        root.appendChild(cleanInstance);
        await nextFrame();
        assertEqual(cleanRuntime.diagnosticsFor(cleanInstance).length, 0, 'a healthy render emits no instance diagnostics');

        // Malformed CEM-ML reports compile diagnostics through the async WASM render path.
        const failRuntime = new CemElementRuntime({ declarationTag: 'cem-element-story-render-fail' });
        const failDeclaration = buildDeclaration({
            tag: 'story-render-fail',
            templates: [{ type: 'text/cem-ml', text: '{$ | name}' }],
        });
        failRuntime.registerDeclaration(failDeclaration);
        const failInstance = document.createElement('story-render-fail');
        root.appendChild(failInstance);
        await failRuntime.whenRenderSettled(failInstance);
        const renderFailure = findDiagnostic(failRuntime.diagnosticsFor(failInstance), 'cem.ql.render.compile_failed');
        assertEqual(renderFailure.source, 'render', 'render failures are render-sourced');
        assertEqual(renderFailure.severity, 'error', 'render failures are error-severity diagnostics');

        // Legacy bridge templates are a supported migration path and should not
        // report the old reserved-slice diagnostic.
        const legacyRuntime = new CemElementRuntime({ declarationTag: 'cem-element-story-render-legacy' });
        const legacyDeclaration = buildDeclaration({
            tag: 'story-render-legacy',
            templates: [{ lang: 'custom-element-v0', html: '<button>x</button>' }],
        });
        legacyRuntime.registerDeclaration(legacyDeclaration);
        const legacyInstance = document.createElement('story-render-legacy');
        root.appendChild(legacyInstance);
        await legacyRuntime.whenRenderSettled(legacyInstance);
        assertEqual(legacyRuntime.diagnosticsFor(legacyDeclaration).length, 0, 'legacy declaration emits no diagnostic');
        assertEqual(legacyRuntime.diagnosticsFor(legacyInstance).length, 0, 'legacy render emits no diagnostic');
    },
};

function storyPanel(title: string, body: string): HTMLElement {
    const section = document.createElement('section');
    const heading = document.createElement('h2');
    const pre = document.createElement('pre');
    heading.textContent = title;
    pre.textContent = body;
    section.append(heading, pre);
    return section;
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) {
        throw new Error(message);
    }
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
    if (actual !== expected) {
        throw new Error(`${label}: expected ${String(expected)}, got ${String(actual)}`);
    }
}

function assertDiagnostic(diagnostics: readonly { code: string }[], code: string): void {
    assert(
        diagnostics.some((diagnostic) => diagnostic.code === code),
        `expected diagnostic ${code}`
    );
}

function findDiagnostic(diagnostics: readonly CemElementDiagnostic[], code: string): CemElementDiagnostic {
    const diagnostic = diagnostics.find((entry) => entry.code === code);
    assert(diagnostic, `expected diagnostic ${code}`);
    return diagnostic;
}

interface InlineDeclarationOptions {
    declarationTag: string;
    producedTag: string;
    ariaLabel?: string;
    innerHTML?: string;
    text?: string;
    type?: string;
    attributes?: Record<string, string>;
}

/**
 * Register an inline declaration directly (no install / no auto-registration) so the
 * produced custom element is defined and ready to upgrade. The declaration host is a
 * plain element, which keeps `registerDeclaration` from running twice on connect.
 */
function registerInlineDeclaration(options: InlineDeclarationOptions): CemElementRuntime {
    const runtime = new CemElementRuntime({ declarationTag: options.declarationTag });
    const declaration = document.createElement('div');
    declaration.setAttribute('tag', options.producedTag);
    const template = document.createElement('template');
    if (options.type) {
        template.setAttribute('type', options.type);
    }
    if (options.innerHTML !== undefined) {
        template.innerHTML = options.innerHTML;
    }
    if (options.text !== undefined) {
        template.textContent = options.text;
    }
    declaration.appendChild(template);
    runtime.registerDeclaration(declaration);
    return runtime;
}

/**
 * Build a detached, mounted instance story: register the declaration, create the
 * instance, and return a root the harness will connect (driving the render loop).
 */
function renderInstanceStory(options: InlineDeclarationOptions): HTMLElement {
    const root = document.createElement('section');
    if (options.ariaLabel) {
        root.setAttribute('aria-label', options.ariaLabel);
    }
    registerInlineDeclaration(options);
    const instance = document.createElement(options.producedTag);
    for (const [name, value] of Object.entries(options.attributes ?? {})) {
        instance.setAttribute(name, value);
    }
    root.appendChild(instance);
    return root;
}

interface DeclarationTemplateSpec {
    type?: string;
    lang?: string;
    html?: string;
    text?: string;
}

interface DeclarationSpec {
    tag?: string;
    src?: string;
    templates?: DeclarationTemplateSpec[];
    liveContent?: boolean;
}

/** Assemble a declaration host element to feed `registerDeclaration` for diagnostics checks. */
function buildDeclaration(spec: DeclarationSpec): HTMLElement {
    const declaration = document.createElement('div');
    if (spec.tag !== undefined) {
        declaration.setAttribute('tag', spec.tag);
    }
    if (spec.src !== undefined) {
        declaration.setAttribute('src', spec.src);
    }
    for (const templateSpec of spec.templates ?? []) {
        const template = document.createElement('template');
        if (templateSpec.type) {
            template.setAttribute('type', templateSpec.type);
        }
        if (templateSpec.lang) {
            template.setAttribute('lang', templateSpec.lang);
        }
        if (templateSpec.html !== undefined) {
            template.innerHTML = templateSpec.html;
        }
        if (templateSpec.text !== undefined) {
            template.textContent = templateSpec.text;
        }
        declaration.appendChild(template);
    }
    if (spec.liveContent) {
        const live = document.createElement('p');
        live.textContent = 'live page content';
        declaration.appendChild(live);
    }
    return declaration;
}

function requiredElement(root: ParentNode, selector: string): Element {
    const element = root.querySelector(selector);
    assert(element, `expected ${selector} to exist`);
    return element;
}

function nextFrame(): Promise<void> {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

/** Concatenated, trimmed text content of a render-plan node list (for WASM-boundary assertions). */
function textOfNodes(nodes: readonly RenderPlanNode[]): string {
    return nodes
        .map((node) => {
            if (node.kind === 'text') {
                return node.text;
            }
            return node.kind === 'element' ? textOfNodes(node.children) : '';
        })
        .join('')
        .trim();
}

/** Poll animation frames until a selector resolves — used for the async WASM render path. */
async function waitForElement(root: ParentNode, selector: string, frames = 120): Promise<Element> {
    for (let attempt = 0; attempt < frames; attempt += 1) {
        const found = root.querySelector(selector);
        if (found) {
            return found;
        }
        await nextFrame();
    }
    throw new Error(`expected ${selector} to appear within ${frames} frames`);
}

/** Poll animation frames until a predicate holds — used for async re-render assertions. */
async function waitForCondition(predicate: () => boolean, message: string, frames = 120): Promise<void> {
    for (let attempt = 0; attempt < frames; attempt += 1) {
        if (predicate()) {
            return;
        }
        await nextFrame();
    }
    throw new Error(`${message} within ${frames} frames`);
}

function projectionSnapshot(
    producedTag: string,
    hostAttributes: Record<string, string | boolean | null>
): DataIslandSnapshot {
    return {
        instanceId: 'story-instance-1',
        producedTag,
        declarationTag: 'cem-element-story-projection',
        templateArtifactId: 'story-template-artifact-1',
        dataRevision: '1',
        outputTarget: 'light-dom',
        scopePolicyStamp: 'story-scope',
        privacyPolicyStamp: 'story-privacy',
        hostAttributes,
        dataset: {},
        payload: emptySerializedPayload(),
        slices: {},
        validationState: {},
        eventPayloads: {},
    };
}

function edgeProjectionSnapshot(label: string, dataRevision: string): DataIslandSnapshot {
    const snapshot = projectionSnapshot('story-edge-card', { label });
    snapshot.instanceId = 'edge-instance-1';
    snapshot.declarationTag = 'cem-element-story-edge';
    snapshot.templateArtifactId = 'edge-template-artifact-1';
    snapshot.dataRevision = dataRevision;
    snapshot.scopePolicyStamp = 'edge-scope';
    snapshot.payload = {
        ...emptySerializedPayload(),
        text: 'Edge detail',
        childCount: 1,
        nodes: [
            {
                kind: 'element',
                key: 'edge-payload-0',
                tag: 'span',
                namespace: null,
                attributes: { slot: 'detail' },
                slot: 'detail',
                children: [{ kind: 'text', key: 'edge-payload-0/0', text: 'Edge detail' }],
            },
        ],
        slots: {
            detail: [
                {
                    kind: 'element',
                    key: 'edge-payload-0',
                    tag: 'span',
                    namespace: null,
                    attributes: { slot: 'detail' },
                    slot: 'detail',
                    children: [{ kind: 'text', key: 'edge-payload-0/0', text: 'Edge detail' }],
                },
            ],
        },
    };
    return snapshot;
}

function opsFromPatchFrames(frames: readonly PatchFrame[]) {
    return frames.flatMap((frame) => (frame.type === 'ops' ? frame.ops : []));
}

function emptySerializedPayload(): DataIslandSnapshot['payload'] {
    return {
        text: '',
        childCount: 0,
        nodes: [],
        slots: {},
        data: [],
        options: [],
        dataByValue: {},
        optionsByValue: {},
    };
}
