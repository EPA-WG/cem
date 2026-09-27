use cem_ml::{
    css_emission::CssManagedScope,
    css_imports::CssImportClosure,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlMapping,
        CemModuleUrlResolutionCapability, CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
    scheduler::AbortSignal,
};
use cem_ql::{
    render::{adopt_dom_stylesheets, DomStylesheetSource},
    retained_template::RetainedTemplate,
};
use std::sync::Arc;

fn template(css: &str) -> RetainedTemplate {
    RetainedTemplate::new(adopt_dom_stylesheets(&[DomStylesheetSource {
        css: css.into(),
        scope: None,
        content_type: None,
    }]))
}
fn closure(owner: &RetainedTemplate, target: &str, abort: AbortSignal) -> Arc<CssImportClosure> {
    closure_at(owner, 0, target, abort)
}
fn closure_at(
    owner: &RetainedTemplate,
    index: usize,
    target: &str,
    abort: AbortSignal,
) -> Arc<CssImportClosure> {
    let handle = CemResolutionContextHandle::new("handle");
    let mut frame = CemModuleUrlFrame::new("page", "https://example.test/page.html");
    frame
        .specifiers
        .resources
        .insert("icon".into(), CemModuleUrlMapping::target(target));
    let resolver = CemScopedModuleUrlResolver::new().with_context(
        handle.clone(),
        CemModuleUrlContext {
            identity: "page".into(),
            resolver_identity: "resolver".into(),
            resource_policy_stamp: "policy".into(),
            frames: vec![frame],
        },
    );
    Arc::new(
        CssImportClosure::new(
            owner.artifact().stylesheets[index].css_tree().clone(),
            "https://example.test/styles/main.css",
            CemModuleUrlResolutionCapability::new(Arc::new(resolver), handle),
            Default::default(),
            abort,
        )
        .unwrap(),
    )
}
fn scope() -> CssManagedScope {
    CssManagedScope::Private {
        tag: "cem-card".into(),
        context: None,
    }
}

#[test]
fn matching_consumers_share_emission_and_release_the_last_owner() {
    let mut owner = template(".card {background:url(icon)}");
    let a = closure(&owner, "./icon.svg", AbortSignal::new());
    let b = closure(&owner, "./icon.svg", AbortSignal::new());
    let first = owner
        .retain_stylesheet("first", 0, a, &scope(), "card")
        .unwrap();
    let second = owner
        .retain_stylesheet("second", 0, b, &scope(), "card")
        .unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert!(first
        .emission()
        .unwrap()
        .css()
        .contains("https://example.test/icon.svg"));
    let weak = Arc::downgrade(&first);
    drop(first);
    drop(second);
    assert_eq!(owner.release_stylesheet_consumer("first"), 1);
    assert!(weak.upgrade().is_some());
    assert_eq!(owner.release_stylesheet_consumer("second"), 1);
    assert!(weak.upgrade().is_none());
    assert_eq!(owner.release_stylesheet_consumer("second"), 0);
}

#[test]
fn different_contexts_are_isolated_and_replacement_releases_old_set() {
    let mut owner = template(".card {background:url(icon)}");
    let a = closure(&owner, "./a.svg", AbortSignal::new());
    let b = closure(&owner, "./b.svg", AbortSignal::new());
    let first = owner
        .retain_stylesheet("first", 0, a, &scope(), "card")
        .unwrap();
    let second = owner
        .retain_stylesheet("second", 0, b.clone(), &scope(), "card")
        .unwrap();
    assert!(!Arc::ptr_eq(&first, &second));
    assert_ne!(
        first.identity().context_marker,
        second.identity().context_marker
    );
    assert!(first.emission().unwrap().css().contains("/a.svg"));
    assert!(second.emission().unwrap().css().contains("/b.svg"));
    let weak = Arc::downgrade(&first);
    drop(first);
    let replacement = owner
        .retain_stylesheet("first", 0, b, &scope(), "card")
        .unwrap();
    assert!(Arc::ptr_eq(&replacement, &second));
    assert!(weak.upgrade().is_none());
    let weak = Arc::downgrade(&second);
    drop(second);
    drop(replacement);
    drop(owner);
    assert!(weak.upgrade().is_none());
}

