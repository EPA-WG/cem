import { applicationNetworkLease } from './application-resources.js';

export interface CemApplicationSocketEvents {
    open?(): void;
    message(data: string | Blob): void;
    closed(reason: 'closed' | 'grant-revoked'): void;
}

/** Only the broker retains the native socket and its implicit credential authority. */
export function openCemApplicationWebSocket(consumer: Element, address: string, events: CemApplicationSocketEvents) {
    const lease = applicationNetworkLease(consumer);
    const host = consumer.ownerDocument.defaultView;
    if (!lease || !host) throw new DOMException('Application boundary required', 'NotAllowedError');
    const url = new URL(address, consumer.ownerDocument.baseURI);
    lease.check(url, 'GET', 'include', false, 'websocket');
    const socket = new host.WebSocket(url);
    let finished = false;
    function finish(reason: 'closed' | 'grant-revoked') {
        if (finished) return;
        finished = true;
        lease!.signal.removeEventListener('abort', revoke);
        socket.onopen = socket.onmessage = socket.onclose = null;
        socket.close();
        if (lease!.ownsConsumer()) events.closed(reason);
    }
    function revoke() { finish('grant-revoked'); }
    function admitted() {
        if (finished) return false;
        try { lease!.current(); return true; }
        catch { revoke(); return false; }
    }
    lease.signal.addEventListener('abort', revoke, { once: true });
    socket.onopen = () => { if (admitted()) events.open?.(); };
    socket.onmessage = event => { if (admitted()) events.message(event.data); };
    socket.onclose = () => finish('closed');
    return Object.freeze({
        send(data: string | ArrayBuffer | Blob) {
            if (!admitted()) throw new DOMException('Socket grant unavailable', 'NotAllowedError');
            socket.send(data);
        },
        close() { finish('closed'); },
    });
}
