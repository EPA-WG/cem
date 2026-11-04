import assets from './demo-worker.mjs';

// These public native sidecars exceed the asset service's per-file limit.
// Hosting stores gzip bytes; HTTP decoding preserves the original publication.
export const compressedSitePaths = ['tokens/index.html.map'];

export default {
    async fetch(request, environment) {
        const url = new URL(request.url);
        if (!compressedSitePaths.includes(url.pathname.slice(1)) || !['GET', 'HEAD'].includes(request.method)) {
            return assets.fetch(request, environment);
        }
        url.pathname += '.gz';
        const upstream = new Request(url, request);
        upstream.headers.set('Accept-Encoding', 'identity');
        upstream.headers.delete('Range'); upstream.headers.delete('If-Range');
        const response = await environment.ASSETS.fetch(upstream);
        if (!response.ok && response.status !== 304) return response;
        const headers = new Headers(response.headers);
        headers.set('Content-Type', 'application/json; charset=utf-8');
        headers.delete('Content-Length'); headers.delete('Content-Range'); headers.delete('Accept-Ranges');
        headers.set('Vary', [...new Set([...(headers.get('Vary') ?? '').split(',').map(value => value.trim()).filter(Boolean), 'Accept-Encoding'])].join(', '));
        const encodings = (request.headers.get('Accept-Encoding') ?? '').split(',').map(value => {
            const [name, ...options] = value.trim().split(';');
            return { name, quality: Number(options.find(option => option.trim().startsWith('q='))?.trim().slice(2) ?? 1) };
        });
        const gzip = encodings.find(encoding => encoding.name === 'gzip') ?? encodings.find(encoding => encoding.name === '*');
        const encoded = !request.headers.get('Accept-Encoding') || (gzip?.quality ?? 0) > 0;
        if (encoded) headers.set('Content-Encoding', 'gzip'); else headers.delete('Content-Encoding');
        const body = request.method === 'HEAD' || response.status === 304 ? null
            : encoded ? response.body : response.body.pipeThrough(new DecompressionStream('gzip'));
        return new Response(body, { status: response.status, headers, encodeBody: encoded ? 'manual' : 'automatic' });
    },
};
