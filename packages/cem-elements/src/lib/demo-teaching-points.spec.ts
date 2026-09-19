import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const inventories: Record<string, string[]> = {
    "attributes": [
        "1. attributes definition",
        "1a. External attribute changes",
        "1b. Container attribute values",
        "2. attribute from slice",
        "3. V attribute matches input value",
        "3a. Container value before input",
        "4. attribute defaults, from container, and from slice",
        "4a. External changes versus user input"
    ],
    "external-template": [
        "1. reference the template in page DOM",
        "2. without TAG, inline instantiation",
        "3. external SVG file",
        "3a. Anonymous external SVG",
        "3b. Missing source fallback",
        "4. external CEM-ML template file",
        "4a. Live HTML payload capture",
        "4b. CEM-ML source payload",
        "5. external HTML template",
        "5a. Anonymous external HTML",
        "6. HTML, SVG by ID within external file",
        "6a. SVG fragment by ID",
        "6b. MathML fragment by ID",
        "7a. external CEM-ML data-island tree template",
        "7b. External XSLT XML payload tree",
        "7c. Missing fragment fallback",
        "7d. Anonymous external XSLT",
        "7e. Embedded XSLT fragment",
        "8. external file with embedding of another external DCE",
        "9. external file with invoking of relative template as hash by enclosed custom-element",
        "10. external file with invoking of template in another relative path file by enclosed custom-element",
        "embed-1.html external file",
        "embed-lib.html with multiple templates"
    ]
};

function source(name: string): string {
    return readFileSync(new URL(`../../demo/${name}.html`, import.meta.url), 'utf8');
}

describe('restored demo teaching points', () => {
    for (const [name, expected] of Object.entries(inventories)) {
        it(`${name} isolates and describes every authored case`, () => {
            const document = source(name);
            const samples = Array.from(document.matchAll(/<cem-demo-element\b[\s\S]*?<\/cem-demo-element>/gu), (match) => match[0]);
            expect(samples.map((sample) => sample.match(/legend="([^"]+)"/u)?.[1])).toEqual(expected);
            for (const sample of samples) {
                expect(sample).toMatch(/description="[^"]+"/u);
                if (sample.includes('<template>')) expect(sample).toMatch(/<template>\n</u);
            }
            expect(document).toContain('See also');
            expect(document).not.toContain('data-set-attr');
        });
    }

    it('keeps attribute mutations outside declarative rendering and includes removal', () => {
        const document = source('attributes');
        expect(document).toContain("setAttribute('p1'");
        expect(document).toContain("removeAttribute('p3')");
        expect(document).toContain('datadom.eventPayloads.s');
        expect(document).not.toContain("window.addEventListener('click'");
        expect(document).not.toContain('/datadom/attributes/');
    });

    it('really uses anonymous declarations and separately documents failures', () => {
        const document = source('external-template');
        expect(document.match(/<cem-element src="#template2"><\/cem-element>/gu)).toHaveLength(2);
        expect(document).toContain('<cem-element src="confused.svg">');
        expect(document).not.toContain('tag="dce-construction"');
        expect(document).toContain('3b. Missing source fallback');
        expect(document).toContain('7c. Missing fragment fallback');
    });

    it('preserves nested declarations and library-local fragment references', () => {
        expect(source('embed-1')).toContain('<cem-element>');
        expect(source('embed-1')).toContain('<template type="text/cem-ml">');
        expect(source('lib-dir/embed-lib')).toContain('@src="./embed-lib.html#embed-lib-component"');
        expect(source('lib-dir/embed-lib')).toContain('@src="../embed-1.html"');
    });

    it('keeps native whole-file XSLT and explicitly labeled legacy fragments distinct', () => {
        const document = source('external-template');
        expect(document).toContain('<cem-element src="data-island-tree.xsl" xslt-template="tree">');
        expect(document).toContain('<xslt-param name="source"');
        expect(readFileSync(new URL('../../demo/data-island-tree.xsl', import.meta.url), 'utf8')).toContain('parse-xml($source)');
        expect(document).toContain('<cem-element src="html-template.xhtml#embedded-xslt">');
        const library = readFileSync(new URL('../../demo/html-template.xhtml', import.meta.url), 'utf8');
        const fragment = library.split('<template id="embedded-xslt" lang="custom-element-v0">')[1]?.split('</template>')[0];
        expect(fragment).toContain('<xsl:stylesheet');
        expect(fragment).toContain('xmlns:xsl="http://www.w3.org/1999/XSL/Transform"');
        expect(fragment).toContain('Embedded XSLT fruit tree');
        expect(fragment).not.toContain('type="text/cem-ml"');
        expect(document).not.toContain('XSLTProcessor');
    });

    it('arms every URL command with its event revision', () => {
        const document = source('set-url');
        for (const slice of ['hashTarget', 'method', 'requestedUrl', 'applyUrl']) {
            expect(document).toContain(`@trigger="{$datadom.eventPayloads.${slice}.revision}"`);
        }
    });

    it('keeps a deterministic malformed response for HTTP failure and recovery', () => {
        const invalid = readFileSync(new URL('../../demo/http-data-invalid.json', import.meta.url), 'utf8');
        expect(invalid).toContain('deliberately malformed'); // Native loader/story fixture checks the parse diagnostic.
        expect(source('http-request')).toContain('./http-data-invalid.json');
        expect(source('http-request')).toContain('press GET to recover');
    });

    it.each([
        ['attributes', ['aria-label="set p1" title="set p1"', '→p1</button>', '−p3</button>', '→v</button>']],
        ['data-slices', ['@aria-label=Increase @title=Increase', '@aria-label=Decrease @title=Decrease', '| −}', '@title="Set nickname to broccoli"']],
        ['form', ['@aria-label=Next @title=Next', '| →}', '@aria-label="Sign in" @title="Sign in" | 🔑', '@title="Submit matching fruit" | ✓']],
        ['http-request', ['@aria-label=GET @title=GET', '| ↓ GET}', '@title="Empty URL"', '| ∅ URL}', '@title="Invalid JSON response"', '| ⚠ JSON}']],
        ['set-url', ['@aria-label=Set @title=Set', '@value=apply', '@value="#conditional-writer"']],
        ['location-element', ['aria-label="Navigate with GET (reloads)"', '>→ GET ↻</button>', 'title="Change hash after initial read"', '→#</button>']],
        ['npm-versions-demo', ['title="Set URL to 0.0.22"', '>→0.0.22</button>', 'title="Clear URL version"', '>∅#</button>']],
        ['for-each', ['@select=\'("🍏", "🍌", "🍒")\'']],
    ] as const)('%s keeps expressive samples grounded in their teaching semantics', (name, required) => {
        const document = source(name).replace(/\s+/gu, ' ');
        for (const value of required) expect(document).toContain(value);
    });
});
