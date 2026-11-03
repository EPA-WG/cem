/** Coordinate existing services; no UI is published before both are ready.
 * @template {{close: () => unknown}} T
 * @param {{createValidator: () => Promise<T>, installComponents: () => Promise<unknown>, serial?: boolean}} options
 * @returns {Promise<T>}
 */
export async function prepareCemStudioRuntime({ createValidator, installComponents, serial = false }) {
    // Package-demand profiles share activation admission and retain their order.
    if (serial) {
        const validator = await createValidator();
        try { await installComponents(); return validator; }
        catch (error) { return failedStartup(validator, [error]); }
    }
    const [validator, components] = await Promise.allSettled([
        Promise.resolve().then(createValidator),
        Promise.resolve().then(installComponents),
    ]);
    if (validator.status === 'fulfilled' && components.status === 'fulfilled') return validator.value;
    const errors = [validator, components].flatMap(result => result.status === 'rejected' ? [result.reason] : []);
    return failedStartup(validator.status === 'fulfilled' ? validator.value : undefined, errors);
}

/** @param {{close: () => unknown} | undefined} validator
 * @param {unknown[]} errors
 * @returns {Promise<never>}
 */
async function failedStartup(validator, errors) {
    if (validator) {
        try { await validator.close(); }
        catch (error) { errors.push(error); }
    }
    if (errors.length === 1) throw errors[0];
    throw new AggregateError(errors, 'CEM Studio runtime startup failed');
}
