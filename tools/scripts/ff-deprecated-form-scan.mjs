// FF-5 — deprecated-form removal scan.
// Guards BR-EV-7 (parallel-change removal gate) and BR-CO-3 (legacy inventoried).
// Generalizes the verify-package-baseline.mjs XSLTProcessor guard into a registry-driven
// workspace scan. See docs/fitness-functions.md.
//
// Exit non-zero when a `forbidden` form is present (outside its allowlist) or a `deprecated`
// form is still used past its removal MAJOR. `deprecated` forms within their window are
// reported as inventory only. Pass `--json` to also emit a machine-readable report.
import { readFile } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { findSubstringHits, pathMatches, toPosix, walkFiles } from '../fitness/lib.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(here, '..', '..');
const registryPath = join(workspaceRoot, 'tools', 'fitness', 'deprecated-forms.json');
const emitJson = process.argv.includes('--json');

const registry = JSON.parse(await readFile(registryPath, 'utf8'));
const { currentMajor, scanRoots, extensions, ignoreDirs, forms } = registry;

const files = [];
for (const root of scanRoots) {
    const found = await walkFiles(join(workspaceRoot, root), { extensions, ignoreDirs });
    for (const abs of found) files.push({ abs, rel: toPosix(relative(workspaceRoot, abs)) });
}

const results = [];
for (const form of forms) {
    const raw = [];
    for (const file of files) raw.push(...(await findSubstringHits(file.abs, file.rel, form.pattern)));
    const allow = form.allowlist ?? [];
    const hits = raw.filter((h) => !allow.some((a) => pathMatches(h.file, a)));
    results.push({ form, hits, allowlistedOut: raw.length - hits.length });
}

const errors = [];
for (const { form, hits } of results) {
    if (form.status === 'forbidden' && hits.length > 0) {
        errors.push({ form, hits, reason: form.reason ?? 'forbidden form present in source' });
    } else if (
        form.status === 'deprecated' &&
        typeof form.removeAtMajor === 'number' &&
        form.removeAtMajor <= currentMajor &&
        hits.length > 0
    ) {
        errors.push({
            form,
            hits,
            reason: `deprecated form past its removal deadline (removeAtMajor ${form.removeAtMajor} <= currentMajor ${currentMajor}); migrate to ${form.replacement ?? 'the replacement'} before this MAJOR`,
        });
    }
}

printReport();
if (emitJson) {
    console.log(
        JSON.stringify(
            {
                currentMajor,
                errorCount: errors.length,
                forms: results.map((r) => ({
                    id: r.form.id,
                    status: r.form.status,
                    removeAtMajor: r.form.removeAtMajor ?? null,
                    count: r.hits.length,
                    allowlisted: r.allowlistedOut,
                    hits: r.hits,
                })),
            },
            null,
            2,
        ),
    );
}
process.exit(errors.length > 0 ? 1 : 0);

function printReport() {
    console.log('FF-5 deprecated-form removal scan');
    console.log(`  roots: ${scanRoots.join(', ')}  currentMajor: ${currentMajor}`);
    console.log('  inventory:');
    for (const r of results) {
        const tag = r.form.status === 'forbidden' ? 'forbidden' : `deprecated→v${r.form.removeAtMajor ?? '?'}`;
        const extra = r.allowlistedOut ? ` (+${r.allowlistedOut} allowlisted)` : '';
        console.log(`    - ${r.form.id} [${tag}]: ${r.hits.length} hit(s)${extra}`);
        for (const h of r.hits.slice(0, 20)) console.log(`        ${h.file}:${h.line}`);
        if (r.hits.length > 20) console.log(`        … ${r.hits.length - 20} more`);
    }
    if (errors.length === 0) {
        console.log('  result: PASS — no forbidden forms; deprecated forms within their window.');
        return;
    }
    console.log(`  result: FAIL — ${errors.length} blocking finding(s):`);
    for (const e of errors) {
        console.log(`    ✗ ${e.form.id}: ${e.reason}`);
        for (const h of e.hits.slice(0, 20)) console.log(`        ${h.file}:${h.line}`);
    }
}
