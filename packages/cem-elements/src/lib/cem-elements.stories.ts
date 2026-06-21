import type { Meta, StoryObj } from '@storybook/web-components-vite';
import {
    CemElementRuntime,
    SNAPSHOT_SCHEMA_VERSION,
    analyzeDeclarationShape,
    cemElements,
    exportDataIslandSnapshotForEdge,
    isValidCustomElementName,
    type CemElementDiagnostic,
    type DataIslandSnapshot,
    type SerializedEventPayload,
} from './cem-elements.js';
import {
    InMemoryEdgeRenderStateStore,
    advanceEdgeRenderState,
    applyRenderPlanToRange,
    createEdgeRenderStateRecord,
    diffRenderPlansToPatchFrames,
    edgeContentAddress,
    edgeRenderStateRevisionMatches,
    materializeRenderPlan,
    projectAndAdvanceEdgeRenderState,
    projectTemplate,
    readEdgeContent,
    readEdgeRenderStateContents,
    readTemplateSource,
    renderPlanIdentity,
    scopeRenderPlan,
    type EdgeContentAddress,
    type EdgeContentKind,
    type EdgeRenderStateRecord,
    type EdgeRenderStateStore,
    type EdgeRenderStateWriteOptions,
    type EdgeRenderStateWriteResult,
    type PatchFrame,
    type RenderPlan,
    type RenderPlanNode,
    type TemplateSourceNode,
} from './projection.js';
import {
    processCemMlTemplate,
    renderCemMlTemplate,
    runtimeVersion,
    type RuntimeSupportDiagnostic,
} from './internal/runtime-support/cem-ql-render.js';
import { domToRecord, normalizeSpace, tokenTableRows } from './data-document.js';

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

export const DataDocumentDomBridge: Story = {
    render: () =>
        storyPanel(
            'DOM → datadom bridge',
            'native DOM queries shape a parsed token document into cem-ql row records (slice 3)'
        ),
    play: () => {
        // A token document fragment shaped like the cem-theme token XHTML: an id'd section heading
        // followed by a data table — the legacy `*[@id]/following-sibling::table[1]`.
        const root = document.createElement('div');
        root.innerHTML = `
            <h6 id="cem-coupling-minimums">minimums</h6>
            <table>
                <thead><tr><th>Token</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td> --cem-gap </td><td>  0.5rem  </td></tr>
                    <tr><td>--cem-inset</td><td>1rem</td></tr>
                </tbody>
            </table>
        `;

        const rows = tokenTableRows(root, 'cem-coupling-minimums');
        assertEqual(rows.length, 2, 'two tbody rows are projected (thead excluded)');
        assertEqual(rows[0].td1, '--cem-gap', 'cell text is whitespace-normalized (row 1, td1)');
        assertEqual(rows[0].td2, '0.5rem', 'cell text is whitespace-normalized (row 1, td2)');
        assertEqual(rows[1].td1, '--cem-inset', 'row 2 td1');
        assertEqual(rows[1].td2, '1rem', 'row 2 td2');

        assertEqual(tokenTableRows(root, 'missing-anchor').length, 0, 'a missing anchor yields no rows');
        assertEqual(normalizeSpace('  a   b \n c '), 'a b c', 'normalizeSpace collapses whitespace');

        const node = domToRecord(root.querySelector('table')!);
        assertEqual(node.tag, 'table', 'generic record carries the element local name');
        assert(
            node.children.some((child) => child.tag === 'tbody'),
            'generic record exposes child elements'
        );
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

export const ImplicitCemMlTemplateShape: Story = {
    render: () => storyPanel('Implicit CEM-ML template', 'inline declarations may use direct CEM-ML content'),
    play: () => {
        const result = analyzeDeclarationShape({
            tag: 'cem-button',
            src: null,
            directTemplateCount: 0,
            directLiveNodeCount: 1,
        });
        assert(result.ok, 'inline declarations without a template should be accepted as implicit CEM-ML');
        assertEqual(result.diagnostics.length, 0, 'implicit CEM-ML declarations should not emit shape diagnostics');
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
        const [buttonText] = button.children;
        assert(buttonText.kind === 'text', 'WASM render carries a text render-plan child');
        assertEqual(
            buttonText.sourceMapRef?.fidelity,
            'author-byte-exact',
            'WASM text render-plan nodes carry author-byte-exact fidelity'
        );
        assert(/^cem:\d+$/.test(buttonText.sourceMapRef?.frame ?? ''), 'WASM text frames are source byte offsets');
        assertEqual(
            button.sourceMapRef?.fidelity,
            'author-byte-exact',
            'WASM render carries author-byte-exact fidelity'
        );
        assertEqual(button.sourceMapRef?.frame, 'cem:0', 'root frame is the source byte offset');

        const processed = await processCemMlTemplate({
            source: '{article @class="card {$tone}" | {slot @name=detail | fallback}}',
            data: { tone: 'primary' },
            identity: {
                producedTag: 'cem-processed',
                instanceId: 'processed-instance-1',
                templateArtifactId: 'processed-template-1',
                dataRevision: '1',
                outputTarget: 'light-dom',
                scopePolicyStamp: 'processed-scope',
            },
            payload: {
                slots: {
                    detail: [{ kind: 'text', key: 'payload-detail-0', text: 'Projected detail' }],
                },
            },
            previousRenderPlan: null,
            patchOptions: { transactionId: 'processed-tx-1' },
        });
        assertEqual(
            processed.diagnostics.length,
            0,
            'template processing boundary returns no diagnostics for well-formed source'
        );
        assertEqual(processed.renderPlan.producedTag, 'cem-processed', 'processing boundary carries render identity');
        assertEqual(processed.renderPlan.nodes.length, 1, 'processing boundary returns a light-DOM render plan');
        const [processedRoot] = processed.renderPlan.nodes;
        assert(processedRoot.kind === 'element', 'processed render-plan root is an element');
        assertEqual(processedRoot.tag, 'article', 'processing boundary preserves rendered root tag');
        assertEqual(
            processedRoot.attributes.find((attribute) => attribute.name === 'class')?.value,
            'card primary',
            'processing boundary runs CEM-QL expressions through WASM'
        );
        assertEqual(processedRoot.children.length, 1, 'processing boundary projects payload into slots');
        const [projectedDetail] = processedRoot.children;
        assert(projectedDetail.kind === 'text', 'projected slot payload remains render-plan data');
        assertEqual(projectedDetail.text, 'Projected detail', 'processing boundary returns slot-projected render data');
        assert(
            processed.patchFrames?.some(
                (frame) =>
                    frame.type === 'ops' &&
                    frame.ops.some(
                        (operation) => operation.op === 'replaceScope' && operation.reason === 'first-render'
                    )
            ),
            'processing boundary can return first-render patch frames without applying DOM'
        );

        // Diagnostics flow through the same boundary: an unknown binding compiles to a
        // mapped render diagnostic rather than throwing.
        const missing = await renderCemMlTemplate('{button | {$missing}}', {}, { renderNodeIdPrefix: 'cem-missing' });
        const missingDiagnostic = findRuntimeSupportDiagnostic(missing.diagnostics, 'cem.ql.render.compile_failed');
        assertEqual(
            missingDiagnostic.sourceMapRef?.fidelity,
            'author-byte-exact',
            'WASM render diagnostics carry author-byte-exact source-map fidelity'
        );
        assert(
            /^cem:\d+$/.test(missingDiagnostic.sourceMapRef?.frame ?? ''),
            'WASM render diagnostics carry source byte-offset frames'
        );
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

export const ProducedTagLifecycleBehavior: Story = {
    render: () => storyPanel('Produced tag lifecycle', 'idempotent registration, reconnect, nested tags, latest render wins'),
    play: async ({ canvasElement }) => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'produced tag lifecycle story');
        canvasElement.appendChild(root);

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-lifecycle' });
        runtime.install(window);
        runtime.install(window);
        assert(window.customElements.get('cem-element-story-lifecycle'), 'installing the runtime twice is idempotent');

        const childDeclaration = buildCemMlDeclaration(
            'cem-element-story-lifecycle',
            'story-lifecycle-child',
            '{attribute @name=label | Child}{strong | {$label}}'
        );
        root.appendChild(childDeclaration);
        assert(runtime.registerDeclaration(childDeclaration), 'manual registration after connected registration is accepted');
        assert(runtime.registerDeclaration(childDeclaration), 're-registering the same declaration is a no-op');
        await runtime.whenDeclarationSettled(childDeclaration);
        assertEqual(
            runtime.diagnosticsFor(childDeclaration).length,
            0,
            'same-declaration registration does not emit duplicate diagnostics'
        );

        const parentDeclaration = buildCemMlDeclaration(
            'cem-element-story-lifecycle',
            'story-lifecycle-parent',
            [
                '{attribute @name=label | Initial}',
                '{attribute @name=child | Nested}',
                '{article @class=card @data-tone="{$datadom.attributes.tone}" |',
                ' {span @class=label | {$label}}',
                ' {story-lifecycle-child @label="{$child}" | }',
                '}',
            ].join('')
        );
        root.appendChild(parentDeclaration);
        runtime.registerDeclaration(parentDeclaration);
        await runtime.whenDeclarationSettled(parentDeclaration);
        assertEqual(runtime.diagnosticsFor(parentDeclaration).length, 0, 'the first produced tag declaration is clean');

        const duplicateDeclaration = buildCemMlDeclaration(
            'cem-element-story-lifecycle',
            'story-lifecycle-parent',
            '{div @class=duplicate | Duplicate}'
        );
        root.appendChild(duplicateDeclaration);
        runtime.registerDeclaration(duplicateDeclaration);
        await runtime.whenDeclarationSettled(duplicateDeclaration);
        assertDiagnostic(runtime.diagnosticsFor(duplicateDeclaration), 'cem-element.tag_already_defined');

        const instance = document.createElement('story-lifecycle-parent');
        root.appendChild(instance);
        await waitForElement(instance, 'article.card');
        assertEqual(requiredElement(instance, '.label').textContent?.trim(), 'Initial', 'declared defaults render first');
        assertEqual(
            requiredElement(instance, 'story-lifecycle-child strong').textContent?.trim(),
            'Nested',
            'nested produced elements render from forwarded declared defaults'
        );
        assertEqual(instance.querySelector('.duplicate'), null, 'a duplicate declaration does not replace the first tag');

        instance.setAttribute('label', 'First');
        instance.setAttribute('label', 'Second');
        instance.setAttribute('tone', 'warm');
        instance.setAttribute('child', 'Inner');
        await waitForCondition(
            () =>
                requiredElement(instance, '.label').textContent?.trim() === 'Second' &&
                requiredElement(instance, 'article.card').getAttribute('data-tone') === 'warm' &&
                requiredElement(instance, 'story-lifecycle-child strong').textContent?.trim() === 'Inner',
            'rapid host mutations render the latest attribute snapshot'
        );
        assert(!instance.textContent?.includes('First'), 'stale intermediate host values do not survive rerender ordering');

        const renderedWhileConnected = requiredElement(instance, '.label');
        const revisionBeforeDisconnect = Number(renderedWhileConnected.getAttribute('data-cem-data-revision'));
        instance.remove();
        instance.setAttribute('label', 'Detached');
        await nextFrame();
        assertEqual(
            requiredElement(instance, '.label').textContent?.trim(),
            'Second',
            'attribute changes while disconnected do not rerender until reconnect'
        );

        root.appendChild(instance);
        await waitForCondition(
            () => requiredElement(instance, '.label').textContent?.trim() === 'Detached',
            'reconnect re-attaches observation and renders current host state'
        );
        assert(
            Number(requiredElement(instance, '.label').getAttribute('data-cem-data-revision')) > revisionBeforeDisconnect,
            'reconnect advances the deterministic render revision'
        );

        instance.setAttribute('label', 'Reconnected');
        await waitForCondition(
            () => requiredElement(instance, '.label').textContent?.trim() === 'Reconnected',
            'post-reconnect host mutations are observed'
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

export const UriAndModuleResolutionPolicy: Story = {
    render: () =>
        storyPanel(
            'URI/module resolution policy',
            'fragment src, document-relative external src, module-url hooks, resolver diagnostics, cache identity'
        ),
    play: async ({ canvasElement }) => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'URI and module resolution policy story');
        canvasElement.appendChild(root);

        const localRuntime = new CemElementRuntime({ declarationTag: 'cem-element-story-uri-local' });
        const localTemplate = document.createElement('template');
        localTemplate.id = 'story-uri-local-template';
        localTemplate.setAttribute('type', 'text/cem-ml');
        localTemplate.textContent = '{span @class=label | {$datadom.attributes.label}}';
        root.appendChild(localTemplate);

        const localDeclaration = document.createElement('cem-element-story-uri-local');
        localDeclaration.setAttribute('tag', 'story-uri-local-fragment');
        localDeclaration.setAttribute('src', '#story-uri-local-template');
        root.appendChild(localDeclaration);
        assert(localRuntime.registerDeclaration(localDeclaration), 'fragment-only src declarations register');
        await localRuntime.whenDeclarationSettled(localDeclaration);

        const localInstance = document.createElement('story-uri-local-fragment');
        localInstance.setAttribute('label', 'Fragment');
        root.appendChild(localInstance);
        assertEqual(
            (await waitForElement(localInstance, '.label')).textContent?.trim(),
            'Fragment',
            'fragment-only src resolves against the declaring document'
        );

        const sourceLoads: string[] = [];
        const sourceRuntime = new CemElementRuntime({
            declarationTag: 'cem-element-story-uri-source',
            loadSrcDocument: async (path, baseDocument) => {
                const href = new URL(path, baseDocument.baseURI).href;
                sourceLoads.push(href);
                return `<template id="card"><span class="source">${href}</span></template>`;
            },
        });
        const firstFrame = await appendResolutionPolicyFrame(canvasElement, 'https://example.test/alpha/');
        const secondFrame = await appendResolutionPolicyFrame(canvasElement, 'https://example.test/beta/');
        sourceRuntime.install(firstFrame.contentWindow as Window);
        sourceRuntime.install(secondFrame.contentWindow as Window);

        const firstSource = await registerExternalSourceInstance(
            sourceRuntime,
            firstFrame.contentDocument as Document,
            'story-uri-source-alpha'
        );
        const secondSource = await registerExternalSourceInstance(
            sourceRuntime,
            secondFrame.contentDocument as Document,
            'story-uri-source-beta'
        );
        assertEqual(
            requiredElement(firstSource, '.source').textContent,
            'https://example.test/alpha/cards.html',
            'external src resolves document-relative to the first source document'
        );
        assertEqual(
            requiredElement(secondSource, '.source').textContent,
            'https://example.test/beta/cards.html',
            'external src cache identity includes the declaring document base URI'
        );
        assertEqual(
            sourceLoads.join('|'),
            'https://example.test/alpha/cards.html|https://example.test/beta/cards.html',
            'same external src path loads once per declaring document identity'
        );

        const specifier = '@scope/widget/icon.svg';
        const moduleRuntimeA = new CemElementRuntime({
            declarationTag: 'cem-element-story-uri-module-a',
            resolveModuleUrl: async (moduleSpecifier) => {
                assertEqual(moduleSpecifier, specifier, 'module resolver receives the resource specifier');
                return 'https://cdn.example.test/a/icon.svg';
            },
        });
        const moduleRuntimeB = new CemElementRuntime({
            declarationTag: 'cem-element-story-uri-module-b',
            resolveModuleUrl: async (moduleSpecifier) => {
                assertEqual(moduleSpecifier, specifier, 'module resolver receives the resource specifier');
                return 'https://cdn.example.test/b/icon.svg';
            },
        });
        const moduleA = await registerModuleUrlInstance(
            root,
            moduleRuntimeA,
            'cem-element-story-uri-module-a',
            'story-uri-module-a',
            specifier,
            'https://cdn.example.test/a/icon.svg'
        );
        const moduleB = await registerModuleUrlInstance(
            root,
            moduleRuntimeB,
            'cem-element-story-uri-module-b',
            'story-uri-module-b',
            specifier,
            'https://cdn.example.test/b/icon.svg'
        );
        assertEqual(
            requiredElement(moduleA, 'a.asset').getAttribute('href'),
            'https://cdn.example.test/a/icon.svg',
            'module-url resolver policy is scoped to its runtime'
        );
        assertEqual(
            requiredElement(moduleB, 'a.asset').getAttribute('href'),
            'https://cdn.example.test/b/icon.svg',
            'changing resolver policy uses a distinct runtime cache'
        );
        const moduleSnapshot = moduleRuntimeA.snapshotInstance(moduleA);
        const modulePayload = moduleSnapshot.eventPayloads.asset as { type?: string; src?: string; value?: string };
        assertEqual(moduleSnapshot.slices.asset, 'https://cdn.example.test/a/icon.svg', 'module-url writes a slice');
        assertEqual(modulePayload.type, 'module-url', 'module-url stores resource payload metadata');
        assertEqual(modulePayload.src, specifier, 'module-url payload records the source specifier');
        assertEqual(modulePayload.value, 'https://cdn.example.test/a/icon.svg', 'module-url payload records the URL');
        const retainedModuleAnchor = requiredElement(moduleA, 'a.asset');
        moduleA.setAttribute('unused', 'rerender-module-url');
        await nextFrame();
        await moduleRuntimeA.whenRenderSettled(moduleA);
        assertEqual(
            requiredElement(moduleA, 'a.asset') === retainedModuleAnchor,
            true,
            'direct module-url setup keeps retained rendered nodes across rerender'
        );
        assert(moduleA.querySelector('module-url') === null, 'direct module-url setup removes helper nodes after rerender');

        const failureRuntime = new CemElementRuntime({
            declarationTag: 'cem-element-story-uri-module-fail',
            resolveModuleUrl: async () => {
                throw new Error('module map missing');
            },
        });
        const failedModule = await registerModuleUrlInstance(
            root,
            failureRuntime,
            'cem-element-story-uri-module-fail',
            'story-uri-module-fail',
            '@missing/icon.svg',
            '@missing/icon.svg'
        );
        assertEqual(
            requiredElement(failedModule, 'a.asset').getAttribute('href'),
            '@missing/icon.svg',
            'failed module-url resolution falls back to the original specifier'
        );
        assertDiagnostic(failureRuntime.diagnosticsFor(failedModule), 'cem-element.module_url_resolve_failed');
    },
};

export const HttpRequestResourceLifecycle: Story = {
    render: () =>
        storyPanel(
            'HTTP request resource lifecycle',
            'pending/header/complete envelopes, resource revision, and stale request abort'
        ),
    play: async ({ canvasElement }) => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'http-request resource lifecycle story');
        canvasElement.appendChild(root);

        const pending = new Map<string, (name: string) => void>();
        const aborted: string[] = [];
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-element-story-http-resource',
            resolveResourceUrl: (request) => ({
                authoredUrl: request.authoredUrl,
                resolvedUrl: `https://resources.example.test/${request.authoredUrl}.json`,
                resolverIdentity: 'story-http-resolver',
                resourcePolicyStamp: 'story-http-policy',
            }),
            loadHttpResource: (request) =>
                new Promise((resolve, reject) => {
                    request.signal.addEventListener(
                        'abort',
                        () => {
                            aborted.push(request.authoredUrl);
                            reject(new Error('aborted'));
                        },
                        { once: true }
                    );
                    pending.set(request.authoredUrl, (name) =>
                        resolve({
                            response: {
                                url: request.resolvedUrl,
                                status: 200,
                                statusText: 'OK',
                                ok: true,
                                redirected: false,
                                headers: { 'content-type': 'application/json' },
                                contentType: 'application/json',
                            },
                            body: utf8Body(JSON.stringify({ name, results: [{ name, status: 'ready' }] })),
                        })
                    );
                }),
        });

        const declaration = buildCemMlDeclaration(
            'cem-element-story-http-resource',
            'story-http-resource-panel',
            [
                '{http-request @slice=page @url="{$datadom.attributes.url}" @content-type="application/json"}',
                '{article |',
                '  {p @class=state | {$datadom.slices.page.state}}',
                '  {p @class=revision | {$datadom.slices.page.resourceRevision}}',
                '  {cem:if @test=\'datadom.slices.page.state = "complete"\' |',
                '    {output @class=name | {$datadom.slices.page.data.name}}',
                '  }',
                '}',
            ].join('\n')
        );
        root.appendChild(declaration);
        assert(runtime.registerDeclaration(declaration), 'http-request declaration registers');
        await runtime.whenDeclarationSettled(declaration);

        const instance = document.createElement('story-http-resource-panel');
        instance.setAttribute('url', 'first');
        root.appendChild(instance);
        await waitForCondition(
            () => instance.querySelector('.state')?.textContent?.trim() === 'pending',
            'http-request renders pending state'
        );

        instance.setAttribute('url', 'second');
        await waitForCondition(() => aborted.includes('first'), 'stale http-request is aborted');
        pending.get('second')?.('second');
        await waitForCondition(
            () => instance.querySelector('.name')?.textContent?.trim() === 'second',
            'latest http-request completion renders data'
        );

        const snapshot = runtime.snapshotInstance(instance);
        const page = snapshot.slices.page as {
            state?: string;
            resourceRevision?: number;
            request?: { authoredUrl?: string };
            sourceId?: {
                kind?: string;
                id?: string;
                authoredUrl?: string;
                finalUrl?: string;
                responseIdentityHash?: string;
            };
            data?: { name?: string };
        };
        assertEqual(page.state, 'complete', 'snapshot stores completed http-request state');
        assertEqual(page.resourceRevision, 2, 'resource revision increments after URL change');
        assertEqual(page.request?.authoredUrl, 'second', 'snapshot stores latest authored URL');
        assertEqual(page.sourceId?.kind, 'http-response', 'snapshot stores source-id kind');
        assertEqual(page.sourceId?.authoredUrl, 'second', 'source-id records authored URL');
        assertEqual(page.sourceId?.finalUrl, 'https://resources.example.test/second.json', 'source-id records response URL');
        assert(page.sourceId?.id?.startsWith('http-source-'), 'source-id uses an opaque public id');
        assert(page.sourceId?.responseIdentityHash, 'source-id records a response identity hash');
        assertEqual(page.data?.name, 'second', 'snapshot stores serializable response data');
        JSON.stringify(page);
    },
};

