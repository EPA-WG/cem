# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in [`wishlist.md`](wishlist.md). Completed implementation
history belongs in git history and the feature-specific docs linked below.

## Immediate Tasks

- [ ] Enable schema behavior, including `{diagnostic}` behavior, to be defined
      entirely through declarative CEM-ML syntax. A diagnostic `@code` must
      resolve to a schema-visible algorithm contract instead of acting only as
      an inert output label: the contract selects either a minimal
      engine-provided behavior or a schema-owned declarative function, defines
      how a failure is matched, and produces the diagnostic result without a
      package-specific Rust validation branch. This is required for custom
      schemas to introduce their own validation algorithms. The initial slice
      now declares an engine behavior catalog in `cem-schema.cem`, resolves
      diagnostic behavior references through schema `{uses}` aliases, compiles
      severity/message/source metadata, and dispatches
      `schema:field-contract` generically. The bootstrap vocabulary now defines
      behavior primitives, function bindings, typed inputs, typed parameters,
      diagnostic result/detail shape, severity/message ownership, and
      source-range propagation in `cem-schema.cem`. Direct inline
      function-backed diagnostics now select candidates with CEM-QL, match
      failures with CEM-QL, and execute schema-declared CEM-ML behavior
      function bodies through the generic bridge without CEMT. Defaulted typed
      behavior parameters now bind into direct CEM-ML behavior functions;
      imported/qualified reusable CEM-ML behavior functions now resolve
      through schema `{uses}` aliases when their declarations opt in with
      reusable visibility. Diagnostic-scoped `{arguments}` now provide
      non-default parameter overrides for function-backed behaviors. The first
      field-contract-backed engine behavior aliases now exist for required
      fields, forbidden fields, dependent-required fields, mutual exclusions,
      child occurrence, and path layout; attribute-owned aliases now exist for
      value vocabularies, scalar type syntax, and datatype parameters.
      Constraint-level bindings now exist for resource readability,
      resource parse/validation, and reference resolution; field-contract-level
      bindings now exist for broader dependency and choice/case algorithms.
      Both preserve stable diagnostic family codes. Attribute-owned datatype
      parameter checks now cover integer and number `minInclusive`/`maxInclusive`/
      `minExclusive`/`maxExclusive`, numeric `totalDigits`/`fractionDigits`,
      string `minLength`/`maxLength`/`length`, `stringPrefixes`/
      `stringSuffixes`, list `itemCount`/`minItems`/`maxItems`, regex `pattern`, path `pathPrefixes`/`pathExtensions`, URI
      `uriSchemes`/`uriHosts`/`uriPorts`/`uriRequiresAuthority`/`uriPathPrefixes`/
      `uriPathExtensions`/`uriQueries`/`uriQueryParameters`/
      `uriQueryParameterValues`/`uriQueryForbiddenParameters`/
      `uriQueryRequiredParameters`/`uriFragments`, and media-type
      `mediaTypes`/`mediaTypeTypes`/`mediaTypeSubtypes`/`mediaTypeSuffixes`/`mediaTypeParameters`/
      `mediaTypeParameterValues`/`mediaTypeForbiddenParameters`/
      `mediaTypeRequiredParameters`.
      Schema-definition validation now rejects numeric bounds/digit-counts on
      non-numeric attributes and string length/prefix/suffix/pattern params on
      non-string attributes, list item-count params on non-list attributes,
      path params on non-path attributes, URI params on non-URI attributes, and
      media-type params on non-media-type attributes.
      Required-one/max-one attribute choice cardinality now executes through
      `schema:choice-case`. Broader child occurrence ranges now execute
      through `schema:child-occurrence`; child-set cardinality now also
      executes through `schema:child-occurrence`; required and forbidden
      relative child ordering now also executes through
      `schema:child-occurrence`; required and forbidden
      boundary child placement now also execute through
      `schema:child-occurrence`; exact, required, forbidden, prefix, and suffix
      child sequences now also execute through `schema:child-occurrence`;
      accepted-child allow-lists also execute
      through `schema:accepted-children`. Presence-gated dependent-required
      field groups now execute when all declared `when-present-attributes` are
      present and all declared `when-absent-attributes` are absent. The
      `schema:path-layout` behavior contract now declares its
      `pathLayout` and `invalidValues` result details. Combined
      value, presence, and child-structure-gated required/forbidden dependent
      field groups now execute through `schema:field-dependency`, and
      dependency result contracts declare their required/missing/forbidden/
      invalid field, condition, and actual-value details. The
      `schema:choice-case` behavior contract now declares flat and grouped
      choice result details emitted by the engine. Required, forbidden, and
      mutual-exclusion field behavior contracts now declare their emitted
      field/value result details. The `schema:datatype-param` result contract
      now declares the emitted family-specific string/list/digit/path/URI/
      media-type detail keys. Constraint-owned resource-read, resource-parse,
      and reference-resolution behavior contracts now declare their emitted
      path, error, source diagnostic, expected-value, and invalid-value
      details. The minimal engine-provided behavior library is now published;
      the complete schema-package and CLI example matrix remains.
  - [x] Allow declarative behavior to select candidate nodes and match failure
        conditions with CEM-QL, then invoke a schema-declared CEM-ML behavior
        function to calculate the result and structured diagnostic details.
        Direct inline behavior functions now execute for AST validation through
        the CEM-QL/schema-behavior bridge, including defaulted typed behavior
        parameters. Qualified function references now resolve through schema
        `{uses}` aliases to visible reusable CEM-ML behavior functions so
        custom schema packages can reuse result builders without registering
        Rust code. Diagnostic-scoped `{arguments}` bind cross-diagnostic
        non-default parameter overrides, and schema-package CLI examples now
        cover invalid CEM-QL `@select` and `@match` expressions.
  - [x] Publish a minimal useful library of CEM engine-provided algorithms that
        schema authors can consume through diagnostic `@code` behavior
        references. Cover the common algorithmic variations needed to create a
        schema, including required/forbidden fields, value vocabularies and
        scalar/datatype parameters, child occurrence, dependency and
        choice/case rules, reference resolution, and source/resource failures.
        Keep these as general schema primitives rather than package-specific
        semantic validators. The bootstrap schema now declares initial
        field-contract-backed aliases for `schema:required-fields`,
        `schema:forbidden-fields`, `schema:dependent-required-fields`,
        `schema:mutual-exclusion`, `schema:child-occurrence`,
        `schema:accepted-children`, and `schema:path-layout`, broader
        `schema:field-dependency` and `schema:choice-case` aliases, plus attribute-owned aliases for
        `schema:value-vocabulary`, `schema:scalar-type`, and
        `schema:datatype-param`, plus constraint-owned bindings for
        `schema:resource-readable`, `schema:resource-parse`, and
        `schema:reference-resolution`; required-one/max-one attribute choice
        cardinality now executes through `schema:choice-case`; broader child
        occurrence ranges, child-set cardinality, required and forbidden
        relative child ordering, required and forbidden boundary child placement, plus
        exact/required/forbidden/prefix/suffix child sequences, now execute
        through `schema:child-occurrence`;
        accepted-child allow-lists now execute through
        `schema:accepted-children`; basic
        identifier, name-list, type-reference, explicit symbol/wildcard
        reference, number, qualified-name, semantic version, URI, media-type,
        and scope-context path scalar syntax now execute through
        `schema:scalar-type`; nested choice/case groups now execute through
        `schema:choice-case`; number inclusive/exclusive bounds
        now reuse the existing range-parameter vocabulary; numeric
        `totalDigits` and `fractionDigits` now execute through
        `schema:datatype-param`; media-type structured suffix, parameter
        value, and forbidden parameter constraints now also execute through
        `schema:datatype-param`; scope-context path prefix and extension
        constraints now also execute through `schema:datatype-param`; string
        prefix and suffix constraints now also execute through
        `schema:datatype-param`; name-list and wildcard-name-list item-count
        constraints now also execute through `schema:datatype-param`; regex
        pattern now also executes through `schema:datatype-param`. This covers
        the currently declared attribute datatype-parameter vocabulary.
        Presence-gated dependent-required field groups with multiple required
        gate attributes now execute through `schema:field-dependency`;
        combined value-and-presence-gated dependent field groups now also
        execute through `schema:field-dependency`; child-structure-gated
        required, forbidden, and forbidden-value dependencies now also execute through
        `schema:field-dependency`; gated forbidden-field and
        forbidden-value dependencies now also execute through
        `schema:field-dependency`. The `schema:path-layout` result contract now
        exposes the emitted path layout and invalid-value details, and the
        `schema:choice-case` result contract now exposes emitted flat/grouped
        choice details. Required, forbidden, and mutual-exclusion field result
        contracts now expose their emitted field/value details, and the
        `schema:value-vocabulary` and `schema:scalar-type` result contracts
        now expose emitted attribute value/type detail keys. The
        `schema:datatype-param` result contract now exposes emitted
        string/list/digit/path/URI/media-type detail keys. Constraint-owned
        `schema:resource-readable`, `schema:resource-parse`, and
        `schema:reference-resolution` result contracts now expose emitted
        resource path, error/source-diagnostic, expected-value, and
        invalid-value detail keys.
  - [x] Compile behavior declarations and references into the generic schema
        model, reject missing or incompatible behavior references, and dispatch
        them through a single runtime evaluation path. The resolved behavior
        must own severity, message/detail construction, CEM-QL match semantics,
        and source-map ranges while `@code` remains stable in CLI and report
        output. `SchemaDocumentModel` now compiles the first behavior catalog,
        rejects unresolved diagnostic references and unknown engine behaviors,
        surfaces those compiler diagnostics during schema-document validation,
        and uses declared severity/message metadata for field-contract
        diagnostics. Function behavior `@select`/`@match` expressions and
        inline CEM-ML result functions now dispatch through the generic runtime
        path outside CEMT, and required function parameters must resolve from
        declared inputs or defaulted behavior parameters. Reusable imported
        functions now resolve by schema URI and `{uses}` alias when visible;
        non-default parameter override binding now comes from diagnostic-scoped
        `{arguments}` for function behaviors. Schema-package CLI examples now
        cover unresolved function references, invalid query expressions,
        incompatible function signatures, unsupported behavior implementations
        and execution placements, missing function/result/body pieces,
        incompatible result/return contracts, invalid engine behavior
        arguments, and invalid behavior body evaluation.
  - [x] Add schema-package and CLI examples for every engine-provided algorithm
        and its meaningful parameter or matching variations. Include passing
        and failing fixtures, expected diagnostic codes, structured details,
        severity, and source ranges so the example set documents the minimal
        behavior library available to schema authors. The typed-resource schema
        example now covers the initial `schema:required-fields` behavior alias
        on a conditional field contract, child-structure-gated required/
        forbidden `schema:field-dependency`, and `schema:value-vocabulary` on an
        attribute declaration, `schema:scalar-type`
        number/qualified-name/semver/URI/media-type/path
        syntax on attribute declarations, plus nested exact-one
        `schema:choice-case` cardinality and `schema:datatype-param` decimal
        number bounds, numeric digit-count constraints, and string
        length/prefix/suffix constraints, list item-count constraints, plus path prefix/extension, URI scheme/host/port/authority/path-prefix/path-extension/query/query-parameter-name/value/forbidden-parameter/required-parameter/fragment, and media-type
        essence/type/subtype/structured-suffix/parameter-name/value/forbidden-parameter/required-parameter constraints, plus child-set exact-one and
        selected-count/selected-distinct-count/distinct-count/ordered/forbidden-ordered/boundary/forbidden-boundary/
        exact-sequence/required-sequence/forbidden-sequence/prefix-sequence/suffix-sequence
        `schema:child-occurrence`;
        schema-package examples cover constraint-level
        `schema:resource-readable`, `schema:resource-parse`, and
        `schema:reference-resolution` bindings through operational artifact
        and example diagnostics, plus field-contract-level
        `schema:required-fields`, `schema:forbidden-fields`,
        `schema:dependent-required-fields`, `schema:mutual-exclusion`,
        `schema:field-dependency`, `schema:child-occurrence`,
        `schema:accepted-children`, and `schema:path-layout` bindings through
        converter/artifact/example diagnostics,
        and ranged, child-set, selected-count, selected-distinct-count, distinct-count, ordered,
        forbidden-ordered, boundary, forbidden-boundary, exact-sequence, required-sequence,
        forbidden-sequence, prefix-sequence, and suffix-sequence `schema:child-occurrence`
        through the typed-resource schema
        example; CLI runtime coverage now also proves regex `pattern`
        `schema:datatype-param` details through a schema-package-loaded custom
        schema; CLI runtime coverage now also proves gated forbidden-field and
        forbidden-value `schema:field-dependency` details through a
        schema-package-loaded custom schema; CLI runtime coverage now also
        proves child-structure-gated required/forbidden
        `schema:field-dependency` details through a schema-package-loaded
        custom schema; CLI runtime coverage now also
        proves passing and failing `schema:required-fields`,
        `schema:forbidden-fields`, and `schema:mutual-exclusion` details
        through a schema-package-loaded custom schema; CLI runtime coverage
        now also proves generic `schema:field-contract` details through a
        schema-package-loaded custom schema; CLI runtime coverage
        now also proves missing and conflicting grouped `schema:choice-case` details
        through a schema-package-loaded custom schema; CLI runtime coverage
        now also proves passing and failing `schema:accepted-children` details
        through a schema-package-loaded custom schema; CLI runtime coverage
        now also proves passing and failing `schema:path-layout` details
        through a schema-package-loaded custom schema; CLI runtime coverage
        now also proves passing, missing, and conflicting child-choice
        `schema:child-occurrence` details through a schema-package-loaded
        custom schema; CLI runtime coverage now also proves ordered,
        forbidden-ordered, boundary, required-sequence, forbidden-sequence, exact-sequence,
        prefix-sequence, and suffix-sequence `schema:child-occurrence` details
        through a schema-package-loaded custom schema; CLI runtime coverage
        now also proves per-child,
        total-child, distinct-child, and selected-child count
        `schema:child-occurrence` details through a schema-package-loaded
        custom schema; CLI runtime coverage now also proves passing and
        failing attribute `schema:value-vocabulary` and `schema:scalar-type`
        details through a schema-package-loaded custom schema; CLI runtime
        coverage now also proves operational payload details for
        `schema:resource-readable`, `schema:resource-parse`, and
        `schema:reference-resolution` through checked-in schema-package
        examples;
        schema-package CLI examples now also cover schema-definition
        failures for invalid datatype parameter declarations on integer/number
        bounds, numeric digit counts, string length/prefix/suffix params, list
        item-count params, regex patterns, path prefixes/extensions, URI
        tokens/parameters, and media-type tokens/parameters.
        They also cover incompatible primitive type declarations for numeric
        bounds, numeric digit counts, string length/prefix/suffix/pattern
        params, list item-count params, path params, URI params, and
        media-type params.
        CLI tests now assert structured schema-definition datatype-param
        details for those failing declarations, plus
        structured `behavior`, `checkKind`, `contract`, severity, and source
        range details for representative schema-package converter, artifact,
        and example engine behavior examples. The native tests cover warning severity,
        message text, structured details, source ranges, unresolved codes,
        field-contract behavior aliases, field-contract-local behavior
        bindings, attribute behavior aliases including string prefix/suffix
        and list item-count datatype-param bindings, constraint behavior aliases, and
        unknown behaviors; the CLI example set also includes a schema that
        fails an unknown behavior
        reference. This completes the current audited algorithm matrix from
        `cem-schema.cem` and the schema-package `single-primary-content-type`
        function behavior.
  - [x] Add a custom-schema example that defines a new algorithm using only
        CEM-ML, CEM-QL, and a declarative function, then prove that changing the
        declared match or function changes validation behavior without adding
        or editing Rust. Include negative tests for unresolved `@code`
        behaviors, invalid parameters, incompatible function signatures, and
        recursive or unsafe evaluation. Checked-in schema-package CLI examples
        now define a custom `page-label` behavior and a stricter variant whose
        CEM-QL match and CEM-ML function result change validation behavior
        without a Rust branch; negative schema-package CLI examples now cover
        unresolved behavior/function bindings, invalid CEM-QL select/match
        expressions, invalid diagnostic argument types, unbound function
        parameters, and a rejected CEMT-style self-call body.

