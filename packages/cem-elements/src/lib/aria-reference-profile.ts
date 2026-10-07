/** Exact consumer contracts. Undefined legacy metadata means the Recommendation. */
export const DEFAULT_ARIA_REFERENCE_PROFILE = 'wai-aria-1.2-rec-20230606' as const;
export const EXPERIMENTAL_ARIA_REFERENCE_PROFILE = 'wai-aria-1.3-wd-20260604' as const;
export type CemAriaReferenceProfile = typeof DEFAULT_ARIA_REFERENCE_PROFILE | typeof EXPERIMENTAL_ARIA_REFERENCE_PROFILE;

export function resolveAriaReferenceProfile(value?: unknown): CemAriaReferenceProfile {
    if (value === undefined) return DEFAULT_ARIA_REFERENCE_PROFILE;
    if (value === DEFAULT_ARIA_REFERENCE_PROFILE || value === EXPERIMENTAL_ARIA_REFERENCE_PROFILE) return value;
    throw new TypeError(`Unknown ARIA reference export profile: ${String(value)}`);
}

/** Keep the old default keys stable while distinguishing an explicit draft. */
export function ariaReferenceProfileKey(value?: unknown): string[] {
    const profile = resolveAriaReferenceProfile(value);
    return profile === DEFAULT_ARIA_REFERENCE_PROFILE ? [] : [profile];
}

/** Only remove the known profile component; every other scope policy stays significant. */
export function withoutAriaReferenceProfileStamp(stamp: string): string {
    return stamp.replace(`:aria-reference:${EXPERIMENTAL_ARIA_REFERENCE_PROFILE}`, '');
}
