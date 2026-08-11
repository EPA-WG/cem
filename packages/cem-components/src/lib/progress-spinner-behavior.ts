import type { CemProducedElementBehavior } from '@epa-wg/cem-elements';

const DEFAULT_MAX = 100;

export const CEM_PROGRESS_SPINNER_BEHAVIOR: CemProducedElementBehavior = {
    beforeRender(instance, context) {
        const indeterminate = !instance.hasAttribute('value');
        const max = normalizeMax(instance.getAttribute('max'));
        const value = indeterminate ? null : normalizeValue(instance.getAttribute('value'), max);
        const completed = percentage(value ?? 0, max);

        context.setSlices(
            {
                dashArray: indeterminate ? '25 75' : `${completed} ${100 - completed}`,
                indeterminate,
                max,
                mode: indeterminate ? 'indeterminate' : 'determinate',
                value,
            },
            { render: false },
        );
    },
};

function normalizeMax(authored: string | null): number {
    if (authored === null || authored.trim() === '') return DEFAULT_MAX;
    const parsed = Number(authored);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : DEFAULT_MAX;
}

function normalizeValue(authored: string | null, max: number): number {
    const parsed = Number(authored ?? '');
    if (!Number.isFinite(parsed)) return 0;
    return Math.min(Math.max(parsed, 0), max);
}

function percentage(value: number, max: number): number {
    return (value / max) * 100;
}