- [ ] Implement schema-owned field contracts for every schema-declared field
      before adding more package-specific Rust validation branches. The current
      implementation compiles `required-attributes`, `optional-attributes`,
      child allow-lists, initial field contracts, and exact/ranged child
      occurrence from `.cem` in `packages/cem_ml/src/schema/document_model.rs`;
      remaining schema-package converter/artifact operational rules and
      example schema/content-type cross-reference execution still live in
      `packages/cem_ml/src/validation/rules.rs`; manifest descriptor loading
      now skips incomplete rows instead of owning field diagnostics, with an
      audit guard in `packages/cem_ml/src/schema/registry.rs`.
  - [x] Add failing tests first for the principle in `document_model.rs`,
        `rules.rs`, and the CLI schema examples: changing a field contract in a
        `.cem` schema must change validation behavior without adding or editing
        a package-specific Rust branch. `document_model.rs` now mutates the
        schema-package `package-required-children` contract in-memory and
        proves the missing `content-type` validation changes through schema
        source alone; `rules.rs` now runs the schema document model rule
        against the same source mutation and proves rule-layer validation
        follows the `.cem` contract. CLI source-mutation coverage now loads a
        temporary schema package through `--schema-package`, registers its
        CEM-ML schema source as the validation document model, and proves that
        changing the package schema field contract changes `validate` results
        without a Rust validation branch.
  - [ ] Expand the initial `field-contracts` vocabulary in
        `packages/cem_ml/schema-packages/schema/v1/schema/cem-schema.cem`.
        The schema language now models element-bound required/optional/
        forbidden fields, value, multi-attribute presence/absence, and child
        presence/absence conditional selectors,
        dependent-required groups with all-present/all-absent and combined
        value/presence/child-structure gates, including gated forbidden
        fields and forbidden values,
        value-specific forbidden fields, exact-one child occurrence contracts,
        required-one/max-one attribute choice cardinality, nested choice/case
        groups, package-relative path-layout contracts, diagnostic families,
        attribute default metadata and validation, and attribute `@values`
        vocabularies, plus
        boolean/integer/number/identifier/name-list/type-reference/
        symbol-reference/wildcard-reference/qualified-name/semver/basic
        URI/media-type/path scalar syntax,
        integer `minInclusive`, `maxInclusive`,
        `minExclusive`, and `maxExclusive` bounds, numeric `totalDigits` and
        `fractionDigits`, plus string `minLength`, `maxLength`, and `length`
        params, string `stringPrefixes` and `stringSuffixes`, list `itemCount`,
        `minItems`, and `maxItems`, regex `pattern`, path `pathPrefixes`/`pathExtensions`,
        URI `uriSchemes`,
        `uriHosts`, `uriPorts`, `uriRequiresAuthority`, `uriPathPrefixes`, and
        `uriPathExtensions`/`uriQueries`/`uriQueryParameters`/
        `uriQueryParameterValues`/`uriQueryForbiddenParameters`/
        `uriQueryRequiredParameters`/`uriFragments`, and media-type `mediaTypes`,
        `mediaTypeTypes`, `mediaTypeSubtypes`, `mediaTypeSuffixes`, `mediaTypeParameters`, `mediaTypeParameterValues`,
        `mediaTypeForbiddenParameters`, and `mediaTypeRequiredParameters` datatype params,
        plus compatibility checks for numeric bounds/digit counts and string
        length/prefix/suffix/pattern params, plus list item-count, path, URI,
        and media-type params, against their target primitive families,
        plus accepted-child allow-lists, forbidden-child occurrence,
        max-one child occurrence, exact child counts, child-set cardinality,
        total, selected, selected-distinct, and distinct child-count
        bounds, min/max child occurrence ranges, required/forbidden relative child
        ordering, and required/forbidden boundary child placement, plus exact/required/forbidden/prefix/suffix child
        sequences; it still needs RELAX NG-style datatype params beyond the
        initial scalar and datatype variations, default value materialization,
        richer dependent-required field groups
        beyond value/presence/absence/child-structure gates, and additional child occurrence
        variants beyond exact/named/total/selected/selected-distinct/distinct count bounds,
        child-set cardinality, required/forbidden relative ordering,
        required/forbidden boundary placement, and
        exact/required/forbidden/prefix/suffix child sequences.
  - [ ] Extend the compiled Rust schema contract model. `SchemaDocumentModel`
        now compiles initial `field-contract` declarations and evaluates
        required/forbidden fields, attribute `@values` vocabularies, and
        `schema:boolean`/`cemml:boolean`, `schema:integer`/`cemml:integer`,
        `schema:number`/`cemml:number`,
        `schema:identifier`/`cemml:identifier`,
        `schema:name-list`/`cemml:name-list`,
        `schema:type-reference`/`cemml:type-reference`,
        `schema:symbol-reference`/`cemml:symbol-reference`,
        `schema:wildcard-name`/`cemml:wildcard-name`,
        `schema:wildcard-name-list`/`cemml:wildcard-name-list`,
        `schema:wildcard-type-reference`/`cemml:wildcard-type-reference`,
        `schema:qualified-name`/`cemml:qualified-name`,
        `schema:semver`/`cemml:semver`, `schema:uri`/`cemml:uri`, `schema:media-type`/
        `cemml:media-type`, plus `schema:path`/`cemml:path` attribute types,
        accepted-child allow-list field contracts, forbidden-child
        occurrence contracts, child-set cardinality contracts, exact
        child-count contracts, total/selected/selected-distinct/distinct child-count bound contracts,
        attribute and child presence/absence conditional selectors,
        required/forbidden relative child ordering contracts,
        required/forbidden boundary child placement contracts,
        exact/required/forbidden/prefix/suffix child sequence contracts, and attribute
        default metadata and validation; it
        still needs reusable string constraints beyond length/prefix/suffix/pattern and path constraints beyond
        prefix/extension and path-layout checks,
        URI/media-type constraints beyond initial allowed scheme/host/port/authority/path-prefix/path-extension/query/query-parameter-name/value/forbidden-parameter/required-parameter/fragment
        and media-type essence/type/subtype/structured-suffix/parameter-name/value/forbidden-parameter/required-parameter lists,
        RELAX NG-style datatype params beyond numeric bounds/digit counts,
        string length/prefix/suffix, list item counts, pattern, and allowed
        URI/media tokens, default value materialization, dependent field
        groups beyond value/presence/child-structure-gated required/forbidden fields, additional child
        occurrence variants beyond exact/named/total/selected/selected-distinct/distinct count
        bounds, child-set cardinality, required/forbidden relative ordering,
        required/forbidden boundary placement,
        and exact/required/forbidden/prefix/suffix child sequences, and richer case grouping
        for all schema elements.
  - [ ] Extend structured diagnostic details beyond initial required/forbidden
        field checks. The first generic field-contract evaluator now emits
        schema URI, element, contract name, check kind, required/optional/
        forbidden fields, missing/invalid fields, actual values, condition, and
        source-map range; required, forbidden, and mutual-exclusion field
        result contracts declare their emitted field/value details; attribute
        `@values` checks emit expected/actual value details; boolean, integer, number, identifier, name-list,
        type-reference, symbol-reference, wildcard reference, qualified-name,
        semver, basic URI, basic media-type, and path type checks now emit
        expected/actual details; integer
        `minInclusive`, `maxInclusive`,
        `minExclusive`, and `maxExclusive` checks, numeric `totalDigits` and
        `fractionDigits` checks, string `minLength`, `maxLength`, and `length`
        checks, string `stringPrefixes`/`stringSuffixes` checks, list
        `itemCount`/`minItems`/`maxItems` checks, plus regex `pattern` checks emit
        datatype-param details; value-specific forbidden checks emit
        forbiddenAttributeValues and invalidValues details; child occurrence
        checks emit required/forbidden/exact/max-one/min/max children,
        required-one/max-one child choices, present/missing/conflicting child
        choices; choice-case checks emit flat and grouped choice details;
        exact/min/max total children, invalid/missing/duplicate/
        invalid-exact/under-min/over-max named children,
        invalid-exact/under-min/over-max total-child flags,
        totalChildCount, exact/min/max distinct children,
        invalid-exact/under-min/over-max distinct-child flags,
        distinctChildCount, selectedChildren, exact/min/max selected children,
        invalid-exact/under-min/over-max selected-child flags,
        selectedChildCount, exact/min/max selected distinct children,
        invalid-exact/under-min/over-max selected-distinct-child flags,
        selectedDistinctChildCount, orderedChildren, orderedChildSequence,
        unorderedChildren, invalidChildOrder, forbiddenOrderedChildren,
        matchedForbiddenOrderedChildren, invalidForbiddenChildOrder,
        firstChild, lastChild,
        forbiddenFirstChild, forbiddenLastChild, actualFirstChild,
        actualLastChild, invalidFirstChild, invalidLastChild,
        invalidForbiddenFirstChild, invalidForbiddenLastChild,
        requiredChildSequence, actualChildSequence,
        matchedChildSequence, invalidChildSequence, forbiddenChildSequence,
        matchedForbiddenChildSequence, invalidForbiddenChildSequence,
        exactChildSequence, invalidExactChildSequence, prefixChildSequence,
        suffixChildSequence, invalidPrefixChildSequence,
        invalidSuffixChildSequence, and childCounts details;
        required-one/max-one attribute choice checks emit choice cardinality
        details; path-layout checks emit pathLayout and invalidValues details;
        path type checks emit expected/actual details; string prefix/suffix
        datatype-param checks emit expected values and actual string details;
        list item-count datatype-param checks emit actual item counts and item arrays; path prefix/extension
        datatype-param checks emit expected values and actual path/extension
        details; nested choice/case
        checks emit declared cases, present cases, missing cases, and
        conflicting cases; accepted-child checks emit accepted/invalid
        children and childCounts details; URI scheme, URI host, URI port, URI authority,
        URI path-prefix/path-extension/query/query-parameter-name/value/forbidden-parameter/required-parameter/fragment, and media-type essence/type/subtype/structured-suffix/parameter-name/value/forbidden-parameter/required-parameter datatype-param checks emit expected values, normalized actual tokens, and invalid/missing parameter arrays;
        schema-definition datatype-param checks now emit expected type and
        actual value type for incompatible numeric, string, list, path, URI,
        and media-type facets; datatype-param result contracts now declare the
        emitted runtime family details;
        URI/media-type constraints, dependency, additional
        child occurrence variants beyond exact/named/total/selected/selected-distinct/distinct count bounds, child-set cardinality, required/forbidden relative ordering, required/forbidden boundary placement, and exact/required/forbidden/prefix/suffix child sequences, and cross-reference checks need the same
        schema-owned detail shape.
  - [ ] Extend the generic field-contract evaluator. The first evaluator runs
        from schema URI plus content type, consumes the compiled contract
        model, preserves source-map ranges, and emits contract-declared
        diagnostic families such as `cem.schema_package.artifact_check`; it
        now emits structured details for required/forbidden field checks and
        attribute `@values` plus
        boolean/integer/number/identifier/name-list/type-reference/
        symbol-reference/wildcard-reference/qualified-name/semver/basic
        URI/media-type/path type, integer
        `minInclusive`/`maxInclusive`/`minExclusive`/`maxExclusive`,
        numeric `totalDigits`/`fractionDigits`, string
        `minLength`/`maxLength`/`length`, string `stringPrefixes`/`stringSuffixes`,
        list `itemCount`/`minItems`/`maxItems`, and regex `pattern`
        datatype-param checks, exact-one, child-choice, exact-count, total-count, selected-count, selected-distinct-count, distinct-count, and
        min/max child occurrence range checks, required-one/max-one attribute choice checks,
        and
        package-relative path-layout checks, scope-context path type checks,
        string prefix/suffix constraints, list item-count constraints, path prefix/extension constraints,
        nested choice/case checks, accepted-child allow-lists, forbidden-child
        occurrence checks, child-set cardinality, exact child-count and total/selected/selected-distinct/distinct child-count checks, URI scheme/host/port/authority/path-prefix/path-extension/query/query-parameter-name/value/forbidden-parameter/required-parameter/fragment
        constraints, and media-type essence/type/subtype/structured-suffix/parameter-name/value/forbidden-parameter/required-parameter constraints; it still needs coverage for additional
        URI/media-type constraints, additional datatype params, dependency variants, and
        additional child occurrence variants beyond exact/named/total/selected/selected-distinct/distinct count bounds, child-set cardinality, required/forbidden relative ordering, required/forbidden boundary placement, and exact/required/forbidden/prefix/suffix child sequences.
  - [ ] Move schema-package manifest field rules from Rust conditionals into
        `packages/cem_ml/schema-packages/schema-package/v1/schema/schema-package.cem`.
        Cover `package`, `schema`, `content-type`, `namespace`, `converter`,
        `from`, `to`, `parity-fixture`, `artifact`, and `example`.
        `content-type` child `value`, `namespace` child `uri`, and
        content-type `primary` boolean validation now stay in schema-owned
        validation instead of registry descriptor extraction; `parity-fixture`
        `id`/`path` requiredness now stays in schema-owned validation instead
        of conversion descriptor extraction; artifact `kind`/`path`
        requiredness now stays in schema-owned validation instead of
        conversion artifact descriptor extraction; converter `id`/
        `implementation`, implementation value, endpoint occurrence, and
        endpoint `content-type` requiredness now stay in schema-owned
        validation instead of conversion descriptor extraction; top-level
        package `id`/`version` and schema `uri`/`source` descriptor fields now
        use schema-root/catalog fallbacks so missing manifest field diagnostics
        stay schema-owned; package-root `schema` and `content-type` child
        presence now stays schema-owned through the
        `package-required-children` child-occurrence contract and
        `cem.schema_package.package_check`; exact-one primary content-type
        cardinality now stays schema-owned through a CEM-ML behavior function
        selected by CEM-QL and emitted as
        `cem.schema_package.content_type_conflict`; converter implementation
        value validation and CEMT template content-type/schema exact values now
        stay schema-owned through the generic `@values` vocabulary instead of
        package-specific runtime branches.
        Stale schema-package diagnostic declarations for retired package/
        converter field-specific codes (`missing_schema`,
        `converter_identity_missing`, `converter_identity_mismatch`, and
        `implementation_missing`) have been removed so the schema vocabulary
        only publishes the generic contract-family diagnostics plus operational
        execution diagnostics that are still emitted; the operational
        `schema_registration_failed` runtime diagnostic is now declared beside
        converter registration failures.
  - [x] Model converter cases in `schema-package.cem`: `implementation=cemt`
        and `implementation=rust` required attribute contracts plus CEMT
        native fallback `fallback-reason` now live in schema-owned
        `field-contract` declarations and emit
        `cem.schema_package.converter_check`; `from`/`to` exact-one endpoint
        occurrence is now schema-owned; enum fields now use schema-declared
        `@values` and boolean fields now use `schema:boolean` in the generic
        document model; `cost` now uses generic integer syntax and RELAX
        NG-style `minInclusive=1`; `implicit=true` with `explicit-only=true`
        is now a schema-owned value-specific forbidden field contract; CEMT
        template `template-content-type` and `template-schema` exact values now
        use schema-declared `@values`; package-specific enum, boolean, cost,
        fallback-reason, planner-state, endpoint cardinality, and CEMT template
        identity diagnostics have been retired in favor of generic
        schema-owned codes; `parity-fixture` `id`/`path` extraction now skips
        incomplete schema-invalid rows and materializes only complete runtime
        fixture descriptors; converter descriptor extraction now skips
        incomplete schema-invalid rows missing `id`, `implementation`, known
        implementation value, `from`/`to`, or endpoint `content-type`;
        formatter/color output profiles now require `output-syntax` and
        `encoding-category` through schema-owned `converter_check`
        field-dependency contracts; converter template source readability and
        template compile constraints are now pinned by schema-owned example
        fixtures and compiled constraint model assertions. The remaining
        endpoint `schema`/`content-type` compatibility check stays declared as
        the schema-owned `endpoint-content-type-schema`
        `schema:reference-resolution` constraint and executed by Rust until
        the generic cross-node/reference vocabulary exists.
  - [x] Finish artifact cases in `schema-package.cem`. Formatter, colorizer,
        formatter-helper, and colorizer-helper required field metadata now
        lives in schema-owned `field-contract` declarations; formatter/
        colorizer stage directory and `.cemt` source-path layout now live in
        schema-owned path-layout contracts; CEMT output function target
        identity/category/profile mismatches now emit the schema-declared
        `cem.schema_package.artifact_check` family with
        `artifact-output-stage-contract` details; CEMT source readability,
        parser validity, and function lookup now also report through
        schema-declared `artifact-output-stage-contract` details while keeping
        Rust only as the execution placement; runtime conversion artifact
        extraction no longer owns artifact `kind`/`path`,
        formatter/colorizer profile required-field checks, or the `generated`
        boolean datatype check; runtime conversion selection now routes
        formatter/colorizer stage-kind-to-function-kind mapping through the
        schema-pinned CEMT stage contract groups. Artifact source readability,
        CEMT parse validity, function lookup, and function metadata contract
        constraints are now pinned in the compiled schema model and CLI
        structured-detail examples. Deeper CEMT body/output assertions are
        deferred to the low-priority vocabulary item below because they need
        generic CEMT body/output assertion syntax rather than package-specific
        schema-package rules.
  - [x] Model example cases in `schema-package.cem`: required example
        metadata and failing-example `expected-diagnostics` now live in
        schema-owned `field-contract` declarations and emit
        `cem.schema_package.example_check`; content type/schema compatibility
        is declared as a schema-owned cross-reference rule and still executes
        in Rust, now emitting `example_check` with structured `checkKind`
        details; package example descriptor extraction no longer owns required
        metadata or `expected-result` value diagnostics, and the stale
        `example_content_type_mismatch` diagnostic has been retired. Example
        source readability, source validation result, and expected diagnostic
        matching constraints are schema-owned and now pinned in the compiled
        schema model and CLI structured-detail examples while Rust remains the
        execution placement for source I/O and validation.
  - [x] Continue replacing one-code-per-field diagnostics with contract-family
        diagnostics declared in schema source. Artifact missing-metadata checks
        now emit `cem.schema_package.artifact_check`; converter and example
        field diagnostics now emit schema-declared `converter_check` and
        `example_check` families where the distinction is field/cross-reference
        contract detail; package-root child presence now emits
        `cem.schema_package.package_check`; artifact CEMT output function
        metadata mismatches now also use `cem.schema_package.artifact_check`
        with contract details instead of a narrow mismatch code, and artifact
        CEMT source read, parse, and function lookup failures now use the same
        generic family with operational `checkKind` details; CEMT native fallback reason
        requirements now stay in schema-owned `converter_check` contracts
        instead of conversion descriptor extraction; Rust converter
        `rust-symbol` requirements now also stay in schema-owned
        `converter_check` contracts, with runtime execution retaining only the
        operational missing-symbol guard; CEMT converter `template`,
        `template-content-type`, and `template-schema` identity requirements
        now stay in schema-owned `converter-cemt-template-identity` and
        `@values` declarations, with runtime execution retaining only
        operational template source read and compile checks; converter endpoint
        schema/content-type compatibility and converter template source/compile
        failures now emit the generic `converter_check` family with structured
        `checkKind` details; converter scalar datatype/value checks for
        readiness, streamability, implicit/explicit flags, cost, output syntax,
        and parity now stay in generic schema-model validation instead of
        descriptor extraction; parity fixture `id`/`path` field checks now stay
        in generic schema-model validation instead of descriptor extraction; artifact
        `kind`/`path` field checks now stay in generic schema-model validation
        instead of descriptor extraction; converter `id`/`implementation`,
        implementation value, endpoint occurrence, and endpoint `content-type`
        field checks now stay in generic schema-model validation instead of
        descriptor extraction or schema-package validator branches; stale
        converter manifest field-specific `MissingAttribute`,
        `MissingEndpoint`, `UnknownImplementation`, and
        `converter_implementation_unknown` errors have been removed; schema
        descriptor top-level `MissingAttribute` extraction errors have been
        removed for package/schema metadata; stale converter template and
        endpoint-specific diagnostic declarations have been retired in favor of
        `cem.schema_package.converter_check`; stale schema-package diagnostic
        declarations for retired package, converter, artifact, and example
        field-specific codes are now test-pinned absent, and the production
        source anti-pattern audit explicitly blocks the retired helper/token
        names.
  - [x] Keep Rust validators only for operational execution that cannot be
        represented as field data: resource read failures, parser failures,
        CEMT compilation, CEMT function lookup, host-hook availability, and
        source-file I/O. Those checks must still be declared as schema-owned
        constraints/rules in `.cem`, with Rust only as the execution placement.
        Schema package source readability, source parse validity, schema URI,
        content-type, and namespace consistency are now split into explicit
        schema-owned `schema:resource-readable`, `schema:resource-parse`, and
        `schema:reference-resolution` constraints; converter, artifact, and
        example runtime checks already use schema-owned operational constraint
        declarations. Remaining fully declarative replacement of cross-node
        lookups and CEMT body/output assertions is deferred to the low-priority
        vocabulary items below.
  - [x] Refactor `SchemaPackageConverterContractRule` toward operational-only
        checks for template readability/compilation and endpoint
        schema/content-type compatibility. CEMT template identity, Rust symbol,
        CEMT fallback reason requirements, converter planner-state conflicts,
        and converter endpoint occurrence are now schema-owned field contracts;
        the legacy package-specific enum, boolean, positive-cost,
        planner-state, and endpoint cardinality branches are now covered by
        generic `@values`, `schema:boolean`, `minInclusive`/`maxInclusive`,
        value-specific forbidden field, and child occurrence checks; CEMT
        template content-type/schema exact values are now covered by generic
        `@values`; formatter/color output-profile dependencies are now
        schema-owned field-dependency contracts; converter template
        source-readability and compile constraints are pinned as schema-owned
        resource-readable/resource-parse constraints, and the remaining
        converter operational/cross-reference branches emit `converter_check`
        with `checkKind` details. Endpoint schema/content-type compatibility
        remains a schema-owned `schema:reference-resolution` constraint with
        Rust execution placement because fully declarative cross-node registry
        lookups need the deferred generic vocabulary below.
  - [x] Refactor `schema_descriptor_from_package_sources`,
        `collect_package_examples`, and `required_attr` in
        `packages/cem_ml/src/schema/registry.rs` so descriptor extraction runs
        after generic schema validation. Loader errors may remain typed, but
        missing/invalid manifest fields must be diagnosed by schema-owned
        contracts, not by descriptor parsing. `collect_package_examples` now
        treats invalid example field data as schema-owned and only materializes
        complete loadable example descriptors; content-type and namespace
        child extraction now skips incomplete schema-invalid rows instead of
        owning field diagnostics; top-level package/schema descriptor
        extraction now falls back to the embedded package id, schema root
        namespace/version, and embedded schema path instead of owning missing
        manifest field diagnostics, and `required_attr` has been removed.
  - [ ] Update runtime diagnostic declaration tests and CLI example coverage to
        assert generic contract-family codes plus structured details instead of
        schema-package-specific field diagnostics. A production-source audit
        now guards against reintroducing schema-package field-specific
        `*_missing`, `*_unknown`, `*_invalid`, `*_duplicate`, and `*_conflict`
        diagnostics except explicitly allowlisted operational execution
        failures, plus retired converter scalar/value/cardinality diagnostic
        names, hard-coded required-field vector helper names, and retired
        descriptor parsing helpers/errors; schema-declared enum value sets for
        conversion manifest materializers are now parity-tested against the
        Rust enum parsers, and formatter/colorizer artifact stage kind groups
        plus runtime output-function-kind mappings are parity-tested against
        schema-owned field-contract `@when-values`.
        Formatter/colorizer CEMT body metadata term checks are centralized on
        the operational stage contract type and test-pinned; moving those terms
        into schema needs a schema vocabulary for CEMT body/output assertions.
        Keep expanding that audit for narrow operational `matches!` lists as
        new declarative contracts move into schema.

