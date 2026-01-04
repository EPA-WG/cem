#!/usr/bin/env node

import { readFile } from 'fs/promises';
import { glob } from 'glob';
import { relative } from 'path';

const VALID_CONTROL_CHARS = new Set([
  0x09, // Tab
  0x0A, // Line Feed (LF)
  0x0D  // Carriage Return (CR)
]);

/**
 * Validates that a file is properly UTF-8 encoded without invalid control characters
 */
async function validateFile(filePath) {
  try {
    const buffer = await readFile(filePath);
    const errors = [];

    // Try to decode as UTF-8 and check for replacement characters
    const content = buffer.toString('utf-8');

    // Check if decoding introduced replacement characters (U+FFFD)
    // This indicates invalid UTF-8 sequences
    if (content.includes('\uFFFD')) {
      errors.push({
        message: 'File contains invalid UTF-8 byte sequences (replacement character detected)'
      });
    }

    // Verify the content can be re-encoded to UTF-8 without loss
    const reencoded = Buffer.from(content, 'utf-8');
    if (!buffer.equals(reencoded)) {
      errors.push({
        message: 'File content changes when re-encoded as UTF-8'
      });
    }

    // Check for ASCII control characters (0x00-0x1F except tab, LF, CR)
    // Only check these in the decoded string to avoid false positives with UTF-8 sequences
    for (let i = 0; i < content.length; i++) {
      const charCode = content.charCodeAt(i);

      // Check for null bytes and other control characters
      if (charCode < 0x20 && !VALID_CONTROL_CHARS.has(charCode)) {
        errors.push({
          position: i,
          char: `U+${charCode.toString(16).padStart(4, '0')}`,
          message: `Invalid control character at position ${i}`
        });

        // Only report first 10 control character errors
        if (errors.filter(e => e.char).length >= 10) {
          break;
        }
      }
    }

    return { valid: errors.length === 0, errors };
  } catch (error) {
    return {
      valid: false,
      errors: [{
        message: `Failed to read file: ${error.message}`
      }]
    };
  }
}

async function main() {
  const patterns = [
    'packages/**/*.md',
    'docs/**/*.md',
    '*.md',
    'tools/**/*.{js,mjs,cjs,ts}',
    '.github/**/*.{yml,yaml}'
  ];

  console.log('🔍 Validating UTF-8 encoding...\n');

  let allValid = true;
  let checkedCount = 0;
  let errorCount = 0;

  for (const pattern of patterns) {
    const files = await glob(pattern, {
      ignore: ['**/node_modules/**', '**/dist/**', '**/.nx/**'],
      absolute: false
    });

    for (const file of files) {
      checkedCount++;
      const result = await validateFile(file);

      if (!result.valid) {
        allValid = false;
        errorCount++;
        console.error(`❌ ${relative(process.cwd(), file)}`);
        for (const error of result.errors.slice(0, 5)) { // Show first 5 errors
          if (error.char) {
            console.error(`   ${error.char} - ${error.message}`);
          } else {
            console.error(`   ${error.message}`);
          }
        }
        if (result.errors.length > 5) {
          console.error(`   ... and ${result.errors.length - 5} more errors`);
        }
        console.error();
      }
    }
  }

  console.log(`\n📊 Checked ${checkedCount} files`);

  if (allValid) {
    console.log('✅ All files are properly UTF-8 encoded\n');
    process.exit(0);
  } else {
    console.error(`❌ Found ${errorCount} files with encoding issues\n`);
    console.error('To fix encoding issues:');
    console.error('  1. Open the file in an editor that supports UTF-8');
    console.error('  2. Look for special characters (box drawing, arrows, etc.)');
    console.error('  3. Re-save the file with UTF-8 encoding');
    console.error('  4. Or run: iconv -f ISO-8859-1 -t UTF-8 file.md > file.md.utf8 && mv file.md.utf8 file.md\n');
    process.exit(1);
  }
}

main().catch(error => {
  console.error('Fatal error:', error);
  process.exit(1);
});
