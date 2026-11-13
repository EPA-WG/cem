// Trusted bootstrap only. Authored processing requests never carry compiled code.
// eslint-disable-next-line @nx/enforce-module-boundaries -- generated build-owned code pins are packaged with the native bindings.
export { CEM_QL_CODE_IDENTITY, assertCemQlCodeIdentity, loadCemQlWasmModule, installCemQlWasmModule } from '../../../../../cem_ql/dist/wasm/cem_ql_code.js';
export const CEM_NATIVE_CODE_PROTOCOL = 'cem-render-code-v1' as const;