- [ ] Complete the schema-package folder frame for
      `packages/cem_ml/schema-packages`: every `{package-id}/vN/` folder must be
      discoverable from `package.cem` with a `.cem` schema source, example
      references, CEMT formatter artifacts, and CEMT colorizer artifacts.
  - [ ] Extend the schema-package manifest and validators so package examples
        and formatter/colorizer artifacts are declared from `package.cem`.
        Examples must include source path, content type, schema URL, expected
        pass/fail result, and expected diagnostics. Artifacts must include
        profile, target content type/schema, target category, and CEMT function
        identity.
  - [x] Add package-folder validation that checks `package.cem`, `schema/`,
        `examples/`, `formatters/`, and `colorizers/` completeness for every
        built-in package before per-package implementation can be marked done.
        Build-time catalog checks now require every embedded built-in package
        to have matching `package.cem`, `.cem` schema source, and at least one
        example fixture; declared CEMT artifacts must exist on disk and be
        embedded in the artifact source catalog; any embedded package that
        declares `formatters/` or `colorizers/` must have those CEMT assets
        indexed by `package.cem`, and any declared output-stage profile set
        must include baseline formatter profiles `compact`, `pretty`,
        `tabular` or colorizer profiles `terminal`, `html`, and `md`.
  - [x] Require example loading to resolve the declared content type plus schema
        URL and validate the source bytes against that schema; filename
        extension inference is only a fallback hint. Declared schema-package
        examples are now read from their manifest-relative `@path`, parsed by
        declared content type/schema instead of extension, validated against
        the built-in document model when available, and checked against
        `expected-result` plus `expected-diagnostics`.
  - [x] Expand example coverage from representative constraint-kind coverage to
        finer diagnostic coverage, starting with schema-package source
        read/invalid cases and artifact source/parse/function-missing cases.
        Schema-package example fixtures now cover unreadable example sources,
        loaded example result mismatches, expected diagnostic mismatches,
        unreadable artifact CEMT sources, invalid artifact CEMT sources, and
        missing artifact CEMT function declarations through generic
        `cem.schema_package.example_check` and
        `cem.schema_package.artifact_check` diagnostics.
  - [x] Implement reusable baseline formatter profiles:
        `compact` as default, `pretty`, and `tabular`; each profile is a CEMT
        transform that preserves source-map ranges.
  - [x] Implement reusable baseline colorizer profiles: `terminal`, `html`,
        and `md`; each profile is a CEMT transform over the formatted CEM tree
        with source-map range preservation.
    - [x] Preserve literal CEM-tree colorizer profile selectors `terminal`,
          `html`, and `md` through transform binding, package-stage execution,
          generated color metadata, and writer artifact identity.
    - [x] Materialize `colorOutput`, terminal `colorCapability`, and
          profile-specific per-node style metadata from the CEMT colorizer
          helper while keeping HTML writer attributes restricted to `html`.
    - [x] Add terminal writer rendering for capability-aware ANSI/SGR output
          from colored CEM-tree ranges.
    - [x] Add Markdown-safe rendered color output forms for `md` without
          losing source-map ranges.
  - [ ] Roll the frame through the supported package scope below in order,
        keeping every content type covered before moving to lower-priority
        package families.

