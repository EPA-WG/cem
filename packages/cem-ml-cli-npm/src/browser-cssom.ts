/** Bounded browser-owned serialization for comparison, never a source parser. */
export interface CssomBudget { rules: number; bytes: number }
export function cssomText(sheet: CSSStyleSheet, budget: CssomBudget): string {
    if (sheet.cssRules.length > 65_536 - budget.rules) throw new Error('Stylesheet CSSOM rule bound exceeded');
    budget.rules += sheet.cssRules.length;
    const values: string[] = [];
    for (const rule of sheet.cssRules) {
        const text = rule.cssText;
        budget.bytes += text.length;
        if (budget.bytes > 4 * 1024 * 1024) throw new Error('Stylesheet CSSOM byte bound exceeded');
        values.push(text);
    }
    return JSON.stringify(values);
}
