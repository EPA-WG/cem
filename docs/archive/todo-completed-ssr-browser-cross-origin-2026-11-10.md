# Cross-origin SSR browser read grants

Base commit `c0ec2784e8b236e84c3d3c6a8d301e1bf091b83d`.

- [x] Accept trusted `ssrCrossOriginReads` grants scoped to the actual page
  origin, exact public URL and media type. Keep the grants outside serialized
  declarations, payloads, and stylesheet receipts.
- [x] Verify bounded CORS responses for declarations, CEM-ML payloads, and
  stylesheet imports. Omit credentials and referrers and refuse redirects.
  Require the exact browser grant for off-origin bytes in native CSS replay.
- [x] Test two real Chromium origins. Check in-place adoption, preserved output
  on refusal, denied URL and MIME grants, missing CORS, partial or changed
  responses, and missing receipts during streamed HTML parsing.

Native browser admission and stripped SSR suites passed. Chromium covered 95
cases, including 15 cross-origin grant cases, with zero unexpected off-origin
requests.
