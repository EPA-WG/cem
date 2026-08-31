import type { Meta, StoryObj } from '@storybook/web-components-vite';
import {
    CemElementRuntime,
    analyzeDeclarationShape,
    deterministicAnonymousTag,
} from './cem-elements.js';

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

export const DeclarationAndDataIslandIsolationMatrix: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'complete declaration and data island isolation matrix');

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-iso-matrix' });
        const declaration = document.createElement('cem-element-iso-matrix');
        declaration.setAttribute('tag', 'iso-matrix-el');
        declaration.setAttribute('data-iso', 'matrix-declaration');
        const template = document.createElement('template');
        template.innerHTML = [
            '<style data-iso="source-style">',
            '.iso-source-sentinel { --iso-source-projected: yes; }',
            '</style>',
            '<div class="iso-source-sentinel" data-iso="source-layout" style="height:24px">',
            '<span data-iso="source-text">rendered-source-text</span>',
            '<label>Rendered field <input data-iso="source-input" name="source-field" value="ok" /></label>',
            '<button data-iso="source-button" type="button" aria-label="Rendered source control">Rendered source control</button>',
            '</div>',
        ].join('');
        declaration.appendChild(template);
        runtime.registerDeclaration(declaration);

        const instance = document.createElement('iso-matrix-el');
        instance.innerHTML = [
            '<style data-iso="island-style">',
            '.iso-island-sentinel { --iso-island-leak: leaked; }',
            '</style>',
            '<div data-iso="island-layout" style="height:5000px">',
            '<span>island-secret-text</span>',
            '<input name="island-secret" value="leak" />',
            '<button data-iso="island-button" type="button" aria-label="Island ghost">Island ghost</button>',
            '</div>',
        ].join('');

        const islandSentinel = document.createElement('div');
        islandSentinel.className = 'iso-island-sentinel';
        islandSentinel.dataset.iso = 'island-sentinel';

        const form = document.createElement('form');
        form.dataset.iso = 'matrix-form';
        form.append(declaration, instance, islandSentinel);
        root.appendChild(form);
        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();

        const root = requiredElement(
            canvasElement,
            'section[aria-label="complete declaration and data island isolation matrix"]'
        ) as HTMLElement;
        const declaration = requiredElement(root, '[data-iso="matrix-declaration"]');
        const declarationTemplate = requiredElement(declaration, ':scope > template') as HTMLTemplateElement;
        const instance = requiredElement(root, 'iso-matrix-el') as HTMLElement;
        const island = requiredElement(
            instance,
            ':scope > template[data-cem-island="instance"]'
        ) as HTMLTemplateElement;
        const form = requiredElement(root, 'form[data-iso="matrix-form"]') as HTMLFormElement;

        const rawSourceLayout = requiredFragmentElement(declarationTemplate.content, '[data-iso="source-layout"]');
        const rawSourceInput = requiredFragmentElement(declarationTemplate.content, '[data-iso="source-input"]');
        const rawSourceButton = requiredFragmentElement(
            declarationTemplate.content,
            '[data-iso="source-button"]'
        ) as HTMLButtonElement;
        const rawSourceStyle = requiredFragmentElement(
            declarationTemplate.content,
            'style[data-iso="source-style"]'
        ) as HTMLStyleElement;
        const managedSourceStyle = requiredElement(
            declaration,
            ':scope > style[data-cem-declaration-style="private"]'
        ) as HTMLStyleElement;
        const islandLayout = requiredFragmentElement(island.content, '[data-iso="island-layout"]');
        const islandButton = requiredFragmentElement(
            island.content,
            '[data-iso="island-button"]'
        ) as HTMLButtonElement;
        const islandStyle = requiredFragmentElement(
            island.content,
            'style[data-iso="island-style"]'
        ) as HTMLStyleElement;

        // Selector APIs stop at both template boundaries. Only the rendered projection
        // is visible to live-tree queries and tag collections.
        assertEqual(declaration.querySelector('[data-iso="source-input"]'), null, 'declaration selectors stay inert');
        assertEqual(instance.querySelector('[data-iso="island-button"]'), null, 'instance selectors stay outside the island');
        assertEqual(root.querySelectorAll('[data-iso="source-button"]').length, 1, 'only the rendered source button matches');
        assertEqual(root.getElementsByTagName('button').length, 1, 'tag collections exclude raw and island controls');
        assertEqual(root.getElementsByTagName('input').length, 1, 'tag collections expose only the rendered field');
        assert(!rawSourceInput.isConnected, 'raw declaration controls remain disconnected');
        assert(!islandButton.isConnected, 'captured island controls remain disconnected');

        // Neither boundary contributes its raw text, layout boxes, or styles. The
        // deliberately rendered source projection remains the sole visible effect.
        assert(!root.textContent?.includes('island-secret-text'), 'island text is absent from live textContent');
        assert(!root.innerText.includes('island-secret-text'), 'island text is absent from visible innerText');
        assertEqual(countOccurrences(root.textContent ?? '', 'rendered-source-text'), 1, 'source text appears once via rendering');
        assertEqual(rawSourceLayout.getBoundingClientRect().height, 0, 'raw declaration content has no layout box');
        assertEqual(islandLayout.getBoundingClientRect().height, 0, 'island content has no layout box');
        assert(
            requiredElement(instance, '[data-iso="source-layout"]').getBoundingClientRect().height > 0,
            'the rendered projection has a layout box'
        );
        assertEqual(getComputedStyle(declarationTemplate).display, 'none', 'the declaration template is not displayed');
        assertEqual(getComputedStyle(island).display, 'none', 'the instance island template is not displayed');
        assertEqual(rawSourceStyle.sheet, null, 'raw declaration styles do not create a stylesheet');
        assertEqual(islandStyle.sheet, null, 'island styles do not create a stylesheet');
        assert(
            managedSourceStyle.sheet !== null,
            'the declaration-owned stylesheet is installed once beside the declaration template'
        );
        assert(
            managedSourceStyle.textContent?.includes('@scope (\n    iso-matrix-el') ?? false,
            'private declaration CSS uses the native produced-tag scope'
        );
        assertEqual(
            instance.querySelector('style[data-iso="source-style"]'),
            null,
            'declaration CSS is not cloned into instance DOM'
        );
        assertEqual(
            getComputedStyle(requiredElement(instance, '[data-iso="source-layout"]')).getPropertyValue(
                '--iso-source-projected'
            ).trim(),
            'yes',
            'only the rendered declaration projection affects computed style'
        );
        assertEqual(
            getComputedStyle(requiredElement(root, '[data-iso="island-sentinel"]')).getPropertyValue(
                '--iso-island-leak'
            ).trim(),
            '',
            'captured island styles do not affect computed style'
        );

        // Form ownership sees the one rendered control and neither inert source.
        const data = new FormData(form);
        assertEqual(data.getAll('source-field').join('|'), 'ok', 'the rendered field submits exactly once');
        assert(!data.has('island-secret'), 'captured island controls do not submit');
        assert(form.elements.namedItem('source-field') !== null, 'the rendered field belongs to the form');
        assertEqual(form.elements.namedItem('island-secret'), null, 'island controls are absent from form.elements');

        // Disconnected controls cannot join focus navigation or the accessibility tree.
        // The following accessibility item separately validates semantics of the live output.
        const renderedButton = requiredElement(instance, '[data-iso="source-button"]') as HTMLButtonElement;
        renderedButton.focus();
        assertEqual(document.activeElement, renderedButton, 'the rendered control receives focus');
        assert(!rawSourceButton.matches(':focus'), 'raw declaration controls are absent from document focus state');
        assert(!islandButton.matches(':focus'), 'island controls are absent from document focus state');
        assert(document.activeElement !== rawSourceButton, 'raw declaration controls cannot be document focus targets');
        assert(document.activeElement !== islandButton, 'island controls cannot be document focus targets');
        assert(!document.contains(rawSourceButton), 'raw declaration controls are outside the document tree');
        assert(!document.contains(islandButton), 'island controls are outside the document tree');
        assertEqual(root.querySelector('[aria-label="Island ghost"]'), null, 'island roles and names are absent live');
    },
};

