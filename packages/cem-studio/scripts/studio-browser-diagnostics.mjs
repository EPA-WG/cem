/** Read only technical diagnostics; never inspect editor, draft or form values.
 * The caller owns the existing bounded browser-evaluation deadline.
 */
export async function captureStudioRenderFailure(page, pageErrors = []) {
    const diagnostics = await page.evaluate(async () => {
        const module = new URL('./assets/@epa-wg/cem-studio/shell.js', document.baseURI).href;
        const { runtime } = await (await import(module)).installCemStudioShellComponents();
        const records = [];
        const tags = ['cem-studio-workbench-controls', 'cem-studio-workbench-center', 'cem-studio-workbench-layout',
            'cem-studio-workbench-navigation', 'cem-source-editor', 'cem-expansion'];
        for (const tag of tags) {
            const instance = tag === 'cem-expansion'
                ? document.querySelector('cem-studio-workbench-controls cem-expansion[label="Editor details"]') ?? document.querySelector('cem-expansion[label="Commands and results"]')
                : document.querySelector(tag);
            const declaration = document.querySelector(`${runtime.declarationTag}[tag="${tag}"]`);
            for (const [kind, target] of [['instance', instance], ['declaration', declaration]]) {
                if (!target) continue;
                for (const diagnostic of runtime.diagnosticsFor(target).slice(-4)) {
                    records.push({ target: `${tag}:${kind}`, code: String(diagnostic.code).slice(0, 160),
                        severity: String(diagnostic.severity).slice(0, 20), message: String(diagnostic.message).slice(0, 500) });
                }
            }
        }
        return records.slice(0, 24);
    });
    return { pageErrors: pageErrors.slice(-20).map(error => String(error?.message ?? error).slice(0, 500)), diagnostics };
}
