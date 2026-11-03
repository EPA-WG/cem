import assert from 'node:assert/strict';

export const amsterdamStudioUrl = 'https://green-morning-fd6e.trans-acf.workers.dev/studio/';

/** Public verification is opt-in for the selected presentation on its exact host. */
export function studioAcceptanceUrl(value, { publicProfile, presentationOnly = false } = {}) {
    assert.ok(publicProfile === undefined || publicProfile === 'amsterdam-2026', 'Unknown public acceptance profile');
    if (!value) {
        assert.equal(publicProfile, undefined, 'Public acceptance requires an explicit target URL');
        return undefined;
    }
    const target = new URL(value);
    assert.equal(target.username, ''); assert.equal(target.password, '');
    assert.equal(target.search, ''); assert.equal(target.hash, '');
    if (publicProfile) {
        assert.equal(presentationOnly, true, 'Public acceptance requires the selected presentation suite');
        assert.equal(`${target.origin}${target.pathname.replace(/\/$/, '')}/`, amsterdamStudioUrl, 'Public acceptance requires the exact Amsterdam HTTPS Studio subtree');
    } else {
        assert.equal(target.protocol, 'http:');
        assert.ok(['localhost', '127.0.0.1', '[::1]'].includes(target.hostname), 'External acceptance must use a local test server');
    }
    return `${target.origin}${target.pathname.replace(/\/$/, '')}`;
}