export const DeclarationShapeGuardrailsPreventLiveData: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'declaration shape guardrails');
        return root;
    },
    play: () => {
        assertShape(buildShapeDeclaration('valid', '<template><button>Valid</button></template>'), true, []);
        assertShape(
            buildShapeDeclaration('implicit-cem-ml', '{button | Valid}'),
            false,
            ['cem-element.inline_template_missing', 'cem-element.declaration_live_content']
        );
        assertShape(
            buildShapeDeclaration('missing-template', ''),
            false,
            ['cem-element.inline_template_missing']
        );
        assertShape(
            buildShapeDeclaration('two-templates', '<template></template><template></template>'),
            false,
            ['cem-element.inline_template_count']
        );
        assertShape(
            buildShapeDeclaration('src-conflict', '<template></template>', './button.html#button'),
            false,
            ['cem-element.src_inline_template_conflict']
        );

        const liveDeclaration = buildShapeDeclaration(
            'live-content',
            '<template></template><p data-iso="live">Live declaration data</p>'
        );
        assertShape(liveDeclaration, false, ['cem-element.declaration_live_content']);

        const live = requiredElement(liveDeclaration, '[data-iso="live"]') as HTMLElement;
        assert(!live.isConnected, 'invalid live declaration content is never mounted by this guardrail test');
        assertEqual(
            live.textContent,
            'Live declaration data',
            'the rejected content is the exact live payload guarded by declaration analysis'
        );
    },
};

