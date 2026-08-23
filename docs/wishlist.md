# Wishlist

This file tracks future capability ideas that are not part of the immediate release queue. Active execution tasks live
in [`todo.md`](todo.md).

## Distribution and Publication

- [ ] **CEM web npm family publication.** Publish and remotely verify the fixed
      `@epa-wg/cem`, `@epa-wg/cem-theme`, `@epa-wg/cem-components`,
      `@epa-wg/cem-elements`, and `@epa-wg/custom-element` family from one new
      Nx-selected SemVer tag. Do not reuse the failed historical `0.1.1` tag or
      publish obsolete source; retain the protected contract preflight, exact
      archives, npm provenance, clean-consumer verification, and immutable-byte
      rule if this work is resumed.
- [ ] **Final CEM-ML GitHub Release and npm publication.** Promote the verified
      browser/WASM, Node/WASM, and Linux AMD64 units from one exact CEM-ML source
      commit, then publish the matching `@epa-wg/cem-ml` and
      `@epa-wg/cem-ml-cli` tarballs. Preserve the existing draft-first,
      redownload, checksum, signature, SBOM, provenance, and non-replacement
      requirements.
- [ ] **CEM Studio npm and static/PWA publication.** Publish
      `@epa-wg/cem-studio` and its deterministic static deployment only from the
      same CEM-ML version and source commit as the verified runtime/CLI family;
      remotely verify clean installation, service-worker update metadata, and
      the deployed static digest.
- [ ] **Immutable public distribution evidence.** If any publication work is
      resumed, record registry, GitHub Release, provenance, checksum,
      supported-host native compile, docs/example, static deployment, and
      clean-consumer URLs for the exact released bytes. Public evidence is a
      publication acceptance gate, not a prerequisite for credential-free Phase
      9 roadmap closure.

## CEM-ML Runtime

- [ ] **macOS ARM64 CEM-ML CLI and Homebrew distribution.** Keep the existing dormant
      `native-macos-arm64` projection outside the active roadmap and release matrix. Reconsider it only with an
      Apple-supported build and native execution environment, a pinned macOS SDK/Xcode/Rust toolchain, Developer ID
      signing and notarization, Homebrew install/upgrade/uninstall smoke tests, checksums, SBOM, provenance, and parity
      with the portable Node/WASM CLI and published Linux native CLI.
- [ ] **Windows AMD64 CEM-ML CLI, MSI, and WinGet distribution.** Keep the existing dormant
      `native-windows-amd64` projection outside the active roadmap and release matrix. A raw
      `x86_64-pc-windows-msvc` executable may be cross-built on Linux AMD64 with `cargo-xwin`, `clang-cl`, `lld-link`,
      and the Microsoft CRT/Windows SDK; treat that output only as cross-build and static-analysis evidence, not as
      native Windows validation. Reconsider distribution only with native Windows execution, Authenticode signing,
      MSI and WinGet validation, Windows Sandbox install/upgrade/uninstall smoke tests, checksums, SBOM, provenance,
      and parity with the portable Node/WASM CLI and published Linux native CLI.
- [ ] **Platform-native Linux npm package `@epa-wg/cem-ml-cli-linux-x64`.** Create a separate Linux x64 package for
      the native `cem-ml` executable and let `@epa-wg/cem-ml-cli` select it through an optional platform dependency
      with a deterministic WASM fallback. Keep the ELF binary out of the policy-free `@epa-wg/cem-ml` WASM package,
      synchronize its version with the CEM-ML product family, and cover glibc compatibility, integrity, signing,
      provenance, SBOM, package selection, and clean-consumer installation before publishing it.
- [ ] **Engine XSLT 3.0/4.0 execution behind G-NVDL-FULL (AC-P-6.9).** The architecture keeps XSLT as a
      capability-gated peer language behind explicit dispatch, not the primary authoring/rendering model or a
      browser-native dependency. Building the XSLT 3/4 engine remains out of scope for the current release.
- [ ] **Web-service schema validation.** Extend CEM-ML's generic schema engine to support service-description schemas
      such as OpenAPI/Swagger and GraphQL. Compose service validation with the existing URL-level validation—including
      URL parameter encoding—so web services can participate in a unified web-application validation chain.
- [ ] **`*.cemt.md` authored transform-doc format with HTML output.** Add a Markdown-adjacent CEMT documentation format
      that can embed CEMT modules/examples and transform them into HTML documentation or previews through the CEM-ML
      pipeline.
- [ ] **Advanced import fallback and substitution policies.** Extend the resolver policy model beyond explicit
      one-step substitution to cover ordered fallback lists, offline mirrors, semver/range module replacement,
      dev/prod import maps, stale-cache use when remote imports are unavailable, and trust-tier downgrade/upgrade
      diagnostics. These policies must preserve requested and resolved identity in reports and artifact/cache stamps.

## CEM-QL Language

- [ ] **User-defined overloads.** Allow user-authored declarations such as functions with the same exported name only
      after CEM-QL has a typed signature model that can distinguish arity and parameter/return types deterministically.
      The design must define overload-set encoding in package artifacts, import/export collision rules, ambiguous-call
      diagnostics, and formatter/HTML/example coverage before relaxing duplicate-declaration errors.
- [ ] **First-class generators and user-defined deferred sources.** Extend CEM-QL's existing lazy `ItemStream` and
      `cem:stdlib/sequence` pipeline with one authorable generator/source protocol, not a second sequence model. Use
      [Novatchev's XPath 3.1/4.0 generator design](https://www.balisage.net/Proceedings/vol31/html/Novatchev01/BalisageVol31-Novatchev01.html)
      and its [executable library and 140-plus-expression test suite](https://github.com/dnovatchev/generators) as
      behavioral inputs for immutable state, current/advance/end, finite and unbounded yields, demand-driven providers,
      early termination, and explicit materialization. Define Rust-first CEM-QL source syntax; typed synchronous and
      asynchronous provider contracts; scope cancellation, budgets, source maps, cleanup, and backpressure; deterministic
      end/error behavior; and explicit replayable-source versus one-shot-cursor semantics. Require a functional-parity
      matrix and native/WASM tests against Java [`Iterator`/`Spliterator` and generated, lazy, single-use `Stream`
      pipelines](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/util/stream/package-summary.html),
      including [stateful `Gatherer` operations](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/util/stream/Gatherer.html), and
      .NET [`IEnumerable<T>`/`IAsyncEnumerable<T>` iterator methods](https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/statements/yield)
      plus [deferred LINQ](https://learn.microsoft.com/en-us/dotnet/standard/linq/deferred-execution-lazy-evaluation).
      Cover source construction, one-item pull, existing map/filter/flat-map and take/drop/short-circuit helpers,
      zip/chunk/concat/distinct, fold/scan, and bounded materialization without importing XPath `gn:*` names or record
      syntax.

## CEM Elements Runtime

- [ ] **Dynamic internal `<textarea>` merge and hydration handling.** Deferred out of the immediate release queue.
      Implement and cross-browser validate the hidden child-node merge model plus explicit `.value` projection, including
      SSR loader conversion from a loader-friendly `<xsl:element name="textarea">`-style or equivalent CEM-ML placeholder
      form.
- [ ] assure `$document` in scope
- [ ] named scope
