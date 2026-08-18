import { describe, expect, it } from 'vitest';

import {
    bindCemDeclarationScopeRegistration,
    createCemDeclarationScope,
    getDefaultCemDeclarationScope,
    lookupCemDeclarationScopeRegistration,
} from './declaration-scope.js';

describe('cem-element logical declaration scope contract', () => {
    it('uses one opaque default root per Document without conflating policy metadata', () => {
        const documentA = fakeDocument();
        const documentB = fakeDocument();

        const rootA = getDefaultCemDeclarationScope(documentA);

        expect(getDefaultCemDeclarationScope(documentA)).toBe(rootA);
        expect(getDefaultCemDeclarationScope(documentB)).not.toBe(rootA);
        expect(rootA.document).toBe(documentA);
        expect(rootA.parent).toBeNull();
        expect(rootA.disposed).toBe(false);
        expect('scopePolicyStamp' in rootA).toBe(false);
    });

    it('creates only explicit, immutable, same-document parent relationships', () => {
        const document = fakeDocument();
        const parent = createCemDeclarationScope({ document });
        const child = createCemDeclarationScope({ document, parent });

        expect(child.document).toBe(document);
        expect(child.parent).toBe(parent);
        expect(() => {
            (child as { parent: unknown }).parent = null;
        }).toThrow();

        expect(() =>
            createCemDeclarationScope({
                document: fakeDocument(),
                parent,
            })
        ).toThrow(expect.objectContaining({ code: 'cem-element.scope_parent_document_mismatch' }));
    });

    it('reports same-scope and nearest inherited registrations without inferring DOM ancestry', () => {
        const document = fakeDocument();
        const root = createCemDeclarationScope({ document });
        const parent = createCemDeclarationScope({ document, parent: root });
        const child = createCemDeclarationScope({ document, parent });
        const rootDeclaration = { owner: 'root' };
        const parentDeclaration = { owner: 'parent' };

        bindCemDeclarationScopeRegistration(root, 'cem-button', {
            registrationIdentity: 'blake3:button-v1',
            declaration: rootDeclaration,
        });
        bindCemDeclarationScopeRegistration(parent, 'cem-button', {
            registrationIdentity: 'blake3:button-v1',
            declaration: parentDeclaration,
        });

        const inherited = lookupCemDeclarationScopeRegistration(child, 'cem-button');
        expect(inherited.sameScope).toBeUndefined();
        expect(inherited.inherited).toEqual({
            registrationIdentity: 'blake3:button-v1',
            declaration: parentDeclaration,
        });

        const inheritedRegistration = inherited.inherited;
        expect(inheritedRegistration).toBeDefined();
        if (!inheritedRegistration) {
            throw new Error('expected the child scope to resolve its nearest parent registration');
        }
        bindCemDeclarationScopeRegistration(child, 'cem-button', inheritedRegistration);
        expect(lookupCemDeclarationScopeRegistration(child, 'cem-button')).toEqual({
            sameScope: {
                registrationIdentity: 'blake3:button-v1',
                declaration: parentDeclaration,
            },
            inherited: {
                registrationIdentity: 'blake3:button-v1',
                declaration: parentDeclaration,
            },
        });

        expect(() =>
            bindCemDeclarationScopeRegistration(child, 'cem-button', {
                registrationIdentity: 'blake3:button-v2',
                declaration: { owner: 'duplicate' },
            })
        ).toThrow(expect.objectContaining({ code: 'cem-element.scope_same_scope_binding_exists' }));
    });

    it('disposes logical ownership idempotently and replaces a disposed default root', () => {
        const document = fakeDocument();
        const root = getDefaultCemDeclarationScope(document);
        const child = createCemDeclarationScope({ document, parent: root });

        bindCemDeclarationScopeRegistration(root, 'cem-button', {
            registrationIdentity: 'blake3:button-v1',
            declaration: { owner: 'root' },
        });
        root.dispose();
        root.dispose();

        expect(root.disposed).toBe(true);
        expect(() => lookupCemDeclarationScopeRegistration(root, 'cem-button')).toThrow(
            expect.objectContaining({ code: 'cem-element.scope_disposed' })
        );
        expect(() => lookupCemDeclarationScopeRegistration(child, 'cem-button')).toThrow(
            expect.objectContaining({ code: 'cem-element.scope_ancestor_disposed' })
        );
        expect(getDefaultCemDeclarationScope(document)).not.toBe(root);
        expect(() => createCemDeclarationScope({ document, parent: root })).toThrow(
            expect.objectContaining({ code: 'cem-element.scope_parent_disposed' })
        );
    });
});

function fakeDocument(): Document {
    return {} as Document;
}
