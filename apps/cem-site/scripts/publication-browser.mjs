import assert from 'node:assert/strict';
import { chromium } from 'playwright';

// An explicit transport diagnostic; ordinary verification keeps Chromium's
// defaults. TLS validation, browser contexts, assertions and deadlines are
// unchanged. This option affects the verifier, never the published application.
export const browserTransport = process.env.CEM_SITE_BROWSER_TRANSPORT ?? 'default';
assert.ok(['default', 'http2'].includes(browserTransport), 'CEM_SITE_BROWSER_TRANSPORT must be default or http2');

export function launchPublicationBrowser(options = {}) {
    return chromium.launch({ ...options, args: [
        ...(options.args ?? []), ...(browserTransport === 'http2' ? ['--disable-quic'] : []),
    ] });
}
