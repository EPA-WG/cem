import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';

import { CemElementRuntime } from './cem-elements.js';

const GROUP_SCOPE = 'css-matrix-lib';
const EXPECTED_RESULTS = {
    declarationScopeResolutionMatrix:
        'Expected result: private styles install once under the component tag, shared styles install separately under the named scope, invalid combinations fail closed, and an outer selector overrides the zero-specificity boundary.',
    instanceStylesStayWithOneInstance:
        'Expected result: declaration CSS is shared by both instances, while payload CSS stays inside and overrides only the instance that supplied it.',
    scopeLimitsAndCascade:
        'Expected result: native scope proximity beats an equal page rule, public specificity can override intentionally, projected and nested roots remain styleable while their owned descendants are limited, and inheritance still crosses the limits.',
    staticOnlyStylesFailClosed:
        'Expected result: static styles are extracted and safely rewritten, unsupported global CSS is suppressed, and dynamic style generation produces diagnostics without rendered styles.',
    fragmentAndAnonymousDeclarationsUseEffectiveTags:
        'Expected result: fragment-reused CSS targets the consuming tag, while an anonymous declaration generates a stable tag and scopes its CSS to that tag.',
} as const;

const meta: Meta = {
    title: 'CEM Elements/CSS Scoping',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

interface ScopingStoryState {
    runtime: CemElementRuntime;
    declarations: Record<string, HTMLElement>;
    instances: HTMLElement[];
}
const createElement = (tag: string) => document.createElement(tag);

export const DeclarationScopeResolutionMatrix: Story = {
    render: () => {
        const root = createElement('section') as HTMLElement & {
            __cssScoping?: ScopingStoryState;
        };
        root.setAttribute('data-css-scope-matrix', '');
        root.setAttribute('aria-label', 'declaration stylesheet scope resolution matrix');
        appendExpectedResult(root, EXPECTED_RESULTS.declarationScopeResolutionMatrix);

        const outerStyle = createElement('style');
        outerStyle.textContent = [
            ':where(section[data-css-scope-matrix]) css-scope-private {',
            '  --cascade-winner: outer;',
            '}',
        ].join('\n');
        root.append(outerStyle);
        const table = createElement('table');
        table.innerHTML = '<tbody><tr><th scope="row" data-native-th>Native row header</th></tr></tbody>';
        root.append(table);

        const runtime = new CemElementRuntime({
            declarationTag: 'cem-element-css-scope-matrix',
        });
        const declarations: Record<string, HTMLElement> = {};
        const instances: HTMLElement[] = [];
        const specs = [
            {
                key: 'private',
                tag: 'css-scope-private',
                template: [
                    '<style>:host { --private-only: yes; --cascade-winner: private; }</style>',
                    '<div>private</div>',
                ].join(''),
                count: 2,
            },
            {
                key: 'sharedBare',
                tag: 'css-scope-shared-bare',
                scope: GROUP_SCOPE,
                template: '<style>:host { --shared-bare: yes; }</style><div>shared bare</div>',
            },
            {
                key: 'sharedExplicit',
                tag: 'css-scope-shared-explicit',
                scope: GROUP_SCOPE,
                template: `<style scope="${GROUP_SCOPE}">:host { --shared-explicit: yes; }</style><div>shared explicit</div>`,
            },
            {
                key: 'mixed',
                tag: 'css-scope-mixed',
                scope: GROUP_SCOPE,
                template: [
                    '<style>:host { --mixed-private: yes; }</style>',
                    `<style scope="${GROUP_SCOPE}">:host { --mixed-shared: yes; }</style>`,
                    '<div>mixed</div>',
                ].join(''),
                count: 2,
            },
            {
                key: 'unscopedExplicit',
                tag: 'css-scope-unscoped-explicit',
                template: `<style scope="${GROUP_SCOPE}">:host { --must-not-install: yes; }</style><div>invalid unscoped explicit</div>`,
            },
            {
                key: 'mismatch',
                tag: 'css-scope-mismatch',
                scope: GROUP_SCOPE,
                template: '<style scope="other-lib">:host { --must-not-install: yes; }</style><div>mismatch</div>',
            },
            {
                key: 'mismatchWithBare',
                tag: 'css-scope-mismatch-bare',
                scope: GROUP_SCOPE,
                template: [
                    '<style>:host { --mismatch-bare-shared: yes; }</style>',
                    '<style scope="other-lib">:host { --must-not-install: yes; }</style>',
                    '<div>mismatch plus valid bare</div>',
                ].join(''),
            },
            {
                key: 'invalidDeclaration',
                tag: 'css-scope-invalid-declaration',
                scope: 'abc lib',
                template:
                    '<style>:host { --invalid-declaration-private: yes; }</style><div>invalid declaration scope</div>',
            },
        ];

        for (const spec of specs) {
            const declaration = domDeclaration('cem-element-css-scope-matrix', spec.tag, spec.template, spec.scope);
            declarations[spec.key] = declaration;
            root.append(declaration);
            runtime.registerDeclaration(declaration);
            for (let index = 0; index < (spec.count ?? 1); index += 1) {
                const instance = createElement(spec.tag);
                instance.setAttribute('data-css-case', spec.key);
                instances.push(instance);
                root.append(instance);
            }
        }

        root.__cssScoping = { runtime, declarations, instances };
        return root;
    },
    play: async ({ canvasElement }) => {
        const root = requiredElement(canvasElement, '[data-css-scope-matrix]') as HTMLElement & {
            __cssScoping?: ScopingStoryState;
        };
        await expect(requiredElement(root, '[data-expected-result]').textContent).toBe(
            EXPECTED_RESULTS.declarationScopeResolutionMatrix,
        );
        const state = root.__cssScoping;
        await expect(state).toBeDefined();
        if (!state) return;
        await settle(state);

        const privateInstances = elements(root, '[data-css-case="private"]');
        const groupInstances = elements(root, `[scope="${GROUP_SCOPE}"]`).filter(
            (element) => element.querySelector(':scope > template[data-cem-island="instance"]') !== null,
        );
        const privateStyles = managedStyles(state.declarations.private);
        await expect(privateInstances).toHaveLength(2);
        await expect(privateStyles).toHaveLength(1);
        await expect(privateStyles[0]?.dataset.cemDeclarationStyle).toBe('private');
        await expect(privateStyles[0]?.textContent).toContain('@scope (\n    css-scope-private');
        await expect(privateInstances.every((instance) => instance.querySelector('style') === null)).toBe(true);
        await expect(cssValue(privateInstances[0], '--private-only')).toBe('yes');
        await expect(cssValue(privateInstances[0], '--cascade-winner')).toBe('outer');
        await expect(privateInstances[0]?.hasAttribute('data-cem-render-scope')).toBe(true);
        await expect(privateInstances[0]?.hasAttribute('scope')).toBe(false);
        await expect(privateInstances[0]?.hasAttribute('data-cem-scope')).toBe(false);
        await expect(privateInstances[0]?.hasAttribute('data-cem-instance-scope')).toBe(false);

        const sharedBareStyles = managedStyles(state.declarations.sharedBare);
        await expect(sharedBareStyles).toHaveLength(1);
        await expect(sharedBareStyles[0]?.dataset.cemDeclarationStyle).toBe('shared');
        await expect(sharedBareStyles[0]?.textContent).toContain(
            `[scope="${GROUP_SCOPE}"]:has(> template[data-cem-island="instance"])`,
        );

        const sharedExplicitStyles = managedStyles(state.declarations.sharedExplicit);
        await expect(sharedExplicitStyles).toHaveLength(1);
        await expect(sharedExplicitStyles[0]?.dataset.cemDeclarationStyle).toBe('shared');

        const mixedStyles = managedStyles(state.declarations.mixed);
        await expect(mixedStyles).toHaveLength(2);
        await expect(mixedStyles.map((style) => style.dataset.cemDeclarationStyle)).toEqual(['private', 'shared']);
        await expect(mixedStyles[0]?.textContent).toContain('@scope (\n    css-scope-mixed');
        await expect(mixedStyles[1]?.textContent).toContain(
            `[scope="${GROUP_SCOPE}"]:has(> template[data-cem-island="instance"])`,
        );
        await expect(
            mixedStyles.some((style) =>
                style.textContent?.includes(`css-scope-mixed, [scope="${GROUP_SCOPE}"]`),
            ),
        ).toBe(false);

        await expect(groupInstances.length).toBeGreaterThanOrEqual(6);
        for (const instance of groupInstances) {
            await expect(cssValue(instance, '--shared-bare')).toBe('yes');
            await expect(cssValue(instance, '--shared-explicit')).toBe('yes');
            await expect(cssValue(instance, '--mixed-shared')).toBe('yes');
            await expect(cssValue(instance, '--mismatch-bare-shared')).toBe('yes');
        }
        await expect(cssValue(requiredElement(root, '[data-native-th]'), '--shared-bare')).toBe('');
        const mixed = requiredElement(root, '[data-css-case="mixed"]');
        await expect(cssValue(mixed, '--mixed-private')).toBe('yes');

        await expect(managedStyles(state.declarations.unscopedExplicit)).toHaveLength(0);
        await expect(managedStyles(state.declarations.mismatch)).toHaveLength(0);
        const mismatchBareStyles = managedStyles(state.declarations.mismatchWithBare);
        await expect(mismatchBareStyles).toHaveLength(1);
        await expect(mismatchBareStyles[0]?.dataset.cemDeclarationStyle).toBe('shared');
        await expect(cssValue(requiredElement(root, '[data-css-case="mismatchWithBare"]'), '--must-not-install')).toBe(
            '',
        );

        const invalidDeclaration = requiredElement(root, '[data-css-case="invalidDeclaration"]');
        await expect(invalidDeclaration.hasAttribute('scope')).toBe(false);
        await expect(managedStyles(state.declarations.invalidDeclaration)).toHaveLength(1);
        await expect(managedStyles(state.declarations.invalidDeclaration)[0]?.dataset.cemDeclarationStyle).toBe(
            'private',
        );
        await expect(cssValue(invalidDeclaration, '--invalid-declaration-private')).toBe('yes');

        await expect(diagnosticCodes(state, 'unscopedExplicit')).toContain('cem-element.stylesheet_scope_mismatch');
        await expect(diagnosticCodes(state, 'mismatch')).toContain('cem-element.stylesheet_scope_mismatch');
        await expect(diagnosticCodes(state, 'mismatchWithBare')).toContain('cem-element.stylesheet_scope_mismatch');
        await expect(diagnosticCodes(state, 'invalidDeclaration')).toContain('cem-element.stylesheet_scope_invalid');

        const shared = requiredElement(root, '[data-css-case="sharedBare"]');
        shared.setAttribute('scope', 'page-owned');
        await nextFrame();
        await expect(shared.getAttribute('scope')).toBe(GROUP_SCOPE);
        await expect(state.runtime.diagnosticsFor(shared).map((diagnostic) => diagnostic.code)).toContain(
            'cem-element.scope_mutation_restored',
        );
        shared.removeAttribute('scope');
        await nextFrame();
        await expect(shared.getAttribute('scope')).toBe(GROUP_SCOPE);
        privateInstances[0]?.setAttribute('scope', GROUP_SCOPE);
        await nextFrame();
        await expect(privateInstances[0]?.hasAttribute('scope')).toBe(false);
    },
};

export const InstanceStylesStayWithOneInstance: Story = {
    render: () => {
        const root = createElement('section') as HTMLElement & {
            __cssScoping?: ScopingStoryState;
        };
        root.setAttribute('data-instance-css-story', '');
        root.setAttribute('aria-label', 'instance stylesheet ownership');
        appendExpectedResult(root, EXPECTED_RESULTS.instanceStylesStayWithOneInstance);
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-element-css-instance',
        });
        const declaration = domDeclaration(
            'cem-element-css-instance',
            'css-scope-instance',
            [
                '<style>',
                ':host { --instance-color: rgb(0, 0, 255); }',
                'button { color: var(--instance-color); }',
                '</style>',
                '<button type="button"><slot></slot></button>',
            ].join(''),
        );
        root.append(declaration);
        runtime.registerDeclaration(declaration);

        const base = createElement('css-scope-instance');
        base.setAttribute('data-css-case', 'base-instance');
        base.textContent = 'blue';
        const overridden = createElement('css-scope-instance');
        overridden.setAttribute('data-css-case', 'overridden-instance');
        overridden.innerHTML = [
            '<template>',
            '<style>:host { --instance-color: rgb(255, 0, 0); }</style>',
            '<span><strong>red</strong></span>',
            '<template data-literal><b>kept inert</b></template>',
            '</template>',
        ].join('');
        const bare = createElement('css-scope-instance');
        bare.setAttribute('data-css-case', 'bare-style-instance');
        bare.innerHTML = '<style>:host { --instance-color: rgb(255, 0, 0); }</style>bare';
        const mixed = createElement('css-scope-instance');
        mixed.setAttribute('data-css-case', 'mixed-payload-instance');
        mixed.innerHTML = [
            '<template><style>:host { --instance-color: rgb(255, 0, 0); }</style><span>inert</span></template>',
            '<span>live sibling</span>',
        ].join('');
        root.append(base, overridden, bare, mixed);
        root.__cssScoping = {
            runtime,
            declarations: { instance: declaration },
            instances: [base, overridden, bare, mixed],
        };
        return root;
    },
    play: async ({ canvasElement }) => {
        const root = requiredElement(canvasElement, '[data-instance-css-story]') as HTMLElement & {
            __cssScoping?: ScopingStoryState;
        };
        await expect(requiredElement(root, '[data-expected-result]').textContent).toBe(
            EXPECTED_RESULTS.instanceStylesStayWithOneInstance,
        );
        const state = root.__cssScoping;
        await expect(state).toBeDefined();
        if (!state) return;
        await settle(state);

        const base = requiredElement(root, '[data-css-case="base-instance"]');
        const overridden = requiredElement(root, '[data-css-case="overridden-instance"]');
        const bare = requiredElement(root, '[data-css-case="bare-style-instance"]');
        const mixed = requiredElement(root, '[data-css-case="mixed-payload-instance"]');
        await expect(managedStyles(state.declarations.instance)).toHaveLength(1);
        await expect(base.querySelector('style')).toBeNull();
        const payloadStyle = Array.from(overridden.children).find((child) => child.localName === 'style') ?? null;
        await expect(payloadStyle).not.toBeNull();
        await expect(payloadStyle?.textContent).toContain('@scope to (');
        await expect(payloadStyle?.textContent).toContain(':scope { --instance-color: rgb(255, 0, 0); }');
        await expect(payloadStyle?.hasAttribute('data-cem-render-scope')).toBe(false);
        await expect(overridden.hasAttribute('data-cem-instance-scope')).toBe(false);
        const island = requiredElement(overridden, 'template[data-cem-island="instance"]') as HTMLTemplateElement;
        await expect(island.content.querySelector('style')?.textContent).toContain(':host');
        const projected = requiredElement(overridden, 'button > span');
        await expect(projected).toHaveAttribute('slot', '');
        await expect(requiredElement(projected, 'strong')).not.toHaveAttribute('slot');
        const literal = requiredElement(overridden, 'button > template[data-literal]') as HTMLTemplateElement;
        await expect(literal).toHaveAttribute('slot', '');
        await expect(literal.content.querySelector('b')?.textContent).toBe('kept inert');
        await expect(getComputedStyle(requiredElement(base, 'button')).color).toBe('rgb(0, 0, 255)');
        await expect(getComputedStyle(requiredElement(overridden, 'button')).color).toBe('rgb(255, 0, 0)');
        await expect(bare.querySelector(':scope > style')).toBeNull();
        await expect(getComputedStyle(requiredElement(bare, 'button')).color).toBe('rgb(0, 0, 255)');
        await expect(state.runtime.diagnosticsFor(bare).map((diagnostic) => diagnostic.code)).toContain(
            'cem-element.instance_style_unenveloped',
        );
        await expect(mixed.querySelector(':scope > style')).toBeNull();
        await expect(requiredElement(mixed, 'button').textContent).toBe('');
        await expect(state.runtime.diagnosticsFor(mixed).map((diagnostic) => diagnostic.code)).toContain(
            'cem-element.instance_payload_mixed',
        );
    },
};

