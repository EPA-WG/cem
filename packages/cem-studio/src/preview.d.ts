export interface CemStudioLimits {
    readonly sourceBytes: number;
    readonly dependencyBytes: number;
    readonly resourceSetBytes: number;
    readonly resourceCount: number;
    readonly resultBytes: number;
    readonly inlinePreviewBytes: number;
    readonly structuredRows: number;
}

export type CemStudioPreview =
    | Readonly<{
          kind: 'sandboxed-html';
          label: string;
          contentType: string;
          byteLength: number;
          displayedBytes: number;
          truncated: false;
          source: string;
      }>
    | Readonly<{
          kind: 'text';
          label: string;
          contentType: string;
          byteLength: number;
          displayedBytes: number;
          truncated: boolean;
          text: string;
      }>
    | Readonly<{
          kind: 'download';
          label: string;
          contentType: string;
          byteLength: number;
          reason: string;
      }>;

export declare const CEM_STUDIO_LIMITS: CemStudioLimits;
export declare const CEM_STUDIO_PREVIEW_CSP: string;

export declare class CemStudioLimitError extends Error {
    readonly code: string;
    readonly details: Readonly<Record<string, unknown>>;
}

export declare function assertCemStudioSourceSet(
    source: ArrayBuffer | ArrayBufferView | readonly number[],
    dependencies?: readonly { readonly bytes: ArrayBuffer | ArrayBufferView | readonly number[] }[],
): Readonly<{ sourceBytes: number; resourceCount: number; totalBytes: number }>;
export declare function assertCemStudioResultSize(
    value: number | ArrayBuffer | ArrayBufferView | readonly number[],
): number;
export declare function assertCemStudioResourceUri(value: string, expectedScheme: string): string;
export declare function createCemStudioPreview(options: {
    bytes: ArrayBuffer | ArrayBufferView | readonly number[];
    contentType?: string;
    label?: string;
}): CemStudioPreview;
export declare function mountCemStudioPreview(root: Element, preview: CemStudioPreview): Element;
export declare function redactCemStudioSecrets(value: unknown): string;
