# Caller namespaces for native XPath source operations

- [x] Reproduce missing source-loader bindings with native static-context tests.
- [x] Retain caller namespaces/default element namespace on the native owner.
- [x] Verify original source, identity precedence, request isolation and refusals.
- [x] Verify CLI flags and saved structured input scopes across source operations.
- [x] Preserve nontrivial validation argv and dispatch selected source validation.
- [x] Verify deployed command repair, independent copies and offline execution.

## Native contract

The XPath lifecycle loader now creates a `StandaloneStaticContext` attachment
from the effective input identity's named namespaces and default namespace,
matching the existing query-source boundary. Explicit input identity retains
its established precedence over root-scope fallback; no namespace maps are
merged across unrelated requests. The attachment owns its binding snapshot.

Bindings resolve native names without changing prefix spelling, token text,
Unicode/CRLF byte positions or source maps. The default namespace applies to
element names, leaving unprefixed attributes and the default function namespace
alone. Variable/function binding tables remain empty. A namespace URI does not
grant resource access: aliased resource-sensitive calls still refuse, missing
prefixes retain their native diagnostics/ranges, and invalid CLI scope bindings
remain errors. No AST is serialized or reconstructed through JSON.

CLI `--namespace`, `--default-namespace` and structured input `rootScope` values
reach validation, Parse/Inspect and source export through this same loader.
Native tests inspect resolved names directly and verify independent maps over
the same lexical source. CLI tests also remove/restore a saved configuration's
binding and check that resource-policy failure leaves existing output intact.

## Ordinary Studio configurations

The XPath project adds `validate-join`, `parse-join` and `format-join`, reusing
the original prefixed supplier query and existing shared controls. Its saved
commands carry `c=urn:example:catalog`; namespace changes belong to each command,
not to the source resource or another project/configuration.

The example generator previously discarded advanced validation arguments when
emitting an identity-only run config. It now retains that legacy representation
only for exactly representable basic validation, including declared schema
packages. Other configurations preserve authored argv in the existing native
command encoding. Existing basic configuration resources remain stable, and
the generator does not implement command grammar in JavaScript.

Studio's persisted-validation service previously validated only source identity.
For a selected single-input validation/check command targeting the editor's
source, it now invokes the existing shared authored-command executor. Bare files,
command resources themselves and other selected resources retain independent
source validation. This is non-UI service orchestration; no markup, event
binding or new component JavaScript behavior is introduced.

## Verification

- Native XPath cohort: **119 passed**, including the two static-context
  regressions, existing typed source projections and evaluator/XSLT behavior.
- CLI XPath source cohort: **8 passed**, including caller flags, structured
  configuration changes, original diagnostics and unchanged output on refusal.
- Generator/catalog audit unit tests: **24 passed**; the new regression first
  reproduced the discarded namespace argument.
- Configured-validation workbench regression first reproduced the unused
  command and now passes; the final browser validation/authored/persisted/source
  cohort has **21 passing cases**.
- Full Studio build and 21 dependencies, type checking and lint pass. The shared
  component gate still records 59 declarative components with no behavior JS.
- Catalog contract unit tests: **23 passed**. The catalog audit records 213
  authored configurations and 218 documented workflows across 22 authored
  projects and 32 native packages.
- All **250 native configuration receipts** meet their declared contracts:
  185 clean commands, 30 verified outputs, 33 intentional diagnostic cases and
  two known Parse limits. Existing companion diagnostic limits remain qualified;
  this is not a uniform source-conformance claim.
- Credential refusal: passed with both worker and fallback runtimes.
- Deployed CEM-QL and XPath acceptance passes. The XPath checks run all source
  workflows, remove/restore the saved namespace binding, verify the original
  unknown-prefix URI/range, keep a second copy independent, then reload offline
  and repeat validation/Parse/format with the saved command repair. Read-only
  catalog originals and saved source bytes remain unchanged.

The tested deployment uses catalog identity
`e4438dbded38d65016ddbd4ab0265d271fdffdf34b5a7079c0b35cbfa190fce2`
and deployed SHA-256
`fcce8b60ea7756760db1c4b4a01ddb4c3f989043d60c3e8dcd3684f9a911ab0f`.
Those catalog/deployment bytes remain unchanged through browser acceptance.

All browser checks use isolated profiles and temporary project copies, preserving
existing user projects and drafts. Entered passwords and one-time codes continue
to block recovery; automatic whole-Studio recovery remains disabled. Generated
screenshots, Vitest attachments, binaries and reports are excluded from Git.
