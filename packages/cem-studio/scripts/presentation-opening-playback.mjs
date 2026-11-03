import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { chromium } from 'playwright';
import { workspaceRoot, presenterHash } from './presenter-project.mjs';
import { presentationId } from './presentation-deck-notes.mjs';

// Fresh anonymous context: no saved profile, credentials, keyboard or mouse input.
const url = `https://docs.google.com/presentation/d/${presentationId}/present?slide=id.p1`;
const output = resolve(workspaceRoot, 'dist/reports/cem-studio/projector-opening-playback');
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ headless: true });
try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30000 });
    assert.equal(page.url(), url, 'Unexpected redirect; do not automate authentication');
    assert.match(await page.title(), /CEM-ML.*Google Slides/);
    // Slide playback may start before DOMContentLoaded. Reference navigation
    // start rather than treating our later readiness callback as slide entry.
    const readyMs = await page.evaluate(() => performance.now());
    const start = performance.now() - readyMs, frames = [];
    for (const milliseconds of [0, 3500, 7500, 12500, 17500, 22500, 26000, 30000]) {
        const wait = start + milliseconds - performance.now();
        if (wait > 0) await new Promise(resolve => setTimeout(resolve, wait));
        const bytes = await page.screenshot();
        const capturedMs = performance.now() - start;
        const counts = await page.evaluate(async bytes => {
            const bitmap = await createImageBitmap(new Blob([new Uint8Array(bytes)], { type: 'image/png' }));
            const canvas = new OffscreenCanvas(bitmap.width, bitmap.height), context = canvas.getContext('2d');
            context.drawImage(bitmap, 0, 0); bitmap.close();
            const result = {};
            for (const [stage, x, width] of [['thought', 70, 320], ['dna', 560, 320], ['cem', 1040, 350]]) {
                const data = context.getImageData(x, 390, width, 300).data; let pixels = 0;
                for (let i = 0; i < data.length; i += 4) {
                    if (Math.max(Math.abs(data[i] - 241), Math.abs(data[i + 1] - 254), Math.abs(data[i + 2] - 254)) > 40) pixels++;
                }
                result[stage] = pixels;
            }
            return result;
        }, [...bytes]);
        const file = `frame-${milliseconds}.png`; await writeFile(resolve(output, file), bytes);
        frames.push({ milliseconds, capturedMs, file, sha256: presenterHash(bytes), pixels: counts });
    }
    const at = ms => frames.find(frame => frame.milliseconds === ms).pixels;
    for (const ms of [3500, 7500]) {
        assert.ok(at(ms).thought > 1000, 'Thought did not appear automatically');
        assert.ok(at(ms).dna < 100 && at(ms).cem < 100, 'Later stages appeared before their interval');
    }
    for (const ms of [12500, 17500, 22500]) {
        assert.ok(at(ms).thought > 1000 && at(ms).dna > 500, 'Thought/DNA stage missing');
        assert.ok(at(ms).cem < 100, 'CEM appeared before its interval');
    }
    for (const ms of [26000, 30000]) for (const count of Object.values(at(ms))) assert.ok(count > 1000, 'Final stage missing');
    assert.ok(frames.filter(frame => frame.milliseconds !== 0).every(frame => frame.capturedMs < frame.milliseconds + 1500), 'Browser too slow to claim the sampled intervals');
    const result = { schemaVersion: 1, checkedDate: new Date().toLocaleDateString('en-CA', { timeZone: 'America/Chicago' }), presentationId, url,
        browser: browser.version(), isolatedAnonymousContext: true, clicks: 0, slideEntryStarts: true,
        sampledStageOrder: ['thought', 'dna', 'cem'], allVisibleAt26Seconds: true,
        earlierStagesRetainedAt30Seconds: true, measurementOrigin: 'navigationStart; first frame captured on DOM readiness; sampled intervals, not exact transition timestamps',
        readyMs,
        nativeTimelineMs: 25100, measuredFullRehearsal: false, frames, pageErrors: errors };
    await writeFile(resolve(output, 'report.json'), JSON.stringify(result, null, 2) + '\n');
    console.log('Verified zero-click opening playback, staged reveals and retained content at 30 seconds.');
} finally { await browser.close(); }
