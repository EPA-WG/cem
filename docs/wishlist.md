# Wishlist

This file tracks future capability ideas that are not part of the immediate release queue. Active execution tasks live
in [`todo.md`](todo.md).

## Rust URL compatibility gaps

User decision, 2026-09-25: keep the Rust parser/toolchain; do not pursue Ada or
add C++/WASI SDK build requirements. Keep these uncovered cases as future
compatibility work. Active implementation remains in [todo.md](todo.md).
The raw-parser baseline is `url` 2.5.8: eight selected WPT cases have 11
field/outcome differences. The Rust origin adapter now fixes the four blob cases
at the native core layer; query registration/integration remains pending. The
pinned upstream Rust revision fixes only case 664; it has not been adopted.
The scoped local parser patch additionally fixes case 138, leaving three native
parse gaps. The table retains raw-parser failures and records native fixes.

The table evaluates consequence and likely relevance, not measured frequency:
we have no application-input telemetry. Priorities are engineering judgments.
All inputs below use no base URL. “Expected” means the unchanged pinned WPT
expectation, consistent with the accepted pure origin profile.

| Done | WPT case / input | Expected → raw Rust 2.5.8 result | Importance and follow-up |
| --- | --- | --- | --- |
| [x] | 782: `blob:blob:https://example.org/` | Origin `null` → `https://example.org` | High semantic priority: recursively invents a tuple origin for a nested blob. Fixed in the native pure-origin adapter; query exposure pending. |
| [x] | 786: `blob:ftp://host/path` | Origin `null` → `ftp://host` | High semantic priority: reports a tuple origin outside the allowed blob inner schemes. Fixed in the same native adapter as 782; query exposure pending. |
| [x] | 787: `blob:ws://example.org/` | Origin `null` → `ws://example.org` | High semantic priority: same origin classification issue. Fixed in the same native adapter as 782; query exposure pending. |
| [x] | 788: `blob:wss://example.org/` | Origin `null` → `wss://example.org` | High semantic priority: same origin classification issue. Fixed in the same native adapter as 782; query exposure pending. |
| [ ] | 664: `file://[1::8]/C:/` | Href unchanged, host/hostname `[1::8]` → `file:///C:/`, empty host/hostname | High consequence if used for file resolution: silently loses the remote authority. Specialized file/drive combination; prioritize before relying on this class of file URLs. Fixed in the probed upstream Rust revision; prefer a released upstream fix when available. |
| [x] | 138: `file:///w\|/m` | Href `file:///w:/m`, pathname `/w:/m` → vertical bar retained in both | Medium for legacy Windows file interoperability; low for ordinary web links. Canonical identity and downstream path handling can differ. Fixed by the scoped Rust file path-start patch; native core coverage passes, query exposure pending. |
| [ ] | 839: `file://xn--/p` | Parse succeeds, href unchanged, host `xn--` → rejected | Low practical priority: empty punycode label compatibility. Rejection prevents a result instead of silently changing its destination. Revisit with upstream IDNA work. |
| [ ] | 921: `https://xn--/` | Parse succeeds, href unchanged, origin `https://xn--` → rejected | Low practical priority: same empty-label issue for HTTPS. Parsing acceptance is separate from DNS resolution or reachability; do not generalize this gap to normal internationalized domains. |

The blob inputs are specialized too, but the error is in a field callers may
use for grouping or origin comparisons; that makes their semantic consequence
more important than frequency alone. No demonstrated CEM authorization bypass
is claimed: the origin core is not yet registered as a query function, and this
pure API does not itself load resources or authorize requests. The four blob
results are fixed by bounded Rust adapter work without a parser fork; all 11
selected WPT blob-origin expectations pass through the adapter. The other four
cases remain parser compatibility debt; retaining them here does not justify claiming
full WHATWG conformance or changing their expected results.

Recommended order after the completed native origin adapter: track the
upstream file-host fix; address drive normalization when file interoperability
requires it; leave empty-punycode compatibility last. Record any remaining
limitations when the production URL API is introduced. This decision selects
Rust and defers compatibility work; it does not authorize a maintained fork or
silently redefine the target contract.