export const LocalStorageResourceLifecycle: Story = {
    render: () =>
        storyPanel(
            'local-storage resource lifecycle',
            'typed hydration, same-document live updates, and slice write-back'
        ),
    play: async ({ canvasElement }) => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'local-storage resource lifecycle story');
        canvasElement.appendChild(root);

        const textKey = 'cem-story-local-storage-text';
        const numberKey = 'cem-story-local-storage-number';
        const jsonKey = 'cem-story-local-storage-json';
        localStorage.removeItem(textKey);
        localStorage.removeItem(numberKey);
        localStorage.removeItem(jsonKey);
        localStorage.setItem(textKey, 'stored initial');
        localStorage.setItem(numberKey, '7');
        localStorage.setItem(jsonKey, JSON.stringify({ answer: 'json initial' }));

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-local-storage' });
        const declaration = buildCemMlDeclaration(
            'cem-element-story-local-storage',
            'story-local-storage-panel',
            [
                `{local-storage @slice=draft @key=${textKey} @type=text @live=true}`,
                `{local-storage @slice=count @key=${numberKey} @type=number @live=true}`,
                `{local-storage @slice=config @key=${jsonKey} @type=json @live=true}`,
                '{article |',
                '  {input @class=draft @value="{$datadom.slices.draft}" @slice=draft @slice-event=input @slice-value="$target.value"}',
                '  {output @class=draft-output | {$datadom.slices.draft}}',
                '  {output @class=count-output | {$datadom.slices.count}}',
                '  {output @class=json-output | {$datadom.slices.config.answer}}',
                '}',
            ].join('\n')
        );
        root.appendChild(declaration);
        assert(runtime.registerDeclaration(declaration), 'local-storage declaration registers');
        await runtime.whenDeclarationSettled(declaration);

        const instance = document.createElement('story-local-storage-panel');
        root.appendChild(instance);
        await waitForCondition(
            () => instance.querySelector('.draft-output')?.textContent?.trim() === 'stored initial',
            'local-storage hydrates text slice'
        );
        assertEqual(
            instance.querySelector('.count-output')?.textContent?.trim(),
            '7',
            'local-storage coerces number slice'
        );
        assertEqual(
            instance.querySelector('.json-output')?.textContent?.trim(),
            'json initial',
            'local-storage coerces JSON slice'
        );

        dispatchInput(instance, 'typed draft');
        await waitForCondition(
            () => localStorage.getItem(textKey) === 'typed draft',
            'slice event writes back to localStorage'
        );

        localStorage.setItem(textKey, 'external update');
        localStorage.setItem(numberKey, '42');
        localStorage.setItem(jsonKey, JSON.stringify({ answer: 'json update' }));
        await waitForCondition(
            () => instance.querySelector('.draft-output')?.textContent?.trim() === 'external update',
            'local-storage live text update renders'
        );
        assertEqual(
            instance.querySelector('.count-output')?.textContent?.trim(),
            '42',
            'local-storage live number update renders'
        );
        assertEqual(
            instance.querySelector('.json-output')?.textContent?.trim(),
            'json update',
            'local-storage live JSON update renders'
        );

        const snapshot = runtime.snapshotInstance(instance);
        assertEqual(snapshot.slices.draft, 'external update', 'snapshot stores live text slice');
        assertEqual(snapshot.slices.count, 42, 'snapshot stores coerced number slice');
        assertEqual(
            (snapshot.slices.config as { answer?: string }).answer,
            'json update',
            'snapshot stores parsed JSON slice'
        );
        const payload = snapshot.eventPayloads.draft as { type?: string; key?: string; storageType?: string; live?: boolean };
        assertEqual(payload.type, 'local-storage', 'local-storage stores resource payload metadata');
        assertEqual(payload.key, textKey, 'local-storage payload records storage key');
        assertEqual(payload.storageType, 'text', 'local-storage payload records storage type');
        assertEqual(payload.live, true, 'local-storage payload records live mode');

        localStorage.removeItem(textKey);
        localStorage.removeItem(numberKey);
        localStorage.removeItem(jsonKey);
    },
};

