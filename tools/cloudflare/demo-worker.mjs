/** Preserve browser module URLs while using the asset service's canonical path. */
export default {
    fetch(request, environment) {
        const url = new URL(request.url);
        if (!url.pathname.includes('@')) return environment.ASSETS.fetch(request);
        url.pathname = url.pathname.replaceAll('@', '%40');
        return environment.ASSETS.fetch(new Request(url, request));
    },
};
