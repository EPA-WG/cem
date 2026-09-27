export * from './lib/cem-elements.js';
export * from './lib/repository.js';
export {
    CemDeclarationScopeError,
    createCemDeclarationScope,
    getDefaultCemDeclarationScope,
    type CemDeclarationScope,
    type CemDeclarationScopeErrorCode,
    type CemDeclarationScopeOptions,
} from './lib/declaration-scope.js';
export * from './lib/legacy-xslt/contract.js';
// The CEM-owned legacy HTML+XSLT compiler (cem_ml engine via the cem_ql WASM module), shared by the
// browser runtime, SSR, and fixture gates.
export { convertLegacyTemplate, type LegacyConvertResult } from './lib/internal/runtime-support/cem-ql-render.js';

export { exportNativeCemAttributes, importNativeCemAttributes, exportNativeCemSlices, importNativeCemSlices,
    type NativeCemValue, type NativeCemAttributeBinding, type NativeCemSliceBinding, type CemValueArtifactLimits,
} from './lib/native-values.js';

export { publishEdgeCssDomUpdate, type CemEdgeCssPublicationOptions } from './lib/edge-css-publication.js';
export { DeclarationStyleOwnership } from './lib/declaration-style-ownership.js';
export type { CemEdgeStylesheetBatch, CemEdgeStylesheetState, CemEdgeSsrRenderUpdateResult } from './lib/edge-ssr-host.js';
