import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdtemp, readFile, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), '../../packages/cem-components');
const sourceStylesPath = join(packageRoot, 'src', 'styles.css');
const builtStylesPath = join(packageRoot, 'dist', 'styles.css');
const builtCatalogPath = join(packageRoot, 'dist', 'catalog', 'cem.components.catalog.json');
const publicCatalogPath = './dist/catalog/cem.components.catalog.json';
const publicComponentExport = './src/components/*/*.xhtml';
const canonicalTags = (await readdir(join(packageRoot, 'src/components'))).filter(tag => existsSync(join(packageRoot, 'src/components', tag, tag + '.xhtml')));
const packedDeclarations = canonicalTags.map(tag => `src/components/${tag}/${tag}.xhtml`);
const packageJsonPath = join(packageRoot, 'package.json');
const sourceStyles = await readFile(sourceStylesPath);
const builtStyles = await readFile(builtStylesPath);
const componentCatalog = JSON.parse(await readFile(builtCatalogPath, 'utf8'));

if (!sourceStyles.equals(builtStyles)) {
    throw new Error('src/styles.css and dist/styles.css must be byte-identical');
}

if (
    componentCatalog.version !== 1 ||
    !Array.isArray(componentCatalog.components) ||
    componentCatalog.components.length === 0 ||
    componentCatalog.$generated?.componentCount !== componentCatalog.components.length
) {
    throw new Error('dist/catalog/cem.components.catalog.json must contain the public component catalog');
}

const suggestions = componentCatalog.components.filter(component => component.tag === 'cem-suggestions');
if (suggestions.length !== 1 || suggestions[0].id !== 'suggestions' || suggestions[0].category !== 'Input'
    || suggestions[0].implementation?.kind !== 'cem-element-xhtml'
    || suggestions[0].implementation?.source !== 'packages/cem-components/src/components/cem-suggestions/cem-suggestions.xhtml') {
    throw new Error('public catalog must identify cem-suggestions as one declarative field attachment');
}

for (const path of ['src/index.ts', 'src/lib', 'dist/index.js', 'dist/index.d.ts', 'dist/lib']) {
    if (existsSync(join(packageRoot, path))) throw new Error(`Retired component runtime must not exist: ${path}`);
}
if (existsSync(join(packageRoot, 'styles.css'))) {
    throw new Error('package-root styles.css is forbidden; publish only dist/styles.css');
}

const packageJson = JSON.parse(await readFile(packageJsonPath, 'utf8'));

if (packageJson.exports?.['./styles.css'] !== './dist/styles.css') {
    throw new Error('package.json must export ./styles.css only from ./dist/styles.css');
}

if (packageJson.exports?.['./catalog/cem.components.catalog.json'] !== publicCatalogPath) {
    throw new Error(
        `package.json must export ./catalog/cem.components.catalog.json from ${publicCatalogPath}`,
    );
}

if (packageJson.exports?.['./components.xhtml'] !== './dist/components.xhtml') {
    throw new Error('package.json must export the generated component bundle');
}

