import { installImageDeliveryTransport } from '../dist/image-delivery-worker.js';
// Simulate an application handler that bypasses the capture transport.
self.addEventListener('fetch', event => {
    if (new URL(event.request.url).pathname === '/recorded-images/bypass.png') {
        event.respondWith(fetch(event.request)); event.stopImmediatePropagation();
    }
});
installImageDeliveryTransport(self);
self.addEventListener('install', event => event.waitUntil(self.skipWaiting()));
self.addEventListener('activate', event => event.waitUntil(self.clients.claim()));