export const ScopeLimitsAndCascade: Story = {
    render: () => {
        const root = createElement('section') as HTMLElement & {
            __cssScoping?: ScopingStoryState;
        };
        root.setAttribute('data-scope-limits-story', '');
        root.setAttribute('aria-label', 'native scope limits and cascade');
        appendExpectedResult(root, EXPECTED_RESULTS.scopeLimitsAndCascade);

        const pageStyle = createElement('style');
        pageStyle.textContent = [
            'button { color: rgb(255, 0, 0); }',
            'css-scope-boundary [part~="control"] { color: rgb(128, 0, 128); }',
        ].join('\n');
        root.append(pageStyle);

        const runtime = new CemElementRuntime({ declarationTag: 'cem-element-css-limits' });
        const boundary = domDeclaration(
            'cem-element-css-limits',
            'css-scope-boundary',
            [
                '<style>',
                'button { color: rgb(0, 128, 0); }',
                '[slot="content"] { color: rgb(0, 0, 255); --scope-inherited: yes; }',
                '[slot="content"] em { background-color: rgb(255, 0, 0); }',
                ':host([data-outer]) .nested-leak { background-color: rgb(255, 0, 0); }',
                '.fallback-owned { border: 0.2rem solid rgb(0, 128, 0); }',
                '</style>',
                '<button type="button" data-proximity>proximity</button>',
                '<button type="button" part="control">public override</button>',
                '<div><slot name="content"></slot></div>',
                '<div><slot name="nested"></slot></div>',
                '<slot name="missing"><span class="fallback-owned" data-fallback>fallback</span></slot>',
            ].join(''),
        );
        const inner = domDeclaration(
            'cem-element-css-limits',
            'css-scope-inner',
            '<style>.nested-leak { background-color: rgb(0, 128, 0); }</style><span class="nested-leak" data-inner-content>inner</span>',
        );
        root.append(boundary, inner);
        runtime.registerDeclaration(boundary);
        runtime.registerDeclaration(inner);

        const instance = createElement('css-scope-boundary');
        instance.setAttribute('data-outer', '');
        instance.innerHTML = [
            '<section slot="content"><em data-projected-descendant>projected descendant</em></section>',
            '<css-scope-boundary slot="nested"><span slot="content" class="nested-leak" data-same-nested>same nested</span></css-scope-boundary>',
            '<css-scope-inner slot="nested"></css-scope-inner>',
        ].join('');
        root.append(instance);
        root.__cssScoping = {
            runtime,
            declarations: { boundary, inner },
            instances: [instance],
        };
        return root;
    },
    play: async ({ canvasElement }) => {
        const root = requiredElement(canvasElement, '[data-scope-limits-story]') as HTMLElement & {
            __cssScoping?: ScopingStoryState;
        };
        await expect(requiredElement(root, '[data-expected-result]').textContent).toBe(
            EXPECTED_RESULTS.scopeLimitsAndCascade,
        );
        const state = root.__cssScoping;
        await expect(state).toBeDefined();
        if (!state) return;
        await settle(state);
        await nextFrame();

        await expect(getComputedStyle(requiredElement(root, '[data-proximity]')).color).toBe('rgb(0, 128, 0)');
        await expect(getComputedStyle(requiredElement(root, '[part~="control"]')).color).toBe('rgb(128, 0, 128)');

        const projectedRoot = requiredElement(root, 'section[slot="content"]');
        const projectedDescendant = requiredElement(projectedRoot, '[data-projected-descendant]');
        await expect(getComputedStyle(projectedRoot).color).toBe('rgb(0, 0, 255)');
        await expect(getComputedStyle(projectedDescendant).color).toBe('rgb(0, 0, 255)');
        await expect(getComputedStyle(projectedDescendant).getPropertyValue('--scope-inherited').trim()).toBe('yes');
        await expect(getComputedStyle(projectedDescendant).backgroundColor).toBe('rgba(0, 0, 0, 0)');

        await expect(getComputedStyle(requiredElement(root, '[data-same-nested]')).backgroundColor).toBe(
            'rgba(0, 0, 0, 0)',
        );
        await expect(getComputedStyle(requiredElement(root, '[data-inner-content]')).backgroundColor).toBe(
            'rgb(0, 128, 0)',
        );
        const fallback = requiredElement(root, '[data-fallback]');
        await expect(fallback).not.toHaveAttribute('slot');
        await expect(getComputedStyle(fallback).borderTopColor).toBe('rgb(0, 128, 0)');
    },
};