#[test]
fn wrong_root_pending_and_cancelled_closures_cannot_enter_the_store() {
    let mut owner = template(".card {background:url(icon)}");
    let other = template(".card {background:url(icon)}");
    let wrong = closure(&other, "./icon.svg", AbortSignal::new());
    assert!(owner
        .retain_stylesheet("first", 0, wrong, &scope(), "card")
        .is_err());
    let abort = AbortSignal::new();
    let c = closure(&owner, "./icon.svg", abort.clone());
    let output = owner
        .retain_stylesheet("first", 0, c.clone(), &scope(), "card")
        .unwrap();
    abort.abort();
    assert!(output.emission().is_err());
    assert!(owner.stylesheet("first", 0).is_none());
    assert!(owner
        .retain_stylesheet("second", 0, c, &scope(), "card")
        .is_err());
    let mut pending = template("@import 'child.css';");
    let c = closure(&pending, "./icon.svg", AbortSignal::new());
    assert!(pending
        .retain_stylesheet("first", 0, c, &scope(), "card")
        .is_err());
}

#[test]
fn independent_ready_closure_can_replace_cancelled_cached_output() {
    let mut owner = template(".card {background:url(icon)}");
    let abort = AbortSignal::new();
    let c = closure(&owner, "./icon.svg", abort.clone());
    let first = owner
        .retain_stylesheet("first", 0, c, &scope(), "card")
        .unwrap();
    abort.abort();
    let c = closure(&owner, "./icon.svg", AbortSignal::new());
    let second = owner
        .retain_stylesheet("first", 0, c, &scope(), "card")
        .unwrap();
    assert_eq!(first.identity(), second.identity());
    assert!(!Arc::ptr_eq(&first, &second));
    assert!(second.emission().is_ok());
}

#[test]
fn disposal_releases_imported_native_owners_and_preserves_diagnostics_until_then() {
    let mut owner = template("@import 'child.css'; .card {color:green}");
    let mut c = closure(&owner, "./icon.svg", AbortSignal::new());
    let native = Arc::get_mut(&mut c).unwrap();
    let request = native.next_import().unwrap().unwrap();
    let child = cem_ml::import::import_data(
        "#blocked {color:red} .card {background:url(icon)}",
        "text/css",
        "cem",
        "urn:child",
    )
    .unwrap();
    let weak = Arc::downgrade(&child);
    native
        .complete_import(request.id, child, &request.resolution.resolved_url)
        .unwrap();
    let retained = owner
        .retain_stylesheet("first", 0, c, &scope(), "card")
        .unwrap();
    let emission = retained.emission().unwrap();
    assert_eq!(emission.diagnostics.len(), 1);
    assert_eq!(emission.diagnostics[0].sheet, 1);
    assert_eq!(
        emission.diagnostics[0].diagnostic.code,
        "cem.scoped_css.id_selector_unsupported"
    );
    assert!(emission.diagnostics[0].diagnostic.source.origin().is_some());
    let source = retained.source(emission.diagnostics[0].sheet).unwrap();
    assert_eq!(source.source_uri, "urn:child");
    assert_eq!(
        source.stylesheet_url,
        "https://example.test/styles/child.css"
    );
    assert!(retained.source(2).is_err());
    assert!(weak.upgrade().is_some());
    drop(owner);
    assert!(retained.source(1).is_err());
    drop(retained);
    assert!(weak.upgrade().is_none());
}

#[test]
fn invalid_scope_or_occurrence_does_not_replace_a_consumers_valid_set() {
    let mut owner = template(".card {color:green}");
    let c = closure(&owner, "./icon.svg", AbortSignal::new());
    let first = owner
        .retain_stylesheet("first", 0, c.clone(), &scope(), "card")
        .unwrap();
    assert!(owner
        .retain_stylesheet("", 0, c.clone(), &scope(), "card")
        .is_err());
    assert!(owner
        .retain_stylesheet("first", 1, c.clone(), &scope(), "card")
        .is_err());
    assert!(owner
        .retain_stylesheet("first", 0, c, &CssManagedScope::Instance, "card")
        .is_err());
    assert!(Arc::ptr_eq(&first, &owner.stylesheet("first", 0).unwrap()));
    let mut shared = RetainedTemplate::new(adopt_dom_stylesheets(&[DomStylesheetSource {
        css: ".card {color:green}".into(),
        scope: Some("controls".into()),
        content_type: None,
    }]));
    let c = closure(&shared, "./icon.svg", AbortSignal::new());
    assert!(shared
        .retain_stylesheet("first", 0, c.clone(), &scope(), "card")
        .is_err());
    let right = CssManagedScope::Shared {
        name: "controls".into(),
        context: None,
    };
    assert!(shared
        .retain_stylesheet("first", 0, c, &right, "card")
        .is_ok());
}

