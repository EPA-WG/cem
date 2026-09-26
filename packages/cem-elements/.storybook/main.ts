import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { defineMain } from '@storybook/web-components-vite/node';
import { mergeConfig } from 'vite';

const config = defineMain({
    staticDirs: [
        { from: '../dist', to: '/cem-manager-assets/runtime' },
        { from: '../../cem-theme/dist', to: '/cem-manager-assets/theme' },
        { from: '../../cem-components/src/components', to: '/cem-manager-assets/components' },
    ],
    stories: [
        '../src/**/!(*.edge-ssr).stories.@(js|jsx|mjs|ts|tsx)',
        '../../cem-components/src/components/**/*.stories.ts',
    ],
    addons: [getAbsolutePath("@storybook/addon-vitest")],
    framework: {
        name: getAbsolutePath("@storybook/web-components-vite"),
        options: {},
    },
    core: {
        disableTelemetry: true,
    },
    viteFinal: async (config) => mergeConfig(config, {
        // CEM uses native @scope and light-dark(); preserve the public CSS.
        build: { cssTarget: 'esnext' },
        plugins: [{
            name: 'cem-demo-static-resources',
            configureServer(server) {
                const demoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../../demo');
                server.middlewares.use((request, response, next) => {
                    const pathname = decodeURIComponent(new URL(request.url ?? '/', 'http://localhost').pathname);
                    const relativePath = pathname.startsWith('/packages/cem-elements/demo/')
                        ? pathname.slice('/packages/cem-elements/demo/'.length)
                        : pathname.startsWith('/demo/') ? pathname.slice('/demo/'.length) : undefined;
                    if (!relativePath || !relativePath.endsWith('.html')) return next();
                    const filePath = resolve(demoRoot, relativePath);
                    if (!filePath.startsWith(`${demoRoot}/`)) return next();
                    try {
                        response.statusCode = 200;
                        response.setHeader('Content-Type', 'text/html; charset=utf-8');
                        response.setHeader('Cache-Control', 'no-store');
                        response.end(readFileSync(filePath));
                    } catch {
                        next();
                    }
                });

                // The external-template demo deliberately requests no.svg.
                // Preserve static-server 404 behavior, including fallback
                // resolution from the host URL, instead of serving SPA HTML.
                server.middlewares.use((request, response, next) => {
                    const pathname = new URL(request.url ?? '/', 'http://localhost').pathname;
                    if (!pathname.endsWith('/no.svg')) return next();
                    response.statusCode = 404;
                    response.end('Not found');
                });
            },
        }],
        resolve: {
            // Source-loaded demo documents are rebased from Storybook's
            // project root. Their standalone import remains local while
            // preview.ts owns the actual one-time registration.
            alias: {
                '/cem-demo-element/dist/index.js': fileURLToPath(
                    new URL('../../cem-demo-element/src/index.ts', import.meta.url)
                ),
            },
        },
    }),
});

export default config;

function getAbsolutePath(value: string): string {
    return dirname(fileURLToPath(import.meta.resolve(`${value}/package.json`)));
}