## Schema Package Frame Scope

Complete each supported package below only when the generic folder frame in
Immediate Tasks is satisfied for that package: `package.cem`, `.cem` schema,
explicit example content-type/schema references, `compact`/`pretty`/`tabular`
CEMT formatters, `terminal`/`html`/`md` CEMT colorizers, and package-folder
validation coverage. The order is dependency-first, then common authoring
formats, then XML/markup families, then projection/debug formats.

Bootstrap and self-hosting packages:

- [x] `cem-ml/v1` (`application/cem`; aliases: `text/cem-ml`, `text/cem`,
      `application/cem+xml`). Baseline formatter/colorizer selectors are
      declared and runtime-selectable over canonical CEMT assets; terminal and
      Markdown rendered color output preserve source-map ranges. Top-level
      examples are declared from `package.cem` with source path, content type,
      schema URL, expected result, and expected diagnostics, and catalog tests
      guard CEM-ML example manifest drift.
- [x] `schema/v1` (`application/vnd.cem.schema+cem`). Top-level schema
      examples are declared from `package.cem` with source path, content type,
      schema URL, expected result, and expected diagnostics, and catalog tests
      guard schema example manifest drift.
- [x] `schema-package/v1` (`application/vnd.cem.schema-package+cem`).
      Top-level schema-package examples are declared from `package.cem` with
      source path, content type, schema URL, expected result, and expected
      diagnostics, and catalog tests guard schema-package example manifest
      drift.