export const DeclarationAndInstanceStylesHaveSeparateOwnership: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'declaration and instance stylesheet ownership');
        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-style-contract' });

        const sharedOnly = styleContractDeclaration(
            'cem-element-style-contract',
            'style-shared-only',
            'abc-lib',
            '<style>:host { --shared-only: yes; }</style><div><slot></slot></div>'
        );
        const mixed = styleContractDeclaration(
            'cem-element-style-contract',
            'style-mixed',
            'abc-lib',
            [
                '<style>:host { --private-only: yes; }</style>',
                '<style scope="abc-lib">:host { --shared-mixed: yes; }</style>',
                '<div><slot></slot></div>',
            ].join('')
        );
        const mismatch = styleContractDeclaration(
            'cem-element-style-contract',
            'style-mismatch',
            'abc-lib',
            '<style scope="other-lib">:host { --mismatch: no; }</style><div></div>'
        );
        for (const declaration of [sharedOnly, mixed, mismatch]) {
            runtime.registerDeclaration(declaration);
        }

        const first = document.createElement('style-shared-only');
        first.dataset.testInstance = 'shared-only';
        const second = document.createElement('style-mixed');
        second.dataset.testInstance = 'mixed';
        second.innerHTML = '<template><style>:host { --instance-only: yes; }</style><span>payload</span></template>';
        const third = document.createElement('style-mismatch');
        third.dataset.testInstance = 'mismatch';
        root.append(sharedOnly, mixed, mismatch, first, second, third);
        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();

        const sharedOnly = requiredElement(canvasElement, '[data-test-instance="shared-only"]') as HTMLElement;
        const mixed = requiredElement(canvasElement, '[data-test-instance="mixed"]') as HTMLElement;
        const sharedDeclaration = requiredElement(canvasElement, 'cem-element-style-contract[tag="style-shared-only"]');
        const mixedDeclaration = requiredElement(canvasElement, 'cem-element-style-contract[tag="style-mixed"]');
        const mismatchDeclaration = requiredElement(canvasElement, 'cem-element-style-contract[tag="style-mismatch"]');

        assertEqual(sharedOnly.getAttribute('scope'), 'abc-lib', 'a declaration scope marks group membership');
        assertEqual(mixed.getAttribute('scope'), 'abc-lib', 'the same group may span component tags');
        assert(sharedOnly.hasAttribute('data-cem-render-scope'), 'render identity is stored separately');

        const sharedStyle = requiredElement(
            sharedDeclaration,
            ':scope > style[data-cem-declaration-style="shared"]'
        );
        assert(
            sharedStyle.textContent?.includes('[scope="abc-lib"]:has(> template[data-cem-island="instance"])') ?? false,
            'a lone bare style on a scoped declaration uses the shared native scope boundary'
        );

        const privateStyle = requiredElement(
            mixedDeclaration,
            ':scope > style[data-cem-declaration-style="private"]'
        );
        const explicitSharedStyle = requiredElement(
            mixedDeclaration,
            ':scope > style[data-cem-declaration-style="shared"]'
        );
        assert(
            privateStyle.textContent?.startsWith('@scope (\n    style-mixed') ?? false,
            'a bare style becomes private when an explicit scoped style coexists'
        );
        assert(
            explicitSharedStyle.textContent?.includes('[scope="abc-lib"]:has(> template[data-cem-island="instance"])') ?? false,
            'the explicit matching style remains shared'
        );

        const instanceStyle = requiredElement(mixed, 'style') as HTMLStyleElement;
        assert(
            instanceStyle.textContent?.startsWith('@scope to (') ?? false,
            'payload CSS uses the implicit parent-rooted native scope'
        );
        assert(!mixed.hasAttribute('data-cem-instance-scope'), 'instance CSS needs no generated marker');
        assert(!instanceStyle.hasAttribute('data-cem-render-scope'), 'style nodes are never stamped as render roots');

        assertEqual(
            getComputedStyle(mixed).getPropertyValue('--shared-only').trim(),
            'yes',
            'shared rules from another declaration participate in the ordinary document cascade'
        );
        assertEqual(getComputedStyle(mixed).getPropertyValue('--private-only').trim(), 'yes', 'private rules target their tag');
        assertEqual(getComputedStyle(mixed).getPropertyValue('--instance-only').trim(), 'yes', 'instance rules override in instance scope');

        assertEqual(
            mismatchDeclaration.querySelectorAll(':scope > style[data-cem-declaration-style]').length,
            0,
            'a mismatched shared stylesheet is not installed'
        );
    },
};

