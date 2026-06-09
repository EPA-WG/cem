// FF-gate map — composing fitness-function gate (FF-1..FF-8).
// Guards OQ-2 / BR-FF-1/2/3: every fitness function is a named, CI-blocking gate, and the FF→gate
// mapping itself is verified so it can't silently drift. FF-5/FF-6 are standalone scanners; the
// other FFs reuse existing gates (cem_ml_cli:validate-fixtures / e2e, cem-elements:verify). This
// script does NOT re-run those heavy gates — they run on their own in CI — it asserts the mapping
// is intact:
//   - every backing Nx target is actually DEFINED,
//   - for `active` FFs, the backing target is invoked in the CI workflow and its evidence
//     fixtures/scripts exist on disk,
//   - `tracked` FFs (AC-P-V dispatch fixtures not authored yet) are reported non-blocking.
// Exit non-zero when an `active` FF's backing target is undefined/unwired or its evidence is
// missing. Pass `--json` for a machine-readable report. See docs/fitness-functions.md.
import { readFile, readdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(here, '..', '..');
const emitJson = process.argv.includes('--json');

const registry = JSON.parse(
    await readFile(join(workspaceRoot, 'tools', 'fitness', 'fitness-gates.json'), 'utf8'),
);

// Map every Nx project name -> Set of its target names (packages/*/project.json + root package.json).
async function loadProjectTargets() {
    const map = new Map();
    const rootPkg = JSON.parse(await readFile(join(workspaceRoot, 'package.json'), 'utf8'));
    map.set(rootPkg.name, new Set(Object.keys(rootPkg.nx?.targets ?? {})));
    const packagesDir = join(workspaceRoot, 'packages');
    for (const entry of await readdir(packagesDir, { withFileTypes: true })) {
        if (!entry.isDirectory()) continue;
        const projectFile = join(packagesDir, entry.name, 'project.json');
        if (!existsSync(projectFile)) continue;
        const project = JSON.parse(await readFile(projectFile, 'utf8'));
        if (project.name) {
            map.set(project.name, new Set(Object.keys(project.targets ?? {})));
        }
    }
    return map;
}

function splitTargetRef(ref) {
    const idx = ref.lastIndexOf(':');
    return { project: ref.slice(0, idx), target: ref.slice(idx + 1) };
}

// True when `target` is invoked as a standalone token in the CI workflow — preceded by whitespace
// or a `:` (an `nx run proj:target` ref) and followed by whitespace/quote/end. Avoids a substring
// false-match such as `e2e` matching the unrelated `e2e-ci` target.
function ciInvokes(target) {
    const escaped = target.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    return new RegExp(`(?:^|[\\s:])${escaped}(?=[\\s"']|$)`, 'm').test(ciWorkflow);
}

const projectTargets = await loadProjectTargets();
const ciWorkflowPath = join(workspaceRoot, registry.ciWorkflow);
const ciWorkflow = existsSync(ciWorkflowPath) ? await readFile(ciWorkflowPath, 'utf8') : '';

const results = [];
const errors = [];

for (const gate of registry.gates) {
    const problems = [];
    let ciWired = false;
    for (const ref of gate.backing ?? []) {
        const { project, target } = splitTargetRef(ref);
        const targets = projectTargets.get(project);
        if (!targets || !targets.has(target)) {
            problems.push(`backing target not defined: ${ref}`);
            continue;
        }
        if (ciInvokes(target)) {
            ciWired = true;
        }
    }
    if (gate.status === 'active') {
        // An active FF must be enforced by at least one of its backing targets running in CI, and
        // its evidence fixtures/scripts must exist.
        if ((gate.backing ?? []).length > 0 && !ciWired) {
            problems.push(`no backing target invoked in ${registry.ciWorkflow}`);
        }
        for (const ev of gate.evidence ?? []) {
            if (!existsSync(join(workspaceRoot, ev))) {
                problems.push(`evidence missing: ${ev}`);
            }
        }
    }
    results.push({ id: gate.id, status: gate.status, title: gate.title, problems, tracks: gate.tracks });
    if (gate.status === 'active' && problems.length > 0) {
        errors.push({ gate, problems });
    }
}

printReport();
if (emitJson) {
    console.log(JSON.stringify({ errorCount: errors.length, gates: results }, null, 2));
}
process.exit(errors.length > 0 ? 1 : 0);

function printReport() {
    console.log('FF-gate map (FF-1..FF-8)');
    const active = results.filter((r) => r.status === 'active');
    const tracked = results.filter((r) => r.status === 'tracked');
    console.log(`  active: ${active.length}   tracked: ${tracked.length}   errors: ${errors.length}`);
    for (const r of results) {
        const mark = r.status === 'tracked' ? '○' : r.problems.length ? '✗' : '✓';
        console.log(`  ${mark} ${r.id} [${r.status}] ${r.title}`);
        if (r.status === 'tracked' && r.tracks) {
            console.log(`      tracks: ${r.tracks}`);
        }
        for (const p of r.problems) {
            console.log(`      - ${p}`);
        }
    }
    if (errors.length === 0) {
        console.log('FF-gate map intact: every active fitness function is backed by a defined, CI-wired gate.');
    }
}
