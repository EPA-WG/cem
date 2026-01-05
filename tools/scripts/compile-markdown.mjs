#!/usr/bin/env node

import MarkdownIt from 'markdown-it';
import { glob } from 'glob';
import { readFile, writeFile, mkdir, copyFile } from 'fs/promises';
import { dirname, relative, join, parse } from 'path';
import { fileURLToPath } from 'url';

// Image extensions to copy alongside markdown files
const IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'ico'];

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(__dirname, '../../packages/cem-theme');

// Configure markdown-it with plugins
const md = new MarkdownIt({
  html: true,           // Enable HTML tags in source
  xhtmlOut: true,       // Use '/' to close single tags (<br />)
  breaks: false,        // Convert '\n' in paragraphs into <br>
  linkify: true,        // Autoconvert URL-like text to links
  typographer: true     // Enable smartquotes and other typographic replacements
});

async function compileMarkdown(srcPath, distPath) {
  const content = await readFile(srcPath, 'utf-8');
  const html = md.render(content);
  const h1Match = html.match(/<h1[^>]*>(.*?)<\/h1>/i);
  const title = h1Match ? h1Match[1].replace(/<[^>]+>/g, '') : 'Documentation';
  // Wrap in XHTML document structure
  const xhtml = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN"
  "http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="en" lang="en">
<head>
  <meta http-equiv="Content-Type" content="text/html; charset=UTF-8" />
  <title>${title}</title>
</head>
<body>
${html}
</body>
</html>`;

  await mkdir(dirname(distPath), { recursive: true });
  await writeFile(distPath, xhtml, 'utf-8');
}

async function copyImage(srcPath, distPath) {
  await mkdir(dirname(distPath), { recursive: true });
  await copyFile(srcPath, distPath);
}

async function compileAll() {
  const srcDir = join(projectRoot, 'src');
  const distDir = join(projectRoot, 'dist');

  // Find all .md files in src/
  const mdFiles = await glob('**/*.md', { cwd: srcDir });

  console.log(`Found ${mdFiles.length} Markdown files to compile`);

  for (const mdFile of mdFiles) {
    const srcPath = join(srcDir, mdFile);
    const { dir, name } = parse(mdFile);
    const distPath = join(distDir, dir, `${name}.xhtml`);

    console.log(`  ${mdFile} → ${relative(projectRoot, distPath)}`);
    await compileMarkdown(srcPath, distPath);
  }

  console.log('✓ Markdown compilation complete');

  // Find and copy all image files in src/
  const imagePattern = `**/*.{${IMAGE_EXTENSIONS.join(',')}}`;
  const imageFiles = await glob(imagePattern, { cwd: srcDir });

  if (imageFiles.length > 0) {
    console.log(`Found ${imageFiles.length} image files to copy`);

    for (const imageFile of imageFiles) {
      const srcPath = join(srcDir, imageFile);
      const distPath = join(distDir, imageFile);

      console.log(`  ${imageFile} → ${relative(projectRoot, distPath)}`);
      await copyImage(srcPath, distPath);
    }

    console.log('✓ Image copying complete');
  }
}

// Handle watch mode
if (process.argv.includes('--watch')) {
  const chokidar = await import('chokidar');
  const srcDir = join(projectRoot, 'src');

  console.log('Watching for changes...');

  // Watch markdown files
  chokidar.watch('**/*.md', { cwd: srcDir }).on('all', async (event, path) => {
    if (event === 'add' || event === 'change') {
      const srcPath = join(srcDir, path);
      const { dir, name } = parse(path);
      const distPath = join(projectRoot, 'dist', dir, `${name}.xhtml`);

      console.log(`  ${path} → ${relative(projectRoot, distPath)}`);
      await compileMarkdown(srcPath, distPath);
    }
  });

  // Watch image files
  const imagePattern = `**/*.{${IMAGE_EXTENSIONS.join(',')}}`;
  chokidar.watch(imagePattern, { cwd: srcDir }).on('all', async (event, path) => {
    if (event === 'add' || event === 'change') {
      const srcPath = join(srcDir, path);
      const distPath = join(projectRoot, 'dist', path);

      console.log(`  ${path} → ${relative(projectRoot, distPath)}`);
      await copyImage(srcPath, distPath);
    }
  });
} else {
  compileAll().catch(console.error);
}
