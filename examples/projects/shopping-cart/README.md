# Shopping cart

Build a cart summary from the original JSON envelope for the GraphQL `Cart` query. The preview displays the saved product names, quantities, shipping estimate and server-provided total.

## Essential files

- [data/cart.json](data/cart.json) is the initial Cart response.
- [graphql/schema.graphql](graphql/schema.graphql) describes the service types.
- [graphql/operations.graphql](graphql/operations.graphql) defines the cart read, quantity change and line removal.
- [graphs/cart.cem](graphs/cart.cem) reads the saved response and writes the HTML entry artifact.
- [templates/snapshot.cemt](templates/snapshot.cemt) renders the cart summary and any service errors.
- [views/cart.cem](views/cart.cem) is the guarded application view for the interactive preview.
- [application.json](application.json) declares the selected application, its response binding and GraphQL operations.
- [expected/cart.html](expected/cart.html) records the initial HTML for comparison.

## Build and preview

In Studio, open **Samples → Content types and schemas → Documents and UI →
Shopping cart**. Choose **Build and preview cart** (`build-cart`), then
**Build project**. The artifact selector opens `generated/cart.html`.
To try another result, clone the sample, edit the saved JSON, then save and
rebuild the clone.

The Studio build reads the saved response from the project and the preview is a
read-only snapshot. The separate selected-runtime cart reads an authorized
same-origin GraphQL service and supports quantity and removal actions.

## Interactive preview

The `live-cart` configuration selects **Interactive cart**. It uses the
deployment's trusted `shopping-cart-graphql` host to query `/graphql` and send
quantity and removal actions. Deploy the optional cart preview build and provide
that same-origin service. The view, schema and operations must match the
deployed native application. A changed source requires a new application build.

The trusted deployment grant must declare `mutationIdempotency: "server-keyed"`.
Every mutation sends its original host operation ID as `Idempotency-Key`. The
service must atomically persist the authenticated caller, key, request identity
and result: duplicate deliveries return the same result, while a different
request reusing that key is refused. The loopback verification service models
this contract; production storage and retention belong to the deployed service.
This matters even when the application calls fetch once: a browser can resend
an upload after a reused connection loses its response headers ([Chromium's
retry decision](https://chromium.googlesource.com/chromium/src.git/+/HEAD/net/http/http_network_transaction.cc#2442)).

The frame runs without scripts. The parent host owns network dispatch and
cancels pending work when the preview closes. The selected heap retains the
accepted response's original bytes and source maps, and claims each mutation
in its native replay fence before transport. A failed response leaves the last
accepted source intact and the host journal blocks further writes.

The native response checkpoint reimports those original bytes under the linked
entry/GraphQL pins and requires an exact terminal journal. Its inert capsule
and retirement pin do not authorize allocation or UI binding. A checkpoint-enabled
shared frame host connects the response and frame/control owners to the same
original physical group. Both heaps must retire before fresh allocation; the
original source binding ticket and every rendering owner ticket must join the
same resume transaction. The frame document and live controls remain in place.

This connection is verified for repeated worker and owned-fallback recovery,
including actions after recovery, missing peers, stale frames and unknown
mutation outcomes. The deployed Studio service worker and cache inventory also
support offline worker/fallback recovery of an accepted response. Recovery does
not query the service or send a mutation. Missing worker WASM refuses allocation
and preserves the retained frame; it does not select a fallback or refresh the
cart. An initial unavailable read never invents a response.

Offline, HTTP 503 and lost-reply mutations remain unknown and block recovery
and further mutations after reconnect. Independent service-side knowledge does
not automatically settle the client journal. Whole-Studio admission remains a
separate gate, and automatic recovery stays disabled.
The capsule, including its metadata, is limited to 1 MiB. The retained-response
byte census counts acquired bytes; it is not a native heap/RSS measurement.

## Resource identities

- The response uses `application/json` and `https://cem.dev/ns/data/json/1`.
- The GraphQL reference files use `text/plain; charset=utf-8`.
- The graph uses `application/cem` and `https://cem.dev/ns/cli/transform-config/1`.
- The template uses `application/vnd.cem.template+cem` and `https://cem.dev/ns/template/cem-native/1`.
- The view uses `application/vnd.cem.element-template+cem` and `https://cem.dev/ns/template/cem-element/1`. Its `response` binding is supplied and validated by the selected application compiler. The runnable configuration parses the JSON application manifest.
- The expected HTML uses `text/html` and `https://cem.dev/ns/data/html/1`.
