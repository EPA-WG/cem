/** Consume an optional command token before navigation can notify live readers. */
export function consumeLocationWriteTrigger(
    consumed: Map<number, string>,
    writer: number,
    trigger: string | undefined,
): boolean {
    if (trigger === undefined) return true;
    if (trigger === '' || consumed.get(writer) === trigger) return false;
    consumed.set(writer, trigger);
    return true;
}
