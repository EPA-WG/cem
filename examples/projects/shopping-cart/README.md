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

The frame runs without scripts. The parent host owns network dispatch and
cancels pending work when the preview closes. Recovery for this interactive
application requires a native resume adapter.

## Resource identities

- The response uses `application/json` and `https://cem.dev/ns/data/json/1`.
- The GraphQL reference files use `text/plain; charset=utf-8`.
- The graph uses `application/cem` and `https://cem.dev/ns/cli/transform-config/1`.
- The template uses `application/vnd.cem.template+cem` and `https://cem.dev/ns/template/cem-native/1`.
- The view uses `application/vnd.cem.element-template+cem` and `https://cem.dev/ns/template/cem-element/1`. Its `response` binding is supplied and validated by the selected application compiler. The runnable configuration parses the JSON application manifest.
- The expected HTML uses `text/html` and `https://cem.dev/ns/data/html/1`.
