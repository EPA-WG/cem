let pending: Array<() => void> | undefined;

/** A joint operation must own its cleanup boundary to report a final result. */
export function stylesheetNotificationsDeferred(): boolean { return pending !== undefined; }

/** Run every cleanup even when another cleanup fails. */
export function notifyStylesheetLifecycle(notifications: readonly (() => void)[]): void {
    if (pending) {
        pending.push(...notifications);
        return;
    }
    const errors: unknown[] = [];
    for (const notify of notifications) {
        try { notify(); } catch (error) { errors.push(error); }
    }
    if (errors.length) throw new AggregateError(errors, 'stylesheet ownership cleanup failed');
}

/**
 * Defer ownership release/abort notifications until synchronous publication ends.
 * Nested publications share the outer queue. This does not defer DOM mutations,
 * browser callbacks or work after an await, and it provides no rollback.
 */
export function deferStylesheetNotifications<T>(publish: () => T): T {
    if (pending) return publish();
    const notifications: Array<() => void> = [];
    pending = notifications;
    const errors: unknown[] = [];
    let result: T | undefined;
    try { result = publish(); } catch (error) { errors.push(error); }
    finally { pending = undefined; }
    try { notifyStylesheetLifecycle(notifications); } catch (error) { errors.push(error); }
    if (errors.length === 1) throw errors[0];
    if (errors.length > 1) throw new AggregateError(errors, 'stylesheet publication and cleanup failed');
    return result as T;
}