export const StaticOnlyStylesFailClosed: Story = {
    render: () => {
        const root = createElement('section') as HTMLElement & {
            __cssScoping?: ScopingStoryState;
        };
        root.setAttribute('data-static-css-story', '');
        root.setAttribute('aria-label', 'static declaration stylesheet diagnostics');
        appendExpectedResult(root, EXPECTED_RESULTS.staticOnlyStylesFailClosed);
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-element-css-static',
        });
        const staticDeclaration = cemMlDeclaration(
            'cem-element-css-static',
            'css-scope-static',
            [
                '{module |',
                '  {body |',
                '    {style |```',
                '      @import url("./suppressed.css");',
                '      @font-face { font-family: Demo; src: url("./demo.woff2"); }',
                '      :host([data-ready]) { --static-style: yes; }',
                '      :global(.legacy), :root { --contained-global: yes; }',
                '      @keyframes pulse { from { opacity: 0; } to { opacity: 1; } }',
                '      span { animation: pulse 1s; }',
                '    ```}',
                '    {span | static}',
                '  }',
                '}',
            ].join('\n'),
        );
        const dynamicDeclaration = cemMlDeclaration(
            'cem-element-css-static',
            'css-scope-dynamic',
            [
                '{module |',
                '  {slice @name=scope | css-matrix-lib}',
                '  {body |',
                '    {style @scope="{$scope}" |``` :host { --dynamic-scope: no; } ```}',
                '    {cem:if @test=scope |',
                '      {style |``` :host { --dynamic-branch: no; } ```}',
                '    }',
                '    {span | dynamic styles rejected}',
                '  }',
                '}',
            ].join('\n'),
        );
        root.append(staticDeclaration, dynamicDeclaration);
        runtime.registerDeclaration(staticDeclaration);
        runtime.registerDeclaration(dynamicDeclaration);

        const staticInstance = createElement('css-scope-static');
        staticInstance.setAttribute('data-ready', '');
        staticInstance.setAttribute('data-css-case', 'static');
        const dynamicInstance = createElement('css-scope-dynamic');
        dynamicInstance.setAttribute('data-css-case', 'dynamic');
        root.append(staticInstance, dynamicInstance);
        root.__cssScoping = {
            runtime,
            declarations: {
                static: staticDeclaration,
                dynamic: dynamicDeclaration,
            },
            instances: [staticInstance, dynamicInstance],
        };
        return root;
    },
    play: async ({ canvasElement }) => {
        const root = requiredElement(canvasElement, '[data-static-css-story]') as HTMLElement & {
            __cssScoping?: ScopingStoryState;
        };
        await expect(requiredElement(root, '[data-expected-result]').textContent).toBe(
            EXPECTED_RESULTS.staticOnlyStylesFailClosed,
        );
        const state = root.__cssScoping;
        await expect(state).toBeDefined();
        if (!state) return;
        await settle(state);

        const staticInstance = requiredElement(root, '[data-css-case="static"]');
        const staticStyles = managedStyles(state.declarations.static);
        await expect(staticStyles).toHaveLength(1);
        await expect(staticInstance.querySelector('style')).toBeNull();
        await expect(cssValue(staticInstance, '--static-style')).toBe('yes');
        await expect(staticStyles[0]?.textContent).not.toContain('@import');
        await expect(staticStyles[0]?.textContent).not.toContain('@font-face');
        await expect(staticStyles[0]?.textContent).toContain(':where(:scope)[data-ready]');
        await expect(staticStyles[0]?.textContent).toContain(':where(:scope).legacy, :where(:scope)');
        await expect(staticStyles[0]?.textContent).toMatch(/@keyframes pulse-.+-s1/);
        await expect(diagnosticCodes(state, 'static')).toEqual(
            expect.arrayContaining([
                'cem.scoped_css.import_unsupported',
                'cem.scoped_css.global_construct_unsupported',
                'cem.scoped_css.global_alias',
            ]),
        );

        const dynamicInstance = requiredElement(root, '[data-css-case="dynamic"]');
        await expect(dynamicInstance.textContent).toContain('dynamic styles rejected');
        await expect(dynamicInstance.querySelector('style')).toBeNull();
        await expect(managedStyles(state.declarations.dynamic)).toHaveLength(0);
        await expect(
            diagnosticCodes(state, 'dynamic').filter(
                (code) => code === 'cem.ql.template.stylesheet_dynamic_unsupported',
            ),
        ).toHaveLength(2);
    },
};

