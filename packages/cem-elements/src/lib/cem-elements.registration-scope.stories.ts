import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

const meta: Meta = {
    title: 'CEM Elements/Registration Scope Contract',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const ScopedLogicalLookupWithDocumentGlobalRegistration: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'logical declaration scope registration contract');
        return root;
    },
    play: async ({ canvasElement }) => {
        const root = requiredElement(
            canvasElement,
            '[aria-label="logical declaration scope registration contract"]'
        );
        const producedTag = 'story-registration-scope-card';
        const parentScope = createCemDeclarationScope({ document });
        const childScope = createCemDeclarationScope({ document, parent: parentScope });
        const parentRuntime = new CemElementRuntime({
            declarationTag: 'cem-element-story-registration-parent',
            declarationScope: parentScope,
        });
        const childRuntime = new CemElementRuntime({
            declarationTag: 'cem-element-story-registration-child',
            declarationScope: childScope,
        });
        const template = '<span data-registration-owner="parent">Parent definition</span>';

        const parentDeclaration = declaration('cem-element-story-registration-parent', producedTag, template);
        root.appendChild(parentDeclaration);
        assert(parentRuntime.registerDeclaration(parentDeclaration), 'the parent declaration is accepted');
        await parentRuntime.whenDeclarationSettled(parentDeclaration);
        assertDiagnosticCodes(parentRuntime, parentDeclaration, []);

        const parentConstructor = window.customElements.get(producedTag);
        assert(parentConstructor, 'the parent declaration defines the document-global constructor');

        const inheritedDeclaration = declaration('cem-element-story-registration-child', producedTag, template);
        root.appendChild(inheritedDeclaration);
        assert(childRuntime.registerDeclaration(inheritedDeclaration), 'the identical child declaration is accepted');
        await childRuntime.whenDeclarationSettled(inheritedDeclaration);
        assertDiagnosticCodes(childRuntime, inheritedDeclaration, []);
        assertEqual(
            window.customElements.get(producedTag),
            parentConstructor,
            'an identical child declaration reuses the inherited document-global constructor'
        );

        const sameScopeDuplicate = declaration('cem-element-story-registration-child', producedTag, template);
        root.appendChild(sameScopeDuplicate);
        childRuntime.registerDeclaration(sameScopeDuplicate);
        await childRuntime.whenDeclarationSettled(sameScopeDuplicate);
        assertDiagnosticCodes(childRuntime, sameScopeDuplicate, ['cem-element.registry_same_scope_duplicate']);
        assertEqual(
            window.customElements.get(producedTag),
            parentConstructor,
            'a same-scope duplicate does not mutate the browser registry'
        );

        const incompatibleChildScope = createCemDeclarationScope({ document, parent: parentScope });
        const incompatibleChildRuntime = new CemElementRuntime({
            declarationTag: 'cem-element-story-registration-incompatible-child',
            declarationScope: incompatibleChildScope,
        });
        const incompatibleInherited = declaration(
            'cem-element-story-registration-incompatible-child',
            producedTag,
            '<span data-registration-owner="child">Incompatible child</span>'
        );
        root.appendChild(incompatibleInherited);
        incompatibleChildRuntime.registerDeclaration(incompatibleInherited);
        await incompatibleChildRuntime.whenDeclarationSettled(incompatibleInherited);
        assertDiagnosticCodes(incompatibleChildRuntime, incompatibleInherited, [
            'cem-element.registry_inherited_collision',
        ]);

        const independentScope = createCemDeclarationScope({ document });
        const independentRuntime = new CemElementRuntime({
            declarationTag: 'cem-element-story-registration-independent',
            declarationScope: independentScope,
        });
        const incompatibleBrowser = declaration(
            'cem-element-story-registration-independent',
            producedTag,
            '<span data-registration-owner="independent">Incompatible browser claimant</span>'
        );
        root.appendChild(incompatibleBrowser);
        independentRuntime.registerDeclaration(incompatibleBrowser);
        await independentRuntime.whenDeclarationSettled(incompatibleBrowser);
        assertDiagnosticCodes(independentRuntime, incompatibleBrowser, ['cem-element.browser_tag_collision']);

        const foreignTag = 'story-registration-scope-foreign';
        if (!window.customElements.get(foreignTag)) {
            window.customElements.define(foreignTag, class extends HTMLElement {});
        }
        const foreignRuntime = new CemElementRuntime({
            declarationTag: 'cem-element-story-registration-foreign',
            declarationScope: createCemDeclarationScope({ document }),
        });
        const foreignCollision = declaration(
            'cem-element-story-registration-foreign',
            foreignTag,
            '<span>Foreign collision</span>'
        );
        root.appendChild(foreignCollision);
        foreignRuntime.registerDeclaration(foreignCollision);
        await foreignRuntime.whenDeclarationSettled(foreignCollision);
        assertDiagnosticCodes(foreignRuntime, foreignCollision, ['cem-element.browser_tag_collision']);

        const missingBehaviorIdentityTag = 'story-registration-scope-behavior-missing';
        const missingBehaviorIdentityRuntime = new CemElementRuntime({
            declarationTag: 'cem-element-story-registration-behavior-missing',
            declarationScope: createCemDeclarationScope({ document }),
        });
        const missingBehaviorIdentity = declaration(
            'cem-element-story-registration-behavior-missing',
            missingBehaviorIdentityTag,
            '<span>Behavior identity is required</span>'
        );
        root.appendChild(missingBehaviorIdentity);
        missingBehaviorIdentityRuntime.registerDeclaration(missingBehaviorIdentity, { behavior: {} });
        await missingBehaviorIdentityRuntime.whenDeclarationSettled(missingBehaviorIdentity);
        assertDiagnosticCodes(missingBehaviorIdentityRuntime, missingBehaviorIdentity, [
            'cem-element.behavior_identity_required',
        ]);
        assertEqual(
            window.customElements.get(missingBehaviorIdentityTag),
            undefined,
            'behavior without a stable host identity fails before browser mutation'
        );

        const instance = document.createElement(producedTag);
        root.appendChild(instance);
        await parentRuntime.whenRenderSettled(instance);
        assertEqual(
            requiredElement(instance, '[data-registration-owner="parent"]').textContent,
            'Parent definition',
            'the retained parent constructor renders the authoritative inherited declaration'
        );
        assertEqual(
            window.customElements.get(producedTag),
            parentConstructor,
            'all rejected collisions leave the original browser constructor unchanged'
        );
    },
};

function declaration(declarationTag: string, producedTag: string, html: string): HTMLElement {
    const element = document.createElement(declarationTag);
    element.setAttribute('tag', producedTag);
    const template = document.createElement('template');
    template.innerHTML = html;
    element.appendChild(template);
    return element;
}

function assertDiagnosticCodes(
    runtime: CemElementRuntime,
    declarationElement: HTMLElement,
    expected: string[]
): void {
    assertEqual(
        runtime.diagnosticsFor(declarationElement).map((diagnostic) => diagnostic.code).join(','),
        expected.join(','),
        `declaration diagnostics are ${expected.length > 0 ? expected.join(', ') : 'empty'}`
    );
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector(selector);
    if (!(element instanceof HTMLElement)) {
        throw new Error(`Expected \`${selector}\` in registration-scope story`);
    }
    return element;
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) {
        throw new Error(message);
    }
}

function assertEqual<T>(actual: T, expected: T, message: string): void {
    if (actual !== expected) {
        throw new Error(`${message}: expected ${String(expected)}, received ${String(actual)}`);
    }
}
