# Versioning and compatibility policy

This policy governs the public CEM product surface. It complements the detailed
CEM-ML compatibility rules in `content-type-switch.md` and the release
procedures in `npm-publish.md`; it is the product-wide decision point when a
change affects more than one package or projection.

## Release families

| Family | Version authority | Relationship | Members |
| --- | --- | --- | --- |
| CEM web | root `package.json` | fixed | `@epa-wg/cem`, `@epa-wg/cem-theme`, `@epa-wg/cem-components`, `@epa-wg/cem-elements`, `@epa-wg/custom-element` |
| CEM-ML platform | `packages/cem_ml/Cargo.toml` | fixed, exact internal dependencies | `cem_ml`, `cem_ml_cli`, `@epa-wg/cem-ml`, `@epa-wg/cem-ml-cli`, `@epa-wg/cem-studio`, and the governed native deployment projects |
| Schema packages | each schema descriptor | independent SemVer identity | the versioned CEM-ML schema packages and generated schema artifacts |
| Trang native | `packages/trang-native/package.json` | independent | `@epa-wg/trang-native` and its platform archives |

Every member of a fixed family is versioned together even when only one member
changes. CEM-ML npm, Studio, and native release metadata use the exact common
CEM-ML version; ranges are not allowed across that runtime boundary. Studio may
consume a separately released CEM web family, but those dependencies are exact
so a built Studio release has one reproducible UI stack.

The repository is pre-1.0. A `0.MINOR` release may contain an incompatible
change, but the deprecation, migration, evidence, and release-note requirements
below still apply. Moving to 1.0 strengthens the usual SemVer major boundary; it
does not change the family topology.

## Governed compatibility axes

### Token names and values

The Markdown token tables are authoritative. Adding an optional or recommended
token is compatible; adding a required token is minor unless every supported
consumer can ignore it. Renaming or removing a public token, changing its type,
or changing an alias to a different semantic meaning is breaking. Value
corrections are patch changes only when the documented semantic meaning is
unchanged; intentional visual-system changes are minor and require before/after
release notes.

Deprecated tokens remain in the source manifest for at least one published
minor and appear in the generated token report. Debug artifacts are never
public contracts. Generated JSON, Swift, Android, and Figma files inherit the
owning `@epa-wg/cem-theme` version and may not create independent token values.

### Component APIs

Public component names, attributes, properties, events, slots/content roles,
form behavior, states, keyboard interaction, accessibility semantics, styles,
and catalog records are one API surface. Additive components and optional
features are minor. A removal, rename, changed event/state meaning, reduced
keyboard or accessibility behavior, or incompatible required markup is
breaking. CSS-only corrections are patch changes when they preserve documented
layout and state semantics.

Application code must not bypass a shared component to avoid this policy. Add
or complete the reusable CEM primitive first, then update its catalog, examples,
state matrix, tests, migration notes, and release evidence together.

### XML and CEM-ML schemas

Schema identity is the stable URI plus the descriptor's complete SemVer. A URI
tail is a constraint, not the authority. Compatible minor/patch schemas retain
existing content, use the documented open-content behavior, and report the
resolved embedded version. An incompatible grammar, required-member, namespace,
or semantic change advances the schema's breaking version. Application run mode
rejects unsupported forward or major-incompatible content; development mode may
offer explicitly documented diagnostics and migration assistance.

Generated schema outputs carry their source descriptor version and provenance.
Changing a generator without changing a governed schema contract is permitted
only when byte or presentation differences do not alter accepted documents or
the typed consumer surface.

### Native token outputs

The installable Swift Package, Android library, and standalone compatibility
copies are projections of `@epa-wg/cem-theme`. Their public token names, types,
mode behavior, package/module names, minimum platform versions, and install/copy
layouts follow the theme version. Removing a compatibility copy, changing a
module/package name, raising a minimum platform, or producing a source-breaking
native identifier is breaking. Toolchain pin updates are minor when generated
consumer source remains compatible and the supported-host compile gates pass.

Android 17/API 37 is currently a preview SDK distribution constraint; it is not
a promise that CEM targets a preview runtime. The generated app target remains
independently governed from the compile SDK.

### CEM-ML runtime, CLI, Studio, and Linux native unit

`packages/cem_ml/Cargo.toml` is authoritative. The runtime, command schema,
Node/browser CLI, Studio build metadata, cache inventory, Linux package,
checksums, signatures, SBOMs, provenance, capability output, and release index
must report that exact version and tagged source commit. Portable commands keep
normalized result, diagnostic, report, source-map, and exit behavior parity;
documented host capability differences are not permission to change the command
contract.

The GitHub Release contains exactly the two npm/WASM units and Linux AMD64 unit.
macOS ARM64 and Windows AMD64 remain wishlist/local-host work and are not release
units. Published release bytes are immutable; correction requires a new version
and tag.

## Deprecation and removal

Every deprecation has an owner, introduction version, replacement, searchable
registry entry or generated report row, migration instructions, and earliest
removal version. A supported form is deprecated in a published minor before a
breaking release may remove it. Removal must prove the governed usage inventory
is empty or explicitly migrated. Security-critical behavior may be disabled
sooner, but the release notes must identify the exception and safe replacement.

Deep `dist/` imports, debug token artifacts, undocumented package paths, and
generated implementation details receive no compatibility promise. A public
path becomes stable only through the package export map and its package verifier.

## Release evidence

A Phase 9 release is coherent only when one evidence record connects:

- source commit, family versions, tags, changelog, and migration/deprecation report;
- package archives, export maps, clean installs, docs links, and executable examples;
- token reports, component catalogs/tests, Studio deterministic output/update metadata,
  and supported-host Swift/Android compiles;
- CEM-ML release assets, checksums, signatures/attestations, SBOMs, provenance,
  capability identity, and Linux install/upgrade/uninstall smoke results; and
- registry and GitHub Release URLs proving that the verified bytes, rather than a
  rebuild, were published.

The credential-free readiness target may pass before publication. The Phase 9
closure target is deliberately separate and fails until immutable public
evidence is checked in and validated.