export const FragmentAndAnonymousDeclarationsUseEffectiveTags: Story = {
    render: () => {
        const root = createElement('section') as HTMLElement & {
            __cssScoping?: ScopingStoryState;
        };
        root.setAttribute('data-effective-tag-story', '');
        root.setAttribute('aria-label', 'fragment and anonymous stylesheet boundaries');
        appendExpectedResult(root, EXPECTED_RESULTS.fragmentAndAnonymousDeclarationsUseEffectiveTags);
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-element-css-effective-tag',
            loadSrcDocument: async () =>
                [
                    '<!doctype html><html><body>',
                    '<template id="fragment-style" type="text/cem-ml">',
                    '{module | {body |',
                    '{style |``` :host { --fragment-style: yes; } ```}',
                    '{strong | fragment}',
                    '}}',
                    '</template>',
                    '</body></html>',
                ].join(''),
        });

        const fragment = createElement('cem-element-css-effective-tag');
        fragment.setAttribute('tag', 'css-scope-fragment');
        fragment.setAttribute('src', './css-scope-library.xhtml#fragment-style');
        fragment.setAttribute('uid-seed', 'storybook/css-scope/fragment');
        const anonymous = domDeclaration(
            'cem-element-css-effective-tag',
            null,
            '<style>:host { --anonymous-style: yes; }</style><strong>anonymous</strong>',
        );
        anonymous.setAttribute('uid-seed', 'storybook/css-scope/anonymous');
        root.append(fragment, anonymous);
        runtime.registerDeclaration(fragment);
        runtime.registerDeclaration(anonymous);

        const fragmentInstance = createElement('css-scope-fragment');
        fragmentInstance.setAttribute('data-css-case', 'fragment');
        root.append(fragmentInstance);
        root.__cssScoping = {
            runtime,
            declarations: { fragment, anonymous },
            instances: [fragmentInstance],
        };
        return root;
    },
    play: async ({ canvasElement }) => {
        const root = requiredElement(canvasElement, '[data-effective-tag-story]') as HTMLElement & {
            __cssScoping?: ScopingStoryState;
        };
        await expect(requiredElement(root, '[data-expected-result]').textContent).toBe(
            EXPECTED_RESULTS.fragmentAndAnonymousDeclarationsUseEffectiveTags,
        );
        const state = root.__cssScoping;
        await expect(state).toBeDefined();
        if (!state) return;
        await Promise.all(
            Object.values(state.declarations).map((declaration) => state.runtime.whenDeclarationSettled(declaration)),
        );
        await nextFrame();

        const fragmentInstance = requiredElement(root, '[data-css-case="fragment"]');
        await state.runtime.whenRenderSettled(fragmentInstance);
        const fragmentStyles = managedStyles(state.declarations.fragment);
        await expect(fragmentStyles).toHaveLength(1);
        await expect(fragmentStyles[0]?.textContent).toContain('@scope (\n    css-scope-fragment');
        await expect(cssValue(fragmentInstance, '--fragment-style')).toBe('yes');

        const anonymousTag = state.declarations.anonymous.getAttribute('tag') ?? '';
        await expect(anonymousTag).toMatch(/^cem-[0-9a-f]{8}-[0-9a-f]{4}-8[0-9a-f]{3}-8[0-9a-f]{3}-[0-9a-f]{12}$/);
        const anonymousInstance = requiredElement(root, `${anonymousTag}[data-cem-anonymous-instance]`);
        await state.runtime.whenRenderSettled(anonymousInstance);
        const anonymousStyles = managedStyles(state.declarations.anonymous);
        await expect(anonymousStyles).toHaveLength(1);
        await expect(anonymousStyles[0]?.textContent).toContain(`@scope (\n    ${anonymousTag}`);
        await expect(cssValue(anonymousInstance, '--anonymous-style')).toBe('yes');
    },
};

