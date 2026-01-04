#!/usr/bin/env node

/**
 * Restore workspace:* protocol for local dependencies
 * This is needed before running nx release
 */

const fs = require('fs');
const path = require('path');

const packagesDir = path.join(__dirname, '../../packages');
const packages = fs.readdirSync(packagesDir);

// Build a map of package names
const packageNames = new Set();

packages.forEach(pkg => {
  const pkgJsonPath = path.join(packagesDir, pkg, 'package.json');
  if (fs.existsSync(pkgJsonPath)) {
    const pkgJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));
    packageNames.add(pkgJson.name);
  }
});

// Restore workspace:* in all packages
packages.forEach(pkg => {
  const pkgJsonPath = path.join(packagesDir, pkg, 'package.json');
  if (fs.existsSync(pkgJsonPath)) {
    const pkgJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));
    let modified = false;

    ['dependencies', 'devDependencies', 'peerDependencies'].forEach(depType => {
      if (pkgJson[depType]) {
        Object.keys(pkgJson[depType]).forEach(depName => {
          if (packageNames.has(depName) && !pkgJson[depType][depName].startsWith('workspace:')) {
            pkgJson[depType][depName] = 'workspace:*';
            modified = true;
            console.log(`✓ ${pkg}: ${depName} → workspace:*`);
          }
        });
      }
    });

    if (modified) {
      fs.writeFileSync(pkgJsonPath, JSON.stringify(pkgJson, null, 4) + '\n');
    }
  }
});

console.log('✓ Workspace protocol restoration complete');
