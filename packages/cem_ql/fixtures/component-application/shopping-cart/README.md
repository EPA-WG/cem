# Shopping cart: compiled UI and GraphQL JSON

This reference describes a small cart that shows products, quantities, totals
and service errors. The build compiles its declarative view once; native tests
reload the binary template and render changing JSON responses without compiling
the view again. It is a deployment-contract fixture, not a runnable storefront:
the quantity/remove controls have no transport binding yet, and no server or
Studio project is installed by this fixture.

- [profile.json](profile.json): explicit app requirements and input files. This
  reference manifest is consumed by native tests; it is not a finalized loader
  protocol or proof that unused compiled engines have been removed.
- [cart-view.cem](cart-view.cem): native declarative view using five existing
  `cem-components` members. It traverses the retained native JSON source tree;
  neither JavaScript nor a JSON AST transport reconstructs the view's data.
- [schema.graphql](schema.graphql): illustrative service contract. The server owns
  stock, pricing and totals; amounts use integer minor units.
- [operations.graphql](operations.graphql): fixed Cart, quantity-change and
  remove operations. Mutation result aliases preserve the same `data.cart` shape.
  Native GraphQL schema/operation validation and generated variable/response
  checks remain implementation work; these files are not yet validated against
  a GraphQL engine by the fixture runner.
- [cart.json](cart.json), [updated.json](updated.json),
  [removed.json](removed.json), [empty.json](empty.json): initial, changed,
  removed-line and empty-cart API responses. The changed response includes text
  that must remain escaped when displayed.
- [partial-error.json](partial-error.json), [failed.json](failed.json): a usable
  cart with a null nullable shipping estimate and its error, and a failed cart
  request. Non-null cart lines/totals remain available in the partial result.

Runtime requirements are compiled native template/expression execution, JSON
decoding, shared form/action behaviors, DOM updates and ownership/recovery
controls. The current template wire format still reparses native CSS on reload;
CSS is retained until a compiled style contract is proven. Other formats and
authoring tools are demand-loaded candidates. The browser still requires source
text for template-artifact admission today, so a source-free application bundle
and a smaller WASM remain pending. No test contacts an API, stores credentials
or changes credential recovery refusal, checkpoint budgets or deadlines.
