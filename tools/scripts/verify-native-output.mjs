import { readFileSync } from 'node:fs';

const [expectedPath, actualPath] = process.argv.slice(2);

if (!expectedPath || !actualPath) {
  console.error('usage: node tools/scripts/verify-native-output.mjs EXPECTED_JSON ACTUAL_FILE');
  process.exit(2);
}

const expected = JSON.parse(readFileSync(expectedPath, 'utf8'));
const actual = readFileSync(actualPath);

if (typeof expected.content !== 'string') {
  console.error(`${expectedPath}: expected JSON field \`content\` must be a string`);
  process.exit(2);
}

const expectedBytes = Buffer.from(expected.content, 'utf8');

if (Buffer.compare(actual, expectedBytes) !== 0) {
  console.error(`native output mismatch: ${actualPath}`);
  console.error(`expected (${expectedBytes.length} bytes):`);
  console.error(expected.content);
  console.error(`actual (${actual.length} bytes):`);
  console.error(actual.toString('utf8'));
  process.exit(1);
}
