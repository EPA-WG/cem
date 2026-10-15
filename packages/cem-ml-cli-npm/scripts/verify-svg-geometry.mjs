import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { resolve } from 'node:path';

/** Explicit fixture source/scalar export, not a runtime layout or paint seal. */
export async function verifySvgGeometry(browser, workspaceRoot) {
    assert.equal(browser.version(), '148.0.7778.96');
    const { stdout } = await promisify(execFile)('cargo', ['run', '--quiet', '-p', 'cem-ml', '--example', 'svg-geometry-parity-emit'], {
        cwd: workspaceRoot, env: { ...process.env, CARGO_TARGET_DIR: resolve(workspaceRoot, 'dist/target') },
        timeout: 300_000, maxBuffer: 4 * 1024 * 1024,
    });
    const native = JSON.parse(stdout);
    assert.equal(native.executionEvidence, false);
    assert.deepEqual([native.transforms.length, native.viewboxes.length, native.gradients.length, native.composites.length], [5, 4, 4, 3]);
    const context = await browser.newContext();
    const cases = [];
    const matrixClose = (actual, expected) => actual.forEach((v, i) => assert.ok(Math.abs(v - expected[i]) <= 1e-5 * Math.max(1, Math.abs(expected[i])), `matrix ${actual} differs from native ${expected}`));
    try {
        const page = await context.newPage();
        for (const fixture of native.transforms) {
            const matrix = await page.evaluate(source => {
                const g = document.createElementNS('http://www.w3.org/2000/svg', 'g');
                g.setAttribute('transform', source);
                const m = g.transform.baseVal.consolidate().matrix;
                return [m.a, m.b, m.c, m.d, m.e, m.f];
            }, fixture.source);
            matrixClose(matrix, fixture.matrix);
            cases.push({ kind: 'transform', ...fixture, actual: matrix });
        }
        for (const fixture of native.viewboxes) {
            const matrix = await page.evaluate(({ viewBox, viewport, aspect }) => {
                document.body.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200"><svg x="${viewport[0]}" y="${viewport[1]}" width="${viewport[2]}" height="${viewport[3]}" viewBox="${viewBox.join(' ')}" preserveAspectRatio="${aspect}"><g/></svg></svg>`;
                const m = document.querySelector('g').getCTM();
                return [m.a, m.b, m.c, m.d, m.e, m.f];
            }, fixture);
            matrixClose(matrix, fixture.matrix);
            cases.push({ kind: 'viewBox', ...fixture, actual: matrix });
        }
        const raster = async (content, gradient, points) => page.evaluate(async ({ content, gradient, points }) => {
            const source = `<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><defs>${content}</defs><rect x="${gradient ? 10 : 0}" y="${gradient ? 20 : 0}" width="${gradient ? 80 : 16}" height="${gradient ? 40 : 16}" fill="url(#p)"/></svg>`;
            const url = URL.createObjectURL(new Blob([source], { type: 'image/svg+xml' }));
            try {
                const image = new Image(); image.src = url; await image.decode();
                const c = document.createElement('canvas'); c.width = c.height = 100;
                const ctx = c.getContext('2d'); ctx.drawImage(image, 0, 0);
                return points.map(([x, y]) => [...ctx.getImageData(x, y, 1, 1).data]);
            } finally { URL.revokeObjectURL(url); }
        }, { content, gradient, points });
        for (const fixture of native.gradients) {
            const pixels = await raster(fixture.content, true, fixture.points.map(p => p.point));
            assert.deepEqual(pixels, fixture.points.map(p => p.pixel), 'native gradient coordinate prediction');
            cases.push({ kind: 'gradient', ...fixture, actual: pixels });
        }
        for (const fixture of native.composites) {
            const [pixel] = await raster(fixture.content, false, [[8, 8]]);
            const premultiplied = pixel.map((v, i) => i === 3 ? v / 255 : v * pixel[3] / (255 * 255));
            // Native arithmetic is continuous; Chromium rasterizes intermediate
            // layers to 8-bit channels. One channel step covers rounding, but
            // cannot hide flattening a group's opacity onto its children.
            premultiplied.forEach((v, i) => assert.ok(Math.abs(v - fixture.premultiplied[i]) <= 1 / 255 + 1e-10, `compositing ${pixel} differs from native ${fixture.premultiplied}`));
            cases.push({ kind: 'compositing', ...fixture, actual: pixel });
        }
        console.log(`Verified ${cases.length} native SVG geometry/compositing cases against Chromium ${browser.version()}.`);
        return { profile: 'explicit-svg-geometry-source-over-v1', version: browser.version(), executionEvidence: false, cases };
    } finally { await context.close(); }
}
