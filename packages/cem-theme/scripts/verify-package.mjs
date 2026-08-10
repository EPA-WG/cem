import { execFileSync } from 'node:child_process';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const packageJsonPath = join(packageRoot, 'package.json');
const builtStylesPath = join(packageRoot, 'dist', 'lib', 'css', 'cem-combined.css');
const publicStylesPath = './dist/lib/css/cem-combined.css';
const packageJson = JSON.parse(await readFile(packageJsonPath, 'utf8'));
const builtStyles = await stat(builtStylesPath);

if (!builtStyles.isFile() || builtStyles.size === 0) {
    throw new Error('dist/lib/css/cem-combined.css must be a non-empty generated file');
}

if (packageJson.exports?.['./styles.css'] !== publicStylesPath) {
    throw new Error(`package.json must export ./styles.css only from ${publicStylesPath}`);
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

const npmCache = await mkdtemp(join(tmpdir(), 'cem-theme-npm-pack-'));

try {
    const packOutput = execFileSync('npm', ['pack', '--dry-run', '--json', '--cache', npmCache], {
        cwd: packageRoot,
        encoding: 'utf8',
        env: { ...process.env, npm_config_update_notifier: 'false' },
    });
    const packResults = JSON.parse(packOutput);
    const packedFiles = packResults[0]?.files?.map(({ path }) => path) ?? [];
    const packedStylesPath = publicStylesPath.slice(2);

    if (!packedFiles.includes(packedStylesPath)) {
        throw new Error(`npm pack must contain ${packedStylesPath}`);
    }

    const buildInfoFiles = packedFiles.filter((path) => path.endsWith('.tsbuildinfo'));
    if (buildInfoFiles.length > 0) {
        throw new Error(`npm pack must exclude tsbuildinfo files: ${buildInfoFiles.join(', ')}`);
    }

    console.log(
        `cem-theme package verified (${packedFiles.length} packed files, public ./styles.css -> ${packedStylesPath}).`,
    );
} finally {
    await rm(npmCache, { force: true, recursive: true });
}
