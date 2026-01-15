/*
  * Capture text content from elements matching a given XPath on a webpage or local HTML file.
  *
  * Usage: node tools/scripts/capture-xpath-text.mjs <url-or-path> <xpath> <output-file>
  *
  * Example:
  *   node tools/scripts/capture-xpath-text.mjs packages/cem-theme/src/lib/css-generators/colors.html "//*[@code='language-css']" /tmp/colors.css
  *   node tools/scripts/capture-xpath-text.mjs https://example.com "//h1" output.txt
  *   node tools/scripts/capture-xpath-text.mjs ./local-file.html "//div[@class='content']" /temp/output.txt
 */
import fs from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

async function main() {
    const [thisScript, urlArg, xpath, outputPath] = process.argv.slice(2);
    if (!urlArg || !xpath || !outputPath) {
        console.error(
            "Usage: node tools/scripts/capture-xpath-text.mjs <url-or-path> <xpath> <output-file>",
        );
        process.exit(1);
    }

    let targetUrl = urlArg;
    if (!/^https?:\/\//i.test(urlArg) && !/^file:\/\//i.test(urlArg)) {
        const candidatePath = path.resolve(urlArg);
        if (existsSync(candidatePath)) {
            targetUrl = pathToFileURL(candidatePath).toString();
        }
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

    const browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    await page.goto(targetUrl, { waitUntil: "domcontentloaded" });

    const locator = page.locator(`xpath=${xpath}`);
    await locator.first().waitFor({ state: "attached", timeout:10000 }).catch(() => {});
    const count = await locator.count();
    if (count === 0) {
        await browser.close();
        throw new Error(`No elements matched XPath: ${xpath}`);
    }

    const texts = await locator.allTextContents();
    const outputDir = path.dirname(path.resolve(outputPath));
    await fs.mkdir(outputDir, { recursive: true });
    await fs.writeFile(outputPath, texts.join("\n"), "utf8");

    await browser.close();
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});
