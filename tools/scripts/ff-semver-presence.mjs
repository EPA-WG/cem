// FF-6 — SemVer-presence lint.
// Guards BR-VC-5 (every governed contract carries an independent SemVer axis) and BR-EV-6
// (governed contracts are exactly the enumerated boundary contracts). See docs/fitness-functions.md.
//
// `required` contracts MUST resolve to a valid SemVer at their locator, or the check fails.
// `pending-version` contracts are known un-versioned gaps: reported, non-blocking, until their
// SemVer axis lands — at which point FF-6 flags them "ready to promote" to `required`.
// Flags: `--json` emits a machine-readable report; `--registry=PATH` overrides the registry.
import { readFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isSemver, readVersionFromLocator } from '../fitness/lib.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(here, '..', '..');
const registryArg = process.argv.find((a) => a.startsWith('--registry='));
const registryPath = registryArg
    ? resolve(registryArg.slice('--registry='.length))
    : join(workspaceRoot, 'tools', 'fitness', 'governed-contracts.json');
const emitJson = process.argv.includes('--json');

const registry = JSON.parse(await readFile(registryPath, 'utf8'));

const rows = [];
for (const contract of registry.contracts) {
    const found = await readVersionFromLocator(workspaceRoot, contract.locator);
    const valid = found.found && isSemver(found.value);
    rows.push({ contract, value: found.value, valid, error: found.error });
}

// A `required` contract without a valid SemVer is a blocking failure.
const errors = rows.filter((r) => r.contract.status === 'required' && !r.valid);
// A `pending-version` contract whose version now resolves is ready to be promoted to `required`.
const promotable = rows.filter((r) => r.contract.status === 'pending-version' && r.valid);

printReport();
if (emitJson) {
    console.log(
        JSON.stringify(
            {
                errorCount: errors.length,
                promotable: promotable.map((r) => r.contract.id),
                contracts: rows.map((r) => ({
                    id: r.contract.id,
                    status: r.contract.status,
                    version: r.valid ? r.value : null,
                    valid: r.valid,
                    error: r.valid ? null : r.error,
                })),
            },
            null,
            2,
        ),
    );
}
process.exit(errors.length > 0 ? 1 : 0);

function printReport() {
    console.log('FF-6 SemVer-presence lint');
    console.log('  governed contracts:');
    for (const r of rows) {
        if (r.contract.status === 'required') {
            const state = r.valid ? `v${r.value}` : `MISSING (${r.error})`;
            console.log(`    - ${r.contract.id} [required]: ${state}`);
        } else {
            const state = r.valid
                ? `v${r.value} — READY TO PROMOTE to 'required'`
                : `PENDING gap (${r.contract.tracks ?? 'untracked'})`;
            console.log(`    - ${r.contract.id} [pending-version]: ${state}`);
        }
    }
    if (errors.length === 0) {
        console.log(
            `  result: PASS — all 'required' contracts declare a valid SemVer` +
                (promotable.length ? `; ${promotable.length} pending contract(s) ready to promote.` : '.'),
        );
        return;
    }
    console.log(`  result: FAIL — ${errors.length} 'required' contract(s) without a valid SemVer:`);
    for (const e of errors) console.log(`    ✗ ${e.contract.id}: ${e.error ?? `invalid SemVer '${e.value}'`}`);
}
