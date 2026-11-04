import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { homedir } from 'node:os';
import { chromium } from 'playwright';
import { verifyDiscussionDemoDraft } from './presentation-resources.mjs';

const workspace = resolve(import.meta.dirname, '../../..');
const file = resolve(workspace, 'dist/reports/cem-studio/public-discussion-demo-links.json');
// The shared draft guard admits only the frozen Amsterdam host, subtree and
// finite project/resource/configuration rows. This has no arbitrary URL option.
const draft = await verifyDiscussionDemoDraft();
let browser;
const report = { schemaVersion: 1, checkedAt: new Date().toISOString(), browserFlags: [],
    status: 'started', navigationOnly: true, fullWorkflowAcceptance: 'recorded separately', discussionPosted: false, results: [] };
async function save() { await mkdir(resolve(workspace, 'dist/reports/cem-studio'), { recursive: true }); await writeFile(file, `${JSON.stringify(report, null, 2)}\n`); }
await save();
try {
    const expected = await readFile(resolve(homedir(), 'cem-bin/demo-deployment.json'));
    const response = await fetch(new URL('/demo-deployment.json', draft.links[0].url), {
        cache: 'no-store', signal: AbortSignal.timeout(30_000),
    });
    assert.equal(response.status, 200);
    const inventory = Buffer.from(await response.arrayBuffer());
    report.inventorySha256 = createHash('sha256').update(inventory).digest('hex');
    assert.ok(inventory.equals(expected), 'Public inventory must match the prepared deployment before navigation checks');
    browser = await chromium.launch({ headless: true }); report.browser = browser.version();
    for (const link of draft.links) {
        const context = await browser.newContext();
        const result = { ...link, status: 'started', errors: [] };
        report.results.push(result);
        try {
            const page = await context.newPage(); page.setDefaultTimeout(30_000);
            page.on('pageerror', error => result.errors.push(error.message));
            const response = await page.goto(link.url);
            assert.equal(response.status(), 200);
            await page.waitForFunction(() => Boolean(globalThis.__cemStudioApplication));
            await page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
            result.actual = await page.evaluate(async () => {
                const app = globalThis.__cemStudioApplication, state = app.workbench.snapshot();
                const projects = await app.repository.query({ protocolVersion: 1, repository: 'studio-projects',
                    requestRevision: 1, operation: 'list-projects', parameters: {} });
                return { selection: state.navigation.selection, configuration: state.projectPreview.configurationId,
                    structure: state.navigation.trees.structure.selected,
                    sampleTextAvailable: state.navigation.sampleTextAvailable, projectCount: projects.value.length };
            });
            assert.equal(page.url(), link.url);
            for (const [key, value] of Object.entries({ collection: 'amsterdam-2026', sample: link.project, file: link.resource })) {
                assert.equal(result.actual.selection[key], value);
                assert.equal(new URLSearchParams(result.actual.structure).get(key), value);
            }
            assert.equal(result.actual.configuration, link.configuration);
            assert.equal(result.actual.sampleTextAvailable, true);
            assert.equal(result.actual.projectCount, 0);
            assert.deepEqual(result.errors, []);
            result.status = 'passed';
            console.log(`Public demo link ${link.slide}/${link.cue}: ${link.project}/${link.resource}/${link.configuration} passed`);
        } catch (error) { result.status = 'failed'; result.error = error.message; throw error; }
        finally { await context.close(); await save(); }
    }
    report.status = 'passed';
} catch (error) { report.status = 'failed'; report.error = error.message; throw error; }
finally { report.finishedAt = new Date().toISOString(); await save(); await browser?.close(); }
