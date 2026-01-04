const fs = require('fs');
const path = require('path');

const rootPath = path.resolve(__dirname, '../../package.json');
const root = JSON.parse(fs.readFileSync(rootPath, 'utf8'));
const version = root.version;

const packagePaths = [
  path.resolve(__dirname, '../../packages/cem-components/package.json'),
  path.resolve(__dirname, '../../packages/cem-theme/package.json'),
];

for (const pkgPath of packagePaths) {
  const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
  if (pkg.version !== version) {
    pkg.version = version;
    fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 4) + '\n');
  }
}
