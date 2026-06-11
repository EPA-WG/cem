export * from './lib/cem-elements.js';
export * from './lib/legacy-xslt/contract.js';
// The CEM-owned legacy HTML+XSLT compiler (cem_ml engine via the cem_ql WASM module), shared by the
// browser runtime, SSR, and fixture gates.
export { convertLegacyTemplate, type LegacyConvertResult } from './lib/internal/runtime-support/cem-ql-render.js';
