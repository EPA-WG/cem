export const themeModes = [
    ['native', 'Native'], ['light', 'Light'], ['dark', 'Dark'],
    ['contrast-light', 'Contrast light'], ['contrast-dark', 'Contrast dark'],
] as const;

export function themeMode(value: unknown): string {
    return themeModes.some(([mode]) => mode === value) ? value as string : 'native';
}
