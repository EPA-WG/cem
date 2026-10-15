import assert from 'node:assert/strict';
import { createServer } from 'node:http';

/** Real document screenshots: SVG-as-image rasterization cannot establish
 * external paint support in the live document. No preparation seal is emitted. */
export async function verifySvgReplayCompatibility(browser) {
    assert.equal(browser.version(), '148.0.7778.96', 'SVG replay compatibility must be reviewed for a new browser');
    const requests = [];
    const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><defs><linearGradient id="gradient"><stop stop-color="lime"/></linearGradient><pattern id="pattern" width="1" height="1"><rect width="16" height="16" fill="lime"/></pattern></defs><rect width="16" height="16" fill="blue"/></svg>';
    const server = createServer((request, response) => {
        if (request.url.startsWith('/paint.svg?')) {
            requests.push({ url: request.url, destination: request.headers['sec-fetch-dest'] ?? null });
            response.writeHead(200, { 'content-type': 'image/svg+xml', 'cache-control': 'max-age=3600' });
            response.end(request.url.includes('-warm') || request.url.includes('-source') ? svg.replaceAll('lime', 'magenta') : svg);
        } else {
            response.writeHead(200, { 'content-type': 'text/html' });
            response.end('<!doctype html><style>html,body{margin:0;background:white}svg{display:block}</style><body></body>');
        }
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const origin = `http://127.0.0.1:${server.address().port}`;
    const context = await browser.newContext({ viewport: { width: 64, height: 64 }, deviceScaleFactor: 1 });
    const cases = [];
    try {
        const page = await context.newPage();
        const sample = async () => {
            await page.waitForLoadState('networkidle');
            const screenshot = (await page.screenshot()).toString('base64');
            return page.evaluate(async png => {
                const image = new Image(); image.src = `data:image/png;base64,${png}`; await image.decode();
                const canvas = document.createElement('canvas'); canvas.width = canvas.height = 16;
                const ctx = canvas.getContext('2d'); ctx.drawImage(image, 0, 0);
                return [...ctx.getImageData(8, 8, 1, 1).data];
            }, screenshot);
        };
        for (const kind of ['gradient', 'pattern']) {
            for (const mode of ['local', 'fresh', 'warm', 'replay', 'fresh-no-fallback', 'replay-no-fallback']) {
                const replay = mode.startsWith('replay');
                const fallback = mode.endsWith('no-fallback') ? '' : ' red';
                const path = `/paint.svg?case=${kind}-${mode}`;
                const sourcePath = replay ? `/paint.svg?case=${kind}-${mode}-source` : path;
                await page.goto(origin);
                const before = requests.length;
                const result = await page.evaluate(async ({ kind, mode, url, sourceUrl, replay, fallback }) => {
                    const settle = () => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
                    let imagePixel;
                    if (mode === 'warm' || replay) {
                        const image = new Image(); image.src = sourceUrl;
                        await image.decode();
                        const canvas = document.createElement('canvas'); canvas.width = canvas.height = 16;
                        const ctx = canvas.getContext('2d'); ctx.drawImage(image, 0, 0);
                        imagePixel = [...ctx.getImageData(8, 8, 1, 1).data];
                    }
                    const defs = '<linearGradient id="gradient"><stop stop-color="lime"/></linearGradient><pattern id="pattern" width="1" height="1"><rect width="16" height="16" fill="lime"/></pattern>';
                    document.body.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><defs>${mode === 'local' ? defs : ''}</defs><rect width="16" height="16" fill="url(${mode === 'local' ? '' : url}#${kind})${fallback}"/></svg>`;
                    await settle();
                    return { computed: getComputedStyle(document.querySelector('svg > rect')).fill, imagePixel };
                }, { kind, mode, url: origin + path, sourceUrl: origin + sourcePath, replay, fallback });
                const pixel = await sample();
                const actualRequests = requests.slice(before);
                assert.deepEqual(pixel, mode === 'warm' ? [255, 0, 255, 255] : [0, 255, 0, 255], `${kind}/${mode} paint pixels`);
                assert.ok(result.computed.includes(`#${kind}`));
                assert.deepEqual(actualRequests, mode === 'local' ? [] : (replay ? [sourcePath, path] : [path]).map(url => ({ url, destination: 'image' })));
                if (mode === 'warm' || replay) assert.deepEqual(result.imagePixel, [0, 0, 255, 255]);
                let restoredPixel;
                if (replay) {
                    await page.evaluate(({ url, kind, fallback }) => document.querySelector('svg > rect').setAttribute('fill', `url(${url}#${kind})${fallback}`), { url: origin + sourcePath, kind, fallback });
                    restoredPixel = await sample();
                    assert.deepEqual(restoredPixel, [255, 0, 255, 255], 'retargeting must use the cached source, not the fresh alias');
                    assert.equal(requests.length - before, 2, 'retargeting unexpectedly reacquired cached source');
                }
                cases.push({ kind, mode, ...result, pixel, restoredPixel, requests: actualRequests });
            }
        }
        console.log(`Verified ${cases.length} live-document SVG paint replay compatibility cases in Chromium ${browser.version()}.`);
        return { profile: 'chromium-148-external-paint-replay-v1', version: browser.version(), cases, executionSeal: false };
    } finally { await context.close(); await new Promise(resolve => server.close(resolve)); }
}
