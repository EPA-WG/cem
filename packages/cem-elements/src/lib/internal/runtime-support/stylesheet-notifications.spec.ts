import { describe, expect, it } from 'vitest';
import { deferStylesheetNotifications, notifyStylesheetLifecycle } from './stylesheet-notifications.js';

describe('stylesheet publication notifications', () => {
    it('defers nested cleanup until the outer publication finishes', () => {
        const events: string[] = [];
        const result = deferStylesheetNotifications(() => {
            notifyStylesheetLifecycle([() => events.push('release')]);
            deferStylesheetNotifications(() => notifyStylesheetLifecycle([() => events.push('abort')]));
            expect(events).toEqual([]);
            events.push('dom');
            return 42;
        });
        expect(result).toBe(42);
        expect(events).toEqual(['dom', 'release', 'abort']);
    });

    it('drains cleanup after failed publication and restores immediate delivery', () => {
        const failure = new Error('DOM publication failed');
        const events: string[] = [];
        expect(() => deferStylesheetNotifications(() => {
            notifyStylesheetLifecycle([() => events.push('release')]);
            throw failure;
        })).toThrow(failure);
        notifyStylesheetLifecycle([() => events.push('later')]);
        expect(events).toEqual(['release', 'later']);
    });

    it('preserves publication and cleanup failures while delivering every notification', () => {
        const publication = new Error('publication');
        const cleanup = new Error('cleanup');
        const events: string[] = [];
        let thrown: unknown;
        try {
            deferStylesheetNotifications(() => {
                notifyStylesheetLifecycle([() => { throw cleanup; }, () => events.push('abort')]);
                throw publication;
            });
        } catch (error) { thrown = error; }
        expect(thrown).toBeInstanceOf(AggregateError);
        expect((thrown as AggregateError).errors[0]).toBe(publication);
        expect(((thrown as AggregateError).errors[1] as AggregateError).errors).toEqual([cleanup]);
        expect(events).toEqual(['abort']);
    });

    it('allows a cleanup callback to start a fresh publication boundary', () => {
        const events: string[] = [];
        deferStylesheetNotifications(() => notifyStylesheetLifecycle([() => {
            deferStylesheetNotifications(() => {
                notifyStylesheetLifecycle([() => events.push('nested release')]);
                events.push('nested dom');
            });
        }, () => events.push('outer abort')]));
        expect(events).toEqual(['nested dom', 'nested release', 'outer abort']);
    });
});