export const LocationElementResourceLifecycle: Story = {
    render: () =>
        storyPanel(
            'location-element resource lifecycle',
            'current URL hydration, href parsing, query params, and live history updates'
        ),
    play: async ({ canvasElement }) => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'location-element resource lifecycle story');
        canvasElement.appendChild(root);
        const originalUrl = location.href;

        try {
            history.replaceState({}, '', './?cemLocation=start#initial');

            const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-location' });
            const declaration = buildCemMlDeclaration(
                'cem-element-story-location',
                'story-location-panel',
                [
                    '{location-element @slice=current @live=true}',
                    '{location-element @slice=sample @href="https://example.test/docs/page?mode=demo&tag=one&tag=two#sample"}',
                    '{slice @name=target | #story-write}',
                    '{article |',
                    '  {input @class=target @value="{$target}" @slice=target @slice-event=input @slice-value="$target.value"}',
                    '  {button @class=write @type=button @slice=applyUrl @slice-event=click @slice-value="$event.type" | Write URL}',
                    '  {cem:if @test="datadom.slices.applyUrl" |',
                    '    {location-element @method="history.pushState" @src="{$target}"}',
                    '  }',
                    '  {output @class=current-hash | {$datadom.slices.current.hash}}',
                    '  {output @class=sample-host | {$datadom.slices.sample.hostname}}',
                    '  {ul @class=sample-params |',
                    '    {cem:for-each @select="datadom.slices.sample.paramEntries" @as=param |',
                    '      {li | {$param.name}: {$param.text}}',
                    '    }',
                    '  }',
                    '}',
                ].join('\n')
            );
            root.appendChild(declaration);
            assert(runtime.registerDeclaration(declaration), 'location-element declaration registers');
            await runtime.whenDeclarationSettled(declaration);

            const instance = document.createElement('story-location-panel');
            root.appendChild(instance);
            await waitForCondition(
                () => instance.querySelector('.current-hash')?.textContent?.trim() === '#initial',
                'location-element hydrates current URL'
            );
            assertEqual(
                instance.querySelector('.sample-host')?.textContent?.trim(),
                'example.test',
                'location-element parses href hostname'
            );
            assert(
                Array.from(instance.querySelectorAll('.sample-params li')).some(
                    (item) => item.textContent?.trim() === 'tag: one,two'
                ),
                'location-element exposes repeated params for rendering'
            );

            history.pushState({}, '', './?cemLocation=updated#live');
            await waitForCondition(
                () => instance.querySelector('.current-hash')?.textContent?.trim() === '#live',
                'location-element live history update renders'
            );

            dispatchInput(instance, '#story-write');
            await waitForCondition(
                () => runtime.snapshotInstance(instance).slices.target === '#story-write',
                'location-element target slice updates'
            );
            (requiredElement(instance, 'button.write') as HTMLButtonElement).click();
            await waitForCondition(
                () => instance.querySelector('.current-hash')?.textContent?.trim() === '#story-write',
                'location-element declarative URL write renders'
            );

            const snapshot = runtime.snapshotInstance(instance);
            const current = snapshot.slices.current as {
                hash?: string;
                params?: Record<string, string[]>;
                paramEntries?: { name?: string; text?: string }[];
            };
            const sample = snapshot.slices.sample as { hostname?: string; hash?: string };
            assertEqual(current.hash, '#story-write', 'snapshot stores written current hash');
            assertEqual(sample.hostname, 'example.test', 'snapshot stores parsed href hostname');
            assertEqual(sample.hash, '#sample', 'snapshot stores parsed href hash');
            assert(
                current.paramEntries?.some((entry) => entry.name === 'cemLocation' && entry.text === 'updated') ?? false,
                'snapshot stores renderable current param entries'
            );
            const payload = snapshot.eventPayloads.current as { type?: string; href?: string | null; live?: boolean };
            assertEqual(payload.type, 'location-element', 'location-element stores resource payload metadata');
            assertEqual(payload.href, null, 'location-element payload records current-window source');
            assertEqual(payload.live, true, 'location-element payload records live mode');
        } finally {
            root.remove();
            history.replaceState({}, '', originalUrl);
        }
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
    render: () => storyPanel('Legacy bridge template', 'custom-element-v0 routes through the shared legacy-xslt engine'),
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
                    '<button type="button" title="{$title}">{$label} {$title}</button>' +
                    '<if test="$label"><span class="label">{$label}</span></if>' +
                    '<slot name="description"><i>fallback</i></slot>',
            }],
        });
        runtime.registerDeclaration(declaration);
        assertEqual(runtime.diagnosticsFor(declaration).length, 0, 'legacy bridge declarations register without diagnostics');

        const instance = document.createElement('story-legacy-bridge');
        instance.setAttribute('title', 'Bridge');
        instance.innerHTML = '<p slot="description">projected</p>';
        root.appendChild(instance);

        await runtime.whenRenderSettled(instance);
        const button = await waitForElement(instance, 'button');
        assertEqual(button.textContent?.trim(), 'Legacy Bridge', 'legacy text interpolation resolves defaults and host attributes');
        assertEqual(button.getAttribute('title'), 'Bridge', 'legacy attribute value templates resolve host attributes');
        assertEqual(requiredElement(instance, '.label').textContent, 'Legacy', 'legacy if test renders through the engine');
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
        // CEM parity allows repeated slot inclusions to project the same payload more than once.
        template.innerHTML = [
            '<div class="card">',
            '<slot name="a"><em class="f1">f1</em></slot>',
            '<slot name="a"><em class="f2">f2</em></slot>',
            '<slot name=""><em class="f3">f3</em></slot>',
            '<slot><em class="f4">f4</em></slot>',
            '</div>',
        ].join('');
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-slot-dup');
        instance.innerHTML = '<span slot="a">X</span><i slot="">Y</i>';
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
            'the first matching slot receives the payload'
        );
        assertEqual(
            card.querySelectorAll('[slot="a"]').length,
            2,
            'a repeated same-name slot projects the payload again'
        );
        assert(card.querySelector('.f1') === null, 'the first named slot drops its fallback once filled');
        assert(card.querySelector('.f2') === null, 'the repeated named slot also drops its fallback once filled');
        assertEqual(
            card.querySelectorAll('[slot=""]').length,
            2,
            'blank-name and omitted-name default slots both project the whole default payload'
        );
        assertEqual(
            Array.from(card.querySelectorAll('[slot=""]'), (node) => node.textContent).join('|'),
            'Y|Y',
            'default payload is reusable across blank-name and omitted-name slots'
        );
        assert(card.querySelector('.f3') === null, 'the blank-name default slot drops its fallback once filled');
        assert(card.querySelector('.f4') === null, 'the omitted-name default slot drops its fallback once filled');
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
        const parserDiagnostic = findDiagnostic(
            parserRuntime.diagnosticsFor(parserDeclaration),
            'cem.tokenizer.bare_brace_text'
        );
        assertEqual(
            parserDiagnostic.sourceMapRef?.fidelity,
            'author-byte-exact',
            'CEM-ML parser diagnostics carry source-byte fidelity'
        );
        assert(
            /^cem:\d+$/.test(parserDiagnostic.sourceMapRef?.frame ?? ''),
            'CEM-ML parser diagnostics carry byte frames'
        );

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

        const renderDiagnostic = findDiagnostic(renderRuntime.diagnosticsFor(instance), 'cem.ql.render.compile_failed');
        assertEqual(
            renderDiagnostic.sourceMapRef?.fidelity,
            'author-byte-exact',
            'CEM-ML render diagnostics carry source-byte fidelity'
        );
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
        const retainedInput = requiredElement(instance, 'input') as HTMLInputElement;
        assertEqual(retainedInput === input, true, 'slice-event rerender keeps the retained input node');
        assert(!retainedInput.hasAttribute('slice-event'), 'direct slice-event setup removes metadata after rerender');

        retainedInput.value = 'Again';
        retainedInput.dispatchEvent(new Event('input', { bubbles: true }));
        await nextFrame();
        assertEqual(
            requiredElement(instance, 'output').textContent,
            'Again',
            'retained slice-event listener updates the slice once on a later rerender'
        );
    },
};

export const SliceEventExpressionParity: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'slice event expression parity story');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-slice-expr' });
        runtime.install(window);

        const declaration = document.createElement('cem-element-story-slice-expr');
        declaration.setAttribute('tag', 'story-slice-expression-field');
        const template = document.createElement('template');
        template.innerHTML = [
            '<slice name="count">0</slice>',
            '<slice name="pointer"></slice>',
            '<slice name="left"></slice>',
            '<slice name="right"></slice>',
            '<button type="button" data-role="increment" slice="count" slice-event="click tap" slice-value="//count + 1">+</button>',
            '<button type="button" data-role="decrement" slice="count" slice-event="click" slice-value="//count - 1">-</button>',
            '<textarea data-role="pointer" slice="pointer" slice-event="mousemove click" slice-value="concat(\'x:\', //@clientX)"></textarea>',
            '<input data-role="fanout" slice="left|right" slice-event="input" slice-value="@value" />',
            '<output data-role="count">${$count}</output>',
            '<output data-role="pointer">${$pointer}</output>',
            '<output data-role="left">${$left}</output>',
            '<output data-role="right">${$right}</output>',
        ].join('');
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-slice-expression-field');
        root.appendChild(instance);
        return root;
    },
    play: async ({ canvasElement }) => {
        const instance = await waitForElement(canvasElement, 'story-slice-expression-field');
        const increment = requiredElement(instance, 'button[data-role="increment"]') as HTMLButtonElement;
        const decrement = requiredElement(instance, 'button[data-role="decrement"]') as HTMLButtonElement;
        const pointer = requiredElement(instance, 'textarea[data-role="pointer"]') as HTMLTextAreaElement;
        const fanout = requiredElement(instance, 'input[data-role="fanout"]') as HTMLInputElement;

        increment.click();
        await waitForCondition(
            () => requiredElement(instance, 'output[data-role="count"]').textContent === '1',
            'slice arithmetic increment renders'
        );
        increment.dispatchEvent(new Event('tap', { bubbles: true }));
        await waitForCondition(
            () => requiredElement(instance, 'output[data-role="count"]').textContent === '2',
            'slice multi-event tap renders'
        );
        decrement.click();
        await waitForCondition(
            () => requiredElement(instance, 'output[data-role="count"]').textContent === '1',
            'slice arithmetic decrement renders'
        );

        pointer.dispatchEvent(new MouseEvent('mousemove', { bubbles: true, clientX: 37 }));
        await waitForCondition(
            () => requiredElement(instance, 'output[data-role="pointer"]').textContent === 'x:37',
            'slice concat reads mouse event fields'
        );

        fanout.value = 'mirrored';
        fanout.dispatchEvent(new Event('input', { bubbles: true }));
        await waitForCondition(
            () =>
                requiredElement(instance, 'output[data-role="left"]').textContent === 'mirrored' &&
                requiredElement(instance, 'output[data-role="right"]').textContent === 'mirrored',
            'slice fan-out writes multiple slices'
        );
    },
};