export const AnonymousDeclarationGetsDeterministicTagAndInstance: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'anonymous declaration lifecycle');
        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-anonymous-contract' });
        runtime.install(window);
        const declaration = document.createElement('cem-element-anonymous-contract');
        declaration.setAttribute('uid-seed', 'anonymous/story');
        const template = document.createElement('template');
        template.innerHTML = '<strong>anonymous content</strong>';
        declaration.append(template);

        const srcTemplate = document.createElement('template');
        srcTemplate.id = 'anonymous-src-story-template';
        srcTemplate.innerHTML = '<article><slot></slot></article>';
        const srcDeclaration = document.createElement('cem-element-anonymous-contract');
        srcDeclaration.id = 'anonymous-src-story-declaration';
        srcDeclaration.setAttribute('uid-seed', 'anonymous/src-story');
        srcDeclaration.setAttribute('src', '#anonymous-src-story-template');
        srcDeclaration.setAttribute('data-flavor', 'pear');
        srcDeclaration.setAttribute('title', 'Anonymous src instance');
        const srcPayload = document.createElement('em');
        srcPayload.textContent = 'anonymous src payload';
        srcDeclaration.append(srcPayload);

        root.append(srcTemplate, declaration, srcDeclaration);
        return root;
    },
    play: async ({ canvasElement }) => {
        await nextFrame();
        await nextFrame();
        const declaration = requiredElement(canvasElement, 'cem-element-anonymous-contract') as HTMLElement;
        const tag = declaration.getAttribute('tag') ?? '';
        assert(
            /^cem-[0-9a-f]{8}-[0-9a-f]{4}-8[0-9a-f]{3}-8[0-9a-f]{3}-[0-9a-f]{12}$/.test(tag),
            'anonymous declarations receive a deterministic UUID-shaped custom-element tag'
        );
        const instance = requiredElement(canvasElement, `${tag}[data-cem-anonymous-instance]`);
        assertEqual(instance.parentElement, declaration, 'the anonymous instance renders inside its declaration container');
        assertEqual(instance.textContent?.trim(), 'anonymous content', 'anonymous declarations create and render their instance');

        const repeat = document.createElement('cem-element-anonymous-contract');
        repeat.setAttribute('uid-seed', 'anonymous/story');
        const repeatTemplate = document.createElement('template');
        repeatTemplate.innerHTML = '<strong>anonymous content</strong>';
        repeat.append(repeatTemplate);
        const firstDetached = declaration.cloneNode(true) as HTMLElement;
        firstDetached.removeAttribute('tag');
        firstDetached.removeAttribute('data-cem-anonymous-declaration');
        assertEqual(
            deterministicAnonymousTag(firstDetached),
            deterministicAnonymousTag(repeat),
            'the same detached declaration seed and source produce the same anonymous tag'
        );

        const srcDeclaration = requiredElement(
            canvasElement,
            '#anonymous-src-story-declaration'
        ) as HTMLElement;
        const srcTag = srcDeclaration.getAttribute('tag') ?? '';
        const srcInstance = await waitForElement(
            canvasElement,
            '#anonymous-src-story-declaration > [data-cem-anonymous-instance]'
        ) as HTMLElement;
        assertEqual(srcInstance.localName, srcTag, 'an anonymous src declaration creates its generated contained instance');
        assertEqual(srcInstance.parentElement, srcDeclaration, 'the anonymous src declaration is the rendered instance owner');
        assertEqual(srcInstance.getAttribute('data-flavor'), 'pear', 'anonymous instance data attributes move from the declaration');
        assertEqual(srcInstance.getAttribute('title'), 'Anonymous src instance', 'anonymous instance public attributes move from the declaration');
        assertEqual(
            requiredElement(srcInstance, 'article em').textContent,
            'anonymous src payload',
            'anonymous src payload enters the generated instance data-island lifecycle'
        );
        assertEqual(
            srcDeclaration.querySelector(':scope > em'),
            null,
            'anonymous src payload is no longer a direct declaration child after ownership moves to the generated instance'
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

function styleContractDeclaration(
    declarationTag: string,
    producedTag: string,
    scope: string,
    templateHTML: string
): HTMLElement {
    const declaration = document.createElement(declarationTag);
    declaration.setAttribute('tag', producedTag);
    declaration.setAttribute('scope', scope);
    const template = document.createElement('template');
    template.innerHTML = templateHTML;
    declaration.append(template);
    return declaration;
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

async function waitForElement(root: ParentNode, selector: string, frames = 120): Promise<Element> {
    for (let frame = 0; frame < frames; frame += 1) {
        const element = root.querySelector(selector);
        if (element) return element;
        await nextFrame();
    }
    throw new Error(`expected ${selector} to exist after ${frames} frames`);
}

function requiredFragmentElement(root: ParentNode, selector: string): HTMLElement {
    return requiredElement(root, selector) as HTMLElement;
}

function countOccurrences(value: string, needle: string): number {
    return value.split(needle).length - 1;
}

function buildShapeDeclaration(label: string, html: string, src?: string): HTMLElement {
    const declaration = document.createElement('div');
    declaration.setAttribute('tag', `iso-shape-${label}`);
    if (src) {
        declaration.setAttribute('src', src);
    }
    declaration.innerHTML = html;
    return declaration;
}

function assertShape(declaration: Element, ok: boolean, diagnostics: string[]): void {
    const result = analyzeDeclarationShape({
        tag: declaration.getAttribute('tag'),
        src: declaration.getAttribute('src'),
        directTemplateCount: directTemplateCount(declaration),
        directLiveNodeCount: directLiveNodeCount(declaration),
    });

    const label = declaration.getAttribute('tag') ?? 'declaration';
    assertEqual(result.ok, ok, `${label} declaration validity`);
    for (const code of diagnostics) {
        assert(
            result.diagnostics.some((diagnostic) => diagnostic.code === code),
            `${label} declaration emits ${code}`
        );
    }
}

function directTemplateCount(element: Element): number {
    return Array.from(element.children).filter((child) => child.localName === 'template').length;
}

function directLiveNodeCount(element: Element): number {
    return Array.from(element.childNodes).filter((node) => {
        if (node.nodeType === Node.ELEMENT_NODE) return (node as Element).localName !== 'template';
        if (node.nodeType === Node.TEXT_NODE) return (node.textContent ?? '').trim().length > 0;
        return false;
    }).length;
}

function nextFrame(): Promise<void> {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}
