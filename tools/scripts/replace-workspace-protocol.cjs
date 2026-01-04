#!/usr/bin/env node

/**
 * Replace workspace:* protocol with actual versions in package.json files
 * This is needed because Yarn doesn't support dynamic replacement during publish
 */

const fs = require('fs');
const path = require('path');

const packagesDir = path.join(__dirname, '../../packages');
const packages = fs.readdirSync(packagesDir);

// Build a map of package names to versions
const versionMap = new Map();

packages.forEach(pkg => {
  const pkgJsonPath = path.join(packagesDir, pkg, 'package.json');
  if (fs.existsSync(pkgJsonPath)) {
    const pkgJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));
    versionMap.set(pkgJson.name, pkgJson.version);
  }
});

// Replace workspace:* in all packages
packages.forEach(pkg => {
  const pkgJsonPath = path.join(packagesDir, pkg, 'package.json');
  if (fs.existsSync(pkgJsonPath)) {
    const pkgJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));
    let modified = false;

    ['dependencies', 'devDependencies', 'peerDependencies'].forEach(depType => {
      if (pkgJson[depType]) {
        Object.keys(pkgJson[depType]).forEach(depName => {
          if (pkgJson[depType][depName].startsWith('workspace:')) {
            const version = versionMap.get(depName);
            if (version) {
              // Use ^ prefix for semantic versioning
              pkgJson[depType][depName] = `^${version}`;
              modified = true;
              console.log(`✓ ${pkg}: ${depName} workspace:* → ^${version}`);
            }
          }
        });
      }
    });

    if (modified) {
      fs.writeFileSync(pkgJsonPath, JSON.stringify(pkgJson, null, 4) + '\n');
    }
  }
});

console.log('✓ Workspace protocol replacement complete');
