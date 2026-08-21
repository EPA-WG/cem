const DEPLOYMENT_INVENTORY = new URL('./cache-inventory.json', self.location.href).href;

self.addEventListener('message', (event) => {
    if (event.data?.type === 'cem-studio-deployment-inventory') {
        event.source?.postMessage({
            type: 'cem-studio-deployment-inventory',
            url: DEPLOYMENT_INVENTORY,
        });
    }
});

// Installation, Cache Storage ownership, activation, and update coordination
// are intentionally added by the later PWA-shell checklist item.
