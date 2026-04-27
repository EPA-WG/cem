#!/usr/bin/env node

import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { glob } from 'glob';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../..');
const projectRoot = path.join(repoRoot, 'packages/cem-theme');
const srcDir = path.join(projectRoot, 'src');
const distDir = path.join(projectRoot, 'dist');
const repoNodeModulesDir = path.join(repoRoot, 'node_modules');
const vendorDir = path.join(distDir, 'vendor');

const urlAttributePattern = /\b(src|href|xlink:href|url)\s*=\s*(["'])(.*?)\2/gis;
const srcsetAttributePattern = /\b(srcset)\s*=\s*(["'])(.*?)\2/gis;
const cssUrlPattern = /url\(\s*(["']?)([^"')]+)\1\s*\)/gis;
const cssImportPattern = /@import\s+(["'])([^"']+)\1/gis;

function splitUrlSuffix(url) {
  const suffixStart = url.search(/[?#]/);
  if (suffixStart === -1) {
    return { pathname: url, suffix: '' };
  }
  return {
    pathname: url.slice(0, suffixStart),
    suffix: url.slice(suffixStart),
  };
}

function isExternalUrl(url) {
  return /^(?:[a-z][a-z0-9+.-]*:|\/\/|#)/i.test(url);
}

function toPosixPath(filePath) {
  return filePath.split(path.sep).join('/');
}

function relativeUrl(fromDir, toPath) {
  let rel = toPosixPath(path.relative(fromDir, toPath));
  if (!rel.startsWith('.')) {
    rel = `./${rel}`;
  }
  return rel;
}

function resolveUrlPath(urlPath, sourceDir) {
  if (urlPath.startsWith('/node_modules/')) {
    return path.join(repoRoot, urlPath.slice(1));
  }
  if (urlPath.startsWith('node_modules/')) {
    return path.join(repoRoot, urlPath);
  }
  if (urlPath.startsWith('/')) {
    return path.join(repoRoot, urlPath.slice(1));
  }
  return path.resolve(sourceDir, urlPath);
}

function distPathForSourceFile(filePath) {
  if (!filePath.startsWith(srcDir + path.sep)) {
    return null;
  }
  return path.join(distDir, path.relative(srcDir, filePath));
}

async function copyOnce(copiedFiles, sourcePath, outputPath) {
  const key = outputPath;
  if (copiedFiles.has(key)) {
    return;
  }
  await mkdir(path.dirname(outputPath), { recursive: true });
  await copyFile(sourcePath, outputPath);
  copiedFiles.add(key);
}

async function rewriteUrl(url, context) {
  const trimmed = url.trim();
  if (!trimmed || isExternalUrl(trimmed)) {
    return url;
  }

  const leading = url.slice(0, url.indexOf(trimmed));
  const trailing = url.slice(url.indexOf(trimmed) + trimmed.length);
  const { pathname, suffix } = splitUrlSuffix(trimmed);

  if (!pathname || isExternalUrl(pathname)) {
    return url;
  }

  const sourceTargetPath = resolveUrlPath(pathname, context.sourceDir);
  let outputTargetPath = null;

  if (sourceTargetPath.startsWith(repoNodeModulesDir + path.sep)) {
    outputTargetPath = path.join(vendorDir, path.relative(repoNodeModulesDir, sourceTargetPath));
    if (!existsSync(sourceTargetPath)) {
      throw new Error(`Referenced node_modules file not found: ${sourceTargetPath}`);
    }
    await copyOnce(context.copiedFiles, sourceTargetPath, outputTargetPath);
  } else if (sourceTargetPath.startsWith(distDir + path.sep)) {
    outputTargetPath = sourceTargetPath;
  } else if (sourceTargetPath.startsWith(srcDir + path.sep)) {
    outputTargetPath = distPathForSourceFile(sourceTargetPath);
    if (outputTargetPath && path.extname(sourceTargetPath) === '.js') {
      if (!existsSync(sourceTargetPath)) {
        throw new Error(`Referenced local JS file not found: ${sourceTargetPath}`);
      }
      await copyOnce(context.copiedFiles, sourceTargetPath, outputTargetPath);
    }
  }

  if (!outputTargetPath) {
    return url;
  }

  return `${leading}${relativeUrl(context.outputDir, outputTargetPath)}${suffix}${trailing}`;
}

async function replaceAsync(input, pattern, replacer) {
  const replacements = [];
  input.replace(pattern, (...args) => {
    replacements.push(replacer(...args));
    return args[0];
  });

  const resolved = await Promise.all(replacements);
  let index = 0;
  return input.replace(pattern, () => resolved[index++]);
}

async function rewriteSrcset(srcset, context) {
  const candidates = srcset.split(',');
  const rewritten = [];

  for (const candidate of candidates) {
    const match = candidate.match(/^(\s*)(\S+)(.*)$/s);
    if (!match) {
      rewritten.push(candidate);
      continue;
    }
    const [, leading, url, descriptor] = match;
    rewritten.push(`${leading}${await rewriteUrl(url, context)}${descriptor}`);
  }

  return rewritten.join(',');
}

async function rewriteCssUrls(css, context) {
  let rewritten = await replaceAsync(css, cssUrlPattern, async (match, quote, url) => {
    const nextUrl = await rewriteUrl(url, context);
    return `url(${quote}${nextUrl}${quote})`;
  });

  rewritten = await replaceAsync(rewritten, cssImportPattern, async (match, quote, url) => {
    const nextUrl = await rewriteUrl(url, context);
    return `@import ${quote}${nextUrl}${quote}`;
  });

  return rewritten;
}

async function rewriteHtml(html, context) {
  let rewritten = await replaceAsync(html, urlAttributePattern, async (match, attr, quote, url) => {
    const nextUrl = await rewriteUrl(url, context);
    return `${attr}=${quote}${nextUrl}${quote}`;
  });

  rewritten = await replaceAsync(rewritten, srcsetAttributePattern, async (match, attr, quote, srcset) => {
    const nextSrcset = await rewriteSrcset(srcset, context);
    return `${attr}=${quote}${nextSrcset}${quote}`;
  });

  return rewriteCssUrls(rewritten, context);
}

async function compileHtmlFile(relativePath, copiedFiles) {
  const sourcePath = path.join(srcDir, relativePath);
  const outputPath = path.join(distDir, relativePath);
  const sourceDir = path.dirname(sourcePath);
  const outputDir = path.dirname(outputPath);
  const html = await readFile(sourcePath, 'utf8');
  const rewritten = await rewriteHtml(html, {
    sourceDir,
    outputDir,
    copiedFiles,
  });

  await mkdir(outputDir, { recursive: true });
  await writeFile(outputPath, rewritten, 'utf8');
  console.log(`  ${relativePath} -> ${toPosixPath(path.relative(projectRoot, outputPath))}`);
}

async function compileAll() {
  const htmlFiles = await glob('**/*.html', { cwd: srcDir });
  const copiedFiles = new Set();

  console.log(`Found ${htmlFiles.length} HTML files to compile`);
  for (const htmlFile of htmlFiles.sort()) {
    await compileHtmlFile(htmlFile, copiedFiles);
  }

  console.log(`Copied ${copiedFiles.size} referenced JS/runtime files`);
  console.log('HTML compilation complete');
}

compileAll().catch((error) => {
  console.error(error);
  process.exit(1);
});