export const EventToDataRenderLoopSnapshot: Story = {
    render: () => storyPanel('Event to data loop', 'slice events update render output and data snapshots'),
    play: async ({ canvasElement }) => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'event to data render loop story');
        canvasElement.appendChild(root);

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-event-data' });
        const declaration = buildDeclaration({
            tag: 'story-event-data-field',
            templates: [
                {
                    html: [
                        '<attribute name="label">Search</attribute>',
                        '<slice name="query"></slice>',
                        '<slice name="custom"></slice>',
                        '<label>',
                        '  <slot name="label">${$label}</slot>',
                        '  <input name="query" data-role="query" value="{$query}" slice="query" slice-event="input" slice-value="{$target.value}" />',
                        '</label>',
                        '<output data-query="{$query}">${$query}</output>',
                        '<button type="button" data-role="custom" slice="custom" slice-event="cem-select" slice-value="\'cem-select\'">Select</button>',
                        '<span class="custom">${$custom}</span>',
                    ].join(''),
                },
            ],
        });
        runtime.registerDeclaration(declaration);

        const first = document.createElement('story-event-data-field');
        first.setAttribute('label', 'First search');
        first.setAttribute('data-tracking-id', 'first-1');
        first.innerHTML = [
            '<span slot="label">Visible label</span>',
            '<data value="alpha" data-rank="1">Alpha</data>',
            '<option value="beta">Beta</option>',
        ].join('');

        const second = document.createElement('story-event-data-field');
        second.setAttribute('label', 'Second search');
        second.setAttribute('data-tracking-id', 'second-1');
        second.innerHTML = '<data value="gamma">Gamma</data><option value="delta">Delta</option>';

        root.append(first, second);
        await waitForElement(first, 'input');
        await waitForElement(second, 'input');

        dispatchInput(first, 'a');
        await waitForCondition(
            () => requiredElement(first, 'output').textContent === 'a',
            'the first input event renders'
        );
        dispatchInput(first, 'ab');
        dispatchInput(first, 'latest');
        await waitForCondition(
            () => requiredElement(first, 'output').textContent === 'latest',
            'repeated input events render the latest value'
        );
        assertEqual(requiredElement(first, 'output').textContent, 'latest', 'stale repeated input output does not survive');

        dispatchInput(second, 'other');
        await waitForCondition(
            () => requiredElement(second, 'output').textContent === 'other',
            'the second instance renders its own event'
        );
        assertEqual(requiredElement(first, 'output').textContent, 'latest', 'the first instance stays isolated');

        requiredElement(first, 'button[data-role="custom"]').dispatchEvent(
            new CustomEvent('cem-select', {
                bubbles: true,
                detail: { id: 'alpha', nested: { ok: true } },
            })
        );
        await waitForCondition(
            () => requiredElement(first, '.custom').textContent === 'cem-select',
            'custom event slice renders from the event type'
        );

        const firstSnapshot = runtime.snapshotInstance(first);
        assertEqual(firstSnapshot.hostAttributes.label, 'First search', 'host attributes serialize into the data snapshot');
        assertEqual(firstSnapshot.dataset.trackingId, 'first-1', 'dataset serializes into the data snapshot');
        assertEqual(firstSnapshot.payload.slots.label[0]?.kind, 'element', 'named slot payload serializes');
        assertEqual(firstSnapshot.payload.dataByValue.alpha.label, 'Alpha', 'data payload choices serialize by value');
        assertEqual(firstSnapshot.payload.optionsByValue.beta.label, 'Beta', 'option payload choices serialize by value');
        assertEqual(firstSnapshot.slices.query, 'latest', 'slice state serializes after repeated events');
        assertEqual(firstSnapshot.slices.custom, 'cem-select', 'custom event slice state serializes');
        assertEqual(
            Object.keys(firstSnapshot.validationState).length,
            0,
            'validation state serializes as an explicit record'
        );

        const eventPayload = firstSnapshot.eventPayloads.query as SerializedEventPayload;
        assertEqual(eventPayload.type, 'input', 'event payload records the event type');
        assertEqual(eventPayload.sliceValue, 'latest', 'event payload records the resolved slice value');
        assertEqual(eventPayload.target?.tag, 'input', 'event payload records the target element');
        assertEqual(eventPayload.target?.name, 'query', 'event payload records target name');
        assertEqual(eventPayload.target?.value, 'latest', 'event payload records target value');
        assertEqual(eventPayload.target?.dataset.role, 'query', 'event payload records target dataset');
        assertEqual(eventPayload.currentTarget?.tag, 'input', 'event payload records currentTarget');
        const customPayload = firstSnapshot.eventPayloads.custom as SerializedEventPayload;
        const customDetail = customPayload.detail as { id?: string; nested?: { ok?: boolean } };
        assertEqual(customPayload.type, 'cem-select', 'custom event payload records event type');
        assertEqual(customPayload.sliceValue, 'cem-select', 'custom event payload records resolved slice value');
        assertEqual(customDetail.id, 'alpha', 'custom event payload records JSON-safe detail');
        assertEqual(customDetail.nested?.ok, true, 'custom event payload preserves nested JSON-safe detail');

        const secondSnapshot = runtime.snapshotInstance(second);
        assertEqual(secondSnapshot.slices.query, 'other', 'second instance owns an isolated slice record');
        assertEqual(
            (secondSnapshot.eventPayloads.query as SerializedEventPayload).target?.value,
            'other',
            'second instance owns an isolated event payload'
        );
        assertEqual(firstSnapshot.instanceId === secondSnapshot.instanceId, false, 'instances receive distinct ids');
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
        (first as Element & { cemRenderNodeId?: string }).cemRenderNodeId = undefined;

        instance.setAttribute('label', 'Updated');
        await nextFrame();

        const second = requiredElement(instance, 'button');
        assertEqual(second === first, true, 'rerender updates the existing render-node DOM object in place');
        assertEqual(second.textContent, 'Updated', 'rerender updates changed text inside the retained node');
        assertEqual(second.getAttribute('data-cem-data-revision'), '2', 'rerender advances the data revision');
        assertEqual(
            second.getAttribute('data-cem-render-node-id'),
            nodeId,
            'render-node identity stays stable across rerenders'
        );
        assertEqual(
            (second as Element & { cemRenderNodeId?: string }).cemRenderNodeId,
            nodeId,
            'serialized render-node identity is mirrored back into the DOM property path'
        );
        assertEqual(second.getAttribute('data-cem-source-frame'), frame, 'source frame stays stable across rerenders');

        instance.setAttribute('label', 'Third');
        await nextFrame();
        assertEqual(
            requiredElement(instance, 'button') === first,
            true,
            'later rerenders keep the same retained render-node DOM object'
        );
        assertEqual(
            requiredElement(instance, 'button').getAttribute('data-cem-data-revision'),
            '3',
            'each invalidation advances the data revision'
        );
    },
};

export const DirectRenderPlanPatchUsesCommentRanges: Story = {
    render: () => {
        const root = document.createElement('section') as HTMLElement & {
            __bounds?: { start: Comment; end: Comment };
        };
        root.setAttribute('aria-label', 'direct render-plan patch story');
        const host = document.createElement('div');
        host.className = 'direct-patch-host';
        const start = document.createComment('cem-render-start');
        const end = document.createComment('cem-render-end');
        host.append(start, end);
        root.__bounds = { start, end };
        root.append(host);
        return root;
    },
    play: ({ canvasElement }) => {
        const root = requiredElement(canvasElement, '[aria-label="direct render-plan patch story"]') as HTMLElement & {
            __bounds?: { start: Comment; end: Comment };
        };
        const bounds = root.__bounds;
        assert(bounds !== undefined, 'direct patch render bounds are available');
        const host = requiredElement(root, '.direct-patch-host');
        const first = directPatchPlan('Hello');
        const firstResult = applyRenderPlanToRange(bounds, first, document, { dynamicTextRanges: true });
        assertEqual(firstResult.mode, 'patch', 'initial direct apply inserts from the render plan');

        const paragraph = requiredElement(host, 'p');
        const startMarker = Array.from(paragraph.childNodes).find((node) => node.nodeValue?.startsWith('cem-start:text:'));
        const text = Array.from(paragraph.childNodes).find((node) => node.nodeType === Node.TEXT_NODE);
        assert(startMarker !== undefined, 'direct apply emits a comment range for dynamic text');
        assert(text !== undefined, 'direct apply keeps dynamic text as a text node inside the range');
        assertEqual(paragraph.textContent, 'Hello', 'comment ranges do not affect rendered text content');

        const second = directPatchPlan('World');
        const secondResult = applyRenderPlanToRange(bounds, second, document, { dynamicTextRanges: true });
        assertEqual(secondResult.mode, 'patch', 'matching render identities patch in place');
        assertEqual(requiredElement(host, 'p') === paragraph, true, 'direct patch keeps the element node');
        assertEqual(
            Array.from(paragraph.childNodes).find((node) => node.nodeType === Node.TEXT_NODE) === text,
            true,
            'direct patch keeps the dynamic text node'
        );
        assertEqual(paragraph.textContent, 'World', 'direct patch updates dynamic range text');

        paragraph.setAttribute('data-cem-render-node-id', 'foreign-root');
        (paragraph as Element & { cemRenderNodeId?: string }).cemRenderNodeId = 'foreign-root';
        const recovery = applyRenderPlanToRange(bounds, second, document, { dynamicTextRanges: true });
        assertEqual(recovery.mode, 'replaceScope', 'root identity mismatch falls back to replaceScope');
        assertEqual(
            recovery.diagnostics[0]?.code,
            'cem.render_plan_apply.replace_scope',
            'replaceScope recovery emits a diagnostic'
        );
        assertEqual(requiredElement(host, 'p') === paragraph, false, 'replaceScope recovery replaces the corrupted root');
        assertEqual(requiredElement(host, 'p').textContent, 'World', 'recovered scope still renders the next plan');
    },
};

export const UnchangedRenderPlanSkipsDomReplacement: Story = {
    render: () => {
        const root = document.createElement('section') as HTMLElement & {
            __domRuntime?: CemElementRuntime;
            __wasmRuntime?: CemElementRuntime;
        };
        root.setAttribute('aria-label', 'unchanged render plan no-op story');

        const domRuntime = new CemElementRuntime({ declarationTag: 'cem-element-story-noop-dom' });
        const domDeclaration = buildDeclaration({
            tag: 'story-noop-dom-card',
            templates: [{ html: '<attribute name="label">Stable</attribute><button type="button">${$label}</button>' }],
        });
        domRuntime.registerDeclaration(domDeclaration);

        const wasmRuntime = new CemElementRuntime({ declarationTag: 'cem-element-story-noop-wasm' });
        const wasmDeclaration = buildDeclaration({
            tag: 'story-noop-wasm-card',
            templates: [{ type: 'text/cem-ml', text: '{button @type=button | {$datadom.attributes.label}}' }],
        });
        wasmRuntime.registerDeclaration(wasmDeclaration);

        const domInstance = document.createElement('story-noop-dom-card');
        domInstance.setAttribute('label', 'Stable');
        const wasmInstance = document.createElement('story-noop-wasm-card');
        wasmInstance.setAttribute('label', 'Stable');
        root.append(domInstance, wasmInstance);
        root.__domRuntime = domRuntime;
        root.__wasmRuntime = wasmRuntime;
        return root;
    },
    play: async ({ canvasElement }) => {
        const root = requiredElement(canvasElement, '[aria-label="unchanged render plan no-op story"]') as HTMLElement & {
            __domRuntime?: CemElementRuntime;
            __wasmRuntime?: CemElementRuntime;
        };
        const domRuntime = root.__domRuntime;
        const wasmRuntime = root.__wasmRuntime;
        assert(domRuntime && wasmRuntime, 'story runtimes are available');

        const domInstance = requiredElement(root, 'story-noop-dom-card') as HTMLElement;
        const wasmInstance = requiredElement(root, 'story-noop-wasm-card') as HTMLElement;
        await domRuntime.whenRenderSettled(domInstance);
        await wasmRuntime.whenRenderSettled(wasmInstance);

        const domButton = requiredElement(domInstance, 'button');
        const wasmButton = requiredElement(wasmInstance, 'button');
        const domRevision = domButton.getAttribute('data-cem-data-revision');
        const wasmRevision = wasmButton.getAttribute('data-cem-data-revision');
        assertEqual(
            (domButton as Element & { cemRenderNodeId?: string }).cemRenderNodeId,
            domButton.getAttribute('data-cem-render-node-id'),
            'DOM-path render identity is mirrored into a DOM property'
        );
        assertEqual(
            (wasmButton as Element & { cemRenderNodeId?: string }).cemRenderNodeId,
            wasmButton.getAttribute('data-cem-render-node-id'),
            'WASM-path render identity is mirrored into a DOM property'
        );

        domInstance.setAttribute('unused', 'same-output');
        wasmInstance.setAttribute('unused', 'same-output');
        await nextFrame();
        await domRuntime.whenRenderSettled(domInstance);
        await wasmRuntime.whenRenderSettled(wasmInstance);

        assertEqual(requiredElement(domInstance, 'button') === domButton, true, 'DOM-path unchanged output keeps the button node');
        assertEqual(
            requiredElement(domInstance, 'button').getAttribute('data-cem-data-revision'),
            domRevision,
            'DOM-path unchanged output does not rewrite render metadata'
        );
        assertEqual(
            requiredElement(wasmInstance, 'button') === wasmButton,
            true,
            'WASM-path unchanged output keeps the button node'
        );
        assertEqual(
            requiredElement(wasmInstance, 'button').getAttribute('data-cem-data-revision'),
            wasmRevision,
            'WASM-path unchanged output does not rewrite render metadata'
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

export const ScopedCssUidSeedRuntime: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'scoped CSS UID seed runtime story');

        const runtime = new CemElementRuntime({
            declarationTag: 'cem-element-story-scoped-css',
            validateGeneratedIds: true,
        });
        const declaration = document.createElement('cem-element-story-scoped-css');
        declaration.setAttribute('tag', 'story-scoped-css-card');
        declaration.setAttribute('uid-seed', 'stories/scoped-css/card');
        const template = document.createElement('template');
        template.innerHTML = [
            '<slice name="value">same</slice>',
            '<style>',
            '@import url("./global.css");',
            ':host { --scoped-border: rgb(0, 128, 0); }',
            ':global(.legacy), :root { color: red; }',
            '@keyframes pulse { from { opacity: 0; } to { opacity: 1; } }',
            'button { border: 3px solid var(--scoped-border); animation: pulse 1s; }',
            '</style>',
            '<button type="button" slice="value" slice-event="click" slice-value="\'same\'">${$value}</button>',
        ].join('');
        declaration.appendChild(template);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('story-scoped-css-card');
        root.append(instance);
        const outside = document.createElement('button');
        outside.textContent = 'outside';
        root.append(outside);

        (root as HTMLElement & { __runtime?: CemElementRuntime }).__runtime = runtime;
        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();

        const root = requiredElement(canvasElement, '[aria-label="scoped CSS UID seed runtime story"]') as HTMLElement & {
            __runtime?: CemElementRuntime;
        };
        const runtime = root.__runtime;
        assert(runtime, 'story runtime is available for diagnostics');
        const instance = requiredElement(root, 'story-scoped-css-card') as HTMLElement;
        const scopeUid = instance.getAttribute('data-cem-scope') ?? '';
        assert(
            /^cem-scope-story-scoped-css-card-ustoriesz2fscoped-cssz2fcard-p[0-9-]+$/.test(scopeUid),
            'uid-seed contributes a deterministic encoded scope UID'
        );

        const style = requiredElement(instance, 'style');
        const css = style.textContent ?? '';
        assert(css.includes(`[data-cem-scope="${scopeUid}"] {`), 'style rules are wrapped in the generated scope');
        assert(css.includes('& { --scoped-border: rgb(0, 128, 0); }'), ':host rewrites to the nesting parent');
        assert(css.includes('&.legacy, & { color: red; }'), ':global and :root rewrite to scoped aliases');
        assert(css.includes(`@keyframes pulse-${scopeUid}`), 'keyframes are renamed with the scope UID');
        assert(css.includes(`animation: pulse-${scopeUid} 1s`), 'animation shorthand references renamed keyframes');
        assert(!css.includes('@import'), '@import is suppressed from scoped CSS output');

        const button = requiredElement(instance, 'button');
        assertEqual(button.getAttribute('data-cem-scope'), scopeUid, 'top-level render roots carry the scope UID');
        const revision = button.getAttribute('data-cem-data-revision');
        button.dispatchEvent(new Event('click', { bubbles: true }));
        await nextFrame();
        assertEqual(
            requiredElement(instance, 'button').getAttribute('data-cem-data-revision'),
            revision,
            'slice events that resolve to the existing value do not rerender the DOM'
        );

        const diagnosticCodes = runtime.diagnosticsFor(instance).map((diagnostic) => diagnostic.code);
        assert(diagnosticCodes.includes('cem.scoped_css.import_unsupported'), '@import suppression is diagnosed');
        assert(diagnosticCodes.includes('cem.scoped_css.global_alias'), ':global/:root aliasing is diagnosed');
    },
};

