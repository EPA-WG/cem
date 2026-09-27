use cem_ml::{
    css_emission::{derive_css_stylesheet_identity, emit_css_import_closure, CssManagedScope},
    css_imports::{CssImportClosure, CssImportLimits},
    import::import_data,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlMapping,
        CemModuleUrlResolutionCapability, CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
    scheduler::AbortSignal,
};
use std::sync::Arc;

fn context() -> CemModuleUrlContext {
    let mut frame = CemModuleUrlFrame::new("page", "https://example.test/page.html");
    frame
        .specifiers
        .resources
        .insert("icon".into(), CemModuleUrlMapping::target("./icon.svg"));
    CemModuleUrlContext {
        identity: "page-context".into(),
        resolver_identity: "resolver:v1".into(),
        resource_policy_stamp: "policy:v1".into(),
        frames: vec![frame],
    }
}
fn closure(source: &str, ctx: CemModuleUrlContext, handle: &str, base: &str) -> CssImportClosure {
    let handle = CemResolutionContextHandle::new(handle);
    let capability = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(handle.clone(), ctx)),
        handle,
    );
    CssImportClosure::new(
        import_data(source, "text/css", "cem", "urn:style:stable").unwrap(),
        base,
        capability,
        CssImportLimits::default(),
        AbortSignal::new(),
    )
    .unwrap()
}
fn scope() -> CssManagedScope {
    CssManagedScope::Private {
        tag: "cem-card".into(),
        context: None,
    }
}
fn identity(c: &CssImportClosure) -> cem_ml::css_emission::CssStylesheetIdentity {
    derive_css_stylesheet_identity(c, &scope(), "declaration:card", "style:0").unwrap()
}
const CSS: &str =
    ".card {background:url(icon);animation:pulse 1s} @keyframes pulse {to {opacity:1}}";
const BASE: &str = "https://example.test/styles/main.css";

#[test]
fn ownership_survives_reimport_and_handle_reallocation() {
    let a = closure(CSS, context(), "handle-1", BASE);
    let b = closure(CSS, context(), "handle-99", BASE);
    let first = identity(&a);
    let second = identity(&b);
    assert_eq!(first, second);
    let emitted = emit_css_import_closure(&a, &first.scope, &first.owner_key).unwrap();
    assert!(emitted
        .css()
        .contains(first.context_marker.as_deref().unwrap()));
    assert_eq!(
        emitted.css(),
        emit_css_import_closure(&b, &second.scope, &second.owner_key)
            .unwrap()
            .css()
    );
}

#[test]
fn map_policy_base_and_scope_changes_cannot_reuse_derived_styles() {
    let first = identity(&closure(CSS, context(), "handle", BASE));
    for ctx in [
        {
            let mut c = context();
            c.frames[0]
                .specifiers
                .resources
                .insert("icon".into(), CemModuleUrlMapping::target("./other.svg"));
            c
        },
        {
            let mut c = context();
            c.resource_policy_stamp = "policy:v2".into();
            c
        },
        {
            let mut c = context();
            c.frames[0].allowed_schemes = Some(["https".into()].into());
            c
        },
        {
            let mut c = context();
            c.frames[0].module_map_base_url = Some("https://example.test/maps/map.json".into());
            c
        },
        {
            let mut c = context();
            c.resolver_identity = "resolver:v2".into();
            c
        },
    ] {
        let changed = identity(&closure(CSS, ctx, "handle", BASE));
        assert_ne!(first.context_marker, changed.context_marker);
        assert_ne!(first.owner_key, changed.owner_key);
        assert_ne!(first.cache_key, changed.cache_key);
    }
    let changed = identity(&closure(
        CSS,
        context(),
        "handle",
        "https://example.test/other.css",
    ));
    assert_eq!(first.context_marker, changed.context_marker);
    assert_ne!(first.owner_key, changed.owner_key);
    let c = closure(CSS, context(), "handle", BASE);
    for (scope, declaration, occurrence) in [
        (scope(), "declaration:other", "style:0"),
        (scope(), "declaration:card", "style:1"),
        (
            CssManagedScope::Shared {
                name: "controls".into(),
                context: None,
            },
            "declaration:card",
            "style:0",
        ),
    ] {
        let changed = derive_css_stylesheet_identity(&c, &scope, declaration, occurrence).unwrap();
        assert_eq!(first.context_marker, changed.context_marker);
        assert_ne!(first.owner_key, changed.owner_key);
        assert_ne!(first.cache_key, changed.cache_key);
    }
}

#[test]
fn content_invalidates_cache_without_changing_owner_and_plain_styles_ignore_context() {
    let first = identity(&closure(CSS, context(), "one", BASE));
    let changed = identity(&closure(
        &CSS.replace("opacity:1", "opacity:.5"),
        context(),
        "two",
        BASE,
    ));
    assert_eq!(first.owner_key, changed.owner_key);
    assert_ne!(first.cache_key, changed.cache_key);
    let mut other = context();
    other.identity = "other-context".into();
    let a = identity(&closure(".card {color:red}", context(), "one", BASE));
    let b = identity(&closure(".card {color:red}", other, "two", BASE));
    assert_eq!(a, b);
    assert!(a.context_marker.is_none());
}

#[test]
fn imported_content_and_final_response_base_invalidate_the_artifact() {
    let load = |source, final_url| {
        let mut c = closure("@import 'child.css';", context(), "handle", BASE);
        assert!(derive_css_stylesheet_identity(&c, &scope(), "card", "0").is_err());
        let request = c.next_import().unwrap().unwrap();
        c.complete_import(
            request.id,
            import_data(source, "text/css", "cem", "urn:child").unwrap(),
            final_url,
        )
        .unwrap();
        identity(&c)
    };
    let a = load(".card {color:red}", "https://example.test/child.css");
    let b = load(".card {color:blue}", "https://example.test/child.css");
    let c = load(".card {color:red}", "https://example.test/cdn/child.css");
    assert_eq!(a.owner_key, b.owner_key);
    assert_eq!(a.owner_key, c.owner_key);
    assert_ne!(a.cache_key, b.cache_key);
    assert_ne!(a.cache_key, c.cache_key);
}

