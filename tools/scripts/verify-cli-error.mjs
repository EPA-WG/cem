import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, rmSync } from 'node:fs';
import { resolve } from 'node:path';

const separatorIndex = process.argv.indexOf('--', 2);
const expectedPath = process.argv[2];

if (!expectedPath || separatorIndex === -1 || separatorIndex === process.argv.length - 1) {
  console.error('usage: node tools/scripts/verify-cli-error.mjs EXPECTED_JSON -- COMMAND [ARGS...]');
  process.exit(2);
}

const expected = JSON.parse(readFileSync(expectedPath, 'utf8'));
const [command, ...commandArgs] = process.argv.slice(separatorIndex + 1);

if (!Array.isArray(expected.args)) {
  console.error(`${expectedPath}: expected JSON field \`args\` must be an array`);
  process.exit(2);
}

for (const filePath of expected.cleanPaths ?? []) {
  rmSync(resolve(filePath), { force: true });
}

const result = spawnSync(command, [...commandArgs, ...expected.args], {
  cwd: process.cwd(),
  encoding: 'utf8',
});

if (result.error) {
  console.error(`${expectedPath}: failed to run ${command}: ${result.error.message}`);
  process.exit(1);
}

const stdout = result.stdout ?? '';
const stderr = result.stderr ?? '';
const exitCode = result.status ?? 1;

if (exitCode !== expected.exitCode) {
  console.error(`${expectedPath}: expected exit code ${expected.exitCode}, got ${exitCode}`);
  console.error('stdout:');
  console.error(stdout);
  console.error('stderr:');
  console.error(stderr);
  process.exit(1);
}

if (expected.stdout !== undefined && stdout !== expected.stdout) {
  console.error(`${expectedPath}: stdout mismatch`);
  console.error('expected:');
  console.error(expected.stdout);
  console.error('actual:');
  console.error(stdout);
  process.exit(1);
}

for (const needle of expected.stderrIncludes ?? []) {
  if (!stderr.includes(needle)) {
    console.error(`${expectedPath}: stderr did not include ${JSON.stringify(needle)}`);
    console.error(stderr);
    process.exit(1);
  }
}

for (const needle of expected.stderrExcludes ?? []) {
  if (stderr.includes(needle)) {
    console.error(`${expectedPath}: stderr unexpectedly included ${JSON.stringify(needle)}`);
    console.error(stderr);
    process.exit(1);
  }
}

for (const filePath of expected.absentPaths ?? []) {
  if (existsSync(resolve(filePath))) {
    console.error(`${expectedPath}: expected ${filePath} to be absent`);
    process.exit(1);
  }
}
