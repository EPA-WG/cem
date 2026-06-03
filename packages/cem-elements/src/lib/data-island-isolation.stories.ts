import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { CemElementRuntime } from './cem-elements.js';

/**
 * Data-island isolation stories (todo §3.1).
 *
 * Both the `<cem-element>` declaration `<template>` and the per-instance
 * `<template data-cem-island="instance">` keep their contents inside a `.content`
 * DocumentFragment: inert, disconnected, and `display:none`. Only the projected render
 * output is committed to the connected light DOM. These stories prove that the
 * declaration and instance template contents do not affect layout, selectors, form
 * submission, accessibility/focus, or visible UI directly.
 */

const meta: Meta = {
    title: 'CEM Elements/Data Island Isolation',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const SelectorsDoNotPierceTheDataIsland: Story = {
    render: () =>
        mountIsolationStory({
            declarationTag: 'cem-element-iso-selector',
            producedTag: 'iso-selector-el',
            ariaLabel: 'data island selector isolation',
            templateHTML: '<button type="button">Go</button>',
            payloadHTML: '<span data-iso="payload">payload-secret</span>',
        }),
    play: async ({ canvasElement }) => {
        await nextFrame();

        const instance = requiredElement(canvasElement, 'iso-selector-el');
        const island = requiredElement(instance, 'template[data-cem-island="instance"]') as HTMLTemplateElement;

        // The captured payload moved into the inert island fragment...
        const captured = island.content.querySelector('[data-iso="payload"]');
        assert(captured !== null, 'fallback payload is captured into the data island');
        assert(!captured.isConnected, 'island content is disconnected from the live document');

        // ...so no live-DOM selector reaches it, from either the document or the instance.
        assertEqual(document.querySelector('[data-iso="payload"]'), null, 'island content is not selectable from the document');
        assertEqual(instance.querySelector('[data-iso="payload"]'), null, 'island content is not selectable from the instance');

        // The rendered output, by contrast, is live and selectable.
        const button = requiredElement(instance, 'button');
        assert(button.isConnected, 'projected render output is connected to the live document');

        assert(!instance.textContent?.includes('payload-secret'), 'island text does not leak into instance.textContent');
        assert(instance.textContent?.includes('Go') ?? false, 'rendered text is present in instance.textContent');
    },
};

export const DataIslandDoesNotAffectLayout: Story = {
    render: () =>
        mountIsolationStory({
            declarationTag: 'cem-element-iso-layout',
            producedTag: 'iso-layout-el',
            ariaLabel: 'data island layout isolation',
            templateHTML: '<button type="button" style="display:block;height:24px">Go</button>',
            payloadHTML: '<div data-iso="huge" style="height:5000px">huge fallback block</div>',
        }),
    play: async ({ canvasElement }) => {
        await nextFrame();

        const instance = requiredElement(canvasElement, 'iso-layout-el');
        const island = requiredElement(instance, 'template[data-cem-island="instance"]') as HTMLTemplateElement;

        assertEqual(getComputedStyle(island).display, 'none', 'the data island template is not displayed');

        const huge = island.content.querySelector('[data-iso="huge"]');
        assert(huge !== null && !huge.isConnected, 'the bulky fallback block lives inert in the island');
        assertEqual((huge as HTMLElement).getBoundingClientRect().height, 0, 'island content has no layout box');

        assert(
            instance.getBoundingClientRect().height < 1000,
            'the 5000px island block does not inflate the rendered layout'
        );
    },
};

export const DataIslandControlsDoNotParticipateInFormSubmission: Story = {
    render: () =>
        mountIsolationStory({
            declarationTag: 'cem-element-iso-form',
            producedTag: 'iso-form-el',
            ariaLabel: 'data island form submission isolation',
            templateHTML: '<input name="visible" value="ok" />',
            payloadHTML: '<input name="island-secret" value="leak" />',
            wrapInForm: true,
        }),
    play: async ({ canvasElement }) => {
        await nextFrame();

        const form = requiredElement(canvasElement, 'form[data-iso="form"]') as HTMLFormElement;
        const instance = requiredElement(canvasElement, 'iso-form-el');
        const island = requiredElement(instance, 'template[data-cem-island="instance"]') as HTMLTemplateElement;

        const data = new FormData(form);
        assertEqual(data.get('visible'), 'ok', 'the rendered control participates in form submission');
        assert(!data.has('island-secret'), 'the island control does not participate in form submission');

        assert(form.elements.namedItem('island-secret') === null, 'island controls are absent from form.elements');
        const secret = island.content.querySelector('[name="island-secret"]');
        assert(secret !== null && !secret.isConnected, 'the island control stays inert inside the island');
    },
};

export const DataIslandContentStaysOutOfTheAccessibilityTree: Story = {
    render: () =>
        mountIsolationStory({
            declarationTag: 'cem-element-iso-a11y',
            producedTag: 'iso-a11y-el',
            ariaLabel: 'data island accessibility isolation',
            templateHTML: '<button type="button" data-iso="real">Real</button>',
            payloadHTML: '<button type="button" data-iso="ghost">Ghost</button>',
        }),
    play: async ({ canvasElement }) => {
        await nextFrame();

        const instance = requiredElement(canvasElement, 'iso-a11y-el');
        const island = requiredElement(instance, 'template[data-cem-island="instance"]') as HTMLTemplateElement;

        // Only the rendered control exists in the live (accessible) DOM.
        assertEqual(instance.querySelectorAll('button').length, 1, 'only the rendered control is in the live DOM');
        const real = requiredElement(instance, 'button[data-iso="real"]') as HTMLButtonElement;

        const ghost = island.content.querySelector('[data-iso="ghost"]') as HTMLButtonElement | null;
        assert(ghost !== null, 'the would-be control is captured into the island');
        assert(!document.contains(ghost), 'island controls are not part of the document accessibility tree');

        // The rendered control is focusable; focus stays on it and can never reach the
        // island control, which is disconnected and thus outside focus/keyboard navigation.
        real.focus();
        assertEqual(document.activeElement, real, 'the rendered control is focusable');
        assert(document.activeElement !== ghost, 'focus never lands on island content');
    },
};

export const DeclarationElementRendersNoVisibleContent: Story = {
    render: () =>
        mountIsolationStory({
            declarationTag: 'cem-element-iso-decl',
            producedTag: 'iso-decl-el',
            ariaLabel: 'declaration element isolation',
            templateHTML: '<button type="button">Go</button>',
        }),
    play: async ({ canvasElement }) => {
        await nextFrame();

        const declaration = requiredElement(canvasElement, '[data-iso="declaration-host"]');
        const instance = requiredElement(canvasElement, 'iso-decl-el');

        // The declaration's template is render source, not page content: it renders nothing
        // where the declaration sits, even though instances render it.
        assertEqual(declaration.querySelector('button'), null, 'the declaration host renders no control of its own');
        assert(!declaration.textContent?.includes('Go'), 'declaration template text is inert (lives in <template>.content)');
        assert(
            declaration.getBoundingClientRect().height < 50,
            'the declaration host occupies no template-driven layout'
        );

        assert(requiredElement(instance, 'button').isConnected, 'the produced instance does render the template');
        assertEqual(
            canvasElement.querySelectorAll('button').length,
            1,
            'exactly one control is rendered — from the instance, not the declaration'
        );
    },
};

interface IsolationStorySpec {
    declarationTag: string;
    producedTag: string;
    ariaLabel: string;
    templateHTML: string;
    payloadHTML?: string;
    wrapInForm?: boolean;
    instanceAttributes?: Record<string, string>;
}

/**
 * Register an inline declaration (no install, so the declaration host stays an inert
 * undefined custom element) and mount a produced instance, optionally with fallback
 * payload and inside a form. Returns a detached root the harness connects to drive the
 * render loop.
 */
function mountIsolationStory(spec: IsolationStorySpec): HTMLElement {
    const root = document.createElement('section');
    root.setAttribute('aria-label', spec.ariaLabel);

    const runtime = new CemElementRuntime({ declarationTag: spec.declarationTag });
    const declaration = document.createElement(spec.declarationTag);
    declaration.setAttribute('tag', spec.producedTag);
    declaration.setAttribute('data-iso', 'declaration-host');
    const template = document.createElement('template');
    template.innerHTML = spec.templateHTML;
    declaration.appendChild(template);
    root.appendChild(declaration);
    runtime.registerDeclaration(declaration);

    const instance = document.createElement(spec.producedTag);
    for (const [name, value] of Object.entries(spec.instanceAttributes ?? {})) {
        instance.setAttribute(name, value);
    }
    if (spec.payloadHTML !== undefined) {
        instance.innerHTML = spec.payloadHTML;
    }

    if (spec.wrapInForm) {
        const form = document.createElement('form');
        form.setAttribute('data-iso', 'form');
        form.appendChild(instance);
        root.appendChild(form);
    } else {
        root.appendChild(instance);
    }
    return root;
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

function requiredElement(root: ParentNode, selector: string): Element {
    const element = root.querySelector(selector);
    assert(element, `expected ${selector} to exist`);
    return element;
}

function nextFrame(): Promise<void> {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}
