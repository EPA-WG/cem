/** Opt-in service-worker transport. Install in an application-owned worker;
 * registration, activation and scope are deliberately owned by the host app.
 * Messages carry acquired bytes, never native trees or preparation seals. */
export function installImageDeliveryTransport(scope: ServiceWorkerGlobalScope): void {
    const sessions = new Map<string, {
        port: MessagePort; abort: AbortController; sealed: boolean; bytes: number;
        responses: Map<string, Promise<{ bytes: Uint8Array; type: string }>>;
        receipts: Map<number, { resolve: () => void; reject: () => void }>; next: number;
        timer: ReturnType<typeof setTimeout>;
    }>();
    const close = (id: string) => {
        const s = sessions.get(id); if (!s) return;
        sessions.delete(id); clearTimeout(s.timer); s.abort.abort();
        for (const receipt of s.receipts.values()) receipt.reject();
        s.receipts.clear(); s.responses.clear(); s.port.close();
    };
    scope.addEventListener('message', event => {
        const data = event.data, client = event.source as Client | null, port = event.ports[0];
        if (data?.kind !== 'cem-image-delivery-open') return;
        if (!client || !('id' in client) || !port || client.url !== data.url
            || !Number.isSafeInteger(data.timeout) || data.timeout < 1 || data.timeout > 30_000
            || sessions.has(client.id) || sessions.size >= 32) { port?.postMessage({ kind: 'error' }); return; }
        const s = { port, abort: new AbortController(), sealed: false, bytes: 0,
            responses: new Map<string, Promise<{ bytes: Uint8Array; type: string }>>(),
            receipts: new Map<number, { resolve: () => void; reject: () => void }>(), next: 0,
            timer: setTimeout(() => { port.postMessage({ kind: 'error' }); close(client.id); }, data.timeout),
        };
        sessions.set(client.id, s);
        port.onmessage = message => {
            const command = message.data;
            if (command?.kind === 'close') { close(client.id); return; }
            if (command?.kind === 'ack') {
                const receipt = s.receipts.get(command.id);
                if (!receipt) { port.postMessage({ kind: 'error' }); close(client.id); return; }
                s.receipts.delete(command.id);
                if (command.ok === true) receipt.resolve(); else { receipt.reject(); close(client.id); }
            } else if (command?.kind === 'seal' && !s.sealed) {
                s.sealed = true;
                void Promise.all(s.responses.values()).then(() => {
                    if (sessions.get(client.id) !== s) return;
                    clearTimeout(s.timer); port.postMessage({ kind: 'sealed' });
                }, () => { port.postMessage({ kind: 'error' }); close(client.id); });
            }
        };
        port.onmessageerror = () => close(client.id);
        port.start(); port.postMessage({ kind: 'ready' });
    });
    scope.addEventListener('fetch', event => {
        const s = sessions.get(event.clientId);
        if (!s || event.request.destination !== 'image') return;
        event.respondWith((async () => {
            try {
                const request = event.request, url = new URL(request.url);
                if (s.sealed || request.method !== 'GET' || url.origin !== scope.location.origin || url.hash
                    || !/^https?:$/u.test(url.protocol) || url.href.length > 4096) throw new Error('Unsupported image delivery');
                let pending = s.responses.get(url.href);
                if (!pending) {
                    if (s.responses.size >= 256) throw new Error('Image delivery count exceeded');
                    pending = (async () => {
                        const response = await fetch(new Request(request, { cache: 'no-store', redirect: 'error', signal: s.abort.signal }));
                        const type = response.headers.get('content-type') ?? '';
                        if (response.status !== 200 || response.url !== url.href || !response.body || type.length > 256
                            || !/^image\/(?:png|jpeg|gif|webp|avif)(?:;|$)/iu.test(type)) throw new Error('Direct raster response required');
                        const reader = response.body.getReader(), chunks: Uint8Array[] = [];
                        let length = 0;
                        try {
                            for (;;) {
                                const part = await reader.read(); if (part.done) break;
                                s.bytes += part.value.byteLength; length += part.value.byteLength;
                                if (s.bytes > 4 * 1024 * 1024) throw new Error('Image delivery byte budget exceeded');
                                chunks.push(part.value);
                            }
                        } finally { await reader.cancel(); reader.releaseLock(); }
                        if (!length || s.abort.signal.aborted) throw new Error('Empty or cancelled image delivery');
                        const bytes = new Uint8Array(length); let offset = 0;
                        for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
                        const id = ++s.next;
                        // Keep the exact body here; transfer an independent ingress copy.
                        await new Promise<void>((resolve, reject) => {
                            s.receipts.set(id, { resolve, reject: () => reject(new Error('Native receipt rejected')) });
                            const copy = bytes.slice();
                            s.port.postMessage({ kind: 'body', id, url: url.href, type, bytes: copy }, [copy.buffer]);
                        });
                        if (s.abort.signal.aborted) throw new Error('Image delivery cancelled before release');
                        return { bytes, type };
                    })();
                    s.responses.set(url.href, pending);
                }
                const body = await pending;
                // Canonical decoded entity body: do not replay Content-Encoding/Length.
                return new Response(body.bytes.slice(), { status: 200, headers: { 'content-type': body.type, 'cache-control': 'no-store' } });
            } catch {
                s.port.postMessage({ kind: 'error' }); close(event.clientId);
                return Response.error();
            }
        })());
    });
}
