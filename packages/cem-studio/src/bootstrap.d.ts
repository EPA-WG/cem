export interface CemStudioMountOptions {
    root?: Element | string;
    baseUrl?: string | URL;
}

export interface CemStudioMount {
    readonly root: Element;
    readonly baseUrl: URL;
    dispose(): void;
}

export function mountCemStudio(options?: CemStudioMountOptions): CemStudioMount;
export function loadCemMlBrowser(): Promise<typeof import('@epa-wg/cem-ml-cli/browser')>;
export function registerCemStudioServiceWorker(options?: RegistrationOptions): Promise<ServiceWorkerRegistration>;

export * from './repository.js';
export * from './shell.js';