- [x] `cem-native-template/v1` (`application/vnd.cem.template+cem`; CEM
      source aliases). Baseline `compact`/`pretty`/`tabular` formatter and
      `terminal`/`html`/`md` colorizer selectors are declared over CEMT
      artifacts, top-level examples are declared from `package.cem`, and
      catalog tests guard native-template artifact/example manifest drift.
- [x] `cem-transform/v1` (`application/vnd.cem.transform+cem`, `.cemt`).
      Baseline `compact`/`pretty`/`tabular` formatter and
      `terminal`/`html`/`md` colorizer selectors are declared over CEMT
      artifacts, top-level examples are declared from `package.cem`, including
      the paired CEM fixture with its own CEM-ML identity, and catalog tests
      guard transform artifact/example manifest drift.
- [x] `cem-ql/v1` (`application/vnd.cem.query+cem-ql`, `text/cem-ql`, query
      artifact aliases). Baseline `compact`/`pretty`/`tabular` formatter and
      `terminal`/`html`/`md` colorizer selectors are declared over CEMT
      artifacts, top-level CEM-QL examples are declared from `package.cem`,
      and catalog tests guard CEM-QL artifact/example manifest drift.

Common structured and authoring formats:

- [x] `json/v1` (`application/json`, `text/json`). Baseline
      `compact`/`pretty`/`tabular` formatter and `terminal`/`html`/`md`
      colorizer selectors are declared over CEMT artifacts, top-level JSON
      examples are declared from `package.cem`, and catalog tests guard JSON
      artifact/example manifest drift.