function domDeclaration(
    declarationTag: string,
    producedTag: string | null,
    templateHtml: string,
    scope?: string,
): HTMLElement {
    const declaration = createElement(declarationTag);
    if (producedTag !== null) {
        declaration.setAttribute('tag', producedTag);
    }
    if (scope !== undefined) {
        declaration.setAttribute('scope', scope);
    }
    const template = createElement('template');
    template.innerHTML = templateHtml;
    declaration.append(template);
    return declaration;
}

function cemMlDeclaration(declarationTag: string, producedTag: string, source: string): HTMLElement {
    const declaration = createElement(declarationTag);
    declaration.setAttribute('tag', producedTag);
    const template = createElement('template');
    template.setAttribute('type', 'text/cem-ml');
    template.textContent = source;
    declaration.append(template);
    return declaration;
}

async function settle(state: ScopingStoryState): Promise<void> {
    await Promise.all(
        Object.values(state.declarations).map((declaration) => state.runtime.whenDeclarationSettled(declaration)),
    );
    await nextFrame();
    await Promise.all(state.instances.map((instance) => state.runtime.whenRenderSettled(instance)));
}

function managedStyles(declaration: HTMLElement): HTMLStyleElement[] {
    return Array.from(declaration.querySelectorAll<HTMLStyleElement>(':scope > style[data-cem-declaration-style]'));
}

function elements(root: ParentNode, selector: string): HTMLElement[] {
    return Array.from(root.querySelectorAll<HTMLElement>(selector));
}

function cssValue(element: Element | undefined, name: string): string {
    return element ? getComputedStyle(element).getPropertyValue(name).trim() : '';
}

function diagnosticCodes(state: ScopingStoryState, key: string): string[] {
    return state.runtime.diagnosticsFor(state.declarations[key] as HTMLElement).map((diagnostic) => diagnostic.code);
}

function appendExpectedResult(root: HTMLElement, result: string): void {
    const description = createElement('p');
    description.setAttribute('data-expected-result', '');
    description.textContent = result;
    root.append(description);
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) {
        throw new Error(`expected ${selector}`);
    }
    return element;
}

function nextFrame(): Promise<void> {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}