if (packageJson.exports?.['./components/*'] !== publicComponentExport) {
    throw new Error(`package.json must export declarative component XHTML from ${publicComponentExport}`);
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

if (!packageJson.files?.includes('src/components/**/*.xhtml')) {
    throw new Error('package.json files must publish declarative component XHTML');
}

if (packageJson.files.some((entry) => entry === 'src' || entry === 'styles.css' || (entry.startsWith('src/') && entry !== 'src/components/**/*.xhtml'))) {
    throw new Error('package.json files may publish only declarative XHTML beneath src');
}

for (const path of ['playgrounds/index.html', 'dist/index.html']) {
    const index = await readFile(join(packageRoot, path), 'utf8');
    for (const tag of canonicalTags) {
        for (const suffix of ['', '-gallery']) {
            if (!index.includes(`href="./${tag}${suffix}.html"`)) throw new Error(`${path} must link ${tag}${suffix}.html`);
        }
    }
}
const componentIndex = await readFile(join(packageRoot, 'index.html'), 'utf8');
if (packageJson.exports?.['./playgrounds/cem-gallery-page.xhtml'] !== './dist/cem-gallery-page.xhtml') {
    throw new Error('The shared gallery page must export its built declaration');
}
for (const tag of canonicalTags) {
    if (!componentIndex.includes(`href="./playgrounds/${tag}-gallery.html">${tag}</a>`)) throw new Error(`Component index must link ${tag}`);
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
    const packedCatalogPath = publicCatalogPath.slice(2);

    if (packedStyles.length !== 1 || packedStyles[0] !== 'dist/styles.css') {
        throw new Error(`npm pack must contain only dist/styles.css, received: ${packedStyles.join(', ') || 'none'}`);
    }

    if (packedFiles.includes('src/styles.css') || packedFiles.includes('styles.css')) {
        throw new Error('npm pack must not contain source or package-root stylesheet copies');
    }

    if (!packedFiles.includes(packedCatalogPath)) {
        throw new Error(`npm pack must contain the public component catalog ${packedCatalogPath}`);
    }

    for (const page of ['cem-bundle.html', ...canonicalTags.flatMap(tag => [`${tag}.html`, `${tag}-gallery.html`])]) {
        if (packageJson.exports?.[`./playgrounds/${page}`] !== `./dist/${page}` || !packedFiles.includes(`dist/${page}`)) {
            throw new Error(`npm pack must expose the built playground ${page}`);
        }
    }
    for (const source of ['dist/cem-gallery-page.xhtml', 'playgrounds/cem-gallery-page.xhtml', 'demo.css', 'playgrounds/cem-icon-button.html', 'playgrounds/cem-menu-item.html', 'dist/components.xhtml', 'build/components.cemt', 'build/components.transform.cem', 'playgrounds/cem-bundle.html', 'playgrounds/bundle.importmap.source.json', 'playgrounds/bundle.importmap.package.json', 'playgrounds/cem-action-gallery.html', 'playgrounds/cem-select.html', 'playgrounds/cem-theme-switch.html', 'playgrounds/cem-action.html', 'playgrounds/importmap.source.json', 'playgrounds/importmap.package.json', 'playgrounds/importmap.transform.cem', 'dist/icon.html']) {
        if (!packedFiles.includes(source)) throw new Error(`npm pack is missing playground source or asset ${source}`);
    }

    for (const tag of canonicalTags) {
        for (const file of [`${tag}.html`, `${tag}-gallery.html`]) {
            if (!packedFiles.includes(`playgrounds/${file}`)) throw new Error(`Missing gallery/playground source: ${file}`);
        }
    }
    for (const declaration of packedDeclarations) {
        if (!packedFiles.includes(declaration)) {
            throw new Error(`npm pack must contain the public declarative component ${declaration}`);
        }
    }

    if (packedFiles.some((path) => path.startsWith('src/') && !path.endsWith('.xhtml'))) {
        throw new Error('npm pack must not contain component-owned JavaScript, TypeScript, or CSS beneath src');
    }

    const authoredCode = packedFiles.filter(path => /\.(?:[cm]?[jt]sx?)$/.test(path));
    if (authoredCode.length) throw new Error(`Declarative package must ship no JavaScript/TypeScript: ${authoredCode.join(', ')}`);
    if (packageJson.main || packageJson.module || packageJson.types || packageJson.exports?.['.']) {
        throw new Error('Declarative components expose XHTML assets; legacy JavaScript entry points must be retired');
    }
    const buildInfoFiles = packedFiles.filter((path) => path.endsWith('.tsbuildinfo'));

    if (buildInfoFiles.length > 0) {
        throw new Error(`npm pack must exclude tsbuildinfo files: ${buildInfoFiles.join(', ')}`);
    }

    console.log(
        `cem-components package verified (${packedFiles.length} packed files, autocomplete, divider, expansion, progress-spinner, sort-header, paginator, slider, tooltip, timepicker, datepicker, tabs, stepper, and tree owners included, ` +
            'public component catalog, one dist/styles.css, zero source/root copies).',
    );
} finally {
    await rm(npmCache, { force: true, recursive: true });
}
