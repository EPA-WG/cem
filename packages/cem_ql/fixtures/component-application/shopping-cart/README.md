# Shopping cart: compiled UI and GraphQL JSON

This reference describes a small cart that shows products, quantities, totals
and service errors. The build compiles its declarative view once; native tests
reload the binary template and render changing JSON responses without compiling
the view again. The browser fixture reads the initial cart and binds quantity
changes and removal to a local GraphQL endpoint through shared guarded
transport. The [Studio example](../../../../../examples/projects/shopping-cart/README.md)
builds a read-only preview from the saved response.

- [profile.json](profile.json) declares app requirements, input files and the
  variable sources for each explicit GraphQL action.
- [cart-view.cem](cart-view.cem): native declarative view using five existing
  `cem-components` members. It traverses the retained native JSON source tree;
  neither JavaScript nor a JSON AST transport reconstructs the view's data.
- [schema.graphql](schema.graphql): illustrative service contract. The server owns
  stock, pricing and totals; amounts use integer minor units.
- [operations.graphql](operations.graphql): fixed Cart, quantity-change and
  remove operations. Mutation result aliases preserve the same `data.cart` shape.
  The application build validates the SDL and operations and generates a native
  JSON guard. The contract tests exercise variables and all six response fixtures.
  The [build-bound runtime](../../../../../docs/cem-components-shopping-cart-profile.md#build-bound-graphql-entry)
  embeds the compiled view, reviewed registry, guard and exact operation document.
- [cart.json](cart.json), [updated.json](updated.json),
  [removed.json](removed.json), [empty.json](empty.json): initial, changed,
  removed-line and empty-cart API responses. The changed response includes text
  that must remain escaped when displayed.
- [partial-error.json](partial-error.json), [failed.json](failed.json): a usable
  cart with a null nullable shipping estimate and its error, and a failed cart
  request. Non-null cart lines/totals remain available in the partial result.

The browser fixture registers the canonical compiled widgets with the shared
`registerCompiledDeclaration` API and uses the selected WASM guard for the
entry, variables and responses. It uses a loopback GraphQL stub and checks
that rebinding a fresh selected heap sends no saved mutation.
See the [build and registration instructions](../../../../../docs/cem-components-shopping-cart-profile.md#build-generated-declarations).

```sh
yarn nx run cem_ql:test:selected-application-actions
```
