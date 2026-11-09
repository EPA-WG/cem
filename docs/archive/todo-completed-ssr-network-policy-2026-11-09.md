# Endpoint-pinned SSR network reads

Base commit `11a43bc232559d5af6ec43c0b15ac9c469c14926`.

- [x] Add repeatable `--ssr-network-read URL=IP` grants to direct and config
  CLI SSR. Retain matching local read maps as the first authority.
- [x] Connect exact HTTPS URLs to their pinned endpoints with normal TLS
  verification. Admit plain HTTP on literal loopback only. Refuse redirects,
  ungranted URLs, proxy and DNS fallback, partial responses and unexpected
  response media or content encoding.
- [x] Bound streamed reads and check cancellation and the original wall and
  CPU budgets on the original operation. Bind stable policy identity and
  actual response hashes into readiness.
- [x] Verify network declaration, stylesheet and payload reads against local
  fixtures. Refuse oversized bodies, delayed headers and invalid responses
  while preserving the prior HTML, readiness report and source maps.

Verification covered 22 CLI SSR tests, three transport tests, 40 CLI argument
tests, and the native and stripped SSR suites. A named HTTPS fixture confirmed
that the pinned address receives a TLS handshake.
