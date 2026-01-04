#!/usr/bin/env node

import MarkdownIt from 'markdown-it';
import { glob } from 'glob';
import { readFile, writeFile, mkdir } from 'fs/promises';
import { dirname, relative, join, parse } from 'path';
import { fileURLToPath } from 'url';

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

  // Wrap in XHTML document structure
  const xhtml = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN"
  "http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="en" lang="en">
<head>
  <meta http-equiv="Content-Type" content="text/html; charset=UTF-8" />
  <title>Documentation</title>
</head>
<body>
${html}
</body>
</html>`;

  await mkdir(dirname(distPath), { recursive: true });
  await writeFile(distPath, xhtml, 'utf-8');
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
}

// Handle watch mode
if (process.argv.includes('--watch')) {
  const chokidar = await import('chokidar');
  const srcDir = join(projectRoot, 'src');

  console.log('Watching for changes...');

  chokidar.watch('**/*.md', { cwd: srcDir }).on('all', async (event, path) => {
    if (event === 'add' || event === 'change') {
      const srcPath = join(srcDir, path);
      const { dir, name } = parse(path);
      const distPath = join(projectRoot, 'dist', dir, `${name}.xhtml`);

      console.log(`  ${path} → ${relative(projectRoot, distPath)}`);
      await compileMarkdown(srcPath, distPath);
    }
  });
} else {
  compileAll().catch(console.error);
}
