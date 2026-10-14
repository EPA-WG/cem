/** Private channel to the shared service-worker byte transport. */
export class BrowserImageDelivery {
    #port?: MessagePort;
    #container?: ServiceWorkerContainer;
    #controller?: ServiceWorker;
    #pending?: { resolve: () => void; reject: (reason: Error) => void };
    #closed = false;
    #nonce = '';
    get nonce(): string { return this.#nonce; }
    readonly #changed = () => { this.dispose(); this.revoke(); };
    constructor(readonly document: Document, readonly accept: (url: string, type: string, bytes: Uint8Array) => void,
        readonly revoke: () => void) {}
    async open(deadline: number, replay = false): Promise<void> {
        const container = this.document.defaultView?.navigator.serviceWorker, controller = container?.controller;
        const timeout = Math.floor(deadline - performance.now());
        if (this.#closed || this.#port || !container || !controller || timeout <= 0) throw new Error('Active image delivery service worker required');
        this.#container = container; this.#controller = controller;
        const channel = new MessageChannel(); this.#port = channel.port1;
        container.addEventListener('controllerchange', this.#changed);
        const ready = this.#wait();
        channel.port1.onmessage = event => {
            const data = event.data;
            try {
                this.validate();
                if (data?.kind === 'ready' || data?.kind === 'sealed' || data?.kind === 'configured') {
                    if (data.kind === 'ready') {
                        if (typeof data.nonce !== 'string' || !/^[a-f0-9]{32}$/u.test(data.nonce)) throw new Error('Fresh transport nonce required');
                        this.#nonce = data.nonce;
                    }
                    if (!this.#pending) throw new Error('Unexpected delivery acknowledgement');
                    const pending = this.#pending; this.#pending = undefined; pending.resolve();
                } else if (data?.kind === 'body') {
                    if (!Number.isSafeInteger(data.id) || data.id < 1 || typeof data.url !== 'string'
                        || typeof data.type !== 'string' || !(data.bytes instanceof Uint8Array)
                        || data.bytes.byteLength > 4 * 1024 * 1024) throw new Error('Invalid image byte ingress');
                    const accepted: unknown = this.accept(data.url, data.type, data.bytes);
                    if (accepted !== undefined) throw new Error('Synchronous native receipt acceptance required');
                    this.validate(); channel.port1.postMessage({ kind: 'ack', id: data.id, ok: true });
                } else throw new Error('Image delivery transport failed');
            } catch { this.#changed(); }
        };
        channel.port1.onmessageerror = this.#changed;
        channel.port1.start();
        const timer = setTimeout(this.#changed, timeout);
        controller.postMessage({ kind: 'cem-image-delivery-open', url: this.document.URL, timeout, replay }, [channel.port2]);
        try { await ready; this.validate(); } finally { clearTimeout(timer); }
    }
    #wait(): Promise<void> {
        if (this.#pending) throw new Error('Image transport command already pending');
        return new Promise((resolve, reject) => { this.#pending = { resolve, reject }; });
    }
    async configure(routes: readonly (readonly [string, string])[], deadline: number): Promise<void> {
        this.validate();
        const done = this.#wait();
        const timer = setTimeout(this.#changed, Math.max(0, deadline - performance.now()));
        this.#port?.postMessage({ kind: 'configure', routes });
        try { await done; this.validate(); } finally { clearTimeout(timer); }
    }
    async seal(deadline: number): Promise<void> {
        this.validate();
        const done = this.#wait(), timeout = deadline - performance.now();
        const timer = setTimeout(this.#changed, Math.max(0, timeout));
        const port = this.#port;
        if (!port) throw new Error('Image transport channel missing');
        port.postMessage({ kind: 'seal' });
        try { await done; this.validate(); } finally { clearTimeout(timer); }
    }
    validate(): void {
        if (this.#closed || this.#container?.controller !== this.#controller) throw new Error('Image delivery transport lifetime ended');
    }
    dispose(): void {
        if (this.#closed) return;
        this.#closed = true; this.#container?.removeEventListener('controllerchange', this.#changed);
        this.#pending?.reject(new Error('Image delivery transport closed')); this.#pending = undefined;
        this.#port?.postMessage({ kind: 'close' }); this.#port?.close();
    }
}