#[test]
fn released_output_handles_cannot_outlive_consumers_or_template_disposal() {
    let mut owner = template(".card {color:green}");
    let c = closure(&owner, "./icon.svg", AbortSignal::new());
    let first = owner
        .retain_stylesheet("first", 0, c.clone(), &scope(), "card")
        .unwrap();
    owner
        .retain_stylesheet("second", 0, c.clone(), &scope(), "card")
        .unwrap();
    owner.release_stylesheet_consumer("first");
    assert!(first.emission().is_ok());
    owner.release_stylesheet_consumer("second");
    assert!(first.emission().is_err());
    assert!(first.source(0).is_err());
    // Reconnect can reuse the still-valid source closure, but gets a fresh lease.
    let reconnected = owner
        .retain_stylesheet("first", 0, c, &scope(), "card")
        .unwrap();
    assert!(!Arc::ptr_eq(&first, &reconnected));
    assert!(reconnected.emission().is_ok());
    drop(owner);
    assert!(reconnected.emission().is_err());
}

#[test]
fn one_consumer_retains_multiple_style_occurrences_without_collisions() {
    let mut owner = RetainedTemplate::new(adopt_dom_stylesheets(&[
        DomStylesheetSource {
            css: ".card {color:green}".into(),
            scope: None,
            content_type: None,
        },
        DomStylesheetSource {
            css: ".card {color:blue}".into(),
            scope: None,
            content_type: None,
        },
    ]));
    let mut records = Vec::new();
    for index in 0..2 {
        let c = closure_at(&owner, index, "./icon.svg", AbortSignal::new());
        records.push(
            owner
                .retain_stylesheet("first", index, c, &scope(), "card")
                .unwrap(),
        );
    }
    assert_ne!(
        records[0].identity().owner_key,
        records[1].identity().owner_key
    );
    assert!(owner
        .stylesheet("first", 0)
        .unwrap()
        .emission()
        .unwrap()
        .css()
        .contains("color:green"));
    assert!(owner
        .stylesheet("first", 1)
        .unwrap()
        .emission()
        .unwrap()
        .css()
        .contains("color:blue"));
    assert_eq!(owner.release_stylesheet_consumer("first"), 2);
    assert!(records.iter().all(|record| record.emission().is_err()));
}

