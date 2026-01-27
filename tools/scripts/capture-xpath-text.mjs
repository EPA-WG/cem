const helpString = `
Capture text content from elements matching a given XPath across local HTML files.

Usage: node tools/scripts/capture-xpath-text.mjs <path-mask> <xpath> <output-mask>

Mask examples:
  node tools/scripts/capture-xpath-text.mjs "packages/cem-theme/src/lib/css-generators/*.html" "//code[@data-generated-css]" "tmp/*.css"
  node tools/scripts/capture-xpath-text.mjs "packages/**/colors.html" "//*[@code='language-css']" "dist/lib/css/*-generated.css"

Output mask rules:
  - Use * to substitute the input file name without extension.
  - If multiple input files match, output mask must include * to avoid overwriting.
`;

import fs from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

function hasGlobMagic(mask) {
    return /[*?[\]{}()!]/.test(mask);
}

async function expandInputFiles(mask) {
    if (!hasGlobMagic(mask)) {
        const candidatePath = path.resolve(mask);
        if (!existsSync(candidatePath)) {
            throw new Error(`Input file not found: ${candidatePath}`);
        }
        return [candidatePath];
    }

    const matches = await fs.glob(mask, { withFileTypes: false });
    if (matches.length === 0) {
        throw new Error(`No input files matched mask: ${mask}`);
    }
    const files = await Array.fromAsync(fs.glob(mask, { withFileTypes: false }));

    return files;// matches.map((match) => path.resolve(match));
}

function resolveOutputPath(mask, inputPath) {
    if (!mask.includes("*")) {
        return path.resolve(mask);
    }
    const name = path.parse(inputPath).name;
    return path.resolve(mask.split("*").join(name));
}

async function main(urlMask, xpath, outputMask) {
    if (!urlMask || !xpath || !outputMask) {
        console.error('Usage: node tools/scripts/capture-xpath-text.mjs <path-mask> <xpath> <output-mask>', helpString);
        process.exit(1);
    }

    const inputFiles = await expandInputFiles(urlMask);
    if (inputFiles.length > 1 && !outputMask.includes("*")) {
        throw new Error("Output mask must include * when multiple input files match.");
    }

    let chromium;
    try {
        ({ chromium } = await import("playwright"));
    } catch (error) {
        console.error(
            "Playwright is required. Install with: yarn add -D playwright && npx playwright install chromium",
        );
        throw error;
    }

    let browser;
    try {
        browser = await chromium.launch({
            headless: true,
            args: ["--allow-file-access-from-files", "--disable-web-security"],
        });
        const context = await browser.newContext({
            javaScriptEnabled: true,
            bypassCSP: true,
        });
        const page = await context.newPage();

        for (const inputFile of inputFiles) {
            const targetUrl = pathToFileURL(inputFile).toString();
            await page.goto(targetUrl, { waitUntil: "domcontentloaded" });
            const locator = page.locator(`xpath=${xpath}`);
            await locator
                .first()
                .waitFor({ state: "attached", timeout: 5000 })
                .catch(() => {});
            const count = await locator.count();
            if (count === 0) {
                throw new Error(`No elements matched XPath: ${xpath} for ${inputFile}`);
            }

            const texts = await locator.allTextContents();
            const outputPath = resolveOutputPath(outputMask, inputFile);
            const outputDir = path.dirname(outputPath);
            await fs.mkdir(outputDir, { recursive: true });
            await fs.writeFile(outputPath, texts.join("\n"), "utf8");
            console.log(`${inputFile} ➤ ${outputPath}`);
        }
    } finally {
        if (browser) {
            await browser.close();
        }
    }
}

const [urlMask, xpath, outputMask] = process.argv.slice(2);

main(urlMask, xpath, outputMask).catch((error) => {
    console.error(error);
    process.exit(1);
});