- [x] `json-schema/v1` (`application/schema+json`). Baseline
      `compact`/`pretty`/`tabular` formatter and `terminal`/`html`/`md`
      colorizer selectors are declared over CEMT artifacts, top-level JSON
      Schema examples are declared from `package.cem`, and catalog tests guard
      JSON Schema artifact/example manifest drift.
- [x] `yaml/v1` (`application/yaml`, YAML aliases). Baseline
      `compact`/`pretty`/`tabular` formatter and `terminal`/`html`/`md`
      colorizer selectors are declared over CEMT artifacts, top-level YAML
      examples are declared from `package.cem`, including alias content types,
      and catalog tests guard YAML artifact/example manifest drift.
- [x] `csv/v1` (`text/csv`). Baseline `compact`/`pretty`/`tabular`
      formatter and `terminal`/`html`/`md` colorizer selectors are declared
      over CEMT artifacts, top-level CSV examples are declared from
      `package.cem`, including pass-with-warning diagnostics, and catalog tests
      guard CSV artifact/example manifest drift.
- [x] `markdown/v1` (`text/markdown`). Baseline
      `compact`/`pretty`/`tabular` formatter and `terminal`/`html`/`md`
      colorizer selectors are declared over CEMT artifacts, top-level Markdown
      examples are declared from `package.cem`, including parameterized
      content types and pass-with-warning diagnostics, and catalog tests guard
      Markdown artifact/example manifest drift.
