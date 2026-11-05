import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';

const project = resolve(import.meta.dirname, '..'), workspace = resolve(project, '../..');
const nx = resolve(workspace, 'node_modules/nx/dist/bin/nx.js');
const env = { ...process.env, NX_DAEMON: 'false' };
const describe = spawnSync(process.execPath, [nx, 'show', 'project', '@epa-wg/cem-studio', '--json'], {
    cwd: workspace, env, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024,
});
assert.equal(describe.status, 0, describe.stderr);
const { targets } = JSON.parse(describe.stdout);
const writers = new Set(['build', 'generate:feature-tour', 'verify:determinism', 'build:schema-assets', 'test:sample-profile-isolation']);
function browser(target) {
    const options = target.options ?? {};
    const cwd = resolve(workspace, (options.cwd ?? '').replace('{projectRoot}', project).replace('{workspaceRoot}', workspace));
    for (const entry of [options.command, ...(options.commands ?? [])]) {
        const command = typeof entry === 'string' ? entry : entry?.command;
        if (!command) continue;
        if (/\bvitest\b/.test(command) || command.includes('verify-catalog-acceptance.mjs')) return true;
        for (const [, path] of command.matchAll(/\bnode\s+(?:--test\s+)?((?:tools\/)?scripts\/[\w.-]+\.mjs)/g)) {
            const file = resolve(cwd, path);
            if (existsSync(file) && /\bfrom\s*['"](?:playwright|@playwright\/test)['"]/.test(readFileSync(file, 'utf8'))) return true;
        }
    }
    return false;
}
const protectedTasks = Object.entries(targets).filter(([name, target]) => writers.has(name) || browser(target));
assert.ok(protectedTasks.some(([name]) => name === 'test:workbench'));
assert.ok(protectedTasks.some(([name]) => name === 'verify:login-navigation'));
if (process.argv.includes('--list')) {
    console.log(JSON.stringify(protectedTasks.map(([name]) => name), null, 2));
} else {
    const root = mkdtempSync(resolve(tmpdir(), 'cem-release-scheduler-'));
    try {
        symlinkSync(resolve(workspace, 'node_modules'), resolve(root, 'node_modules'), process.platform === 'win32' ? 'junction' : 'dir');
        writeFileSync(resolve(root, 'nx.json'), JSON.stringify({ parallel: 4 }));
        writeFileSync(resolve(root, 'package.json'), JSON.stringify({ name: 'cem-release-scheduler-fixture', private: true, type: 'module' }));
        writeFileSync(resolve(root, '.gitignore'), 'node_modules\n.nx\ncache\nworkspace-data\n');
        mkdirSync(resolve(root, 'active')); mkdirSync(resolve(root, 'started'));
        writeFileSync(resolve(root, 'probe.mjs'), `
import assert from 'node:assert/strict';
import { appendFileSync, existsSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
const [id, kind] = process.argv.slice(2), marker = 'active/' + kind + '-' + id;
const event = phase => appendFileSync('events.jsonl', JSON.stringify({ id, kind, phase, time: Date.now() }) + '\\n');
const check = () => {
    const active = readdirSync('active');
    assert.ok(kind === 'critical' ? active.length === 1 : !active.some(name => name.startsWith('critical-')),
        'protected workload overlap: ' + active.join(', '));
};
writeFileSync(marker, '', { flag: 'wx' }); writeFileSync('started/' + id, ''); event('start');
try {
    check();
    if (kind === 'light') {
        const peer = id === 'light-a' ? 'light-b' : 'light-a', deadline = Date.now() + 5000;
        while (!existsSync('started/' + peer)) {
            assert.ok(Date.now() < deadline, 'unrelated build probes were serialized');
            await new Promise(done => setTimeout(done, 10));
        }
    }
    await new Promise(done => setTimeout(done, 100)); check();
} finally { event('end'); rmSync(marker); }
`);
        const probes = [...protectedTasks.map(([name, target], index) => ({ name, id: `gate-${index}`, kind: 'critical', parallelism: target.parallelism })),
            ...['light-a', 'light-b'].map(id => ({ name: id, id, kind: 'light', parallelism: true }))];
        for (const probe of probes) {
            const directory = resolve(root, 'projects', probe.id); mkdirSync(directory, { recursive: true });
            writeFileSync(resolve(directory, 'project.json'), JSON.stringify({ name: probe.id, projectType: 'library', targets: {
                probe: { executor: 'nx:run-commands', cache: false, parallelism: probe.parallelism,
                    options: { command: `node probe.mjs ${probe.id} ${probe.kind}`, cwd: '{workspaceRoot}' } },
            } }));
        }
        const run = spawnSync(process.execPath, [nx, 'run-many', '--target=probe', '--projects=*', '--parallel=4', '--skip-nx-cache'], {
            cwd: root, env: { ...env, NX_CACHE_DIRECTORY: resolve(root, 'cache'), NX_WORKSPACE_DATA_DIRECTORY: resolve(root, 'workspace-data') },
            encoding: 'utf8', maxBuffer: 16 * 1024 * 1024,
        });
        assert.equal(run.status, 0, `${run.stdout}\n${run.stderr}`);
        const events = readFileSync(resolve(root, 'events.jsonl'), 'utf8').trim().split('\n').map(line => JSON.parse(line));
        const interval = id => ({ start: events.find(event => event.id === id && event.phase === 'start')?.time,
            end: events.find(event => event.id === id && event.phase === 'end')?.time });
        const overlap = (a, b) => a.start < b.end && b.start < a.end;
        for (const probe of probes) {
            const own = interval(probe.id); assert.ok(Number.isFinite(own.start) && Number.isFinite(own.end));
            if (probe.kind === 'critical') for (const peer of probes.filter(peer => peer !== probe))
                assert.equal(overlap(own, interval(peer.id)), false, `${probe.name} overlaps ${peer.name}`);
        }
        assert.ok(overlap(interval('light-a'), interval('light-b')), 'unrelated build work must remain concurrent');
        console.log(`Verified ${protectedTasks.length} protected Studio workloads at Nx parallelism four; unrelated build probes overlap.`);
    } finally { rmSync(root, { recursive: true, force: true }); }
}
