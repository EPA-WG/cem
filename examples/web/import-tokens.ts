import {
    cemTokenMetaByName,
    cemTokens,
    type CemTokenMeta,
    type CemTokenName,
} from "@epa-wg/cem-theme/tokens/cem.tokens.ts";

export function tokenMeta(name: CemTokenName): CemTokenMeta {
    return cemTokenMetaByName[name];
}

export function cssVariable(name: CemTokenName, fallback?: string): string {
    return fallback ? `var(${name}, ${fallback})` : `var(${name})`;
}

export function tokensBySpec(spec: string): CemTokenMeta[] {
    return cemTokens.filter((token) => token.spec === spec);
}

export const primaryActionBackground = cssVariable("--cem-action-primary-default-background");
export const comfortPalette = tokenMeta("--cem-palette-comfort");

// Uncommenting the next line should fail type-checking because the token name is not in CemTokenName.
// export const invalidToken = tokenMeta("--cem-does-not-exist");
