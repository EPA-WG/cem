import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

export const markdownAuditHash = bytes => createHash('sha256').update(bytes).digest('hex');

export async function readMarkdownAudit(workspace) {
    const bytes = await readFile(resolve(workspace, 'dist/reports/cem-site/markdown-tables.json'));
    const audit = JSON.parse(bytes);
    assert.equal(audit.status, 'passed', 'Run cem-site:test:markdown before packaging');
    assert.equal(audit.sourceCount, audit.sources.length);
    for (const [path, expected] of [
        ['apps/cem-site/site.cem', audit.siteGraphSha256],
        ['apps/cem-site/layouts/page.cemt', audit.pageLayoutSha256],
    ]) assert.equal(markdownAuditHash(await readFile(resolve(workspace, path))), expected, `Stale native Markdown audit: ${path}`);
    for (const source of audit.sources) {
        assert.equal(markdownAuditHash(await readFile(resolve(workspace, 'apps/cem-site', source.source))),
            source.sourceSha256, `Stale native Markdown source audit: ${source.source}`);
    }
    return { bytes, audit };
}