export const HostAndSourceHashUidSeedFallbacks: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'host and source hash UID seed story');

        const hostRuntime = new CemElementRuntime({
            declarationTag: 'cem-element-story-host-seed',
            uidSeed: ({ producedTag }) => `host-seed/${producedTag}`,
            validateGeneratedIds: true,
        });
        const hostDeclaration = document.createElement('cem-element-story-host-seed');
        hostDeclaration.setAttribute('tag', 'story-host-seed-card');
        const hostTemplate = document.createElement('template');
        hostTemplate.innerHTML = '<button type="button">host</button>';
        hostDeclaration.appendChild(hostTemplate);

        const blankDeclaration = document.createElement('cem-element-story-host-seed');
        blankDeclaration.setAttribute('tag', 'story-blank-seed-card');
        blankDeclaration.setAttribute('uid-seed', '');
        const blankTemplate = document.createElement('template');
        blankTemplate.innerHTML = '<button type="button">blank</button>';
        blankDeclaration.appendChild(blankTemplate);
        root.append(hostDeclaration, blankDeclaration);
        hostRuntime.registerDeclaration(hostDeclaration);
        hostRuntime.registerDeclaration(blankDeclaration);

        const sourceHashRuntime = new CemElementRuntime({
            declarationTag: 'cem-element-story-source-seed',
            runMode: 'build-ssr',
        });
        const sourceDeclaration = document.createElement('cem-element-story-source-seed');
        sourceDeclaration.setAttribute('tag', 'story-source-seed-card');
        const sourceTemplate = document.createElement('template');
        sourceTemplate.innerHTML = '<button type="button">source</button>';
        sourceDeclaration.appendChild(sourceTemplate);
        root.appendChild(sourceDeclaration);
        sourceHashRuntime.registerDeclaration(sourceDeclaration);

        const runtimeFallback = new CemElementRuntime({
            declarationTag: 'cem-element-story-runtime-seed',
            uidSeedFallback: 'runtime',
        });
        const runtimeDeclaration = document.createElement('cem-element-story-runtime-seed');
        runtimeDeclaration.setAttribute('tag', 'story-runtime-seed-card');
        const runtimeTemplate = document.createElement('template');
        runtimeTemplate.innerHTML = '<button type="button">runtime</button>';
        runtimeDeclaration.appendChild(runtimeTemplate);
        root.appendChild(runtimeDeclaration);
        runtimeFallback.registerDeclaration(runtimeDeclaration);

        root.append(
            document.createElement('story-host-seed-card'),
            document.createElement('story-blank-seed-card'),
            document.createElement('story-source-seed-card'),
            document.createElement('story-runtime-seed-card')
        );
        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();

        const hostScope = requiredElement(canvasElement, 'story-host-seed-card').getAttribute('data-cem-scope') ?? '';
        assert(
            /^cem-scope-story-host-seed-card-uhost-seedz2fstory-host-seed-card-p[0-9-]+$/.test(hostScope),
            'host uidSeed resolver supplies the fallback seed'
        );

        const blankScope = requiredElement(canvasElement, 'story-blank-seed-card').getAttribute('data-cem-scope') ?? '';
        assert(
            /^cem-scope-story-blank-seed-card-p[0-9-]+$/.test(blankScope),
            'explicit blank uid-seed overrides the host seed and omits the seed token'
        );

        const sourceScope = requiredElement(canvasElement, 'story-source-seed-card').getAttribute('data-cem-scope') ?? '';
        assert(
            /^cem-scope-story-source-seed-card-usource-[0-9a-f]{16}-p[0-9-]+$/.test(sourceScope),
            'build-ssr mode falls back to a stable source hash seed'
        );

        const runtimeScope = requiredElement(canvasElement, 'story-runtime-seed-card').getAttribute('data-cem-scope') ?? '';
        assert(
            /^cem-scope-story-runtime-seed-card-uruntime-[0-9]+-p[0-9-]+$/.test(runtimeScope),
            'normal runtime fallback remains dynamic when no stable seed is supplied'
        );
    },
};

