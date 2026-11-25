# Localized application resources

**Status:** design proposal, 2026-11-23. Neither `locate(lang)` nor a command
localization registry is currently implemented. This proposal complements
[application.md](application.md) and [command.md](command.md); it preserves the
existing CEM native import, resource lifecycle and data-island boundaries.

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
override-generation identity even when source bundles are shared.

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

These conventions motivate the proposed resource split; they do not guarantee
that an already-rendered menu relocalizes merely because an OS setting changes.

For example, both of these views resolve the same semantic `file.save-as`
definition and parameter schema:

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

## Localizable fields

The base command owns references to presentation messages. A locale bundle owns
their translations and presentation metadata. Different presentation variants
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
Runtime enabled/checked/busy state comes from the handler. A translation bundle
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
| Locale bundle | Locale identity, resource/message keys, complete message values or typed message nodes, per-value direction where needed, translator metadata |
| Shortcut profile | Stable command references, structured modifier/key or physical-code bindings, platform/layout applicability, primary/alternative designation |
| User preferences | Explicit UI/format locale, command-ID-based keybinding overrides and profile versions; stored separately from shipped resources |

Suggested source organization is a resource manifest plus semantic definitions,
locale bundles such as `en.cem` and `fr-CA.cem`, and independent shortcut profiles.
Paths are packaging examples, never resource identity. A manifest reference can
move without rewriting every menu consumer. Split large catalogs by feature and
load admitted bundles on demand; preload labels/schemas for imminent command UI
so activation does not block on network access.

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

Binary catalogs and compressed application bundles are proposed extensions,
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
reference targets, Unicode/encoding, allowed assets and size limits. Unknown
fields require an explicit schema extension; silently accepting misspelled
`description` defeats reliable fallback.

Keep missing distinct from explicitly empty. Empty optional help can suppress
inherited help; an empty required command name is invalid. A translation must
not alter placeholder types or insert executable expressions. CEM-ML imports
resource text as inert data under the declared schema; resemblance to query or
template syntax does not authorize evaluation.

## `locate(lang)` and language negotiation

Use **`locate(lang)`** as the proposed localization-context
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

Resolve a whole message/structured plural unit at one fallback level; never
assemble a sentence out of fragments from several locales. Its placeholders
are formatted using that message's declared/effective locale unless an explicit
format-locale policy says otherwise. Return provenance with the resolved value:
requested locale, actual message locale, fallback source, resource ID/revision,
field and direction. Mark fallback text with its actual language for speech and
direction without changing surrounding application language.

If a required key is missing even in the default bundle, emit a stable
`resource-missing` diagnostic and disable the affected unnamed command control;
do not render a misleading blank accessible action. Development tools can show
the key. Optional resources can use a schema-declared neutral omission policy.

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
fetch replacement bundles, render, or fire locale callbacks in the stopped app.
On explicit activation/start, prepare its current resources/theme under fresh
admission before reopening business processing. Never replay missed preference
events as a queue. Running apps use this transaction:

1. Capture requested language/theme/profile input revision and a new generation token.
2. Load/import/validate the selected bundle and fallback dependencies through the
   existing resource lifecycle. Pin command definition/schema revisions.
3. Resolve messages, direction, mnemonic groups, platform/keybinding defaults
   and user overrides; diagnose conflicts before publication.
4. If the token is still current, publish one coherent resource-view revision.
   Reproject visible labels, status/tooltip text, accessible names/descriptions,
   parameter-dialog strings, native-shell menu resources and effective bindings.
   Fence shortcut dispatch until affected controls acknowledge that generation.
5. Release old resource owners when subscribers/pending references no longer
   retain them. Emit one locale-change completion notification after publication;
   it does not itself request any command.

Locale changes during load supersede earlier tokens. A failed new bundle leaves
the previous generation and shortcuts active, with an available localized error
message; initial failure falls back to the packaged default. An optional feature
bundle can degrade independently only when its dependency boundary is declared.
Do not mix an old command label and a newly installed shortcut for the same
resource. Persist preferences, not transient decode handles or an entire mutable
resolved JavaScript catalog.

Open menus/dialogs retain their native owner identity, input values, selection,
validation state and invocation context. Recompute geometry through existing
surface behavior when text expands. Preserve the semantic parameter schema;
changing language cannot change MIME enum values or reinterpret a typed number.
Reformat display-only feedback while preserving canonical values and edits.

## Messages, formatting and direction

Use complete messages with named, typed arguments. Plural/select alternatives
are structured resource data with a required fallback branch. Locale-sensitive
plural rules, dates, numbers, lists, units and collation should use admitted
platform `Intl`/Unicode data or a pinned native equivalent. Pin the chosen
message vocabulary and version before implementation; this proposal does not
silently introduce ICU syntax, a new CEM expression language or a dependency.
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

Cache imported bundles by resource identity/content revision, schema/import
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

SSR uses an explicit locale/profile snapshot and emits that generation's
language/direction/resources. Hydration adopts it before reconciling client
preferences. Preference reconciliation may relocalize but never replay commands,
reopen dialogs or overwrite entered form data. A locale mismatch is a resource
revision issue, not permission to replace the whole application DOM.

Before implementation adoption, verify:

- Equivalent resource lookup from CEM-ML, JSON, YAML and declared CSV mappings;
  native identity/provenance survives without JavaScript catalog records.
- Complete defaults, missing/empty distinctions, duplicate keys, incompatible
  placeholders, fallback cycles, script-sensitive lookup and unsupported tags.
- Ordered browser/host preferences, explicit preference overrides and multiple
  independently localized applications; stale/failing bundle loads.
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

These are acceptance requirements for later implementation. This design change
adds no importer, codec, runtime API, browser compatibility claim or fixture.
