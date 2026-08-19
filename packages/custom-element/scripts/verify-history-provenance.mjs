import { execFileSync, spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptRoot = dirname(fileURLToPath(import.meta.url));
const projectRoot = dirname(scriptRoot);
const workspaceRoot = resolve(projectRoot, '../..');
const manifest = JSON.parse(
    await readFile(resolve(projectRoot, 'test-fixtures/history-provenance.json'), 'utf8')
);

assertEqual(manifest.schemaVersion, 1, 'manifest schema version');

const head = git(['rev-parse', 'HEAD']);
const join = manifest.join;
const rewritten = manifest.source.rewritten;

assertEqual(git(['cat-file', '-t', join.commit]), 'commit', 'join object type');
assertEqual(git(['show', '-s', '--format=%s', join.commit]), join.subject, 'join subject');
assertArrayEqual(
    git(['show', '-s', '--format=%P', join.commit]).split(/\s+/u),
    join.parents,
    'join parents'
);
assertEqual(git(['rev-parse', `${join.commit}^{tree}`]), join.tree, 'join tree');
assertEqual(git(['rev-parse', `${join.parents[0]}^{tree}`]), join.tree, 'first-parent tree');
assertAncestor(join.commit, head, 'join must be reachable from HEAD');
assertAncestor(rewritten.main, join.commit, 'rewritten main must be a join ancestor');
assertAncestor(rewritten.develop, join.commit, 'rewritten develop must be a join ancestor');

const roots = lines(git(['rev-list', '--max-parents=0', rewritten.main, rewritten.develop]));
assertArrayEqual(roots, [rewritten.root], 'rewritten roots');

const commits = new Set(lines(git(['rev-list', rewritten.main, rewritten.develop])));
assertEqual(commits.size, manifest.source.commitCount, 'rewritten reachable commit count');

for (const [label, tip] of Object.entries({ main: rewritten.main, develop: rewritten.develop })) {
    const paths = lines(git(['ls-tree', '-r', '--name-only', tip]));
    assert(paths.length > 0, `${label} tip must contain files`);
    const invalid = paths.filter((path) => !path.startsWith(manifest.source.pathPrefix));
    assertEqual(invalid.length, 0, `${label} tip paths outside ${manifest.source.pathPrefix}`);
}

verifyTags(manifest.tipTags, 'tip');
verifyTags(manifest.versionTags, 'version');

for (const [label, commit] of Object.entries(manifest.excludedDistributionHistory)) {
    if (label === 'repository' || !objectExists(commit)) {
        continue;
    }
    assertNotAncestor(commit, head, `excluded distribution ${label}`);
}

console.log(
    `Verified custom-element history provenance: ${commits.size} commits, ` +
        `${Object.keys(manifest.versionTags).length} version tags, 2 source-tip tags, ` +
        `join ${join.commit.slice(0, 12)}.`
);

function verifyTags(expected, kind) {
    const prefix = kind === 'version' ? 'custom-element-v' : 'custom-element-history-';
    const actualNames = lines(git(['tag', '--list', `${prefix}*`, '--sort=refname']));
    const expectedNames = Object.keys(expected).sort();
    assertArrayEqual(actualNames, expectedNames, `${kind} tag names`);
    for (const [name, commit] of Object.entries(expected)) {
        assertEqual(git(['rev-parse', `refs/tags/${name}^{commit}`]), commit, `${name} target`);
    }
}

function git(args) {
    return execFileSync('git', args, {
        cwd: workspaceRoot,
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
    }).trim();
}

function objectExists(commit) {
    return spawnSync('git', ['cat-file', '-e', `${commit}^{commit}`], {
        cwd: workspaceRoot,
        stdio: 'ignore',
    }).status === 0;
}

function isAncestor(ancestor, descendant) {
    return spawnSync('git', ['merge-base', '--is-ancestor', ancestor, descendant], {
        cwd: workspaceRoot,
        stdio: 'ignore',
    }).status === 0;
}

function assertAncestor(ancestor, descendant, label) {
    assert(isAncestor(ancestor, descendant), `${label}: expected ${ancestor} to be an ancestor of ${descendant}`);
}

function assertNotAncestor(ancestor, descendant, label) {
    assert(!isAncestor(ancestor, descendant), `${label}: ${ancestor} must not be an ancestor of ${descendant}`);
}

function lines(value) {
    return value ? value.split(/\r?\n/u) : [];
}

function assertArrayEqual(actual, expected, label) {
    assertEqual(JSON.stringify(actual), JSON.stringify(expected), label);
}

function assertEqual(actual, expected, label) {
    assert(actual === expected, `${label}: expected ${expected}, got ${actual}`);
}

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}
