# Native CLI acquisition for external stylesheet imports

Base commit `c4f1db84c58d85ebb5b8eec4074f8b0aec23a676`.

- [x] Give current stylesheet tickets a bounded synchronous CLI read. Direct
  commands and config graphs accept explicit public-URL read maps. The
  first import resolves from the effective document base and nested imports
  resolve from the admitted stylesheet URL.
- [x] Use the original SSR operation for acquisition and native CSS parsing.
  Check the remaining byte allowance, cancellation, elapsed time and thread
  CPU during the read. Close the acquisition phase before delivering bytes
  to the coordinator's measured shared importer.
- [x] Verify scoped CSS in Chromium and atomic CLI publication. Cover nested
  imports, policy denial, CPU exhaustion, byte and wall-time limits, parser
  refusal, unmapped URLs and read-map path escapes.

An explicit read map selects regular files beneath its configured root.
The SSR resolver purpose is separate from ordinary CLI input reads. The
readiness proof retains public request and response identities and content
hashes. Changing stylesheet bytes changes the final render-input identity.
Moving a read-map directory preserves it when the public URLs and bytes agree.
Repeated responses in one page must agree on content before publication.

CLI publication covers external CSS imports. Browser adoption of
resource-dependent CSS, live network SSR, external declaration and payload
adapters, and initial page-source CPU accounting have separate admission
boundaries.

Native verification passed eight CLI SSR cases, six stylesheet project
regressions, seven named-export cases per adapter profile, three stripped
artifact unit cases, 129 stripped SSR cases and the bounded-reader unit test.
The native CLI build and lint passed.
The optimized package passed 48 of 48 CLI-to-Chromium cases with zero external
requests, including the mapped CSS scope and adoption refusal. WASM consumer
regressions passed. The source-admission suite passed 318 native checks and
30 browser cases.

The optimized CEM-QL core and packaged WASM match at 24,346,723 bytes with
33 imports and 178 exports. SHA-256 is
`347d04550050d04e77176f8a52bc76c439f07a6c834a1c8b21a792df818c412c`.
