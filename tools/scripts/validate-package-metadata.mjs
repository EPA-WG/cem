#!/usr/bin/env node

import { readdir, readFile } from 'fs/promises';
import { join } from 'path';

const EXPECTED_AUTHOR_NAME = 'Sasha Firsov';
const EXPECTED_AUTHOR_EMAIL = 'sasha@firsov.net';

async function fileExists(path) {
  try {
    await readFile(path, 'utf8');
    return true;
  } catch {
    return false;
  }
}

async function packageJsonPaths() {
  const paths = ['package.json'];
  const packageDirs = await readdir('packages', { withFileTypes: true });

  for (const dirent of packageDirs) {
    if (!dirent.isDirectory()) {
      continue;
    }

    const packagePath = join('packages', dirent.name, 'package.json');
    if (await fileExists(packagePath)) {
      paths.push(packagePath);
    }
  }

  return paths.sort();
}

function validatePublishedPackage(path, manifest) {
  const errors = [];

  if (manifest.private === true) {
    return errors;
  }

  if (!manifest.name) {
    errors.push('missing name');
  }

  if (!manifest.license) {
    errors.push('missing npm license field');
  }

  if (!manifest.author || typeof manifest.author !== 'object') {
    errors.push('missing author object');
    return errors;
  }

  if (manifest.author.name !== EXPECTED_AUTHOR_NAME) {
    errors.push(`author.name must be "${EXPECTED_AUTHOR_NAME}"`);
  }

  if (manifest.author.email !== EXPECTED_AUTHOR_EMAIL) {
    errors.push(`author.email must be "${EXPECTED_AUTHOR_EMAIL}"`);
  }

  return errors;
}

async function main() {
  const paths = await packageJsonPaths();
  const failures = [];

  for (const path of paths) {
    const manifest = JSON.parse(await readFile(path, 'utf8'));
    const errors = validatePublishedPackage(path, manifest);

    if (errors.length > 0) {
      failures.push({ path, errors });
    }
  }

  if (failures.length > 0) {
    console.error('Published npm package metadata validation failed:');
    for (const failure of failures) {
      console.error(`- ${failure.path}`);
      for (const error of failure.errors) {
        console.error(`  - ${error}`);
      }
    }
    process.exit(1);
  }

  console.log(`Published npm package metadata validated for ${paths.length} package.json files.`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
