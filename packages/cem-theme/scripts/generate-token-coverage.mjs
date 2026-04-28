/**
 * Generate the token coverage report from built token XHTML and generated CSS.
 *
 * Usage:
 *   node scripts/generate-token-coverage.mjs
 */

import fs from "node:fs/promises";
import path from "node:path";
import MarkdownIt from "markdown-it";
import anchor from "markdown-it-anchor";
import {
    analyzeCSS,
    compareManifestToCss,
    COVERAGE_CATEGORIES,
    deriveManifestForSpec,
    SPEC_ORDER,
    tryPostcssValidation,
} from "./manifest-utils.mjs";

const packageRoot = process.cwd();
const reportMarkdownPath = path.join(packageRoot, "dist/lib/tokens/generated-token-coverage.md");
const reportXhtmlPath = path.join(packageRoot, "dist/lib/tokens/generated-token-coverage.xhtml");

const md = new MarkdownIt({
    html: true,
    xhtmlOut: true,
    breaks: false,
    linkify: true,
    typographer: true,
}).use(anchor, {
    permalink: false,
    slugify: (s) => s.toLowerCase().replace(/[^\w]+/g, "-").replace(/^-|-$/g, ""),
});

async function readRequired(filePath) {
    try {
        return await fs.readFile(filePath, "utf8");
    } catch (err) {
        throw new Error(`Cannot read ${path.relative(packageRoot, filePath)}\n${err.message}`);
    }
}

function renderXhtml(markdown) {
    const html = md.render(markdown).replace(/\.md(["'\s#)])/g, ".xhtml$1").replace(/\.md/g, "");
    const h1Match = html.match(/<h1[^>]*>(.*?)<\/h1>/i);
    const title = h1Match ? h1Match[1].replace(/<[^>]+>/g, "") : "Generated Token Coverage";
    return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN"
  "http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="en" lang="en">
<head>
  <meta http-equiv="Content-Type" content="text/html; charset=UTF-8" />
  <title>${title}</title>
  <style type="text/css">@import url("./index.css");</style>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/prismjs/themes/prism.css"/>
  <script src="https://cdn.jsdelivr.net/npm/prismjs/prism.js" type="application/javascript"></script>
  <script src="https://cdn.jsdelivr.net/npm/prismjs/components/prism-css.min.js" type="application/javascript"></script>
  <script type="application/javascript" defer="defer">
    Prism.highlightAll();
  </script>
</head>
<body>
${html}
</body>
</html>`;
}

async function loadSpecCoverage(spec) {
    const xhtmlPath = path.join(packageRoot, `dist/lib/tokens/${spec.name}.xhtml`);
    const cssPath = path.join(packageRoot, `dist/lib/css/${spec.name}.css`);
    const xhtml = await readRequired(xhtmlPath);
    const cssText = await readRequired(cssPath);
    const { tokens, warnings } = deriveManifestForSpec(spec.name, xhtml);
    const { defined, violations } = analyzeCSS(cssText);
    await tryPostcssValidation(cssText, violations);
    const { missing, extras } = compareManifestToCss(tokens, defined);

    return {
        spec,
        tokens,
        defined,
        warnings,
        violations,
        missing,
        extras,
    };
}

function categoryRows(specReports) {
    const bySpec = new Map(specReports.map((report) => [report.spec.name, report]));

    return COVERAGE_CATEGORIES.map((category) => {
        const report = bySpec.get(category.spec);
        if (!report) {
            return { ...category, defined: 0, generated: 0, gap: 0, status: "Missing spec" };
        }

        const tokens = report.tokens.filter((token) => token.categoryId === category.id);
        const generated = tokens.filter((token) => report.defined.has(token.name)).length;
        const gap = tokens.length - generated;
        return {
            ...category,
            defined: tokens.length,
            generated,
            gap,
            status: gap === 0 ? "Complete" : "Gap",
        };
    });
}

function renderMarkdown(rows, specReports) {
    const totalDefined = rows.reduce((sum, row) => sum + row.defined, 0);
    const totalGenerated = rows.reduce((sum, row) => sum + row.generated, 0);
    const totalGap = rows.reduce((sum, row) => sum + row.gap, 0);
    const hasReportIssues = specReports.some((report) =>
        report.warnings.length || report.violations.length || report.missing.length || report.extras.length
    );

    const lines = [
        "# Generated Token Coverage",
        "",
        "This report is generated from built token XHTML manifests and generated CSS. Do not edit it by hand.",
        "",
        "| Category | Defined | Generated | Gap | Status |",
        "|---|---:|---:|---:|---|",
    ];

    for (const row of rows) {
        lines.push(`| ${row.label} | ${row.defined} | ${row.generated} | ${row.gap} | ${row.status} |`);
    }

    lines.push(`| **Total** | **${totalDefined}** | **${totalGenerated}** | **${totalGap}** | **${totalGap === 0 ? "Complete" : "Gap"}** |`);
    lines.push("");
    lines.push("## Source");
    lines.push("");
    lines.push("- Manifests: `dist/lib/tokens/*.xhtml`");
    lines.push("- CSS: `dist/lib/css/*.css`");
    lines.push("- Generator: `scripts/generate-token-coverage.mjs`");

    if (hasReportIssues) {
        lines.push("");
        lines.push("## Validation Notes");
        lines.push("");
        for (const report of specReports) {
            const issues = [
                ...report.warnings.map((message) => `manifest warning: ${message}`),
                ...report.violations.map((message) => `CSS violation: ${message}`),
                ...report.missing.map(({ name }) => `missing token: ${name}`),
                ...report.extras.map((name) => `extra token: ${name}`),
            ];
            if (issues.length === 0) continue;
            lines.push(`### ${report.spec.name}`);
            lines.push("");
            for (const issue of issues) {
                lines.push(`- ${issue}`);
            }
            lines.push("");
        }
    }

    return `${lines.join("\n")}\n`;
}

async function main() {
    const reports = [];
    for (const spec of SPEC_ORDER) {
        reports.push(await loadSpecCoverage(spec));
    }
    const rows = categoryRows(reports);
    const markdown = renderMarkdown(rows, reports);
    const xhtml = renderXhtml(markdown);

    await fs.mkdir(path.dirname(reportMarkdownPath), { recursive: true });
    await fs.writeFile(reportMarkdownPath, markdown, "utf8");
    await fs.writeFile(reportXhtmlPath, xhtml, "utf8");

    const totalDefined = rows.reduce((sum, row) => sum + row.defined, 0);
    const totalGenerated = rows.reduce((sum, row) => sum + row.generated, 0);
    const totalGap = rows.reduce((sum, row) => sum + row.gap, 0);
    console.log(`✓ token coverage report generated (${totalGenerated}/${totalDefined}, gap ${totalGap})`);
    console.log(`  ${path.relative(packageRoot, reportMarkdownPath)}`);
    console.log(`  ${path.relative(packageRoot, reportXhtmlPath)}`);
}

main().catch((err) => {
    console.error(err);
    process.exit(2);
});
