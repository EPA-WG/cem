# cem-url

CEM-maintained fork of upstream `url` 2.5.8, with scoped WHATWG URL
compatibility fixes used by CEM-QL. This is a separately released package;
`cem-url` 0.1.0 is its initial release candidate, not an upstream `url` release.
It retains the Rust library name `url` and the upstream MIT OR Apache-2.0 license.

```toml
[dependencies]
url = { package = "cem-url", version = "=0.1.0" }
```

The registry dependency above becomes usable after publication. CEM's workspace
uses the same exact version with a local path during development.

```rust
use url::Url;
let url = Url::parse("https://example.test/docs").unwrap();
assert_eq!(url.host_str(), Some("example.test"));
```

Read [CEM-PATCH.md](CEM-PATCH.md) for provenance, the runtime patch and fixture
history. Known compatibility gaps remain; this package does not claim complete
WHATWG conformance. [README.upstream.md](README.upstream.md) retains upstream
project information. [UPSTREAM-Cargo.toml](UPSTREAM-Cargo.toml) preserves the
original upstream manifest independently of Cargo's generated `Cargo.toml.orig`.

The maintained code lives in `vendor/url` in the
[CEM repository](https://github.com/EPA-WG/cem/tree/develop/vendor/url).
Report fork issues there. See [RELEASING.md](RELEASING.md) for independent
versioning, archive verification and dependency-first publication order.
