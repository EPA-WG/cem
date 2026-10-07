import { expect, it } from 'vitest';
import { CemElementRuntime, exportDataIslandSnapshotForEdge } from './cem-elements.js';
import { DEFAULT_ARIA_REFERENCE_PROFILE, EXPERIMENTAL_ARIA_REFERENCE_PROFILE, resolveAriaReferenceProfile } from './aria-reference-profile.js';
import { edgeSsrSnapshotFixture } from './processing-boundary.fixtures.js';
import { projectTemplate, renderPlanIdentity, renderRevisionKey } from './projection.js';

it('requires an exact pinned profile and keeps the Recommendation default', () => {
    expect(resolveAriaReferenceProfile()).toBe(DEFAULT_ARIA_REFERENCE_PROFILE);
    expect(resolveAriaReferenceProfile(EXPERIMENTAL_ARIA_REFERENCE_PROFILE)).toBe(EXPERIMENTAL_ARIA_REFERENCE_PROFILE);
    for (const value of [null, '', 'wai-aria-1.3', 'unknown']) expect(() => resolveAriaReferenceProfile(value)).toThrow('Unknown ARIA');
    const standard = new CemElementRuntime(), draft = new CemElementRuntime({ ariaReferenceProfile: EXPERIMENTAL_ARIA_REFERENCE_PROFILE });
    expect(standard.ariaReferenceProfile).toBe(DEFAULT_ARIA_REFERENCE_PROFILE);
    expect(draft.scopePolicyStamp).not.toBe(standard.scopePolicyStamp);
});

it('carries profile identity in exported snapshots, plans and revision keys', () => {
    const snapshot = { ...edgeSsrSnapshotFixture(), ariaReferenceProfile: EXPERIMENTAL_ARIA_REFERENCE_PROFILE };
    expect(exportDataIslandSnapshotForEdge(snapshot).ariaReferenceProfile).toBe(EXPERIMENTAL_ARIA_REFERENCE_PROFILE);
    const plan = projectTemplate([], { snapshot, values: {} });
    expect(renderPlanIdentity(plan).ariaReferenceProfile).toBe(EXPERIMENTAL_ARIA_REFERENCE_PROFILE);
    expect(renderRevisionKey(plan)).not.toBe(renderRevisionKey({ ...plan, ariaReferenceProfile: DEFAULT_ARIA_REFERENCE_PROFILE }));
    expect(renderRevisionKey({ ...plan, ariaReferenceProfile: undefined })).toBe(renderRevisionKey({ ...plan, ariaReferenceProfile: DEFAULT_ARIA_REFERENCE_PROFILE }));
});
