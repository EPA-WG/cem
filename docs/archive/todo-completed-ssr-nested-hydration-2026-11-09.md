# Nested browser adoption of canonical SSR islands

Base commit `4103bae2c03032e4b4d8dfb187644314874abe36`.

The browser captures the outer marked host and its nested participants in one
native DOM import. Native admission replays the current declaration at each
participant. The parent's projection identifies the child's position and its
expected host attributes and HTML payload. The child's canonical island and
retained range must match. Capture keys bind the resulting decisions to the
actual browser hosts.

The browser checks all connections, declarations, ranges and instance IDs before
publishing. It prepares every input mirror before adding render metadata, which
also covers a child host serving as the parent's rendered root. The group keeps
its nodes and connects every participant after the proof. An invalid child,
incompatible group version or stale capture freezes the group. A standalone
participant still uses its version fallback when its canonical payload is valid.

The scope is passive inline CEM-ML with HTML payloads. The next increment is
scoped stylesheet admission, including declaration and instance ownership and
reuse of the server's style nodes.

The native admission tests passed 7/7 and the stripped SSR suite passed 123/123.
The optimized package passed 19/19 CLI-to-Chromium cases, including 10 nested
cases, with zero external requests. Its 24,093,120-byte WASM matches the core
and stays below the 26,214,400-byte limit. The interface has 28 imports and
177 exports. The core SHA-256 is
`5dadf74b2f118a60cb56d75c0e42d6bf951f68c4955da42375a4e597061bcda6`.

The source-admission gate passed 312 native checks and 30 browser cases. Four
legacy hydration stories passed, along with Clippy, browser lint, and the
template cache and stylesheet WASM consumer checks.
