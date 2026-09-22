import { whenCemRendered } from './preview.js';

/** Opt-in diagnostic control records, emitted immediately so timeouts retain progress. */
export function storyTiming(story: string): (phase: string, instance?: HTMLElement) => void {
    const enabled = import.meta.env.STORYBOOK_CEM_STORY_TIMING === '1';
    const start = performance.now();
    const emit = (phase: string, instance?: HTMLElement): void => {
        const now = performance.now();
        // Vite forwards warnings even when the passing-test console output is hidden.
        console.warn('[cem-story-timing]', JSON.stringify({
            story, phase, at: new Date(performance.timeOrigin + now).toISOString(),
            elapsedMs: Math.round((now - start) * 10) / 10,
            ...(instance ? { tag: instance.localName, connected: instance.isConnected } : {}),
        }));
    };
    return (phase, instance) => {
        if (!enabled) return;
        emit(phase, instance);
        if (instance) {
            // Observe the existing lifecycle without delaying the next assertion/action.
            // This identifies renderer settlement separately from the DOM assertion.
            void whenCemRendered(instance).then(
                () => emit(`${phase}:render-settled`, instance),
                () => emit(`${phase}:render-rejected`, instance),
            );
        }
    };
}
