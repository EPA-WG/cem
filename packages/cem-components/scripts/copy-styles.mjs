import { copyFile, mkdir, stat } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const sourcePath = join(packageRoot, 'src', 'styles.css');
const outputPath = join(packageRoot, 'dist', 'styles.css');

const sourceStat = await stat(sourcePath);

if (!sourceStat.isFile()) {
    throw new Error(`${sourcePath} must be a regular file`);
}

await mkdir(dirname(outputPath), { recursive: true });
await copyFile(sourcePath, outputPath);

console.log('cem-components stylesheet copied to dist/styles.css.');