struct MutableResolver {
    delegate: CemScopedModuleUrlResolver,
    stamp: Arc<std::sync::atomic::AtomicUsize>,
    cacheable: bool,
}
impl cem_ml::module_resolution::CemModuleUrlResolver for MutableResolver {
    fn context_cache_identity(&self, _: &CemResolutionContextHandle) -> Option<String> {
        self.cacheable.then(|| {
            format!(
                "context-{}",
                self.stamp.load(std::sync::atomic::Ordering::SeqCst)
            )
        })
    }
    fn resolve_module_url(
        &self,
        request: &cem_ml::module_resolution::CemModuleUrlResolutionRequest,
    ) -> Result<
        cem_ml::module_resolution::CemModuleUrlResolution,
        cem_ml::module_resolution::CemModuleUrlResolutionError,
    > {
        self.delegate.resolve_module_url(request)
    }
}

#[test]
fn changed_unknown_and_cancelled_contexts_do_not_produce_cacheable_identity() {
    for cacheable in [true, false] {
        let handle = CemResolutionContextHandle::new("handle");
        let stamp = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let capability = CemModuleUrlResolutionCapability::new(
            Arc::new(MutableResolver {
                delegate: CemScopedModuleUrlResolver::new().with_context(handle.clone(), context()),
                stamp: stamp.clone(),
                cacheable,
            }),
            handle,
        );
        let abort = AbortSignal::new();
        let c = CssImportClosure::new(
            import_data(CSS, "text/css", "cem", "urn:style").unwrap(),
            BASE,
            capability,
            CssImportLimits::default(),
            abort.clone(),
        )
        .unwrap();
        let derive = || derive_css_stylesheet_identity(&c, &scope(), "card", "0");
        assert_eq!(derive().is_ok(), cacheable);
        stamp.store(1, std::sync::atomic::Ordering::SeqCst);
        assert!(derive().is_err());
        stamp.store(0, std::sync::atomic::Ordering::SeqCst);
        abort.abort();
        assert!(derive().is_err());
    }
}

#[test]
fn resolver_fingerprint_covers_mapping_metadata_order_and_scoped_maps() {
    use cem_ml::module_resolution::CemModuleUrlScopedMap;
    let first = context().cache_identity();
    for ctx in [
        {
            let mut c = context();
            c.frames[0]
                .specifiers
                .resources
                .get_mut("icon")
                .unwrap()
                .integrity = Some("sha256-changed".into());
            c
        },
        {
            let mut c = context();
            c.frames[0]
                .specifiers
                .resources
                .get_mut("icon")
                .unwrap()
                .content_type_hint = Some("image/svg+xml".into());
            c
        },
        {
            let mut c = context();
            c.frames[0]
                .specifiers
                .resources
                .insert("icon".into(), CemModuleUrlMapping::blocked());
            c
        },
        {
            let mut c = context();
            let specifiers = c.frames[0].specifiers.clone();
            c.frames[0].scopes.push(CemModuleUrlScopedMap {
                prefix: "https://example.test/styles/".into(),
                specifiers,
            });
            c
        },
    ] {
        assert_ne!(first, ctx.cache_identity());
    }
    let mut c = context();
    c.frames.push(CemModuleUrlFrame::new(
        "inner",
        "https://example.test/inner/",
    ));
    let ordered = c.cache_identity();
    c.frames.reverse();
    assert_ne!(ordered, c.cache_identity());
    let mut a = context();
    a.identity = "ab".into();
    a.resolver_identity = "c".into();
    let mut b = context();
    b.identity = "a".into();
    b.resolver_identity = "bc".into();
    assert_ne!(a.cache_identity(), b.cache_identity());
}

#[test]
fn identity_rejects_missing_owner_and_prequalified_scope() {
    let c = closure(CSS, context(), "handle", BASE);
    assert!(derive_css_stylesheet_identity(&c, &scope(), "", "0").is_err());
    assert!(derive_css_stylesheet_identity(&c, &scope(), "card", "").is_err());
    assert!(derive_css_stylesheet_identity(
        &c,
        &CssManagedScope::Private {
            tag: "cem-card".into(),
            context: Some("guessed".into()),
        },
        "card",
        "0"
    )
    .is_err());
}

#[test]
fn browser_fixture_emits_ownership_context_variants() {
    for name in ["first", "second"] {
        let mut ctx = context();
        ctx.frames[0].specifiers.resources.insert(
            "icon".into(),
            CemModuleUrlMapping::target(format!("./{name}.svg")),
        );
        let c = closure(".card {background-image:url(icon)}", ctx, "handle", BASE);
        let id = identity(&c);
        let emitted = emit_css_import_closure(&c, &id.scope, &id.owner_key).unwrap();
        assert!(emitted.diagnostics.is_empty());
        assert!(emitted
            .css()
            .contains(&format!("https://example.test/{name}.svg")));
        if let Some(dir) = std::env::var_os("CEM_CSS_SUBTREE_FIXTURE_DIR") {
            let dir = std::path::Path::new(&dir);
            std::fs::write(dir.join(format!("context-{name}.css")), emitted.css()).unwrap();
            std::fs::write(
                dir.join(format!("context-{name}.txt")),
                id.context_marker.unwrap(),
            )
            .unwrap();
        }
    }
}
