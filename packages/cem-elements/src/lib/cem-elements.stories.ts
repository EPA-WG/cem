import type { Meta, StoryObj } from '@storybook/web-components-vite';
import {
    CemElementRuntime,
    analyzeDeclarationShape,
    cemElements,
    isValidCustomElementName,
    type DataIslandSnapshot,
} from './cem-elements.js';
import { projectTemplate, readTemplateSource, type TemplateSourceNode } from './projection.js';

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
            {button @type=button @aria-busy={$busy} | \${$label}}
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
        assertEqual(button.getAttribute('aria-busy'), '', 'canonical CEM-ML braced attribute values should render');
        assertEqual(
            button.getAttribute('data-cem-source-fidelity'),
            'author-byte-exact',
            'canonical CEM-ML templates carry source fidelity'
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
        parserTemplate.textContent = '{button @ | Broken}';
        parserDeclaration.appendChild(parserTemplate);
        root.appendChild(parserDeclaration);
        parserRuntime.registerDeclaration(parserDeclaration);

        assertDiagnostic(
            parserRuntime.diagnosticsFor(parserDeclaration),
            'cem-element.cem_ml.attribute_name_missing'
        );

        const renderRuntime = new CemElementRuntime({ declarationTag: 'cem-element-story-render-diagnostic' });
        renderRuntime.install(window);
        const renderDeclaration = document.createElement('cem-element-story-render-diagnostic');
        renderDeclaration.setAttribute('tag', 'story-render-diagnostic');
        const renderTemplate = document.createElement('template');
        renderTemplate.setAttribute('type', 'text/cem-ml');
        renderTemplate.textContent = '{$ | .name}';
        renderDeclaration.appendChild(renderTemplate);
        root.appendChild(renderDeclaration);
        renderRuntime.registerDeclaration(renderDeclaration);

        const instance = document.createElement('story-render-diagnostic');
        root.appendChild(instance);
        await nextFrame();

        assertDiagnostic(renderRuntime.diagnosticsFor(instance), 'cem-element.render_failed');
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

function assertDiagnostic(diagnostics: Array<{ code: string }>, code: string): void {
    assert(
        diagnostics.some((diagnostic) => diagnostic.code === code),
        `expected diagnostic ${code}`
    );
}

function requiredElement(root: ParentNode, selector: string): Element {
    const element = root.querySelector(selector);
    assert(element, `expected ${selector} to exist`);
    return element;
}

function nextFrame(): Promise<void> {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
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
        payload: { text: '', childCount: 0 },
        slices: {},
        validationState: {},
        eventPayloads: {},
    };
}