- [x] `css/v1` (`text/css`). Baseline `compact`/`pretty`/`tabular`
      formatter and `terminal`/`html`/`md` colorizer selectors are declared
      over CEMT artifacts, top-level CSS examples are declared from
      `package.cem`, including parameterized charset and pass-with-warning
      diagnostics, and catalog tests guard CSS artifact/example manifest drift.

XML and markup family formats:

- [x] `xml/v1` (`application/xml`, XML aliases). Baseline
      `compact`/`pretty`/`tabular` formatter and `terminal`/`html`/`md`
      colorizer selectors are declared over CEMT artifacts, top-level XML
      examples are declared from `package.cem`, including text XML alias
      content type metadata, and catalog tests guard XML artifact/example
      manifest drift while preserving the Rust XML-to-DOM converter metadata.
- [x] `html/v1` (`text/html`). Baseline `compact`/`pretty`/`tabular`
      formatter and `terminal`/`html`/`md` colorizer selectors are declared
      over CEMT artifacts, top-level HTML examples are declared from
      `package.cem`, including parser-recovery pass cases, SVG/MathML islands,
      executable script/resource/custom-element failures, and charset conflict
      warning metadata, with catalog tests guarding HTML artifact/example
      manifest drift while preserving the Rust HTML-to-DOM converter metadata.
