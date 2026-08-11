import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const sourceStylesPath = join(packageRoot, 'src', 'styles.css');
const builtStylesPath = join(packageRoot, 'dist', 'styles.css');
const packageJsonPath = join(packageRoot, 'package.json');
const sourcePrimitivesPath = join(packageRoot, 'src', 'lib', 'primitives.ts');
const builtPrimitivesPath = join(packageRoot, 'dist', 'lib', 'primitives.js');
const builtAutocompleteBehaviorPath = join(packageRoot, 'dist', 'lib', 'autocomplete-behavior.js');
const sourceEntries = [join(packageRoot, 'src', 'index.ts'), join(packageRoot, 'src', 'lib', 'cem-components.ts')];
const builtEntries = [join(packageRoot, 'dist', 'index.js'), join(packageRoot, 'dist', 'lib', 'cem-components.js')];
const forbiddenJavaScriptPatterns = [
    { label: 'static CSS import', pattern: /(?:import|export)\s+(?:[^'";]+?\s+from\s+)?['"][^'"]+\.css['"]/ },
    { label: 'dynamic CSS import', pattern: /import\s*\(\s*['"][^'"]+\.css['"]\s*\)/ },
    { label: 'adopted stylesheet', pattern: /adoptedStyleSheets|new\s+CSSStyleSheet\b/ },
    { label: 'runtime style element', pattern: /createElement\s*\(\s*['"](?:link|style)['"]\s*\)/ },
];

const sourceStyles = await readFile(sourceStylesPath);
const builtStyles = await readFile(builtStylesPath);
const sourcePrimitives = await readFile(sourcePrimitivesPath, 'utf8');
const builtPrimitives = await readFile(builtPrimitivesPath, 'utf8');

if (!sourceStyles.equals(builtStyles)) {
    throw new Error('src/styles.css and dist/styles.css must be byte-identical');
}

if (!sourcePrimitives.includes("tag: 'cem-autocomplete'")) {
    throw new Error('source primitive inventory must contain cem-autocomplete');
}

if (!builtPrimitives.includes("tag: 'cem-autocomplete'")) {
    throw new Error('built primitive inventory must contain cem-autocomplete');
}

if (!existsSync(builtAutocompleteBehaviorPath)) {
    throw new Error('built package must contain the autocomplete behavior artifact');
}

if (existsSync(join(packageRoot, 'styles.css'))) {
    throw new Error('package-root styles.css is forbidden; publish only dist/styles.css');
}

const packageJson = JSON.parse(await readFile(packageJsonPath, 'utf8'));

if (packageJson.exports?.['./styles.css'] !== './dist/styles.css') {
    throw new Error('package.json must export ./styles.css only from ./dist/styles.css');
}

const cssExports = Object.entries(packageJson.exports ?? {}).filter(
    ([key, value]) => key.endsWith('.css') || (typeof value === 'string' && value.endsWith('.css')),
);

if (cssExports.length !== 1 || cssExports[0]?.[0] !== './styles.css') {
    throw new Error('package.json must expose exactly one CSS subpath: ./styles.css');
}

if (!packageJson.files?.includes('dist')) {
    throw new Error('package.json files must include dist');
}

if (packageJson.files.some((entry) => entry === 'src' || entry === 'styles.css' || entry.startsWith('src/'))) {
    throw new Error('package.json files must not publish source or a package-root stylesheet');
}

for (const entryPath of [...sourceEntries, ...builtEntries]) {
    const source = await readFile(entryPath, 'utf8');

    for (const { label, pattern } of forbiddenJavaScriptPatterns) {
        if (pattern.test(source)) {
            throw new Error(`${entryPath} contains forbidden ${label} behavior`);
        }
    }
}

const npmCache = await mkdtemp(join(tmpdir(), 'cem-components-npm-pack-'));

try {
    const packOutput = execFileSync('npm', ['pack', '--dry-run', '--json', '--cache', npmCache], {
        cwd: packageRoot,
        encoding: 'utf8',
        env: { ...process.env, npm_config_update_notifier: 'false' },
    });
    const packResults = JSON.parse(packOutput);
    const packedFiles = packResults[0]?.files?.map(({ path }) => path) ?? [];
    const packedStyles = packedFiles.filter((path) => path.endsWith('styles.css'));

    if (packedStyles.length !== 1 || packedStyles[0] !== 'dist/styles.css') {
        throw new Error(`npm pack must contain only dist/styles.css, received: ${packedStyles.join(', ') || 'none'}`);
    }

    if (packedFiles.includes('src/styles.css') || packedFiles.includes('styles.css')) {
        throw new Error('npm pack must not contain source or package-root stylesheet copies');
    }

    for (const artifact of ['dist/lib/autocomplete-behavior.js', 'dist/lib/primitives.js']) {
        if (!packedFiles.includes(artifact)) {
            throw new Error(`npm pack must contain the autocomplete runtime artifact ${artifact}`);
        }
    }

    const buildInfoFiles = packedFiles.filter((path) => path.endsWith('.tsbuildinfo'));

    if (buildInfoFiles.length > 0) {
        throw new Error(`npm pack must exclude tsbuildinfo files: ${buildInfoFiles.join(', ')}`);
    }

    console.log(
        `cem-components package verified (${packedFiles.length} packed files, autocomplete runtime included, ` +
            'one dist/styles.css, zero source/root copies).',
    );
} finally {
    await rm(npmCache, { force: true, recursive: true });
}