#[test]
fn load_protocol_validates_delivery_supersession_and_consumer_release() {
    use cem_ql::retained_template::{StylesheetLoadOptions, StylesheetLoadProgress};
    let mut owner = template("@import 'child.css'; .card {color:green}");
    let options = || {
        let handle = CemResolutionContextHandle::new("test");
        let resolver = CemScopedModuleUrlResolver::new().with_context(
            handle.clone(),
            CemModuleUrlContext {
                identity: "test".into(),
                resolver_identity: "test".into(),
                resource_policy_stamp: "test".into(),
                frames: vec![CemModuleUrlFrame::new("page", "https://example.test/page")],
            },
        );
        StylesheetLoadOptions {
            consumer: "one".into(),
            index: 0,
            scope: scope(),
            declaration_identity: "card".into(),
            base_url: "https://example.test/main.css".into(),
            capability: CemModuleUrlResolutionCapability::new(Arc::new(resolver), handle),
            response_policy: Default::default(),
        }
    };
    let mut invalid = options();
    invalid.scope = CssManagedScope::Instance;
    assert!(owner.begin_stylesheet_load(invalid).is_err());
    let first = owner.begin_stylesheet_load(options()).unwrap();
    let second = owner.begin_stylesheet_load(options()).unwrap();
    assert_eq!(owner.release_stylesheet_generation("one", first), 0);
    assert!(owner.advance_stylesheet_load(first, "one").is_err());
    assert!(owner.advance_stylesheet_load(second, "other").is_err());
    let StylesheetLoadProgress::Pending(request) =
        owner.advance_stylesheet_load(second, "one").unwrap()
    else {
        panic!("expected import")
    };
    let output = owner
        .deliver_stylesheet_response(
            second,
            "one",
            request.id,
            cem_ml::resolver::ResolvedRead {
                uri: "https://example.test/cdn/child.css".into(),
                bytes: b".card {background:url(icon.svg)}".to_vec(),
                content_type: Some("text/css".into()),
            },
        )
        .unwrap();
    let StylesheetLoadProgress::Ready(output) = output else {
        panic!("expected ready CSS")
    };
    assert!(output
        .emission()
        .unwrap()
        .css()
        .contains("https://example.test/cdn/icon.svg"));
    assert!(owner.advance_stylesheet_load(second, "one").is_err());
    let third = owner.begin_stylesheet_load(options()).unwrap();
    assert_eq!(owner.release_stylesheet_consumer("one"), 2);
    assert!(owner.advance_stylesheet_load(third, "one").is_err());
    assert!(output.emission().is_err());
    let fourth = owner.begin_stylesheet_load(options()).unwrap();
    let StylesheetLoadProgress::Pending(request) =
        owner.advance_stylesheet_load(fourth, "one").unwrap()
    else {
        panic!()
    };
    assert!(owner
        .deliver_stylesheet_response(
            fourth,
            "one",
            request.id,
            cem_ml::resolver::ResolvedRead {
                uri: request.resolution.resolved_url,
                bytes: b"not CSS".to_vec(),
                content_type: Some("text/html".into()),
            }
        )
        .is_err());
    assert!(owner.advance_stylesheet_load(fourth, "one").is_err());
}

#[test]
fn load_protocol_checks_byte_limits_before_import_and_cancels_exact_generations() {
    use cem_ql::retained_template::{StylesheetLoadOptions, StylesheetLoadProgress};
    let mut owner = template("@import 'child.css'; .card {color:green}");
    let handle = CemResolutionContextHandle::new("limits");
    let capability = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(
            handle.clone(),
            CemModuleUrlContext {
                identity: "limits".into(),
                resolver_identity: "limits".into(),
                resource_policy_stamp: "limits".into(),
                frames: vec![CemModuleUrlFrame::new("page", "https://example.test/page")],
            },
        )),
        handle,
    );
    let options = |limit| StylesheetLoadOptions {
        consumer: "one".into(),
        index: 0,
        scope: scope(),
        declaration_identity: "card".into(),
        base_url: "https://example.test/main.css".into(),
        capability: capability.clone(),
        response_policy: cem_ml::css_imports::CssImportResponsePolicy {
            max_response_bytes: limit,
            max_total_bytes: limit,
        },
    };
    let first = owner.begin_stylesheet_load(options(3)).unwrap();
    let StylesheetLoadProgress::Pending(request) =
        owner.advance_stylesheet_load(first, "one").unwrap()
    else {
        panic!()
    };
    let error = owner
        .deliver_stylesheet_response(
            first,
            "one",
            request.id,
            cem_ml::resolver::ResolvedRead {
                uri: request.resolution.resolved_url,
                bytes: b".card{}".to_vec(),
                content_type: Some("text/css".into()),
            },
        )
        .err()
        .unwrap();
    assert!(error.to_string().contains("byte limits"), "{error}");
    assert!(owner.advance_stylesheet_load(first, "one").is_err());
    let second = owner.begin_stylesheet_load(options(100)).unwrap();
    assert_eq!(owner.release_stylesheet_generation("one", first), 0);
    let StylesheetLoadProgress::Pending(request) =
        owner.advance_stylesheet_load(second, "one").unwrap()
    else {
        panic!()
    };
    let StylesheetLoadProgress::Ready(output) = owner
        .deliver_stylesheet_response(
            second,
            "one",
            request.id,
            cem_ml::resolver::ResolvedRead {
                uri: request.resolution.resolved_url,
                bytes: b".card{color:blue}".to_vec(),
                content_type: Some("text/css".into()),
            },
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(owner.release_stylesheet_generation("other", second), 0);
    assert!(output.emission().is_ok());
    assert_eq!(owner.release_stylesheet_generation("one", second), 1);
    assert!(output.emission().is_err());
    assert_eq!(owner.release_stylesheet_generation("one", second), 0);
}
