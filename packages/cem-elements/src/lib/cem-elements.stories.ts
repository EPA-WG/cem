import type { Meta, StoryObj } from '@storybook/web-components-vite';
import {
    CemElementRuntime,
    analyzeDeclarationShape,
    cemElements,
    isValidCustomElementName,
} from './cem-elements.js';

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
