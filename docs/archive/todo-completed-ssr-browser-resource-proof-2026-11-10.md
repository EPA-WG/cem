# Browser verification of external SSR resources

Base commit `6e394b8e3c328bf40d3ff55d7f92d2581c7bc969`.

- [x] Publish bounded receipts from the native readiness proof for standalone
  passive declarations and external CEM-ML payloads. Record the public URL,
  MIME, byte length and BLAKE3 digest.
- [x] Verify fresh same-origin responses and use the native adapters to bind
  the verified bytes to the live declaration and payload island before range
  comparison. Preserve the server-rendered nodes and marker on refusal.
- [x] Verify native publication and admission. Exercise matching, missing,
  changed, redirected, and tampered resources in Chromium and compare node
  identity on both adoption and refusal.