Evidence: [pinned WPT cases and exact differences](../packages/cem_ql/fixtures/url/README.md),
[upstream comparison](../packages/cem_ql/fixtures/url/parser-probe/README.md), and
[accepted URL contract](cem-ql-url-contract.md). The relevant standard algorithms
are [origin](https://url.spec.whatwg.org/#origin),
[file parsing](https://url.spec.whatwg.org/#file-state), and
[domain-to-ASCII](https://url.spec.whatwg.org/#concept-domain-to-ascii).

## Rust URL setter compatibility gaps

The next native probe finds **22/277 WPT component cases differing across 36
fields** in the seed-parse/setter path of `url` 2.5.8. These are separate from
the eight parse cases above. Keep Rust. No production setter API or broader
compatibility waiver is introduced. Exact inputs, expected/actual values and
provenance are in the [setter evidence](../packages/cem_ql/fixtures/url/SETTERS.md).
Indices are zero-based within each pinned setter array. Priorities below are
engineering judgments about consequences, not measured input frequency.

| Done | Cases | Importance and suggested action |
| --- | --- | --- |
| [x] | port[26] | Fixed in the native adapter: whitespace-only assignment preserves the existing port and reports an ignored port; truly empty input still clears it. All 27 pinned port cases pass. Query exposure pending. |
| [x] | pathname[24], [25], [26] | Fixed in the native adapter: serialize the normalized hostless components with the required `/.` guard. Reparse preserves the absent authority, path, query and fragment. Query exposure pending. |
| [x] | pathname[21], [22], [23] | High consequence for file consumers, specialized inputs: repeated slash/path segments are lost. Fixed by the scoped Rust parser patch: expected slash segments now survive parsing, setters and round trips. Query exposure pending. |
| [x] | search[10], [11], [12], [13]; hash[16], [17], [18], [19] | High consequence for opaque payloads, specialized inputs: trailing spaces are lost or not encoded as expected. Fixed at the opaque parsing boundary: encode the final space before query/fragment so later clearing preserves it. Query exposure pending. |
| [x] | hostname[34], [35] | Medium: adding authority retained an extra serialization guard. Fixed in the scoped Rust dependency; host and hostname updates preserve path, query, fragment and existing ports. Query exposure pending. |
| [x] | pathname[27] | Fixed in the native adapter: remove the stale hostless `/.` guard when the replacement path no longer starts with `//`. Query exposure pending. |
| [x] | pathname[5] | Medium for custom schemes: empty path became `/`, changing the identifier. Fixed by distinguishing empty authority from absent authority. Query exposure pending. |
| [x] | pathname[13] | Lower priority: caret encoding affected interoperability and exact href comparisons. Fixed the path percent-encode set; query, fragment and opaque payload carets stay intact. Query exposure pending. |
| [ ] | host[59]; hostname[41] | Low: empty-punycode compatibility already affects parsing; setters preserve the old host instead of accepting `xn--`. Track together with parse cases 839/921. |
| [x] | Authored: equal `file:` and HTTPS-to-file protocol assignments | Medium: raw setter rejects an equal valid request (false warning risk) and ignores an eligible conversion. Fixed scheme preconditions now accept equal file assignments and eligible conversions; invalid transitions still report ignored protocol. Query exposure pending. Outside the 277-case count. |
| [x] | Authored: partial host-port and opaque pathname outcomes | High for the accepted diagnostics: a successful host return hides an ignored port; a void pathname setter hides inapplicability. Native outcomes now report ignored host.port while preserving hostname effects, and report opaque pathname inapplicability. Source-mapped warning integration remains pending. Outside the 277-case count. |

The user chose to fix the Rust setter path before exposing assembly/updates.
The bounded adapters and compatibility patch fix 20 cases;
two of 277 cases (six field differences) remain. Raw registry characterization stays
unchanged. The user accepted the file parser patch; it is now a private CEM-QL
path dependency with [provenance and maintenance notes](../vendor/url/CEM-PATCH.md).
Remaining empty-punycode host updates stay with the deferred IDNA cases.
Seeded assembly and immutable updates now implement the accepted fixed field
order in the native core, with ordered warning metadata and strict validation.
The five parse/assembly query functions now register source-mapped errors and
ordered setter warnings. Next: parameter query registration and broader runtime
parity, tracked in
[todo.md](todo.md).

## Distribution and Publication

- [ ] **CEM URL fork and dependent Cargo releases.** Deferred by the user on
      2026-09-25 into release work. Publish the reviewed `cem-url` release before
      any `cem-ql` crate release that depends on it: confirm registry name and
      maintainer access, rerun the clean archive gates, publish, verify exact
      registry resolution in a clean consumer, then verify/package dependent
      crates. Follow [the fork release checklist](../vendor/url/RELEASING.md)
      and [the Cargo release prerequisite](cem-ml-deployment-contract.md#cargo-source-release-prerequisite).
      This is not a blocker for ordinary development or local package checks.

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

- [ ] **Progressive document loading over CEM-ML AST streams (accepted later phase).**
      User direction, 2026-09-17: after the materialized
      [loader custom element](cem-data-loader-plan.md) and migrated samples are
      verified, support progressive typed CEM-ML AST chunks/events through the
      same resource lifecycle. Define backpressure, bounded retention, node/source
      identity across chunks, partial-input errors, cancellation/disposal and
      transactional updates for the active revision. Identify incremental consumers
      and explicitly materialize for queries requiring the whole tree. Cover native,
      WASM and browser parity. HTTP byte streaming alone does not satisfy this item;
      no JavaScript JSON-object document representation is permitted.
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

- [ ] **Context-aware `url(#id)` substitution in CSS and template bodies.**
      Automatically bind local references to generated DOM owned by the correct
      DCE declaration, instance, or repeated template context, including matching
      target IDs and HTML/SVG references. Preserve external-template provenance
      and closest-scope module-map resolution for explicit external URLs.
      Design owner identity admission, shared stylesheet resource slots,
      missing/ambiguous targets, nested ownership, and browser/worker/SSR
      hydration and lifecycle behavior before implementation. The
      [deferred binding design](scoped-css-fragment-bindings.md) records prior
      proposals. For now authors supply matching IDs and complete URL-valued
      CSS custom properties explicitly; no automatic `#id` substitution.
- [ ] **Dynamic internal `<textarea>` merge and hydration handling.** Deferred out of the immediate release queue.
      Implement and cross-browser validate the hidden child-node merge model plus explicit `.value` projection, including
      SSR loader conversion from a loader-friendly `<xsl:element name="textarea">`-style or equivalent CEM-ML placeholder
      form.
- [ ] assure `$document` in scope
- [ ] named scope
