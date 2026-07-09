import { readFileSync } from 'node:fs';

const [expectedPath, actualPath] = process.argv.slice(2);

if (!expectedPath || !actualPath) {
  console.error('usage: node tools/scripts/verify-json-output.mjs EXPECTED_JSON ACTUAL_JSON');
  process.exit(2);
}

const expected = JSON.parse(readFileSync(expectedPath, 'utf8'));
const actual = JSON.parse(readFileSync(actualPath, 'utf8'));

const expectedJson = JSON.stringify(expected, null, 2);
const actualJson = JSON.stringify(actual, null, 2);

if (expectedJson !== actualJson) {
  console.error(`JSON output mismatch: ${actualPath}`);
  console.error('expected:');
  console.error(expectedJson);
  console.error('actual:');
  console.error(actualJson);
  process.exit(1);
}
