export interface SchemaAssetPackageSource { readonly packageId: string; readonly version: string; readonly path: string; readonly bytes: Uint8Array }
export type SchemaAssetPackageOperation =
    | { readonly operation: 'bootstrap' | 'release' }
    | { readonly operation: 'summary'; readonly index: Uint8Array; readonly pin: string }
    | { readonly operation: 'import'; readonly index: Uint8Array; readonly pin: string; readonly source: SchemaAssetPackageSource }
    | { readonly operation: 'activate'; readonly index: Uint8Array; readonly pin: string; readonly sources: readonly SchemaAssetPackageSource[]; readonly selection: string };
export type SchemaAssetPackageRequest = SchemaAssetPackageOperation & { readonly type: string; readonly actionId: number };

export declare function parseSchemaAssetPackageRequest(value: Record<string, unknown>, type?: string): SchemaAssetPackageRequest;