export const ScopeUidDuplicateDiagnostics: Story = {
    render: () => {
        const root = document.createElement('section') as HTMLElement & {
            __runtime?: CemElementRuntime;
            __first?: HTMLElement;
            __second?: HTMLElement;
        };
        root.setAttribute('aria-label', 'scope UID duplicate diagnostics story');

        const runtime = new CemElementRuntime({
            declarationTag: 'cem-element-story-scope-duplicate',
            validateGeneratedIds: true,
        });
        const first = buildCemMlDeclaration(
            'cem-element-story-scope-duplicate',
            'story-collision-card',
            '{button | first}'
        );
        first.setAttribute('uid-seed', 'collision');
        const second = buildCemMlDeclaration(
            'cem-element-story-scope-duplicate',
            'story_collision-card',
            '{button | second}'
        );
        second.setAttribute('uid-seed', 'collision');

        runtime.registerDeclaration(first);
        runtime.registerDeclaration(second);
        root.__runtime = runtime;
        root.__first = first;
        root.__second = second;
        return root;
    },
    play: async ({ canvasElement }) => {
        const root = requiredElement(canvasElement, '[aria-label="scope UID duplicate diagnostics story"]') as HTMLElement & {
            __runtime?: CemElementRuntime;
            __first?: HTMLElement;
            __second?: HTMLElement;
        };
        const runtime = root.__runtime;
        const first = root.__first;
        const second = root.__second;
        assert(runtime && first && second, 'duplicate UID story fixtures are available');

        await runtime.whenDeclarationSettled(first);
        await runtime.whenDeclarationSettled(second);
        assertEqual(runtime.diagnosticsFor(first).length, 0, 'the first generated scope owner is accepted');
        assertDiagnostic(runtime.diagnosticsFor(second), 'cem-element.scope_uid_duplicate');
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
        const serverScopeUid = 'cem-scope-story-ssr-card-userver-p0';
        snapshot.hostAttributes['data-cem-scope'] = serverScopeUid;
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

        const plan = scopeRenderPlan(projectTemplate(source, { snapshot, values: { label: 'Server Card' } }), serverScopeUid)
            .renderPlan;
        const serverFragment = materializeRenderPlan(plan, document);
        const serverNodes = Array.from(serverFragment.childNodes);

        registerInlineDeclaration({
            declarationTag: 'cem-element-story-ssr',
            producedTag: 'story-ssr-card',
            innerHTML: templateHtml,
        });

        const instance = document.createElement('story-ssr-card');
        instance.setAttribute('label', 'Server Card');
        instance.setAttribute('data-cem-scope', serverScopeUid);
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
            instance.getAttribute('data-cem-scope'),
            'cem-scope-story-ssr-card-userver-p0',
            'client hydration preserves the server host scope UID'
        );
        assertEqual(
            article.getAttribute('data-cem-scope'),
            'cem-scope-story-ssr-card-userver-p0',
            'client hydration preserves the server render-root scope UID'
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

export const SsrHydrationRejectsUnsupportedSnapshotVersion: Story = {
    render: () =>
        storyPanel(
            'SSR hydration version gate',
            'a higher-MINOR snapshot version is rejected (BR-VC-9 data/security), hydration falls back to a fresh render'
        ),
    play: async ({ canvasElement }) => {
        const root = document.createElement('section');
        canvasElement.appendChild(root);

        const templateHtml =
            '<attribute name="label">Fallback</attribute>' +
            '<article class="ssr-reject-card">' +
            '<h2>${$label}</h2>' +
            '<div class="detail"><slot name="detail"></slot></div>' +
            '</article>';

        // Default runMode is `application`: the snapshot/`datadom` is a
        // data/security contract, so a snapshot whose schema MINOR is ahead of
        // this build (unknown optional features) must be rejected per BR-VC-9
        // rather than adopted.
        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-ssr-reject' });
        runtime.install(window);
        const declaration = document.createElement('cem-element-story-ssr-reject');
        declaration.setAttribute('tag', 'story-ssr-reject-card');
        const declTemplate = document.createElement('template');
        declTemplate.innerHTML = templateHtml;
        declaration.appendChild(declTemplate);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);

        const template = document.createElement('template');
        template.innerHTML = templateHtml;
        const source = readTemplateSource(template.content);
        const snapshot = projectionSnapshot('story-ssr-reject-card', { label: 'Server Card' });
        snapshot.instanceId = 'ssr-reject-instance-1';
        snapshot.declarationTag = 'cem-element-story-ssr-reject';
        snapshot.templateArtifactId = 'ssr-reject-artifact-1';
        snapshot.dataRevision = '7';
        // A schema MINOR ahead of the build version — the un-understood case.
        const [major, minor, patch] = SNAPSHOT_SCHEMA_VERSION.split('.').map((n) => Number.parseInt(n, 10));
        snapshot.version = `${major}.${minor + 1}.${patch}`;

        const plan = projectTemplate(source, { snapshot, values: { label: 'Server Card' } });
        const serverFragment = materializeRenderPlan(plan, document);
        const serverNodes = Array.from(serverFragment.childNodes);

        const instance = document.createElement('story-ssr-reject-card');
        instance.setAttribute('label', 'Server Card');
        const island = document.createElement('template');
        island.setAttribute('data-cem-island', 'instance');
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
        root.appendChild(instance);

        await runtime.whenRenderSettled(instance);

        // The un-understood snapshot version is rejected at the hydration ingest
        // seam; hydration is refused and the instance falls back to a fresh
        // client render (which still produces the article).
        assertDiagnostic(runtime.diagnosticsFor(instance), 'cem-element.snapshot_version_rejected');
        await waitForElement(instance, 'article.ssr-reject-card');
    },
};

export const SsrHydrationRejectsIncompleteMarkup: Story = {
    render: () =>
        storyPanel(
            'SSR hydration incomplete markup',
            'partial hydration markup fails closed with specific diagnostics before client fallback'
        ),
    play: async ({ canvasElement }) => {
        const root = document.createElement('section');
        canvasElement.appendChild(root);

        const templateHtml =
            '<attribute name="label">Fallback</attribute>' +
            '<article class="ssr-incomplete-card">' +
            '<h2>${$label}</h2>' +
            '</article>';
        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-story-ssr-incomplete' });
        runtime.install(window);
        const declaration = document.createElement('cem-element-story-ssr-incomplete');
        declaration.setAttribute('tag', 'story-ssr-incomplete-card');
        const declTemplate = document.createElement('template');
        declTemplate.innerHTML = templateHtml;
        declaration.appendChild(declTemplate);
        root.appendChild(declaration);
        runtime.registerDeclaration(declaration);
        const sourceTemplate = document.createElement('template');
        sourceTemplate.innerHTML = templateHtml;
        const source = readTemplateSource(sourceTemplate.content);

        const snapshot = projectionSnapshot('story-ssr-incomplete-card', { label: 'Server Card' });
        snapshot.instanceId = 'ssr-incomplete-instance-1';
        snapshot.declarationTag = 'cem-element-story-ssr-incomplete';
        snapshot.templateArtifactId = 'ssr-incomplete-artifact-1';
        snapshot.dataRevision = '5';
        const serverNodes = () =>
            Array.from(
                materializeRenderPlan(
                    projectTemplate(source, { snapshot, values: { label: 'Server Card' } }),
                    document
                ).childNodes
            );
        const hydrationMetadata = (textContent: string) => {
            const metadata = document.createElement('script');
            metadata.setAttribute('type', 'application/json');
            metadata.setAttribute('data-cem-hydration', 'snapshot');
            metadata.textContent = textContent;
            return metadata;
        };
        const hydratedCase = (
            label: string,
            textContent: string,
            mutateFirstRenderedElement?: (element: Element) => void
        ) => {
            const instance = document.createElement('story-ssr-incomplete-card');
            instance.setAttribute('label', label);
            const island = document.createElement('template');
            island.setAttribute('data-cem-island', 'instance');
            const nodes = serverNodes();
            const firstRenderedElement = nodes.find((node) => node.nodeType === 1);
            if (firstRenderedElement) {
                mutateFirstRenderedElement?.(firstRenderedElement as Element);
            }
            instance.append(
                island,
                document.createComment('cem-render-start'),
                ...nodes,
                document.createComment('cem-render-end'),
                hydrationMetadata(textContent)
            );
            root.appendChild(instance);
            return instance;
        };

        const metadataOnly = document.createElement('story-ssr-incomplete-card');
        metadataOnly.setAttribute('label', 'Metadata only');
        const metadataOnlyIsland = document.createElement('template');
        metadataOnlyIsland.setAttribute('data-cem-island', 'instance');
        metadataOnly.append(metadataOnlyIsland, hydrationMetadata(JSON.stringify(snapshot)));
        root.appendChild(metadataOnly);

        const boundsOnly = document.createElement('story-ssr-incomplete-card');
        boundsOnly.setAttribute('label', 'Bounds only');
        const boundsOnlyIsland = document.createElement('template');
        boundsOnlyIsland.setAttribute('data-cem-island', 'instance');
        boundsOnly.append(
            boundsOnlyIsland,
            document.createComment('cem-render-start'),
            document.createElement('article'),
            document.createComment('cem-render-end')
        );
        root.appendChild(boundsOnly);

        const emptySnapshot = hydratedCase('Empty snapshot', '');
        const invalidSnapshot = hydratedCase('Invalid snapshot', JSON.stringify({ producedTag: 'story-ssr-incomplete-card' }));
        const invalidJson = hydratedCase('Invalid JSON', '{not-json');
        const missingIdentity = hydratedCase('Missing identity', JSON.stringify(snapshot), (element) => {
            element.removeAttribute('data-cem-template-artifact-id');
            element.removeAttribute('data-cem-data-revision');
        });
        const artifactMismatch = hydratedCase('Artifact mismatch', JSON.stringify(snapshot), (element) => {
            element.setAttribute('data-cem-template-artifact-id', 'stale-artifact');
        });
        const revisionMismatch = hydratedCase('Revision mismatch', JSON.stringify(snapshot), (element) => {
            element.setAttribute('data-cem-data-revision', '4');
        });
        const sourceMapModeMismatch = hydratedCase('Source-map mode mismatch', JSON.stringify(snapshot), (element) => {
            element.removeAttribute('data-cem-source-fidelity');
        });

        await runtime.whenRenderSettled(metadataOnly);
        await runtime.whenRenderSettled(boundsOnly);
        await runtime.whenRenderSettled(emptySnapshot);
        await runtime.whenRenderSettled(invalidSnapshot);
        await runtime.whenRenderSettled(invalidJson);
        await runtime.whenRenderSettled(missingIdentity);
        await runtime.whenRenderSettled(artifactMismatch);
        await runtime.whenRenderSettled(revisionMismatch);
        await runtime.whenRenderSettled(sourceMapModeMismatch);

        assertDiagnostic(runtime.diagnosticsFor(metadataOnly), 'cem-element.hydration_boundaries_missing');
        assertDiagnostic(runtime.diagnosticsFor(boundsOnly), 'cem-element.hydration_metadata_missing');
        assertDiagnostic(runtime.diagnosticsFor(emptySnapshot), 'cem-element.hydration_snapshot_missing');
        assertDiagnostic(runtime.diagnosticsFor(invalidSnapshot), 'cem-element.hydration_snapshot_invalid');
        assertDiagnostic(runtime.diagnosticsFor(invalidJson), 'cem-element.hydration_json_invalid');
        assertDiagnostic(runtime.diagnosticsFor(missingIdentity), 'cem-element.hydration_render_plan_identity_missing');
        assertDiagnostic(runtime.diagnosticsFor(artifactMismatch), 'cem-element.hydration_template_artifact_mismatch');
        assertDiagnostic(runtime.diagnosticsFor(revisionMismatch), 'cem-element.hydration_render_revision_mismatch');
        assertDiagnostic(runtime.diagnosticsFor(sourceMapModeMismatch), 'cem-element.hydration_source_map_mode_mismatch');
        await waitForElement(metadataOnly, 'article.ssr-incomplete-card');
        await waitForElement(boundsOnly, 'article.ssr-incomplete-card');
        await waitForElement(emptySnapshot, 'article.ssr-incomplete-card');
        await waitForElement(invalidSnapshot, 'article.ssr-incomplete-card');
        await waitForElement(invalidJson, 'article.ssr-incomplete-card');
        await waitForElement(missingIdentity, 'article.ssr-incomplete-card');
        await waitForElement(artifactMismatch, 'article.ssr-incomplete-card');
        await waitForElement(revisionMismatch, 'article.ssr-incomplete-card');
        await waitForElement(sourceMapModeMismatch, 'article.ssr-incomplete-card');
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
        const attributeTemplate = document.createElement('template');
        attributeTemplate.innerHTML =
            '<attribute name="label">Fallback</attribute>' +
            '<article class="edge-card" data-kind="{$kind}">' +
            '<h2>${$label}</h2>' +
            '</article>';
        const attributeSource = readTemplateSource(attributeTemplate.content);
        const attributePrevious = projectTemplate(attributeSource, {
            snapshot: edgeProjectionSnapshot('Edge Before', '13'),
            values: { label: 'Edge Before', kind: 'summary' },
        });
        const attributeNext = projectTemplate(attributeSource, {
            snapshot: edgeProjectionSnapshot('Edge After', '14'),
            values: { label: 'Edge After', kind: 'featured' },
        });
        const attributePatch = opsFromPatchFrames(diffRenderPlansToPatchFrames(attributePrevious, attributeNext)).find(
            (op) => op.op === 'setAttribute' && op.name === 'data-kind'
        );
        assert(attributePatch?.op === 'setAttribute', 'edge diff emits stable attribute patches');
        assertEqual(attributePatch.value, 'featured', 'attribute patch carries the next attribute value');

        const changedTemplatePlan = cloneRenderPlan(nextPlan);
        changedTemplatePlan.templateArtifactId = 'edge-template-artifact-2';
        assert(
            opsFromPatchFrames(diffRenderPlansToPatchFrames(previousPlan, changedTemplatePlan)).every(
                (op) => op.op === 'replaceScope' && op.reason === 'fallback'
            ),
            'template artifact changes fall back to constrained scope replacement'
        );

        const extraRootPlan = cloneRenderPlan(nextPlan);
        extraRootPlan.nodes.push(cloneRenderPlan(nextPlan).nodes[0]);
        assert(
            opsFromPatchFrames(diffRenderPlansToPatchFrames(previousPlan, extraRootPlan)).every(
                (op) => op.op === 'replaceScope' && op.reason === 'fallback'
            ),
            'root-count changes fall back to constrained scope replacement'
        );

        const structuralPlan = cloneRenderPlan(nextPlan);
        const structuralRoot = structuralPlan.nodes[0];
        if (structuralRoot.kind === 'element') {
            structuralRoot.tag = 'section';
        }
        const structuralReplace = opsFromPatchFrames(diffRenderPlansToPatchFrames(previousPlan, structuralPlan)).find(
            (op) => op.op === 'replace'
        );
        assert(structuralReplace?.op === 'replace', 'unsupported structural deltas replace the affected render node');
        assertEqual(
            structuralReplace.node.node.kind === 'element' ? structuralReplace.node.node.tagName : '',
            'section',
            'structural replacement carries the next serialized node'
        );

        const targetMismatchPlan = cloneRenderPlan(nextPlan);
        targetMismatchPlan.producedTag = 'story-edge-card-alt';
        assert(
            opsFromPatchFrames(diffRenderPlansToPatchFrames(previousPlan, targetMismatchPlan)).every(
                (op) => op.op === 'replaceScope' && op.reason === 'fallback'
            ),
            'target mismatches fall back to constrained scope replacement'
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

export const EdgeRenderStateHybridStorageModel: Story = {
    render: () =>
        storyPanel(
            'Edge render-state storage',
            'content-addressed render blobs + revisioned pointer record'
        ),
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

        const previousSnapshot = edgeProjectionSnapshot('Edge Before', '21');
        const nextSnapshot = edgeProjectionSnapshot('Edge After', '22');
        const previousPlan = projectTemplate(source, { snapshot: previousSnapshot, values: { label: 'Edge Before' } });
        const nextPlan = projectTemplate(source, { snapshot: nextSnapshot, values: { label: 'Edge After' } });
        const sanitizedSnapshot = exportDataIslandSnapshotForEdge(nextSnapshot, {
            privacyPolicyStamp: 'edge-export-policy-v1',
            fields: {
                hostAttributes: 'allow',
                payload: 'redact',
                validationState: 'redact',
            },
        });

        const previousRecord = createEdgeRenderStateRecord({
            renderPlan: previousPlan,
            templateArtifact: source,
            privacyPolicyStamp: 'edge-export-policy-v1',
            sanitizedSnapshot: exportDataIslandSnapshotForEdge(previousSnapshot, {
                privacyPolicyStamp: 'edge-export-policy-v1',
                fields: { hostAttributes: 'allow', payload: 'redact' },
            }),
        });
        const nextRecord = createEdgeRenderStateRecord({
            renderPlan: nextPlan,
            templateArtifact: source,
            privacyPolicyStamp: 'edge-export-policy-v1',
            sanitizedSnapshot,
            stateKey: previousRecord.stateKey,
        });

        assertEqual(
            nextRecord.storageModel,
            'content-addressed-cache-with-revision-pointer-v1',
            'edge state uses the accepted hybrid storage model'
        );
        assertEqual(
            nextRecord.stateKey,
            previousRecord.stateKey,
            'revisioned pointer records keep a stable per-instance state key'
        );
        assertEqual(nextRecord.currentRenderPlan.kind, 'render-plan', 'render plan state is content-addressed');
        assertEqual(
            nextRecord.currentSnapshot?.kind,
            'sanitized-snapshot',
            'only sanitized snapshot exports are content-addressed'
        );
        assertEqual(nextRecord.renderRevision.dataRevision, '22', 'pointer record names the current data revision');
        assertEqual(nextRecord.privacyPolicyStamp, 'edge-export-policy-v1', 'pointer record carries the export policy stamp');
        assert(previousRecord.etag !== nextRecord.etag, 'revision pointer ETags change when the addressed state changes');
        assert(
            edgeRenderStateRevisionMatches(previousRecord, renderPlanIdentity(previousPlan)),
            'stored revision can be compared before advancing a pointer'
        );
        assert(
            !edgeRenderStateRevisionMatches(previousRecord, renderPlanIdentity(nextPlan)),
            'stale revision comparison prevents blind edge-state overwrites'
        );
        assertEqual(
            nextRecord.currentTemplateArtifact?.kind,
            'template-artifact',
            'template artifacts are content-addressed in pointer records when supplied'
        );

        const store = new InMemoryEdgeRenderStateStore();
        const initialWrite = store.writeRenderState({
            renderPlan: previousPlan,
            templateArtifact: source,
            privacyPolicyStamp: 'edge-export-policy-v1',
            sanitizedSnapshot: exportDataIslandSnapshotForEdge(previousSnapshot, {
                privacyPolicyStamp: 'edge-export-policy-v1',
                fields: { hostAttributes: 'allow', payload: 'redact' },
            }),
        });
        assert(initialWrite.ok, 'initial edge render state write succeeds');
        const storedPreviousPlan = store.getContent<typeof previousPlan>(initialWrite.record.currentRenderPlan);
        assertEqual(
            storedPreviousPlan?.dataRevision,
            '21',
            'content-addressed cache stores the previous render plan by address'
        );
        assert(initialWrite.record.currentTemplateArtifact, 'initial edge write stores a template artifact address');
        const storedTemplateArtifact = readEdgeContent<typeof source>(
            store,
            initialWrite.record.currentTemplateArtifact
        );
        assert(storedTemplateArtifact.ok, 'template artifact content reads back with a matching content address');
        assertEqual(
            storedTemplateArtifact.value[0]?.kind,
            'element',
            'stored template artifacts preserve the serialized template source'
        );

        const staleWrite = store.writeRenderState(
            {
                renderPlan: nextPlan,
                templateArtifact: source,
                privacyPolicyStamp: 'edge-export-policy-v1',
                sanitizedSnapshot,
                stateKey: initialWrite.record.stateKey,
            },
            { expectedEtag: 'stale-etag' }
        );
        assert(!staleWrite.ok, 'edge render state rejects writes with a stale ETag');
        assertEqual(
            staleWrite.current?.etag,
            initialWrite.record.etag,
            'stale write returns the current pointer record for retry decisions'
        );

        const acceptedWrite = store.writeRenderState(
            {
                renderPlan: nextPlan,
                templateArtifact: source,
                privacyPolicyStamp: 'edge-export-policy-v1',
                sanitizedSnapshot,
                stateKey: initialWrite.record.stateKey,
            },
            { expectedEtag: initialWrite.record.etag }
        );
        assert(acceptedWrite.ok, 'edge render state advances when the expected ETag matches');
        assertEqual(
            store.readRecord(initialWrite.record.stateKey)?.renderRevision.dataRevision,
            '22',
            'revision pointer advances to the accepted render revision'
        );
        assert(acceptedWrite.record.currentSnapshot, 'accepted edge state stores a sanitized snapshot address');
        const storedSnapshot = readEdgeContent<typeof sanitizedSnapshot>(
            store,
            acceptedWrite.record.currentSnapshot
        );
        assert(storedSnapshot.ok, 'sanitized snapshot content reads back with a matching content address');
        assertEqual(
            storedSnapshot.value.payload?.text,
            '',
            'stored snapshots are policy-sanitized before content addressing'
        );
        assertEqual(
            storedSnapshot.address.key,
            acceptedWrite.record.currentSnapshot.key,
            'verified snapshot content reports the pointer address'
        );
        const explicitEmptyWrite = store.writeRenderState(
            {
                renderPlan: nextPlan,
                templateArtifact: source,
                privacyPolicyStamp: 'edge-export-policy-v1',
                sanitizedSnapshot: null,
                renderedHtml: '',
                stateKey: `${initialWrite.record.stateKey}:explicit-empty`,
            }
        );
        assert(explicitEmptyWrite.ok, 'edge render state records explicit null snapshots and empty HTML');
        assert(explicitEmptyWrite.record.currentSnapshot, 'explicit null snapshot still receives a content address');
        assert(explicitEmptyWrite.record.currentHtml, 'empty rendered HTML still receives a content address');
        const explicitNullSnapshot = readEdgeContent<null>(
            store,
            explicitEmptyWrite.record.currentSnapshot
        );
        assert(explicitNullSnapshot.ok, 'explicit null snapshot reads back from content-addressed storage');
        assertEqual(explicitNullSnapshot.value, null, 'explicit null snapshot content is preserved');
        const explicitEmptyHtml = readEdgeContent<string>(
            store,
            explicitEmptyWrite.record.currentHtml
        );
        assert(explicitEmptyHtml.ok, 'empty rendered HTML reads back from content-addressed storage');
        assertEqual(explicitEmptyHtml.value, '', 'empty rendered HTML content is preserved');
        const explicitContents = readEdgeRenderStateContents(store, explicitEmptyWrite.record);
        assert(explicitContents.ok, 'edge state contents helper verifies every pointer in a record');
        assertEqual(explicitContents.contents.renderPlan.dataRevision, '22', 'contents helper returns the render plan');
        assertEqual(
            Array.isArray(explicitContents.contents.templateArtifact),
            true,
            'contents helper returns the template artifact when addressed'
        );
        assertEqual(
            explicitContents.contents.sanitizedSnapshot,
            null,
            'contents helper preserves an explicit null sanitized snapshot'
        );
        assertEqual(explicitContents.contents.renderedHtml, '', 'contents helper preserves empty rendered HTML');
        const missingHtmlContents = readEdgeRenderStateContents(
            new ContentOverrideStore(store, explicitEmptyWrite.record.currentHtml, 'missing'),
            explicitEmptyWrite.record
        );
        assert(!missingHtmlContents.ok, 'contents helper fails closed when addressed HTML is missing');
        assertEqual(missingHtmlContents.reason, 'missing-content', 'missing HTML reports missing content');
        assertEqual(
            missingHtmlContents.reason === 'missing-content' ? missingHtmlContents.field : '',
            'currentHtml',
            'missing HTML content reports the failed pointer field'
        );
        const corruptSnapshotContents = readEdgeRenderStateContents(
            new ContentOverrideStore(store, explicitEmptyWrite.record.currentSnapshot, 'replace', { redacted: false }),
            explicitEmptyWrite.record
        );
        assert(!corruptSnapshotContents.ok, 'contents helper fails closed when addressed snapshot content is corrupt');
        assertEqual(
            corruptSnapshotContents.reason,
            'content-address-mismatch',
            'corrupt snapshot content reports an address mismatch'
        );
        assertEqual(
            corruptSnapshotContents.reason === 'content-address-mismatch' ? corruptSnapshotContents.field : '',
            'currentSnapshot',
            'corrupt snapshot content reports the failed pointer field'
        );
        assert(explicitEmptyWrite.record.currentTemplateArtifact, 'explicit edge state stores a template artifact address');
        const missingTemplateContents = readEdgeRenderStateContents(
            new ContentOverrideStore(store, explicitEmptyWrite.record.currentTemplateArtifact, 'missing'),
            explicitEmptyWrite.record
        );
        assert(!missingTemplateContents.ok, 'contents helper fails closed when addressed template artifact is missing');
        assertEqual(
            missingTemplateContents.reason === 'missing-content' ? missingTemplateContents.field : '',
            'currentTemplateArtifact',
            'missing template artifact reports the failed pointer field'
        );
        const corruptTemplateContents = readEdgeRenderStateContents(
            new ContentOverrideStore(store, explicitEmptyWrite.record.currentTemplateArtifact, 'replace', { nodes: [] }),
            explicitEmptyWrite.record
        );
        assert(!corruptTemplateContents.ok, 'contents helper fails closed when addressed template artifact is corrupt');
        assertEqual(
            corruptTemplateContents.reason === 'content-address-mismatch' ? corruptTemplateContents.field : '',
            'currentTemplateArtifact',
            'corrupt template artifact reports the failed pointer field'
        );
        const missingRenderPlanContents = readEdgeRenderStateContents(
            new ContentOverrideStore(store, explicitEmptyWrite.record.currentRenderPlan, 'missing'),
            explicitEmptyWrite.record
        );
        assert(!missingRenderPlanContents.ok, 'contents helper fails closed when the required render plan is missing');
        assertEqual(
            missingRenderPlanContents.reason === 'missing-content' ? missingRenderPlanContents.field : '',
            'currentRenderPlan',
            'missing render plan reports the required pointer field'
        );
        const corruptRenderPlanContents = readEdgeRenderStateContents(
            new ContentOverrideStore(store, explicitEmptyWrite.record.currentRenderPlan, 'replace', previousPlan),
            explicitEmptyWrite.record
        );
        assert(!corruptRenderPlanContents.ok, 'contents helper fails closed when the required render plan is corrupt');
        assertEqual(
            corruptRenderPlanContents.reason === 'content-address-mismatch' ? corruptRenderPlanContents.field : '',
            'currentRenderPlan',
            'corrupt render plan reports the required pointer field'
        );

        const helperStore = new InMemoryEdgeRenderStateStore();
        const helperInitial = helperStore.writeRenderState({
            renderPlan: previousPlan,
            templateArtifact: source,
            privacyPolicyStamp: 'edge-export-policy-v1',
            sanitizedSnapshot: exportDataIslandSnapshotForEdge(previousSnapshot, {
                privacyPolicyStamp: 'edge-export-policy-v1',
                fields: { hostAttributes: 'allow', payload: 'redact' },
            }),
        });
        assert(helperInitial.ok, 'helper fixture can seed an initial edge state');
        const advanced = advanceEdgeRenderState(
            helperStore,
            {
                renderPlan: nextPlan,
                templateArtifact: source,
                privacyPolicyStamp: 'edge-export-policy-v1',
                sanitizedSnapshot,
                stateKey: helperInitial.record.stateKey,
            },
            { patchOptions: { batchSize: 1, transactionId: 'edge-store-tx-1' } }
        );
        assert(advanced.ok, 'store-backed edge advance succeeds from a matching pointer record');
        assertEqual(
            advanced.previousRenderPlan?.dataRevision,
            '21',
            'store-backed edge advance reads the previous content-addressed render plan'
        );
        const advancedTextPatch = opsFromPatchFrames(advanced.frames).find((op) => op.op === 'setText');
        assert(advancedTextPatch?.op === 'setText', 'store-backed edge advance emits patch frames');
        assertEqual(advancedTextPatch.value, 'Edge After', 'store-backed patch frames use the next render plan');
        assertEqual(
            helperStore.readRecord(helperInitial.record.stateKey)?.etag,
            advanced.record.etag,
            'store-backed edge advance commits the next pointer record'
        );

        const projectionStore = new InMemoryEdgeRenderStateStore();
        const projectionSeed = projectAndAdvanceEdgeRenderState(
            projectionStore,
            {
                source,
                projection: { snapshot: previousSnapshot, values: { label: 'Edge Before' } },
                privacyPolicyStamp: 'edge-export-policy-v1',
                sanitizedSnapshot: exportDataIslandSnapshotForEdge(previousSnapshot, {
                    privacyPolicyStamp: 'edge-export-policy-v1',
                    fields: { hostAttributes: 'allow', payload: 'redact' },
                }),
            }
        );
        assert(projectionSeed.ok, 'project-and-advance seeds edge state from serializable source and snapshot');
        const projectionAdvance = projectAndAdvanceEdgeRenderState(
            projectionStore,
            {
                source,
                projection: { snapshot: nextSnapshot, values: { label: 'Edge After' } },
                privacyPolicyStamp: 'edge-export-policy-v1',
                sanitizedSnapshot,
                stateKey: projectionSeed.record.stateKey,
            },
            { patchOptions: { transactionId: 'edge-project-tx-1' } }
        );
        assert(projectionAdvance.ok, 'project-and-advance renders and advances from serializable edge inputs');
        assertEqual(
            opsFromPatchFrames(projectionAdvance.frames).find((op) => op.op === 'setText')?.value,
            'Edge After',
            'project-and-advance emits patch frames from the projected render plan'
        );
        assertEqual(
            projectionStore.readRecord(projectionSeed.record.stateKey)?.renderRevision.dataRevision,
            '22',
            'project-and-advance commits the projected render revision'
        );
        assertEqual(
            projectionAdvance.record.currentTemplateArtifact?.kind,
            'template-artifact',
            'project-and-advance stores the serialized source as a template artifact'
        );

        const rejectedAdvance = advanceEdgeRenderState(
            helperStore,
            {
                renderPlan: previousPlan,
                templateArtifact: source,
                privacyPolicyStamp: 'edge-export-policy-v1',
                sanitizedSnapshot: exportDataIslandSnapshotForEdge(previousSnapshot, {
                    privacyPolicyStamp: 'edge-export-policy-v1',
                    fields: { hostAttributes: 'allow', payload: 'redact' },
                }),
                stateKey: helperInitial.record.stateKey,
            },
            { expectedEtag: 'stale-etag' }
        );
        assert(!rejectedAdvance.ok, 'store-backed edge advance rejects stale expected ETags');
        assertEqual(rejectedAdvance.reason, 'etag-mismatch', 'stale edge advance fails before returning frames');

        const firstRenderStore = new InMemoryEdgeRenderStateStore();
        const firstRender = advanceEdgeRenderState(
            firstRenderStore,
            {
                renderPlan: previousPlan,
                templateArtifact: source,
                privacyPolicyStamp: 'edge-export-policy-v1',
                sanitizedSnapshot: exportDataIslandSnapshotForEdge(previousSnapshot, {
                    privacyPolicyStamp: 'edge-export-policy-v1',
                    fields: { hostAttributes: 'allow', payload: 'redact' },
                }),
            },
            { patchOptions: { transactionId: 'edge-first-render-tx' } }
        );
        assert(firstRender.ok, 'store-backed first render succeeds without a previous pointer');
        assertEqual(firstRender.previousRenderPlan, null, 'first render has no previous content-addressed render plan');
        assert(
            opsFromPatchFrames(firstRender.frames).every((op) => op.op === 'replaceScope' && op.reason === 'first-render'),
            'first edge render emits first-render scope replacement frames'
        );

        const missingAddress = edgeContentAddress('render-plan', previousPlan);
        const brokenStore = new MissingRenderPlanStore(helperInitial.record, missingAddress);
        const missingPlan = advanceEdgeRenderState(
            brokenStore,
            {
                renderPlan: nextPlan,
                privacyPolicyStamp: 'edge-export-policy-v1',
                sanitizedSnapshot,
                stateKey: helperInitial.record.stateKey,
            }
        );
        assert(!missingPlan.ok, 'edge advance fails closed when the previous render-plan blob is missing');
        assertEqual(missingPlan.reason, 'missing-render-plan', 'missing previous content reports a specific failure reason');
        assertEqual(
            missingPlan.reason === 'missing-render-plan' ? missingPlan.address.key : '',
            missingAddress.key,
            'missing previous content reports the missing render-plan address'
        );

        const corruptStore = new CorruptRenderPlanStore(helperInitial.record, missingAddress, nextPlan);
        const corruptPlan = advanceEdgeRenderState(
            corruptStore,
            {
                renderPlan: nextPlan,
                privacyPolicyStamp: 'edge-export-policy-v1',
                sanitizedSnapshot,
                stateKey: helperInitial.record.stateKey,
            }
        );
        assert(!corruptPlan.ok, 'edge advance fails closed when content does not match its address');
        assertEqual(
            corruptPlan.reason,
            'content-address-mismatch',
            'corrupt previous content reports an address mismatch'
        );
        assertEqual(
            corruptPlan.reason === 'content-address-mismatch' ? corruptPlan.expected.key : '',
            missingAddress.key,
            'corrupt previous content reports the expected content address'
        );

        const revisionMismatchStore = new RevisionMismatchStore(helperStore, helperInitial.record.stateKey);
        const revisionMismatch = advanceEdgeRenderState(
            revisionMismatchStore,
            {
                renderPlan: nextPlan,
                privacyPolicyStamp: 'edge-export-policy-v1',
                sanitizedSnapshot,
                stateKey: helperInitial.record.stateKey,
            }
        );
        assert(!revisionMismatch.ok, 'edge advance fails closed when pointer revision metadata mismatches content');
        assertEqual(
            revisionMismatch.reason,
            'render-revision-mismatch',
            'mismatched pointer metadata reports a render revision mismatch'
        );
        assertEqual(
            revisionMismatch.reason === 'render-revision-mismatch' ? revisionMismatch.actual.dataRevision : '',
            '22',
            'revision mismatch reports the render-plan revision found in content'
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
        assertEqual(
            tagDiagnostic.sourceMapRef?.fidelity,
            'declaration-only',
            'declaration shape diagnostics use declaration-only source-map fidelity'
        );
        assertEqual(
            tagDiagnostic.sourceMapRef?.frame,
            'decl:Bad-Tag',
            'declaration-only diagnostics identify the owning declaration tag when available'
        );

        const missingTag = buildDeclaration({ templates: [{ html: '<button>x</button>' }] });
        runtime.registerDeclaration(missingTag);
        const missingTagDiagnostic = findDiagnostic(runtime.diagnosticsFor(missingTag), 'cem-element.tag_missing');
        assertEqual(
            missingTagDiagnostic.sourceMapRef?.frame,
            'decl:unknown',
            'declaration-only diagnostics fall back to an unknown declaration frame when no tag exists'
        );

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

        const noTemplate = buildDeclaration({ tag: 'story-decl-empty', liveContent: true });
        noTemplate.textContent = '{button | implicit}';
        runtime.registerDeclaration(noTemplate);
        assertEqual(
            runtime.diagnosticsFor(noTemplate).length,
            0,
            'inline declaration content is accepted as an implicit CEM-ML template'
        );
        assertEqual(
            (noTemplate.querySelector('template[type="text/cem-ml"]') as HTMLTemplateElement | null)?.content.textContent?.trim(),
            '{button | implicit}',
            'implicit declaration content is moved into an inert CEM-ML template'
        );

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
            assertEqual(
                diagnostic.sourceMapRef?.fidelity,
                'author-byte-exact',
                'CEM-ML parse diagnostics carry author-byte-exact source-map fidelity'
            );
            assert(/^cem:\d+$/.test(diagnostic.sourceMapRef?.frame ?? ''), 'CEM-ML parse diagnostics carry byte frames');
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
        assertEqual(
            renderFailure.sourceMapRef?.fidelity,
            'author-byte-exact',
            'render diagnostics carry author-byte-exact source-map fidelity'
        );
        assert(/^cem:\d+$/.test(renderFailure.sourceMapRef?.frame ?? ''), 'render diagnostics carry byte frames');

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

function findRuntimeSupportDiagnostic(
    diagnostics: readonly RuntimeSupportDiagnostic[],
    code: string
): RuntimeSupportDiagnostic {
    const diagnostic = diagnostics.find((entry) => entry.code === code);
    assert(diagnostic, `expected diagnostic ${code}`);
    return diagnostic;
}

class MissingRenderPlanStore implements EdgeRenderStateStore {
    private readonly fallback = new InMemoryEdgeRenderStateStore();

    constructor(
        private readonly record: EdgeRenderStateRecord,
        private readonly missingAddress: EdgeContentAddress
    ) {}

    putContent(kind: EdgeContentKind, value: unknown): EdgeContentAddress {
        return this.fallback.putContent(kind, value);
    }

    getContent<T = unknown>(address: EdgeContentAddress): T | undefined {
        return address.key === this.missingAddress.key ? undefined : this.fallback.getContent<T>(address);
    }

    readRecord(stateKey: string): EdgeRenderStateRecord | undefined {
        return stateKey === this.record.stateKey
            ? { ...this.record, currentRenderPlan: this.missingAddress }
            : undefined;
    }

    writeRecord(
        record: EdgeRenderStateRecord,
        options?: EdgeRenderStateWriteOptions
    ): EdgeRenderStateWriteResult {
        return this.fallback.writeRecord(record, options);
    }

    writeRenderState(input: Parameters<EdgeRenderStateStore['writeRenderState']>[0]): EdgeRenderStateWriteResult {
        return this.fallback.writeRenderState(input);
    }
}

class CorruptRenderPlanStore implements EdgeRenderStateStore {
    private readonly fallback = new InMemoryEdgeRenderStateStore();

    constructor(
        private readonly record: EdgeRenderStateRecord,
        private readonly expectedAddress: EdgeContentAddress,
        private readonly corruptPlan: RenderPlan
    ) {}

    putContent(kind: EdgeContentKind, value: unknown): EdgeContentAddress {
        return this.fallback.putContent(kind, value);
    }

    getContent<T = unknown>(address: EdgeContentAddress): T | undefined {
        return address.key === this.expectedAddress.key ? this.corruptPlan as T : this.fallback.getContent<T>(address);
    }

    readRecord(stateKey: string): EdgeRenderStateRecord | undefined {
        return stateKey === this.record.stateKey
            ? { ...this.record, currentRenderPlan: this.expectedAddress }
            : undefined;
    }

    writeRecord(
        record: EdgeRenderStateRecord,
        options?: EdgeRenderStateWriteOptions
    ): EdgeRenderStateWriteResult {
        return this.fallback.writeRecord(record, options);
    }

    writeRenderState(input: Parameters<EdgeRenderStateStore['writeRenderState']>[0]): EdgeRenderStateWriteResult {
        return this.fallback.writeRenderState(input);
    }
}

class RevisionMismatchStore implements EdgeRenderStateStore {
    constructor(
        private readonly source: EdgeRenderStateStore,
        private readonly mismatchStateKey: string
    ) {}

    putContent(kind: EdgeContentKind, value: unknown): EdgeContentAddress {
        return this.source.putContent(kind, value);
    }

    getContent<T = unknown>(address: EdgeContentAddress): T | undefined {
        return this.source.getContent<T>(address);
    }

    readRecord(stateKey: string): EdgeRenderStateRecord | undefined {
        const record = this.source.readRecord(stateKey);
        if (!record || stateKey !== this.mismatchStateKey) {
            return record;
        }
        return {
            ...record,
            renderRevision: {
                ...record.renderRevision,
                dataRevision: 'stale-pointer-revision',
            },
        };
    }

    writeRecord(
        record: EdgeRenderStateRecord,
        options?: EdgeRenderStateWriteOptions
    ): EdgeRenderStateWriteResult {
        return this.source.writeRecord(record, options);
    }

    writeRenderState(input: Parameters<EdgeRenderStateStore['writeRenderState']>[0]): EdgeRenderStateWriteResult {
        return this.source.writeRenderState(input);
    }
}

class ContentOverrideStore implements EdgeRenderStateStore {
    constructor(
        private readonly source: EdgeRenderStateStore,
        private readonly targetAddress: EdgeContentAddress,
        private readonly mode: 'missing' | 'replace',
        private readonly replacement?: unknown
    ) {}

    putContent(kind: EdgeContentKind, value: unknown): EdgeContentAddress {
        return this.source.putContent(kind, value);
    }

    getContent<T = unknown>(address: EdgeContentAddress): T | undefined {
        if (address.key !== this.targetAddress.key) {
            return this.source.getContent<T>(address);
        }
        return this.mode === 'missing' ? undefined : this.replacement as T;
    }

    readRecord(stateKey: string): EdgeRenderStateRecord | undefined {
        return this.source.readRecord(stateKey);
    }

    writeRecord(
        record: EdgeRenderStateRecord,
        options?: EdgeRenderStateWriteOptions
    ): EdgeRenderStateWriteResult {
        return this.source.writeRecord(record, options);
    }

    writeRenderState(input: Parameters<EdgeRenderStateStore['writeRenderState']>[0]): EdgeRenderStateWriteResult {
        return this.source.writeRenderState(input);
    }
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

function buildCemMlDeclaration(declarationTag: string, tag: string, text: string): HTMLElement {
    const declaration = document.createElement(declarationTag);
    declaration.setAttribute('tag', tag);
    const template = document.createElement('template');
    template.setAttribute('type', 'text/cem-ml');
    template.textContent = text;
    declaration.appendChild(template);
    return declaration;
}

async function appendResolutionPolicyFrame(parent: HTMLElement, baseHref: string): Promise<HTMLIFrameElement> {
    const frame = document.createElement('iframe');
    frame.hidden = true;
    frame.title = `resolution policy ${baseHref}`;
    const loaded = new Promise<void>((resolve) => frame.addEventListener('load', () => resolve(), { once: true }));
    frame.srcdoc = `<!doctype html><html><head><base href="${baseHref}"></head><body></body></html>`;
    parent.appendChild(frame);
    await loaded;
    assert(frame.contentWindow && frame.contentDocument, 'resolution policy frame should expose a same-origin document');
    return frame;
}

async function registerExternalSourceInstance(
    runtime: CemElementRuntime,
    doc: Document,
    producedTag: string
): Promise<HTMLElement> {
    const declaration = doc.createElement('cem-element-story-uri-source');
    declaration.setAttribute('tag', producedTag);
    declaration.setAttribute('src', './cards.html#card');
    doc.body.appendChild(declaration);
    assert(runtime.registerDeclaration(declaration), `${producedTag} external src declaration registers`);
    await runtime.whenDeclarationSettled(declaration);

    const instance = doc.createElement(producedTag);
    doc.body.appendChild(instance);
    await runtime.whenRenderSettled(instance);
    return instance;
}

async function registerModuleUrlInstance(
    root: HTMLElement,
    runtime: CemElementRuntime,
    declarationTag: string,
    producedTag: string,
    specifier: string,
    expectedHref: string
): Promise<HTMLElement> {
    const declaration = document.createElement(declarationTag);
    declaration.setAttribute('tag', producedTag);
    const template = document.createElement('template');
    template.innerHTML = `<module-url slice="asset" src="${specifier}"></module-url><a class="asset" href="{$asset}">${'${$asset}'}</a>`;
    declaration.appendChild(template);
    root.appendChild(declaration);
    assert(runtime.registerDeclaration(declaration), `${producedTag} module-url declaration registers`);
    await runtime.whenDeclarationSettled(declaration);

    const instance = document.createElement(producedTag);
    root.appendChild(instance);
    await waitForCondition(
        () => instance.querySelector('a.asset')?.getAttribute('href') === expectedHref,
        `${producedTag} module-url settles`
    );
    return instance;
}

function dispatchInput(root: ParentNode, value: string): void {
    const input = requiredElement(root, 'input') as HTMLInputElement;
    input.value = value;
    input.dispatchEvent(new Event('input', { bubbles: true }));
}

function requiredElement(root: ParentNode, selector: string): Element {
    const element = root.querySelector(selector);
    assert(element, `expected ${selector} to exist`);
    return element;
}

function nextFrame(): Promise<void> {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

async function* utf8Body(text: string): AsyncIterable<Uint8Array> {
    yield new TextEncoder().encode(text);
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
        sourceMapMode: 'dev',
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

function directPatchPlan(text: string): RenderPlan {
    return {
        producedTag: 'direct-patch-host',
        instanceId: 'direct-patch-instance',
        templateArtifactId: 'direct-patch-template',
        dataRevision: text,
        outputTarget: 'light-dom',
        scopePolicyStamp: 'direct-patch-scope',
        nodes: [{
            kind: 'element',
            namespace: null,
            tag: 'p',
            renderNodeId: 'direct-patch-1',
            attributes: [{ name: 'class', value: 'message' }],
            sourceMapRef: { fidelity: 'dom-canonical', frame: 'direct:0' },
            children: [{
                kind: 'text',
                text,
                sourceMapRef: { fidelity: 'dom-canonical', frame: 'direct:0/0' },
            }],
        }],
    };
}

function cloneRenderPlan(plan: RenderPlan): RenderPlan {
    return JSON.parse(JSON.stringify(plan)) as RenderPlan;
}

function emptySerializedPayload(): DataIslandSnapshot['payload'] {
    return {
        text: '',
        childCount: 0,
        nodes: [],
        slots: {},
        elementsByAttribute: {},
        data: [],
        options: [],
        dataByValue: {},
        optionsByValue: {},
    };
}
