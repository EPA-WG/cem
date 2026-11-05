/** Development-only control metadata. Never retain source, values or DOM nodes. */
export function createJourneyCapture(story: string, inspect: () => object, clock = () => performance.now()) {
    let readState: (() => object) | undefined = inspect;
    const start = clock();
    const records: { phase: string; elapsedMs: number }[] = [];
    let dropped = 0;
    let result: { story: string; elapsedMs: number; records: typeof records; dropped: number; state: object } | undefined;
    const elapsed = () => Math.round((clock() - start) * 10) / 10;
    return {
        mark(phase: string) {
            if (result) return;
            if (records.length === 32) { records.shift(); dropped++; }
            records.push({ phase, elapsedMs: elapsed() });
            if (import.meta.env.STORYBOOK_CEM_STORY_TIMING === '1') {
                console.warn('[cem-journey-phase]', JSON.stringify({ story, ...records.at(-1) }));
            }
        },
        freeze() {
            if (!result) {
                let state: object;
                try { state = structuredClone(readState?.() ?? { unavailable: true }); } catch { state = { unavailable: true }; }
                readState = undefined;
                result = { story, elapsedMs: elapsed(), records: structuredClone(records), dropped, state };
            }
            return structuredClone(result);
        },
    };
}

/** Runner hooks retain global-deadline failures without introducing another timer. */
export async function captureJourneyFailure(story: string, inspect: () => object) {
    const capture = createJourneyCapture(story, inspect);
    if (import.meta.env.MODE === 'test') {
        const { onTestFailed, onTestFinished } = await import('vitest');
        onTestFinished(() => { capture.freeze(); });
        onTestFailed(() => { throw new Error(`[cem-journey-failure] ${JSON.stringify(capture.freeze())}`); });
    }
    return capture;
}
