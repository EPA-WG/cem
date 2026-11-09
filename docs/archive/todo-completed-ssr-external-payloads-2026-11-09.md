# Mapped external CEM-ML payloads for SSR

Base commit `921e959e4e098669c92677372049afeaaad2045f`.

- [x] Admit an empty external CEM-ML payload envelope through the existing
  resolver policy and inert native parser. Bound each read by the remaining
  page resource and byte allowances and a 32 KiB source ceiling.
- [x] Bind the parsed source to its exact authored envelope and operation.
  Keep public URI and byte ranges in the native tree and HTML source map.
  Charge reads and parsing to the same CPU phase as participant preparation.
- [x] Prove per-read accounting and resource identity in readiness. Verify
  refusal of malformed, mixed, oversized and unmapped sources without
  replacing prior HTML, reports or source maps.
- [x] Keep the SSR range in Chromium when the payload only has server-side
  proof. The browser made zero payload requests.

Verification covered the CLI and native source and export suites, the stripped
SSR suites, and 52 Chromium scenarios.
