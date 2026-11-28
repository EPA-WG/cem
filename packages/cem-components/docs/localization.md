# Localized application resources

**Adopted design · 2026-11-27.** Stable resource IDs resolve to a locale-specific
view inside the owning application. The two-editor Save As flow verifies French
label overrides and coordinated shortcut publication for that scenario. The
general resolver and catalog formats are specified here. [Application design](application.md)
· [Command design](command.md) · [Verification](#caching-ssr-and-implementation-acceptance)

**Insulation revision — 2026-11-24:** app contexts and mutable preferences are
private by default. A host can admit inheritance/sharing for selected families;
native CEM implicit per-property inheritance operates inside the chosen context,
without an added generic `inherit` or `inherited` flag.

## Stable resources, replaceable presentation

Keep semantic resource identity independent of language, package path and
display text. A menu item references `file.save-as`; localization resolves its
presentation for a locale, platform and binding profile. The command ID,
parameter names/types, handler and operation do not change. Bound controls in
each app subscribe to its resolved resource generation, so labels, accessible help
and installed shortcuts agree after a language switch.

## Virtual application context and container overrides

Localization resources are properties of the same native CEM context used by
the [virtual application](application.md#native-context-and-downward-authority).
The top-level app matches the page. Each embedded app starts with an insulated
resource/context set; the host may explicitly admit selected container inputs
or shared resource families. Structural nesting does not grant ambient fallback
to parent resources or mutable preferences. Within the selected app context,
ordinary CEM scope properties inherit implicitly and local admitted overrides
follow their native rules. Resource inheritance does not create a shared mutable
catalog or allow a child to change its parent.

Named scopes bind native context/group descriptors. A scope name or matching
vendor/package is not membership authority; the host admits access separately.
Common-vendor apps may share immutable catalogs or specifically granted mutable
preference providers without sharing every context family. Sharing catalog bytes
does not share evaluated module singletons, app settings, storage or handlers.
Effective resource/override generations and lifecycle gates remain per app.

The container can replace the entire child resource set, selected resources or
schema-declared values, such as a command's name, description or shortcut
defaults. Retain the container's override layer and provenance separately from
child-owned resources. Insulated contexts exclude ambient parent fallback, but
still honor explicit container overrides and ancestor restrictions. Semantic
overrides are validated native-context inputs; they are not translator powers.
Changing a schema/handler through an admitted override changes semantic revision;
changing only presentation changes the resource generation.

Apply partial overrides according to the resource schema. Missing inherits the
selected underlying value; explicit empty/removal has a declared meaning.
Replace a complete plural/message unit, preserving placeholder compatibility;
do not deep-merge translated fragments or implicitly execute an overridden
reference. Missing values inherit only from an admitted underlying layer, never
from a private external app merely because it is a structural ancestor. Resolve
definitions/overlays before message lookup, using the same native
context/import/reference machinery as other resources. A child's update
never edits the parent's retained tree. Caches retain app context/grant and
override-generation identity even when apps share the source resources.

Inherited language/theme preferences are **requests**, matched against the
child's own available choices. An inherited French preference does not imply
that a child with only Japanese/English resources can display French; use that
app's declared available default and retain diagnostic provenance. An insulated
app can still read browser/host preferences through its admitted environment
provider. Neither case grants it write access to the browser/container settings.
All browser preference observation and settings persistence pass through guarded
`cem-elements` providers. App code uses local resource/service names and ordinary
keys such as `locale` or `theme`; partition selection belongs to the framework,
not a manually authored app prefix or wrapper component. An inherited environment
request is separate from a shared mutable settings provider: selecting one does
not admit the other. Light-DOM and iframe hosts expose the same app-local
localization/theme API; frame transport and browser restrictions are host details.

This is inspired by native systems, with important distinctions:

| Platform convention | Consequence for CEM |
| --- | --- |
| Windows MUI separates language-specific resources from executable code and uses UI-language preferences/fallback | Store translations separately from command semantics; stable IDs select equivalent resource presentations. [MUI resource management](https://learn.microsoft.com/en-us/windows/win32/intl/mui-resource-management) |
| Windows has separate menu, string, accelerator, dialog, icon and other resource types | CEM's integrated command resource is a CEM design choice, not a claim about one universal Windows command resource record. [Resource configuration](https://learn.microsoft.com/en-us/windows/win32/intl/preparing-a-resource-configuration-file) |
| Windows MRT Core does not notify applications about qualifier changes automatically | Updating existing UI needs an application/host orchestration step. [Qualifier change migration](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/migrate-to-windows-app-sdk/guides/mrtcore#resource-qualifier-value-change) |
| AppKit separates menu key equivalents from modifier masks and supports user mappings | Resolve shortcuts structurally; do not translate a shortcut display string into behavior. [Key equivalents](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/MenuList/Articles/SettingMenuKeyEquiv.html) |
| GTK actions omit presentation, while menu models carry labels, icons, action references and targets | Keep command schemas/handlers separate from translated labels and menu organization. [Actions](https://docs.gtk.org/gtk4/actions.html), [menu models](https://docs.gtk.org/gtk4/class.PopoverMenu.html#menu-models) |

These conventions motivate the resource split; they do not guarantee
that an already-rendered menu relocalizes merely because an OS setting changes.

These English and French views use the same desktop `file.save-as` schema.
The browser profile has its own destination inputs. Choosing a language
preserves the selected host profile. [Save As across hosts](command.md#save-as-across-hosts)

| Field | English view | French view |
| --- | --- | --- |
| Command ID | `file.save-as` | `file.save-as` |
| Menu name | Save As… | Enregistrer sous… |
| Description | Save the document with another name or format. | Enregistrer le document sous un autre nom ou dans un autre format. |
| Parameter keys | `contentType`, `filePath` | `contentType`, `filePath` |
| Parameter display labels | Content type; File path | Type de contenu ; Chemin du fichier |
| Effective shortcut | Resolved from platform/layout/user profile | Independently resolved from platform/layout/user profile |

Changing the language replaces the name and help at every subscribed
presentation; it neither changes the ID nor inherently changes the shortcut.

The [versioned Save As example](command.md#version-1-save-as-example) shows a
parent replacing one child's French menu label. The form and its labels still
resolve in the child's localized resources under the same command ID. During
a locale switch, preserve any edited destination in the live private form.
Localize field names and validation codes without interpolating the private
path into captured messages. Give **Download a copy** and its handoff feedback
distinct messages in each locale.

## Localizable fields

The base command references presentation messages. Each locale’s `resources`
container holds their translations and presentation metadata. Different presentation variants
may share one message when their wording is truly identical.

| Resource field | Intended use |
| --- | --- |
| `name`, `menuName`, `compactName` | Full command name, menu wording and bounded compact variant; no manual concatenated shortcut suffix |
| `description`, `statusHint`, `tooltip` | Fuller explanation, status-bar hint and nonessential advisory help |
| `accessibleName`, `accessibleDescription` | Meaningful accessible text when visible wording needs clarification; retain name-in-label agreement |
| `mnemonicCandidates` | Locale-specific menu access characters, separate from direct command shortcuts |
| `category`, `keywords`, `aliases` | Localized command-palette discovery; aliases never become executable IDs |
| `dialogTitle`, parameter labels/help/placeholders | Parameter collection UI, options, units, accepted/cancel action text and required-field guidance |
| validation, permission, progress and result messages | Locale-aware feedback keyed by stable diagnostic codes |
| `undoName`, `redoName` | Templates supplied by the domain transaction, e.g. Undo plus operation/document wording |
| help references | Locale-specific documentation destinations admitted through resource/link policy |
| asset references | Culturally specific illustrations, icon alternatives, alt text, captions, speech/audio assets where needed |
| language/direction metadata | The actual language and base direction of a value, including a fallback value |
| translator context | Meaning, usage, constraints, screenshots/reference text, parameter meanings and notes; not visible UI |

Both custom and schema-generated nonmodal forms read parameter display names,
descriptions, option labels and messages from the command owner's localized view.
For example, each translation of the content type field binds the same
`contentType` key. The form resource lookup uses the stable command ID across
languages. A language switch updates the open form's text and accessible title
in place while retaining its draft, selected values, and original target.

Additional application resources include window/document title templates, menu
group names, empty/busy/error states, dialog text, date/number/list/unit
formatting preferences and localized search collation. Locale-specific assets
must retain equivalent intent and accessible alternatives. Do not mirror every
icon mechanically: directional navigation and a brand mark have different rules.

Do **not** translate command IDs, parameter keys, enum wire values, MIME types,
permission names, handler references, capability requirements or result codes.
Runtime enabled/checked/busy state comes from the handler. Translation resources
cannot change authorization, operation kind, parameter schema, executable
provider or concurrency policy. Platform policies and translations can propose
shortcut defaults, but the active host/user profile controls installation.

### Mnemonics, shortcuts and ellipses

A mnemonic/access key helps navigate a menu/control; an accelerator directly
invokes an operation. Store each separately from the visible name. Win32's `&`
marker and GTK's underscore convention are adapter encodings, not text to embed
in CEM's canonical labels. Resolve mnemonic uniqueness per visible sibling menu
group after localization; an unresolved collision omits the ambiguous mnemonic
and reports it, without rewriting the translation arbitrarily.
[Windows keyboard conventions](https://learn.microsoft.com/en-us/windows/win32/uxguide/inter-keyboard).

An ellipsis signals that more input is needed to complete the requested action,
not simply that some window opens. Keep `requiresInput` as invocation/presentation
metadata so a menu binding can render localized punctuation deliberately. Do not
blindly append an ellipsis twice or remove a translator-authored one. A supplied
parameter shortcut can execute directly while the generic Save As menu still
offers parameter collection.
[Apple menu guidance](https://developer.apple.com/design/human-interface-guidelines/menus).

Shortcut resources contain structured bindings and localized **display** names.
UI language, OS conventions and keyboard layout are three independent inputs.
Resolve user override (including unbound), host policy, applicable profile
default and command default in that order; policy restrictions can reject any
binding, including an override. Store primary and alternative bindings
explicitly. Recompute visible/ARIA hints from what is actually installed.
GTK likewise permits multiple accelerators for an action, with a primary
display accelerator. [GTK accelerator API](https://docs.gtk.org/gtk4/method.Application.set_accels_for_action.html).

## Resource storage and the CEM-ML boundary

Prefer authored CEM-ML for strings and structured resources. Preserve values as
typed native CEM trees, so declarations, schemas, typed references, source maps
and resource lifetimes use the same machinery as other CEM data. The logical
vocabulary below specifies fields, not new CEM-ML syntax.

| Resource document | Required logical contents |
| --- | --- |
| Application resource manifest | Stable resource-set identity/version, supported languages, default locale, locale-to-resource references, schema revisions, admitted asset/shortcut profiles and explicit inheritance links |
| Command definitions | Stable IDs, semantic schema, presentation message keys, optional parameter-dialog references and execution policies |
| Localized resources | Locale identity, resource/message keys, complete message values or typed message nodes, per-value direction where needed, translator metadata |
| Shortcut profile | Stable command references, structured modifier/key or physical-code bindings, platform/layout applicability, primary/alternative designation |
| User preferences | Explicit UI/format locale, command-ID-based keybinding overrides and profile versions; stored separately from shipped resources |

Use `resources` as the grouping container in the manifest. For example,
`{resources @fallback=en @lang=fr | ...}` groups the French entries and declares
where to look for a missing key. Individual entries keep their own type and
key. A parent can override one entry for one child and language. The container
can be inline or supplied from files such as `en.cem` and `fr-CA.cem`. Command
definitions and shortcut profiles have their own entries in the manifest.
Paths are packaging examples, never resource identity. A manifest reference can
move without rewriting every menu consumer. Split large catalogs by feature and
load admitted resources on demand. Preload labels and schemas for imminent
command UI so activation can proceed without a network request.

### Alternative source formats

| Format | Allowed boundary and tradeoff |
| --- | --- |
| CEM-ML | Preferred authoring format; canonical retained typed vocabulary and source provenance |
| JSON | Explicit import/media type and vocabulary mapping; useful for existing translation tooling; never a runtime JavaScript catalog record |
| YAML / `.yml` | Shared native YAML importer under its supported bounded profile; no consumer-local alias/tag processing |
| CSV | Shared importer plus declared headers/mapping for flat translation interchange; columns such as resource ID, field, locale and text; nested/plural data needs an explicit mapping or another format |
| Binary CEM artifact | Optional versioned distributable encoding; requires a registered codec and full native structural/schema validation before publication |
| Compressed bundle | Transport/storage encoding over a declared inner format, not a separate semantic vocabulary; bounded decoding before native import |

All alternate formats enter through
[the shared CEM AST import boundary](../../../docs/cem-data-import-principle.md).
No JSON/YAML parser in a component, `JSON.parse()` catalog cache, format-specific
XPath evaluator, or DOM-to-record substitute is permitted. Imported resources
converge on the same typed vocabulary. Export to translation-tool JSON/CSV is an
explicit interchange boundary and preserves stable keys and translator context.

Binary catalogs and compressed application bundles are optional extensions,
not a promise that every current binary projection codec is an executable
resource codec. Pin magic/version, inner MIME/schema, byte order/encoding,
integrity checks, resource revision and source provenance as applicable.
Enforce compressed size, decompressed size, expansion ratio, node/depth limits
and cancellation; reject malformed/trailing data. Prefer ordinary HTTP
compression when it meets deployment needs. Do not add an opaque custom archive
merely to store a small string table. Binary builds should be reproducible from
reviewable sources and ship enough mapping metadata for diagnostics.

### Schema and translation validation

Reject duplicate semantic command IDs and duplicate locale message keys. Locale
overlays may replace only declared presentation fields. Validate required
default-locale coverage, placeholder types/names, plural/select branches,
reference targets, Unicode/encoding, allowed assets and size limits. Apply the
[admission rule for the affected scope](#missing-resources-and-admission). Unknown
fields require an explicit schema extension; silently accepting misspelled
`description` defeats reliable fallback.

Keep missing distinct from explicitly empty. Empty optional help can suppress
inherited help; an empty required command name is invalid. A translation must
not alter placeholder types or insert executable expressions. CEM-ML imports
resource text as inert data under the declared schema; resemblance to query or
template syntax does not authorize evaluation.

## `locate(lang)` and language negotiation

Use **`locate(lang)`** as the localization-context
operation. It is not an existing global JavaScript function or an OS API. Its
contract is: prepare a read-only resolved resource view for a requested language
(or the application's preference list when omitted), then return the view with
its effective language, fallback chain and generation identity. It may require
async resource loading. It does not silently mutate the active application.
`cem-application` publishes a prepared view through the transaction below.

The conceptual resource lookup inside that view is
`resource(commandId, field, messageArguments?)`. This notation describes the
contract, not an added CEM-QL function or invocation syntax. The implementation
must expose a concise declarative native binding before component adoption.

Choose the highest-precedence applicable preference source first, within
host/ancestor restrictions:

1. Container-owned override of this app's requested locale/preferences, if set.
2. Explicit application UI-language setting for this instance/subtree.
3. Container/group environment preferences only when that family is explicitly
   admitted as an inherited/shared input.
4. Host-provided per-application/OS or browser `navigator.languages` preferences
   through the admitted environment provider when automatic selection applies.
5. The app's packaged default locale, which must be in its supported set.

Match only that source's ordered list against available app resources, then use
the app default if no entry matches. An unsupported explicit app/container choice
does not select a lower-precedence browser/inherited preference. An explicit
exact-match requirement instead diagnoses/refuses an unsupported configuration.
`auto` selects the configured environment provider; a container can mandate that
automatic policy instead of a child's saved setting. Do not silently fetch an
unknown locale or force a missing resource. Explicit app settings replace
automatic environment selection, but cannot replace the container override layer.

`lang` on a document/element describes content language; it is not an OS setting
and must not be mistaken for a persistent user preference. Set effective `lang`
and `dir` on the application region after resolution. User content can retain
its own language. Multiple applications on a page may use different languages.

Canonicalize valid BCP 47 tags, then use documented RFC 4647 lookup over supported
locales for each preference in order. Pin application-supported matching behavior
and tests, including scripts and region tags. Do not turn a tag into a guessed
filename or casually strip every suffix. Unicode extension preferences for
numbers/calendars belong to formatting resolution, not an uncontrolled catalog
path. [RFC 4647 lookup](https://www.rfc-editor.org/rfc/rfc4647.html#section-3.4).

Keep **locale matching** separate from **message inheritance**. After selecting
a locale, use its manifest-declared inheritance chain, ending at the required
default locale. For example, an explicitly declared `fr-CA -> fr -> en` can
inherit missing messages. Reject cycles. Do not silently cross script families
or invent a regional parent; pin CLDR parent data if an implementation adopts
it. Likely subtags are not resource inheritance.
[Unicode inheritance distinctions](https://unicode.org/reports/tr35/#Inheritance_vs_Related_Information).

### Per-message language and formatting

Resolve the whole message, including its plural or select branches, at the first
admitted fallback level containing the field. Record the requested UI locale,
actual message language and direction, format locale, field, source and revision.
The surrounding app keeps its resolved UI language. A translator supplies one
complete message with the declared typed arguments and required fallback branch.

| Decision | Rule |
| --- | --- |
| Grammatical branch | Apply the actual message language's cardinal or ordinal rules to the typed count. Honor the message's declared precision and visible fraction digits when selecting a plural and displaying its number. A categorical select uses its declared locale-neutral selector. |
| Argument display | An explicit app `format-locale` formats number and date arguments. Otherwise use the actual message locale inside that message. Standalone numbers and dates default to the resolved UI locale. Pass the typed value directly and keep canonical form and command values intact. |
| Visible and accessible language | Mark the rendered text with its actual language and base direction. For a control whose whole label is a fallback message, put `lang` on the control and derive its accessible name from the visible label. A separately translated description can use a labelled text node in its own language referenced by `aria-describedby`. |

A mixed-language label can use language-tagged runs and bidirectional isolation
around interpolated content. Preserve those runs for native accessibility APIs
that support them. Browser accessible names are flat strings. Keep the language
on the control or labelled source nodes so it is available independently of the
computed name. Spoken output needs separate platform and assistive-technology
verification. [Accessible name computation](https://www.w3.org/TR/accname-1.2/#terminology) ·
[Language of parts](https://www.w3.org/WAI/WCAG22/Understanding/language-of-parts.html) ·
[Unicode plural rules](https://www.unicode.org/reports/tr35/tr35-numbers.html#Language_Plural_Rules)

**Example.** A French app falls back to an English button label with a typed
integer count and an explicit German (`de-DE`) format locale. The English
singular branch reads “Open {count} file” and the other branch reads “Open
{count} files”. For zero, French and English select different cardinal
categories. Use the English branch. German grouping applies to the digits.

| Count | Expected visible label and accessible name in native UI, browser and SSR |
| --- | --- |
| 0 | Open 0 files |
| 1 | Open 1 file |
| 1234 | Open 1.234 files |

For 1234, browser and server output can both use
`<main lang="fr" dir="ltr"><button lang="en" dir="ltr">Open 1.234 files</button></main>`.
The native surface carries the same label, English language and left-to-right
direction. SSR pins locale data, precision, formatting options and, for dates,
calendar and time zone so hydration can adopt the rendered text.

### Missing resources and admission

| Where resolution fails | Behavior |
| --- | --- |
| Package or initial view | At package validation, diagnose incomplete default resources for the scope that declares them required. If the default cannot supply the app's required first view, reject initial admission and let the host show its localized error surface. During an update, preserve the last coherent admitted view under current grants. |
| Independently declared feature | Validate its required defaults when admitting its resources. If they fail, keep that feature unavailable and its commands fenced. The admitted shell and independent features continue to work. |
| Individual field of an admitted view | A missing translation follows the declared fallback chain. An optional field missing throughout is omitted. Explicit empty optional help suppresses inherited help. An empty required label is invalid. If a dynamic presentation cannot obtain a required name after fallback, emit `resource-missing` and withhold the unnamed control or provide a safely named, disabled presentation. Other valid bindings keep their own eligibility. |

An invalid override fails its validation transaction. Apply the normal
publication failure rule and retain the last admitted generation where
available. Diagnostic projections identify the resource key, field and source.
Keep private argument values out of all captured diagnostics.

**Verification.** Windows ICU, a Node server-rendering probe, Chromium and
Firefox produced all three example strings. The browser probes also matched
the button by its accessible name and checked its English language tag in the
server-produced markup. CEM projection, native accessibility metadata, spoken
output and package/feature/field failures remain implementation acceptance
cases.

## Environment theme variations

UI theme variations follow the same preference/availability pattern as locale:
container override, explicit app setting, expressly admitted inherited/shared
environment, browser/host preferences, then the app's declared available default.
App manifests declare supported variations and their mapping to existing CEM
theme semantics;
automatic selection cannot assume every app provides every brand variation.
Keep requested and effective values/provenance separate.

As with language, choose the preference source before matching its variations.
If an explicit choice has no supported match, use the app default (or refuse an
explicit exact-match requirement); do not substitute a lower-priority browser
choice. Automatic policy uses the admitted environment provider.

For automatic light/dark, contrast and native/system behavior, use the browser's
preference signals through the admitted provider and existing CEM theme contracts.
A container may align child appearance or permit an independent app theme.
Changing a child choice affects that subtree's managed projection only. It must
not set page-global theme state, install an unscoped replacement stylesheet or
redefine theme-owned token defaults. Guarded CEM projection applies theme values
to their owning app context in either hosting form; a child cannot acquire global
style mutation through resource inheritance. Hybrid unmanaged styles/scripts
require the stronger host boundary described in application.md; they are not
the default authoring model for a CEM-only app.

Respect forced colors, keyboard focus, reduced motion and applicable accessibility
constraints even when an explicit app theme is selected. A fallback variation
uses the same semantic token ownership, not hard-coded colors introduced to
imitate a missing theme. [CSS color adjustment](https://www.w3.org/TR/css-color-adjust-1/).

## Reacting to language and theme changes

Browsers expose preferred language changes through `languagechange` when the
user agent's understanding changes. This is not guaranteed observation of all
OS changes; preferences may also be privacy-limited. A native host adapter must
supply notifications for stronger OS integration. Rechecking preferences on
foregrounding is a useful resynchronization step, not a guarantee of real-time
OS notification. Explicit application choices remain in force until cleared or
overridden by the admitted container context.
[HTML language preferences](https://html.spec.whatwg.org/multipage/system-state.html#language-preferences).

For paused/offline/offloaded apps, the manager records only the latest relevant
environment/context revision. Do not deliver language/theme/storage events,
fetch replacement resources, render, or fire locale callbacks in the stopped app.
On explicit activation/start, prepare its current resources/theme under fresh
admission before reopening business processing. Never replay missed preference
events as a queue. Running apps use this transaction:

1. Capture requested language/theme/profile input revision and a new generation token.
2. Load/import/validate the selected resources and their fallback dependencies
   through the existing resource lifecycle. Pin command definition/schema revisions.
3. Resolve messages, direction, mnemonic groups, platform/keybinding defaults
   and user overrides; diagnose conflicts before publication.
4. If the request is current, fence affected command dispatch and apply the
   prepared generation to its registered projections and shortcut registry.
5. After the remaining participants acknowledge that generation, reopen eligible
   routes and publish one completion. Release older resources when their owners
   and pending references no longer retain them. Persist the preference in the
   app’s settings. Decoded handles and the resolved catalog are runtime state.

Open menus/dialogs retain their native owner identity, input values, selection,
validation state and invocation context. Recompute geometry through existing
surface behavior when text expands. Preserve the semantic parameter schema;
changing language cannot change MIME enum values or reinterpret a typed number.
Reformat display-only feedback while preserving canonical values and edits.

### Locale publication and command dispatch

A running app prepares the requested resource generation while the last
coherent view remains usable under current grants. Immediately before applying
a winning generation, capture its affected participants. Include the shortcut
registry and each live projection that presents changed text or accessibility
metadata, advertises a changed binding, or can invoke an affected command.
An open menu, a parameter form and an installed native-shell accelerator can
all participate. Data-only subscribers read the current generation on demand.
Closed surfaces and unloaded features adopt it when they open.

Fence affected command entry points before changing the first projection.
This covers both old and new shortcuts, menu actions and native callbacks.
Native typing, composition and focus navigation continue. Leave fenced keys
unclaimed and discard stale command callbacks. A participant acknowledges
its registered owner and generation after applying the complete projection,
including accessible text and the effective bindings it owns. A render
queued or a message sent to a native shell is insufficient evidence of adoption.

| Change during the switch | Coordinator action |
| --- | --- |
| Menu closes or a participant unmounts | Remove its pending acknowledgment once its old invocation endpoint is fenced. Opening it again requires the current generation. |
| A new surface opens | It adopts the latest winning generation before it becomes actionable. It cannot activate an old binding or prolong the switch by registering late. |
| A native subscriber disconnects or fails to acknowledge | Fence its old callbacks and retire the projection if it can be safely withdrawn. If a required surface can still act or advertise an obsolete shortcut, fail the publication. |
| The owning app pauses, stops or changes admission epoch | Invalidate the pending publication. Keep the latest request with the manager. A running app prepares it under the new epoch. A stopped app adopts it on start before opening business dispatch. |
| A newer request supersedes the generation | Fence stale renders and acknowledgments. If application of the old request had begun, keep affected routes fenced until a coherent view is established. |
| A command grant is revoked | Disable that command immediately at every entry point. Check current grants on each invocation. Neither a late acknowledgment nor restoration of an older view can re-enable it. |

Complete the switch when the shortcut registry and every remaining affected
participant have acknowledged the same generation. Recheck the app epoch
and current request, enable only eligible bindings, then emit one locale-change
completion. Acknowledgments for a retired owner or an older generation cannot
release the fence. A key pressed during the switch never becomes a deferred
command.

On a preparation failure, keep the previous coherent view and report the
error using its available localized messages. On first load, try the packaged default and enable only features with
valid resources. If a required participant fails after partial application,
keep affected routes fenced while restoring the last coherent view under
current grants. Treat expiry of the host adapter's acknowledgment wait as a
failure. Resume affected commands only after the restored projections and
shortcut registry agree. Where restoration cannot be confirmed, keep those
routes unavailable and report the failed switch. A failed or superseded
generation emits no successful completion. Independent apps and features
with declared separate resource dependencies continue running.

For acceptance, switch to French with the File menu open, an edited nonmodal
form and a native Save accelerator. Close the menu before it acknowledges.
Disconnect the native adapter and deliver a late callback. Revoke Save while
its old label is visible and press its key before and after the switch.
Verify zero saves, preserved form input and exactly one completion for a
successful switch. Also pause during publication, supersede French with a
second request, and fail a required projection after another has updated.
Verify coherent recovery or unavailable affected routes, with no replay.

## Messages, formatting and direction

Use complete messages with named, typed arguments. Plural/select alternatives
are structured resource data with a required fallback branch. Locale-sensitive
plural rules, dates, numbers, lists, units and collation should use admitted
platform `Intl`/Unicode data or a pinned native equivalent. The message vocabulary and its version still need a decision before implementation.
Never concatenate translated fragments such as Save + document + failed.

Default message output is text, escaped at the output boundary. Rich messages
require an explicit allowed semantic vocabulary and safe typed interpolation;
translation text is never arbitrary HTML, an event attribute or a URL execution
grant. Validate localized help/asset references using the existing resource
policy. Avoid putting secrets into message arguments or diagnostic captures.

Retain language and base direction metadata with resource values. Isolate
interpolated user names, filenames, shortcuts and numbers using appropriate
native text-direction semantics such as `bdi`; application `dir` alone cannot
handle every embedded language. Keep native logical layout and focus order,
and use bounded compact variants when needed rather than silently truncating
essential command meaning.
[W3C strings and bidi](https://www.w3.org/International/articles/strings-and-bidi/).

Localized text must survive long translations, RTL, zoom and fallback fonts.
Use the existing CEM semantic typography, spacing, focus and state tokens; do
not create language-specific color defaults or bypass theme ownership to make
a label fit. An advisory `title` is insufficient for essential guidance.
Expose visible help and accessible descriptions deliberately, without announcing
every locale update as a separate live-region message.

## Caching, SSR and implementation acceptance

Cache imported resources by resource identity/content revision, schema/import
profile and locale, with resolver/base identity, active registry/security policy
and relevant admission/grant context in the cache identity. Reusing cached bytes
or a retained tree in another scope still requires fresh admission; cached
content never confers fetching, execution or reference-crossing authority.
Immutable source/artifact reuse does not share mutable resolved views, evaluated
module state or settings. A named shared context still requires current
membership and provider grants; naming a cache entry cannot admit a new member.
Cache resolved views additionally by definition revision,
fallback chain, format/profile inputs and user-binding revision. Application
instance/context eligibility is not reusable global translation state. Release
owners on teardown; resource text/provenance remains available for diagnostics
without preserving private command arguments.

SSR uses an explicit locale/profile snapshot and emits the initial view with
resolved text, accessible names, language and direction. Optional feature
catalogs can load when needed. On arrival, resolve the feature against the
then-current locale and override generation before displaying it. A delayed
catalog leaves the admitted first view visible. Hydration adopts that view
before reconciling client preferences. Preference reconciliation may relocalize but never replay commands,
reopen dialogs or overwrite entered form data. A locale mismatch is a resource
revision issue, not permission to replace the whole application DOM.

Implementation acceptance covers the following cases.

- Equivalent resource lookup from CEM-ML, JSON, YAML and declared CSV mappings;
  native identity/provenance survives without JavaScript catalog records.
- Complete defaults, missing/empty distinctions, duplicate keys, incompatible
  placeholders, fallback cycles, script-sensitive lookup and unsupported tags.
- Ordered browser/host preferences, explicit preference overrides and multiple
  independently localized applications; stale or failed resource loads.
- Container full/partial resource overrides, insulated fallback refusal and
  child-only settings; unavailable locale/theme choices select declared app
  defaults without upward mutation. Stopped apps defer resource/theme work until
  explicit activation/start and use the latest revision without event replay.
- Private-by-default preferences, rejected named-group self-membership and
  separately admitted catalog/environment/settings sharing. App-local keys and
  resource APIs behave equivalently in light-DOM and iframe hosts without author
  wrappers, prefixes or ambient browser-state fallback.
- A language switch updates name, description, shortcut display/installation,
  mnemonic and accessible metadata for the same command ID together.
- Open edited dialogs, selection and focus survive; new keyboard layout and
  user unbinding do not become changes to command semantics.
- Plurals, dates, numbers, RTL/mixed-direction filenames, long labels, inherited
  themes, forced colors and spoken accessible names.
- Bounded binary/decompression refusal if those extensions are implemented;
  deterministic sources/artifacts and no execution of translated resource data.
- SSR/hydration generation reconciliation, owner release and no event replay.

The spoken mixed-locale label check remains open. [Application acceptance checklist](../../../docs/todo.md#cem-application-acceptance)
