import { afterEach, describe, expect, it, vi } from 'vitest';
import { CemCheckpointPublicationCoordinator } from './checkpoint-publication.js';
import { CemNativeRecoveryScope } from './internal/runtime-support/native-generation.js';
import { CemRecoveryStatusObserver } from './recovery-status.js';

const metadata = { instanceId: 'private-owner', declarationIdentity: 'declaration', dataRevision: 'data' };
const options = () => ({ runtimeId: 'private-runtime', generation: 1, policyStamp: 'private-policy', seal: vi.fn(() => { throw Error('must not seal'); }) });
const endpoint = () => ({ host: { localName: 'private-tag' } as HTMLElement, resolve: vi.fn(),
    resume: { witness: vi.fn(), prepare: vi.fn() } });

afterEach(() => vi.useRealTimers());
describe('advisory recovery status', () => {
    it('reports no configuration without inventing checkpoint availability or authority', () => {
        const observer = new CemRecoveryStatusObserver();
        expect(observer.snapshot()).toEqual({ version: 'cem-recovery-status/1', state: 'unconfigured', reason: 'not-configured',
            runtimeCount: 0, ownerCount: 0, acknowledgedCount: 0, admissionCount: 0, unknownOperationCount: 0, unsupportedOwnerCount: 0,
            automaticRecovery: false, credentialPolicy: 'block-entered' });
        expect(Object.isFrozen(observer.snapshot())).toBe(true); observer.dispose();
        expect(observer.snapshot().state).toBe('unavailable');
    });
    it('aggregates nested publications but never reads owners, credentials, errors, bytes or readiness', () => {
        const config = options(), publication = new CemCheckpointPublicationCoordinator(config), child = publication.fork(), nested = child.fork();
        const privateProbe = vi.fn(() => { throw Error('password=TOP_SECRET'); });
        const a = publication.register('private-owner', privateProbe, privateProbe, 0, privateProbe, endpoint());
        const b = nested.register('other-private-owner', privateProbe, privateProbe);
        const scope = new CemNativeRecoveryScope(), prepare = vi.fn();
        const enrollment = scope.enroll({ id: 'private-runtime', kind: 'rendering', prepare });
        const observer = new CemRecoveryStatusObserver({ publication, recoveryScope: scope });
        for (const method of ['current', 'lastCommitted', 'capture', 'prepareResume', 'recoveryEnrollment'] as const) vi.spyOn(publication, method).mockImplementation(privateProbe);
        expect(observer.snapshot()).toMatchObject({ state: 'blocked', reason: 'owner-adapters', ownerCount: 2, acknowledgedCount: 0, unsupportedOwnerCount: 1, runtimeCount: 1 });
        a.published(metadata); b.published({ ...metadata, instanceId: 'other-private-owner' });
        expect(observer.snapshot().acknowledgedCount).toBe(2);
        b.remove();
        expect(observer.snapshot()).toMatchObject({ state: 'observing', reason: 'observed', ownerCount: 1, acknowledgedCount: 1 });
        const text = JSON.stringify(observer.snapshot());
        for (const value of ['private-owner', 'private-tag', 'private-runtime', 'private-policy', 'TOP_SECRET', 'ready', 'checkpointBytes']) expect(text).not.toContain(value);
        expect(privateProbe).not.toHaveBeenCalled(); expect(prepare).not.toHaveBeenCalled(); expect(config.seal).not.toHaveBeenCalled();
        enrollment.remove(); expect(observer.snapshot().runtimeCount).toBe(0);
        scope.dispose(); expect(observer.snapshot()).toMatchObject({ state: 'unavailable', reason: 'scope-unavailable' });
        observer.dispose(); publication.dispose();
    });
    it('observes incomplete acknowledgments, unknown outcomes, admissions, capture and disposal', async () => {
        let complete!: () => void;
        const publication = new CemCheckpointPublicationCoordinator({ ...options(), allowOwner: () => true,
            seal: text => new Promise(resolve => { complete = () => resolve(text); }) });
        const owner = publication.register('private-owner', () => '<template/>', () => true, 0, () => [], endpoint());
        const observer = new CemRecoveryStatusObserver({ publication });
        expect(observer.snapshot().reason).toBe('owner-publications');
        owner.published(metadata); const operation = publication.operation('private-owner');
        expect(observer.snapshot()).toMatchObject({ state: 'blocked', reason: 'unknown-operations', unknownOperationCount: 1 });
        operation.acknowledge(); const release = publication.admit();
        expect(observer.snapshot()).toMatchObject({ state: 'busy', reason: 'admissions', admissionCount: 1 });
        release(); const captured = publication.capture();
        expect(observer.snapshot()).toMatchObject({ state: 'busy', reason: 'preparing' });
        complete(); await captured;
        expect(observer.snapshot()).toMatchObject({ state: 'observing', reason: 'observed' });
        owner.input(); expect(observer.snapshot()).toMatchObject({ state: 'blocked', reason: 'owner-publications', acknowledgedCount: 0 });
        publication.dispose(); expect(observer.snapshot()).toMatchObject({ state: 'unavailable', reason: 'publication-unavailable' }); observer.dispose();
    });
    it('bounds traversal and fails closed without revealing arbitrary inspection failures', () => {
        const publication = new CemCheckpointPublicationCoordinator(options()), observer = new CemRecoveryStatusObserver({ publication });
        for (let i = 0; i < 256; i++) publication.fork();
        expect(observer.snapshot()).toMatchObject({ state: 'blocked', reason: 'coverage-limit' });
        vi.spyOn(publication, 'inspectRecovery').mockImplementation(() => { throw Error('secret-value'); });
        expect(observer.snapshot()).toMatchObject({ state: 'unavailable', reason: 'inspection-failed' });
        expect(JSON.stringify(observer.snapshot())).not.toContain('secret-value'); observer.dispose(); publication.dispose();
    });
    it('quiesces status inputs during original checkpoint handoff and publishes the latest state after release', () => {
        vi.useFakeTimers();
        const publication = new CemCheckpointPublicationCoordinator(options()), observer = new CemRecoveryStatusObserver({ publication });
        const notify = vi.fn(); observer.subscribe(notify);
        const release = observer.suspend(), peerRelease = observer.suspend();
        publication.register('private-owner', () => '<template/>');
        expect(vi.getTimerCount()).toBe(0); observer.refresh(); expect(notify).toHaveBeenCalledTimes(1);
        release(); release(); expect(vi.getTimerCount()).toBe(0);
        peerRelease(); expect(notify).toHaveBeenCalledTimes(2); expect(vi.getTimerCount()).toBe(1);
        observer.suspend(); observer.dispose(); expect(vi.getTimerCount()).toBe(0); publication.dispose();
    });
    it('does not restart a timer when a subscriber suspends publication during release', () => {
        vi.useFakeTimers();
        const publication = new CemCheckpointPublicationCoordinator(options()), observer = new CemRecoveryStatusObserver({ publication });
        let suspendOnChange = false;
        observer.subscribe(() => { if (suspendOnChange) observer.suspend(); });
        const release = observer.suspend(); publication.register('private-owner', () => '<template/>');
        suspendOnChange = true; release(); expect(vi.getTimerCount()).toBe(0);
        observer.dispose(); publication.dispose();
    });
    it('shares one bounded timer, coalesces unchanged snapshots, isolates callbacks and stops on unsubscribe/disposal', () => {
        vi.useFakeTimers();
        const publication = new CemCheckpointPublicationCoordinator(options()), observer = new CemRecoveryStatusObserver({ publication });
        const first = vi.fn(), second = vi.fn(), broken = vi.fn(() => { throw Error('secret'); });
        const stopFirst = observer.subscribe(first), stopSecond = observer.subscribe(second), stopBroken = observer.subscribe(broken);
        expect(vi.getTimerCount()).toBe(1); vi.advanceTimersByTime(3000); expect(first).toHaveBeenCalledTimes(1);
        const owner = publication.register('private-owner', () => '<template/>');
        vi.advanceTimersByTime(1000); expect(first).toHaveBeenCalledTimes(2); expect(second).toHaveBeenCalledTimes(2);
        stopFirst(); stopBroken(); expect(vi.getTimerCount()).toBe(1);
        owner.remove(); observer.refresh(); expect(first).toHaveBeenCalledTimes(2); expect(second).toHaveBeenCalledTimes(3);
        stopSecond(); expect(vi.getTimerCount()).toBe(0);
        const final = vi.fn(); observer.subscribe(final); observer.dispose(); expect(vi.getTimerCount()).toBe(0);
        expect(final).toHaveBeenLastCalledWith(expect.objectContaining({ state: 'unavailable' }));
        observer.subscribe(final); expect(vi.getTimerCount()).toBe(0); publication.dispose();
    });
});
