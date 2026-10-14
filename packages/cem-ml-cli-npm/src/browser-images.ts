import type { BrowserImageDelivery } from './browser-image-delivery.js';
/** Browser selection and separate, bounded same-origin raster reacquisition.
 * These bytes are not asserted to be the browser's original image response. */
export type ImagePredicate = (owner: number) => [string, number, number, boolean];
export interface NativeImagePreparation {
    beginImageDelivery(): void;
    acceptImageDelivery(url: string, contentType: string, bytes: Uint8Array): void;
    recordDeliveredImages(environment: string, observe: ImagePredicate): void;
    selectImages(observe: ImagePredicate, request: (owner: number, url: string) => void): void;
    addImageResponse(requested: string, finalUrl: string, contentType: string, revision: string, bytes: Uint8Array): void;
    recordImages(environment: string, observe: ImagePredicate): void;
}
export class BrowserImageObservation {
    readonly #media: MediaQueryList[] = [];
    readonly #abort = new AbortController();
    constructor(readonly document: Document, readonly node: (id: number) => Node | undefined,
        readonly guard: () => void, readonly revoke: () => void, readonly run: <T>(work: () => T) => T) {}
    readonly observe: ImagePredicate = owner => {
        this.guard();
        const image = this.node(owner) as HTMLImageElement | undefined;
        if (!image || image.localName !== 'img' || image.namespaceURI !== 'http://www.w3.org/1999/xhtml'
            || image.ownerDocument !== this.document || image.getRootNode() !== this.document || !image.isConnected)
            throw new Error('Native image owner has no live DOM installation');
        const result: ReturnType<ImagePredicate> = [image.currentSrc, image.naturalWidth, image.naturalHeight, image.complete];
        this.guard(); return result;
    };
    async prepare(native: NativeImagePreparation, deadline: number, revision: () => string, delivery?: BrowserImageDelivery): Promise<void> {
        this.guard();
        const signal = this.#abort.signal;
        const remaining = deadline - performance.now();
        if (remaining <= 0) throw new Error('Image acquisition deadline elapsed');
        const timer = setTimeout(() => this.#abort.abort(), remaining);
        const cancelled = new Promise<never>((_, reject) => {
            signal.addEventListener('abort', () => reject(new Error('Image acquisition cancelled or timed out')), { once: true });
        });
        try {
            await Promise.race([this.#prepare(native, revision, signal, deadline, delivery), cancelled]);
            this.guard();
        } finally { clearTimeout(timer); }
    }
    async #prepare(native: NativeImagePreparation, revision: () => string, signal: AbortSignal, deadline: number, delivery?: BrowserImageDelivery): Promise<void> {
        const images = [...this.document.images].filter(img => img.hasAttribute('src') || img.hasAttribute('srcset') || img.parentElement?.localName === 'picture');
        if (images.length > 256) throw new Error('Image observation count exceeded');
        for (const image of images) {
            if (image.hasAttribute('crossorigin') || image.hasAttribute('referrerpolicy')) throw new Error('Custom image request policy is outside the acquisition profile');
            await image.decode();
            this.guard();
        }
        const requests = new Set<string>();
        this.run(() => native.selectImages(this.observe, (_owner, url) => { requests.add(url); }));
        this.guard();
        const environment = revision();
        let budget = 4 * 1024 * 1024;
        if (delivery) await delivery.seal(deadline);
        for (const url of delivery ? [] : requests) {
            const requested = new URL(url);
            if (requested.origin !== new URL(this.document.URL).origin || requested.hash || !/^https?:$/u.test(requested.protocol)) throw new Error('Same-origin HTTP(S) image acquisition required');
            // Redirect errors avoid sending a second request to an unowned origin.
            const response = await fetch(url, { signal, credentials: 'same-origin', redirect: 'error', cache: 'no-store' });
            this.guard();
            const contentType = response.headers.get('content-type') ?? '';
            if (!response.ok || !/^image\/(?:png|jpeg|gif|webp|avif)(?:;|$)/iu.test(contentType) || !response.body) throw new Error('Readable raster image acquisition required');
            const reader = response.body.getReader(), chunks: Uint8Array[] = [];
            let size = 0;
            try {
                for (;;) {
                    const part = await reader.read();
                    this.guard();
                    if (part.done) break;
                    budget -= part.value.byteLength;
                    if (budget < 0) throw new Error('Image acquisition byte budget exceeded');
                    size += part.value.byteLength; chunks.push(part.value);
                }
            } finally { await reader.cancel(); reader.releaseLock(); }
            if (!size) throw new Error('Empty image acquisition');
            const bytes = new Uint8Array(size);
            let offset = 0;
            for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
            const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
            this.guard();
            this.run(() => native.addImageResponse(url, response.url, contentType, [...digest].map(n => n.toString(16).padStart(2, '0')).join(''), bytes));
        }
        if (revision() !== environment) throw new Error('Image environment changed during acquisition');
        this.run(() => delivery ? native.recordDeliveredImages(environment, this.observe) : native.recordImages(environment, this.observe));
        this.guard();
        if (!requests.size) return;
        const view = this.document.defaultView;
        if (!view) throw new Error('Live image environment required');
        const queries = new Set([...this.document.querySelectorAll('picture > source[media]')].map(n => n.getAttribute('media') ?? ''));
        queries.add(`(resolution: ${view.devicePixelRatio}dppx)`);
        if (queries.size > 256) throw new Error('Image media budget exceeded');
        for (const query of queries) {
            const media = view.matchMedia(query);
            media.addEventListener('change', this.revoke); this.#media.push(media);
        }
    }
    dispose(): void {
        this.#abort.abort();
        for (const media of this.#media) media.removeEventListener('change', this.revoke);
        this.#media.length = 0;
    }
}
