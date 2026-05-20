#!/usr/bin/env node
/**
 * Verify the SHA-256 of a downloaded archive against an expected
 * checksum. Used by postinstall.mjs and by the release workflow's
 * smoke check.
 *
 *   node scripts/verify-checksum.mjs <file> <expected-hex>
 */
import { createReadStream } from 'node:fs';
import { createHash } from 'node:crypto';
import { pipeline } from 'node:stream/promises';

const [, , filePath, expected] = process.argv;
if (!filePath || !expected) {
  console.error('usage: node scripts/verify-checksum.mjs <file> <expected-hex>');
  process.exit(2);
}

const hash = createHash('sha256');
await pipeline(createReadStream(filePath), hash);
const actual = hash.digest('hex');

if (actual.toLowerCase() !== expected.toLowerCase()) {
  console.error(`[trang-native] checksum mismatch for ${filePath}`);
  console.error(`[trang-native]   expected: ${expected}`);
  console.error(`[trang-native]   actual:   ${actual}`);
  process.exit(1);
}
console.log(`[trang-native] checksum ok: ${actual}  ${filePath}`);
