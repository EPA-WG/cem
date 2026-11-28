# API firewall design for cem-application

**Status: adopted design.** The existing browser policy implements location and storage.
The firewall applies one grant stack to browser services and gives each service
a matcher for its own resources. Provider selection determines **which** service
an app receives. The firewall determines **which operations** it can perform on
**which resources** within that service. A private storage partition can have a
read-only subtree. A shared provider can give two apps different views. [Application browser policy](../packages/cem-components/docs/application.md#browser-isolation-policy)

## All CEM consumers

**Every CEM element that reads, changes, enumerates or observes a protected API resource must check the owning application’s grant stack.** This includes operations that return only metadata.

**Every CEM element that opens, triggers or delegates a network connection must
obey the entire grant stack of its owning application.** This applies equally
to library elements, custom declarations, host adapters acting for an element
and future network consumers. Ownership follows the logical app through slots,
portals, frames, workers and shared services. A page-level element uses the
host's root policy. A missing owner or missing grant denies the operation.

| Where the request originates | Where to enforce the stack |
| --- | --- |
| HTTP requests, GraphQL actions, network-backed queries and command handlers | At the shared transport immediately before sending, including retries and follow-up requests. |
| External declarations, module or WASM loads, stylesheets and their imports, fonts and CSS URLs | After resolution against the admitted base or module map. Carry the requesting app through every dependent load. |
| Rendered images, media, frames, forms, navigations and resource hints | Before publishing an active URL, submitting a form or issuing a preload or preconnect. A logical route update needs a network grant only if it leads to network I/O. |
| Workers, server-side rendering, event streams and sockets | Carry the app's host-issued operation identity across execution boundaries. Apply the same decision before opening, reconnecting and delivering results. |

Adapters must retain control for the lifetime of the operation. If the browser
would automatically follow redirects, fetch subresources or reconnect beyond
that control, use a mediated loader or leave the consumer unavailable under
that policy. For example, a path-limited grant cannot authorize a speculative
origin-wide preconnect. Server-rendered markup must keep such URLs inert until
enforcement is available. Recheck the requesting app before serving a shared
cache entry or attaching it to an in-flight load. A loader's own broader grants
cannot authorize its callers. Raw vendor JavaScript requires a separate
execution boundary. [HTTP resource design](cem-elements-http-request-design.md#12-security-and-privacy)

## Declaration and delegation

Under `browser-policy`, use `export` with `@api`, `@to` and an optional
`@scope` to offer rights to one direct child. Use `filter @api=…` in the
recipient to narrow its effective service. Both contain typed `allow` rules.
An export is a container-owned input installed by the trusted host against
the mounted recipient. An app cannot issue itself an export by editing markup.
In the example, the outer host has already granted the shell permission to
delegate reads and event streams at the indicated destinations.

```cem-ml
{cem-application @id=shell |
    {browser-policy |
        {export @api=network @to=editor @scope=workspace |
            {allow @transport=http @scheme=https @host="**.example.com"
                @path="/v2/**" @methods="GET HEAD" @redirect=checked}
            {allow @transport=event-stream @scheme=https @host=events.example.com
                @path="/projects/*/events"}
        }
    }
    {cem-application @id=editor @private |
        {browser-policy @scope=workspace |
            {filter @api=network |
                {allow @transport=http @scheme=https @host=api.eu.example.com
                    @path="/v2/projects/blue/**" @methods=GET @redirect=checked}
                {allow @transport=event-stream @scheme=https @host=events.example.com
                    @path="/projects/blue/events"}
            }
        }
    }
}
```

* The root host supplies the maximum authority. To use an exported service across an embedding edge, require a
  live grant for the recipient instance and selected scope. Omitting the scope
  selects that recipient's explicit default export. The same alias on another
  app, common origin, `@shared` and DOM reparenting confer no authority.
* Within each layer, any **one complete rule** can match. Across layers, **every
  layer** must match the concrete operation, including destination, transport,
  method and credentials. Never combine the host from one rule with the path
  or method from another. Evaluate the stack at use time so a broader child
  mask cannot enlarge a narrower parent grant.
* An omitted filter leaves that API's existing rights in force. An empty
  `{filter @api=network}` allows no network operations. A new network
  capability or shared provider requires an explicit export. Existing private
  local/session storage starts with its admitted app-local rights. Adding a
  filter can reduce those rights. Reject a malformed policy for the affected
  API before use. Diagnose duplicate filters for the same API.
* Forwarding requires an explicit upstream right to delegate and a fresh export
  to the next direct child. `@delegate=true` on an export permits
  further delegation within its ceiling. The default is use only. Each leaf
  operation retains the identities and generations of all contributing grants.
* In the example, the editor may read the blue project and subscribe to its
  events. A red-project GET, a blue-project HEAD and a sibling's use of the
  workspace alias all fail for different reasons.

## Network destination grammar

Keep `scheme`, `host`, `port` and `path` separate. Match the **entire** hostname
and pathname. Require an explicit transport, scheme, host and path so a typo
or omission cannot silently turn into a wildcard. An omitted port means the
scheme's default. A supplied port is one exact effective port.

| Intent | Declaration | Matches | Outside the grant |
| --- | --- | --- | --- |
| Exact domain | `@host=example.com` | `example.com` | `www.example.com` |
| Exact subdomain | `@host=api.eu.example.com` | Only that host | Its parent and its children |
| One subdomain label | `@host="*.example.com"` | `api.example.com` | The apex and `api.eu.example.com` |
| Descendants at any depth | `@host="**.example.com"` | Both examples above | The apex and `otherexample.com` |
| Mask within a DNS label | `@host="api-*.example.com"` | `api-west.example.com` | `api.west.example.com` |
| Exact path | `@path="/v2/docs"` | `/v2/docs` | `/v2/docs/` and `/v2/docs-old` |
| Path segment mask | `@path="/projects/*/report-*.json"` | `/projects/blue/report-1.json` | `/projects/blue/old/report-1.json` |
| Path subtree | `@path="/v2/docs/**"` | `/v2/docs`, `/v2/docs/` and descendants | `/v2/docs-old` |

A standalone host `*` label consumes one nonempty DNS label. Within a label,
`*` consumes zero or more label characters. Host `**` is valid only as the
leading whole label before a literal suffix and consumes one or more labels.
List the apex separately to grant both apex and descendants. V1 host masks
require a literal DNS suffix and do not match IP literals. IP grants, when
supported by the host, use exact addresses.

For paths, a standalone `*` consumes one nonempty segment. Within a segment,
`*` consumes zero or more characters up to the next slash. Whole-segment `**`
is valid only at the end and includes the subtree root. `/**` covers all
paths. There are no regular expressions, implicit prefixes or rule-order effects.

Resolve relative references before admission. Use the URL parser's canonical DNS
hostname (including IDNA), lowercased scheme, effective port and case-sensitive
pathname. Normalize an optional final DNS dot consistently. Compare the same
canonical URL that will be sent. Reject userinfo and ambiguous encoded separators
or paths the adapter and destination cannot interpret consistently. Fragments do
not participate. A path grant admits all queries at that path, so an API that
routes privileged operations by query also needs an application-level rule.
Server-side adapters retain their own endpoint and address restrictions.
[WHATWG URL](https://url.spec.whatwg.org/#url-parsing) · [Existing pinned SSR reads](archive/todo-completed-ssr-network-policy-2026-11-09.md)

## Cookies, storage and other APIs

Use the same `export`, `filter` and `allow` nodes. Each API defines its own
resource fields and operation names. Reject unknown fields and unsupported
rights before opening the provider. A missing filter never adds new operations
to a provider. Host policy, platform restrictions and grants on existing
named-store handles continue to apply.

| API | What a rule matches | Operations to distinguish |
| --- | --- | --- |
| Virtual cookie jar | Jar identity, canonical domain, host-only or domain scope, cookie path, case-sensitive cookie name, and the cookie partition when applicable | Read, enumerate, set, remove, subscribe. Sending cookies with a request and accepting response cookies require separate rights. |
| Local and session storage | Effective app partition or exported store, storage lifetime, and the **logical key** supplied by the caller | Read, set, remove, enumerate, clear, subscribe. Local and session grants are independent. |
| Named store | Host-granted store identity, lifetime and logical key. Resolve the recipient's alias first. | Preserve the existing read, write, subscription and version-check contract. Apply key masks within the granted key set. |
| IndexedDB | Database and object-store names, index when used, and typed primary key | Separate get, put, delete, scan/count, schema upgrade and database deletion. Cursor and index results also require authorization for the underlying primary records. |
| Cache Storage | Cache name and canonical request URL with method | Separate match, put, delete and enumerate. Fetching on a cache miss also requires a network grant. |

In this example the shell already owns the workspace draft provider and virtual
jar. The reviewer observes blue-project drafts and reads blue-project UI cookies.
It can keep its own session notes. The app's current route plays no part in a
storage-key decision.

```cem-ml
{cem-application @id=shell |
    {browser-policy |
        {export @api=local-storage @to=review @scope=workspace |
            {allow @key-mask="drafts/**" @operations="read subscribe"}
        }
        {export @api=cookies @to=review @scope=workspace |
            {allow @host=api.example.com @cookie-scope=host-only
                @path="/projects/**" @name-mask="ui-*" @operations=read}
        }
    }
    {cem-application @id=review @private |
        {browser-policy @scope=workspace @local-storage=shared @cookies=shared |
            {filter @api=local-storage |
                {allow @key-mask="drafts/blue/**" @operations="read subscribe"}
            }
            {filter @api=cookies |
                {allow @host=api.example.com @cookie-scope=host-only
                    @path="/projects/blue/**" @name=ui-theme @operations=read}
            }
            {filter @api=session-storage |
                {allow @key=layout @operations="read set"}
            }
        }
    }
}
```

**Storage keys.** Use `@key` for an exact key, including an empty key or a
literal asterisk. Use `@key-mask` for slash-delimited segment matching.
`drafts/*` grants direct entries and `drafts/blue/**` grants that prefix as
a whole key plus descendants. It excludes `drafts/blueprint`. Match the original
case-sensitive string before applying any physical storage prefix. Slashes are
ordinary characters with significance only to the mask. Preserve empty segments,
dots and percent signs literally. Never URL-decode or resolve a storage key.
For IndexedDB, key masks apply only to string keys. Numbers, dates, binary and
array keys require typed exact or range selectors in a future adapter. A string
mask cannot authorize them by coercion. [Web Storage](https://html.spec.whatwg.org/multipage/webstorage.html) · [IndexedDB keys](https://www.w3.org/TR/IndexedDB-3/#key-construct)

**Cookie scope.** Select the virtual jar first. Apply the host grammar above
to the cookie domain and the path grammar to the cookie path. Use `@name`
for an exact cookie name or `@name-mask` for a case-sensitive name glob.
For a read at a URL, check the URL and the cookie identity and apply native
domain, path, security and partition eligibility. A host-only grant cannot
create or remove a domain-wide cookie. For writes, prove that the **entire
delivery scope** of the proposed cookie is permitted. A grant for
`/projects/blue/**` must refuse a cookie scoped to `Path=/projects`, even
when the app currently displays the blue project. Refuse an operation whose
delivery scope the adapter cannot establish. Keep HttpOnly values opaque to
CEM readers, including through enumeration and change events.
[Cookie domain and path matching](https://www.rfc-editor.org/rfc/rfc6265.html#section-5.1)

**Operations spanning two APIs.** A credentialed request needs the network grant
and authority to attach credentials for that destination. A cookie received
from a response also needs authority to update the jar. Recompute those rights
for each redirect. Browser Fetch cannot select native ambient cookies by
cookie name or expose individual Set-Cookie response headers to app script.
Use an enforceable host-managed jar or issue a credential-omitting request
when the caller permits it. Otherwise refuse before sending.
[Fetch credentials](https://fetch.spec.whatwg.org/#credentials) · [Forbidden response headers](https://fetch.spec.whatwg.org/#forbidden-response-header-name)

**Operations over multiple entries.** Filter enumeration, counts, cursor results
and subscription events before exposing keys, values or metadata. Subscribe
requires read access to each delivered resource. A write-only caller receives
the same acknowledgment for an insert, replacement or removal and cannot
request the old value, an existence test or a version precondition. Clearing a
store, dropping a database or upgrading a schema requires an explicit right
over the whole affected resource. A partially authorized atomic mutation fails
before any write. If the provider cannot enforce these semantics, deny that
operation. [Existing shared-store contract](../packages/cem-components/docs/application.md#shared-storage-access-and-commit-order)

**Revocation for data APIs.** Fence handles, transactions and subscriptions
with the same grant generations as connections. Recheck queued mutations at
the provider's commit boundary. Abort uncommitted work when revocation wins.
When commit wins, preserve its authorized effect and record its disposition
even if the app closes before acknowledgment. Suppress later reads and queued
notifications from that generation. Reopening the same alias requires a fresh
binding.

## Redirects and live connections

**Default to refusing redirects.** `@redirect=checked` permits a
trusted adapter to follow an HTTP GET or HEAD redirect only when every applicable
layer permits following it. Resolve each target against the previous URL and
authorize the complete next request before contacting it. Recheck the actual
method, effective port, path, credentials and header policy at every hop.
Keep one bounded hop budget across the chain. A denial stops the chain before
the target receives a request. Keep the existing POST redirect refusal.

Browser Fetch exposes an opaque redirect to ordinary script in manual mode.
Use `redirect: "error"` for direct browser requests unless an admitted transport
can inspect and authorize each hop before sending it. Checking the final URL
after automatic following is too late. A mediated transport also needs its own
endpoint policy. [Fetch redirect processing](https://fetch.spec.whatwg.org/#http-redirect-fetch) · [Opaque redirects](https://fetch.spec.whatwg.org/#concept-filtered-response-opaque-redirect)

Scope grants by transport. V1 candidates are HTTP, event streams and WebSocket.
HTTP rules name their methods explicitly. Event streams use GET. WebSocket rules
use the logical `ws` or `wss` scheme. Credentials default to omitted and require
separate explicit host authority wherever the adapter uses them. A credentials
rule is a ceiling. An `include` grant also admits a request that omits credentials. Native WebSocket
handshakes include credentials and refuse redirects. Admit that adapter only if
the grant permits its credential behavior. Use a controlled stream reader when
native EventSource would hide a redirect or automatic reconnect.
[WebSocket handshake](https://websockets.spec.whatwg.org/#opening-handshake) · [EventSource processing](https://html.spec.whatwg.org/multipage/server-sent-events.html#processing-model)

**Revocation takes effect at the grant boundary.** When an issuer withdraws or
replaces a grant, invalidate that generation and its dependent leases before any
new send, retry, callback or publication. Abort pending fetches and body readers,
close event streams and sockets, discard application-queued sends and suppress
late data. Publish one terminal `grant-revoked` result to the still-live owning
app. Siblings using independent grants continue. A replacement grant permits a
newly authorized open. It never revives an old handle or retries a write.
Bytes already sent or handed to the browser for transmission can still reach the
peer. Completed remote work and data already delivered remain effective.
[WebSocket close semantics](https://websockets.spec.whatwg.org/#dom-websocket-close)

## Browser implementation

The trusted host calls `grantCemApplicationNetwork(null, root, ceiling)` with
an out-of-band `export` element targeting the mounted root. For each embedding
edge it calls `grantCemApplicationNetwork(container, child)`. The runtime
selects the container's snapshotted export by recipient ID and scope, intersects
it with the child's filter and returns a generation-specific `revoke()` handle.
Reissuing an ancestor grant requires reissuing dependent child grants.

`openCemApplicationHttpResource(consumer, request)` supplies guarded HTTP to
host services. Declarative `http-request` resolves its owner at initiation.
The same grant checks cover app-bound GraphQL requests, compiled declaration
assets, external declaration and template reads, and retained stylesheet reads.
Application HTTP reads use the guarded transport. The runtime option
`applicationHttpHop` is for a trusted transport that honors the abort signal and
exposes each redirect without following it. Direct browser Fetch refuses redirects.
`openCemApplicationWebSocket(consumer, url, events)` guards native socket
opens, sends and incoming messages and requires `credentials="include"`.

The [browser check](../packages/cem-elements/tests/application-network.mjs) runs
against two loopback endpoints in Chromium and Firefox, in light DOM and
same-origin frame hosts. Run `nx run cem-elements:test:application-network`.
The complete consumer inventory, browser-initiated subresources in rendered DOM
and CSS, native EventSource, worker and SSR grant propagation, and cross-origin
app bridges remain implementation work. Existing page-level code outside a
mounted application continues to use its host's page policy. Cookie and
storage resource masks remain at the design stage.

## Implementation acceptance

1. Inventory every network-producing CEM element and adapter. For each, prove
   that a denied operation produces zero hits at a controlled destination,
   including indirectly rendered resources and worker-initiated reads.
2. Exercise every matcher row, combining allowed hosts with disallowed ports,
   methods, paths and credentials. Cover IDNA, trailing dots, dot segments,
   ambiguous encodings and exact apex versus nested subdomains.
3. Run a three-level app and an independent sibling in light DOM and same-origin
   iframes in Chromium and Firefox. Check narrower grants, missing delegation,
   equal scope names, shared loaders and a cached response after revocation.
4. Use two local endpoints to distinguish a refused first hop, a permitted
   redirect chain and a denied later hop. Count requests at each server.
   Direct browser Fetch must refuse the redirect when per-hop checks are unavailable.
5. Revoke an ancestor grant during a streaming read and during a socket
   session. Verify closure, suppression of queued delivery and reconnects,
   one terminal outcome, and uninterrupted delivery to the independent sibling.
6. Give two apps the same key strings in different partitions. Verify exact and
   masked local/session keys, a literal asterisk, filtered enumeration and
   subscriptions, write-only privacy and all-or-nothing refusal of a bulk mutation.
7. Give a child a host-only, blue-project cookie grant. Exercise a matching
   cookie, a sibling path, a broader Domain or Path write, HttpOnly filtering,
   and a credentialed request where only one of the two required grants exists.
8. For every additional adapter, test its indirect operations. Include an
   IndexedDB index scan or count, a Cache Storage network fill and revocation
   on either side of an atomic commit.