- [ ] `relax-ng/v1` (`application/relax-ng+xml`,
      `application/relax-ng-compact-syntax`).
- [ ] `xhtml/v1` (`application/xhtml+xml`).
- [ ] `svg/v1` (`image/svg+xml`).
- [ ] `mathml/v1` (`application/mathml+xml`, MathML aliases).
- [ ] `xslt/v1` (`application/xslt+xml`, XSLT aliases).

Projection and debug/interchange formats:

- [ ] `cem-dom-projection/v1` (`application/vnd.cem.dom+cem-bin`,
      `application/vnd.cem.dom+json`).
- [ ] `cem-ast-projection/v1` (`application/vnd.cem.ast+cem-bin`,
      `application/vnd.cem.ast+json`).
- [ ] `cem-events-projection/v1` (`application/vnd.cem.events+cem-bin`,
      `application/vnd.cem.events+json`).

## Low Priority Deferred Design

- [ ] Design richer declarative cross-node/reference vocabulary for
      schema-owned constraints currently declared in CEM-ML but executed by
      Rust. Immediate background: `schema-package.cem` declares converter
      endpoint compatibility as
      `{constraint @kind="endpoint-content-type-schema" @target="from to"
      @diagnostic="cem.schema_package.converter_check"
      @behavior="schema:reference-resolution" ...}`, and
      `SchemaPackageConverterContractRule` executes it by reading each
      endpoint's `@schema`, resolving that URI through `SchemaRegistry`, then
      checking whether `content_type_essence(@content-type)` is included in
      the referenced schema's registered content-type essences. Similar
      Rust-executed reference-resolution shapes exist for example
      content-type/schema compatibility and artifact CEMT function lookup/
      metadata matching. The future vocabulary should let a schema declare:
      candidate selection, reference attributes, registry/document lookup
      target, normalized comparison such as media-type essence or URI equality,
      expected/invalid value detail projection, source-range propagation, and
      whether execution is pure CEM-ML/CEM-QL or engine-assisted. Reuse
      `schema:reference-resolution`, CEM-QL candidate selection, and
      schema-declared behavior functions where possible; avoid introducing
      package-specific syntax for converter endpoints.
- [ ] Design declarative CEMT body/output assertion vocabulary for
      schema-owned artifact constraints currently centralized in Rust helper
      checks. Immediate background: schema-package artifact declarations now
      express source readability (`artifact-source-readable`), CEMT parse
      validity (`artifact-cemt-valid`), CEMT output function lookup
      (`artifact-function-declared`), and output-function metadata matching
      (`artifact-function-contract`) as schema-owned constraints using
      `schema:resource-readable`, `schema:resource-parse`, and
      `schema:reference-resolution`. Rust still executes the CEMT-specific
      body/output inspection by parsing the CEMT module, selecting the declared
      output function by `@function-name`, and comparing function kind,
      target content type/schema/category, and optional function profile
      against manifest attributes. Future syntax should let schemas declare
      CEMT body/output selectors, function metadata projections, profile
      expectations, normalized media-type/URI comparisons, expected/invalid
      detail projection, and source-range propagation without hard-coding
      schema-package artifact semantics.

# [] believes schema + registry
stop for sync up with author
## Current Verification Commands

- `yarn nx run @epa-wg/cem-theme:verify:phase13`
- `yarn nx run cem-elements:verify`
- `yarn nx run @epa-wg/cem-components:verify`
- `yarn nx run cem-elements:verify-edge-ssr`
- `yarn nx run @epa-wg/custom-element:verify`

## Externally Gated

These are intentionally not active in the current workspace because the required native toolchains are unavailable.
Keep the existing offline platform artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for `packages/cem-theme/dist/lib/token-platforms/android/`.
