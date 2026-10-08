use cem_ml::{ast::reload::ReloadLimits, schema::reference_policy::ReferenceScopePolicy};
use cem_ql::{
    api::{
        element_references::ElementReferenceSource,
        native_capability_session::NativeCapabilitySession,
        reference_transport::RetainedReferenceSource,
    },
    eval::{retained_cem_node, AtomValue, Item, ItemStream},
    render::{compile_template, render_plan_to_html, CompileTemplateOptions, TemplateData},
    suggestions::{SuggestionsConfig, SuggestionsPlan, UNICODE_VERSION},
};
use std::sync::Arc;

fn session(text: &str) -> NativeCapabilitySession {
    session_with_limits(text, Default::default())
}
fn session_with_limits(
    text: &str,
    limits: cem_ml::value::artifact::CemValueArtifactLimits,
) -> NativeCapabilitySession {
    let xml = text.starts_with('<');
    let source = RetainedReferenceSource::parse(
        text.as_bytes(),
        if xml {
            "application/xml"
        } else {
            "text/cem-ml"
        },
        "memory:suggestions.cem",
        ReloadLimits::default(),
    )
    .unwrap();
    NativeCapabilitySession::prepare(
        vec![ElementReferenceSource {
            source,
            context: true,
            policy: ReferenceScopePolicy::schema_defaults().unwrap(),
        }],
        0,
        &[],
        &[],
        TemplateData::default(),
        if xml {
            "input.children.children"
        } else {
            "seq:where(input.children, fn(n) => n.kind == \"element\" && n.name != \"@ns\")"
        },
        true,
        limits,
    )
    .unwrap()
}
fn strings(values: ItemStream) -> Vec<String> {
    values
        .items
        .iter()
        .map(|n| match n.atom().unwrap() {
            AtomValue::String(s) => s,
            atom => format!("{atom:?}"),
        })
        .collect()
}
fn config(query: &str) -> SuggestionsConfig {
    SuggestionsConfig {
        query: query.into(),
        ..Default::default()
    }
}

#[test]
fn published_suggestions_frame_retains_source_edges_and_reserves_slice_names() {
    let s = session(
        "{cem-option @value=same @label=First | {#later}}{cem-option @value=same | Second}",
    );
    let mut data = TemplateData::default();
    s.bind_suggestions_frame(&mut data, &config("First"))
        .unwrap();
    let template = compile_template(
        "{output | {$datadom.slices.suggestions.children.source.children.expression}}",
        &CompileTemplateOptions {
            host_bindings: vec!["datadom".into()],
            ..Default::default()
        },
    );
    let html = render_plan_to_html(&cem_ql::render::render_compiled_template(&template, &data));
    assert!(html.contains("#later"), "{html}");
    let original = s.evaluate("input", None).unwrap();
    let root = data.bindings.get("suggestions").unwrap().items[0]
        .view()
        .unwrap();
    let source = root.field("children").unwrap()[0]
        .view()
        .unwrap()
        .field("source")
        .unwrap();
    assert_eq!(
        source[0].view().unwrap().identity(),
        original.items[0].view().unwrap().identity()
    );
    let query_template = compile_template(
        "{output | {$datadom.slices.suggestions.dom:attribute(\"query\").value}}",
        &CompileTemplateOptions {
            host_bindings: vec!["datadom".into()],
            ..Default::default()
        },
    );
    let mut independent = TemplateData::default();
    s.bind_suggestions_frame(&mut independent, &config("Second"))
        .unwrap();
    assert!(
        render_plan_to_html(&cem_ql::render::render_compiled_template(
            &query_template,
            &independent
        ))
        .contains("Second")
    );
    assert!(
        render_plan_to_html(&cem_ql::render::render_compiled_template(
            &query_template,
            &data
        ))
        .contains("First")
    );
    assert!(s.bind_suggestions_frame(&mut data, &config("")).is_err());
    let mut forged = TemplateData::default().with_binding(
        "suggestions",
        ItemStream::once(Item::Atomic(AtomValue::String("authored".into()))),
    );
    assert!(s.bind_suggestions_frame(&mut forged, &config("")).is_err());
    for name in [
        "datadom.slices.suggestions",
        "datadom.slices.suggestions.query",
    ] {
        let mut forged = TemplateData::default().with_binding(
            name,
            ItemStream::once(Item::Atomic(AtomValue::String("authored".into()))),
        );
        assert!(s.bind_suggestions_frame(&mut forged, &config("")).is_err());
    }
    let mut forged = TemplateData::default();
    forged
        .bind_native_slice(
            "suggestions",
            ItemStream::once(Item::Atomic(AtomValue::String("authored".into()))),
        )
        .unwrap();
    forged.bindings.remove("suggestions");
    assert!(s.bind_suggestions_frame(&mut forged, &config("")).is_err());
}

#[test]
fn failed_suggestions_publication_keeps_the_original_control_envelope() {
    let s = session("{cem-option @value=x | Choice}");
    let scalar = Item::Atomic(AtomValue::String("authored".into()));
    for envelope in [
        scalar.clone(),
        Item::Record([("slices".into(), vec![scalar])].into()),
    ] {
        let mut data = TemplateData::default().with_binding("datadom", ItemStream::once(envelope));
        let before = format!("{:?}", data.bindings);
        assert!(s.bind_suggestions_frame(&mut data, &config("")).is_err());
        assert_eq!(format!("{:?}", data.bindings), before);
    }
}

#[test]
fn retained_publication_routes_independent_frames_without_rebuilding_its_view() {
    let s = session("{cem-option @value=x | Original {#later}}");
    let published = s
        .publish_suggestions("publication", &config("Original"))
        .unwrap();
    let template = compile_template("{output | {$consumer}|{$datadom.slices.suggestions.dom:attribute(\"query\").value}|{$datadom.slices.suggestions.children.source.children.expression}}",
        &CompileTemplateOptions { host_bindings: vec!["consumer".into(), "datadom".into()], ..Default::default() });
    for consumer in ["first", "second"] {
        let data = TemplateData::default().with_binding(
            "consumer",
            ItemStream::once(Item::Atomic(AtomValue::String(consumer.into()))),
        );
        let plan = s
            .render_suggestions_frame("publication", &template, data)
            .unwrap();
        let html = render_plan_to_html(&plan);
        assert!(
            html.contains(&format!("{consumer}|Original|#later")),
            "{html}"
        );
        let same = s.suggestions_publication("publication").unwrap();
        assert_eq!(
            published.root().view().unwrap().identity(),
            same.root().view().unwrap().identity()
        );
    }
    assert!(s
        .publish_suggestions("publication", &config("Other"))
        .is_err());
    let forged = TemplateData::default().with_binding("suggestions", ItemStream::empty());
    assert!(s
        .render_suggestions_frame("publication", &template, forged)
        .is_err());
    assert!(s.release_suggestions("publication"));
    assert!(!s.release_suggestions("publication"));
    assert!(s
        .render_suggestions_frame("publication", &template, TemplateData::default())
        .is_err());
    assert!(s.publish_suggestions("publication", &config("")).is_err());
    assert!(s.publish_suggestions("fresh", &config("")).is_ok());
    assert!(s.publish_suggestions("", &config("")).is_err());
}

#[test]
fn publication_bounds_include_retired_keys_and_release_active_control_bytes() {
    use cem_ml::value::artifact::CemValueArtifactLimits;
    let s = session_with_limits(
        "",
        CemValueArtifactLimits {
            max_values: 1,
            ..Default::default()
        },
    );
    s.publish_suggestions("one", &config("")).unwrap();
    assert!(s.publish_suggestions("two", &config("")).is_err());
    assert!(s.release_suggestions("one"));
    assert!(s.publish_suggestions("two", &config("")).is_err()); // a fresh session is required once identities are exhausted
    let s = session_with_limits(
        "",
        CemValueArtifactLimits {
            max_bytes: 1024,
            ..Default::default()
        },
    );
    let query = "x".repeat(450);
    s.publish_suggestions("one", &config(&query)).unwrap();
    assert!(s.publish_suggestions("two", &config(&query)).is_err());
    assert!(s.release_suggestions("one"));
    assert!(s.publish_suggestions("two", &config(&query)).is_ok());
}

#[test]
fn option_text_and_empty_values_do_not_borrow_labels_or_insert_spaces() {
    let s = session("<options xmlns=\"http://www.w3.org/1999/xhtml\"><option label=\"Display\">A<b>B</b> <!--gap--> <img alt=\"Image\"/> <script>ignored</script> C</option><option value=\"\">Empty</option><option value=\"x\" label=\"\">Fallback</option></options>");
    let plan = s.suggestions().unwrap();
    assert_eq!(plan.len(), 3);
    assert_eq!(
        strings(
            s.evaluate_suggestions(
                &config(""),
                "input.children.dom:attribute(\"value\").value",
                None
            )
            .unwrap()
        ),
        ["AB C", "", "x"]
    );
    assert_eq!(
        strings(
            s.evaluate_suggestions(
                &config(""),
                "input.children.dom:attribute(\"label\").value",
                None
            )
            .unwrap()
        ),
        ["Display", "Empty", "Fallback"]
    );
    let image = session("{option | A{img @alt=Image}B}");
    assert_eq!(
        strings(
            image
                .evaluate_suggestions(
                    &config(""),
                    "input.children.dom:attribute(\"label\").value",
                    None
                )
                .unwrap()
        ),
        ["AImageB"]
    );
    assert_eq!(
        strings(
            image
                .evaluate_suggestions(
                    &config(""),
                    "input.children.dom:attribute(\"value\").value",
                    None
                )
                .unwrap()
        ),
        ["AB"]
    );
}

#[test]
fn native_view_keeps_original_owners_content_and_duplicate_value_identity() {
    let s = session("{cem-option @value=same @label=One | {#datadom.slices.later}}{cem-option @value=same | Two}");
    let source = s.evaluate("input", None).unwrap();
    let targets = s
        .evaluate_suggestions(&config(""), "input.children.source", None)
        .unwrap();
    for (source, target) in source.items.iter().zip(&targets.items) {
        let (a, b) = (
            retained_cem_node(source).unwrap(),
            retained_cem_node(target).unwrap(),
        );
        assert!(Arc::ptr_eq(a.owner().ast_owner(), b.owner().ast_owner()));
        assert_eq!(a.node_id(), b.node_id());
    }
    assert_ne!(
        targets.items[0].view().unwrap().identity(),
        targets.items[1].view().unwrap().identity()
    );
    assert!(strings(
        s.evaluate_suggestions(&config(""), "input.content.kind", Some(0))
            .unwrap()
    )
    .contains(&"reference".into()));
    assert_eq!(
        strings(
            s.evaluate_suggestions(
                &config(""),
                "datadom.slices.suggestions.children.source.children.expression",
                None
            )
            .unwrap()
        ),
        ["#datadom.slices.later"]
    );
    let template = compile_template(
        "{span | {$dom:attribute(suggestion, \"label\").value}|{$suggestion.content.expression}}",
        &CompileTemplateOptions {
            host_bindings: vec!["suggestion".into()],
            ..Default::default()
        },
    );
    let html = render_plan_to_html(
        &s.render_suggestion(&config(""), &template, Some(0))
            .unwrap(),
    );
    assert!(
        html.contains("One") && html.contains("#datadom.slices.later"),
        "{html}"
    );
    let view = s.evaluate_suggestions(&config(""), "input", None).unwrap();
    assert!(cem_ql::eval::portable::export_values(&view, &Default::default()).is_err());
    let labels = s
        .evaluate_suggestions(
            &config(""),
            "input.children.dom:attribute(\"label\").value",
            None,
        )
        .unwrap();
    assert!(cem_ql::eval::portable::export_values(&labels, &Default::default()).is_ok());
}

#[test]
fn adapter_bounds_cover_text_work_depth_and_fold_expansion() {
    let s = session("{option @selected=false | A{b | B{em | C}}}");
    assert_eq!(s.suggestions().unwrap().warnings().len(), 1);
    let values = s.evaluate("input", None).unwrap();
    for limits in [
        cem_ml::value::artifact::CemValueArtifactLimits {
            max_values: 2,
            ..Default::default()
        },
        cem_ml::value::artifact::CemValueArtifactLimits {
            max_depth: 1,
            ..Default::default()
        },
        cem_ml::value::artifact::CemValueArtifactLimits {
            max_bytes: 3,
            ..Default::default()
        },
    ] {
        assert_eq!(
            SuggestionsPlan::prepare(&values, limits).unwrap_err().code,
            "cem.suggestions.limit"
        );
    }
    let s = session("{option | A\u{a0}B}");
    assert_eq!(
        strings(
            s.evaluate_suggestions(
                &config(""),
                "input.children.dom:attribute(\"value\").value",
                None
            )
            .unwrap()
        ),
        ["A\u{a0}B"]
    );
    assert_eq!(
        s.evaluate_suggestions(
            &config("A B"),
            "input.children.dom:attribute(\"matched\").value",
            None
        )
        .unwrap()
        .items[0]
            .atom(),
        Some(AtomValue::Boolean(false))
    );
}

#[test]
fn group_label_frame_and_group_depth_keep_native_owning_structure() {
    let s = session("{cem-option-group @label=Group | {cem-option @value=x | Option}}");
    let a = s.suggestions().unwrap();
    let b = s.suggestions().unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    let template = compile_template(
        "{span | {$dom:attribute(group, \"label\").value}|{$group.source.name}}",
        &CompileTemplateOptions {
            host_bindings: vec!["group".into()],
            ..Default::default()
        },
    );
    let html = render_plan_to_html(
        &s.render_suggestion_group(&config(""), &template, 0)
            .unwrap(),
    );
    assert!(
        html.contains("Group") && html.contains("cem-option-group"),
        "{html}"
    );
    assert!(s
        .render_suggestion_group(&config(""), &template, 1)
        .is_err());
    let values = s.evaluate("input", None).unwrap();
    assert_eq!(
        SuggestionsPlan::prepare(
            &values,
            cem_ml::value::artifact::CemValueArtifactLimits {
                max_depth: 1,
                ..Default::default()
            }
        )
        .unwrap_err()
        .code,
        "cem.suggestions.limit"
    );
}

#[test]
fn grouped_filtering_retains_rows_and_combines_availability_by_presence() {
    let presence = session("{option @value=x @disabled={#datadom.slices.unavailable} | Option}");
    assert_eq!(
        presence
            .evaluate_suggestions(
                &config(""),
                "input.children.dom:attribute(\"disabled\").value",
                None
            )
            .unwrap()
            .items[0]
            .atom(),
        Some(AtomValue::Boolean(true))
    );
    let s = session("{optgroup @label=Group @disabled=false | {option @value=a | Apple}{option @value=b | Banana}}{option @value=c @hidden=false | Cherry}");
    let c = config("BAN");
    assert_eq!(s.suggestions().unwrap().len(), 3);
    assert_eq!(
        strings(
            s.evaluate_suggestions(
                &c,
                "input.children.children.dom:attribute(\"label\").value",
                None
            )
            .unwrap()
        ),
        ["Apple", "Banana"]
    );
    assert_eq!(
        s.evaluate_suggestions(
            &c,
            "input.children.children.dom:attribute(\"matched\").value",
            None
        )
        .unwrap()
        .items
        .iter()
        .map(Item::atom)
        .collect::<Vec<_>>(),
        [
            Some(AtomValue::Boolean(false)),
            Some(AtomValue::Boolean(true))
        ]
    );
    assert_eq!(
        s.evaluate_suggestions(&c, "dom:attribute(input, \"eligible-count\").value", None)
            .unwrap()
            .items[0]
            .atom(),
        Some(AtomValue::Integer(0))
    );
    assert!(s
        .evaluate_suggestions(
            &SuggestionsConfig {
                expanded: true,
                ..c.clone()
            },
            "input",
            None
        )
        .is_err());
    assert!(s
        .evaluate_suggestions(
            &SuggestionsConfig {
                active: Some(1),
                expanded: true,
                ..c
            },
            "input",
            None
        )
        .is_err());
}

#[test]
fn invalid_revisions_reject_whole_plan_with_attributed_diagnostics() {
    for text in [
        "{data | Missing value}",
        "{cem-option @value={1} @label=Expression}",
        "{option @value=x @label={1}}",
        "{cem-option @value=x}",
        "{option |   }",
        "{data @value=x | Good}{option | Mixed}",
        "{optgroup | {option | No group label}}",
        "{optgroup @label=A | {optgroup @label=B | {option | Nested}}}",
        "{optgroup @label=A | {data @value=x | Wrong}}",
        "@ns v = urn:wrong\n{v:option | Wrong namespace}",
    ] {
        let s = session(text);
        let error = s.suggestions().unwrap_err();
        assert_eq!(
            error.code, "cem.suggestions.source_invalid",
            "{text}: {error:?}"
        );
        assert!(!error.source.frames.is_empty(), "{text}: {error:?}");
    }
    let s = session("{option | Same}");
    let value = s.evaluate("input", None).unwrap().items[0].clone();
    assert!(SuggestionsPlan::prepare(
        &ItemStream::from_items(vec![value.clone(), value]),
        Default::default()
    )
    .is_err());
    assert!(SuggestionsPlan::prepare(
        &ItemStream::once(Item::Atomic(AtomValue::String("record substitute".into()))),
        Default::default()
    )
    .is_err());
}

#[test]
fn denied_native_owning_axes_do_not_become_an_empty_vocabulary() {
    use cem_ql::eval::{
        QueryContextScope, QueryItemView, QueryItemViewKind, QueryNodeAccessError,
        QueryNodeIterator,
    };
    #[derive(Debug)]
    struct Denied;
    impl QueryItemView for Denied {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.denied-option"
        }
        fn identity(&self) -> String {
            "denied".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Node
        }
        fn field(&self, name: &str) -> Option<Vec<Item>> {
            let value = match name {
                "kind" => "element",
                "name" => "option",
                "namespace" => "",
                _ => return None,
            };
            Some(vec![Item::Atomic(AtomValue::String(value.into()))])
        }
        fn attributes(
            &self,
            _: QueryContextScope,
        ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
            Err(QueryNodeAccessError::ScopeViolation)
        }
    }
    let error =
        SuggestionsPlan::prepare(&ItemStream::once(Item::native(Denied)), Default::default())
            .unwrap_err();
    assert_eq!(error.code, "cem.suggestions.source_unavailable");
}

#[test]
fn full_default_casefold_is_pinned_and_never_normalizes_values() {
    assert_eq!(UNICODE_VERSION, "17.0.0");
    let s = session("{data @value=original | Straße}{data @value=other | Σςσ}{data @value=empty | ﬃ}{data @value=dotted | İ}{data @value=accent | é}");
    for (query, expected) in [("STRASSE", 0), ("σσσ", 1), ("FFI", 2), ("i\u{307}", 3)] {
        let v = s
            .evaluate_suggestions(
                &config(query),
                "input.children.dom:attribute(\"matched\").value",
                None,
            )
            .unwrap();
        assert_eq!(
            v.items
                .iter()
                .position(|n| n.atom() == Some(AtomValue::Boolean(true))),
            Some(expected)
        );
    }
    assert!(s
        .evaluate_suggestions(
            &config("e\u{301}"),
            "input.children.dom:attribute(\"matched\").value",
            None
        )
        .unwrap()
        .items
        .iter()
        .all(|n| n.atom() == Some(AtomValue::Boolean(false))));
    assert_eq!(
        strings(
            s.evaluate_suggestions(
                &config("STRASSE"),
                "input.children.dom:attribute(\"value\").value",
                None
            )
            .unwrap()
        ),
        ["original", "other", "empty", "dotted", "accent"]
    );
}

#[test]
fn filter_modes_and_native_state_are_strict_and_independent() {
    let s = session("{data @value=stored | Some Label}");
    assert_eq!(
        s.evaluate_suggestions(
            &config("LABEL"),
            "input.children.dom:attribute(\"matched\").value",
            None
        )
        .unwrap()
        .items[0]
            .atom(),
        Some(AtomValue::Boolean(true))
    );
    let prefix = SuggestionsConfig {
        filter: "prefix".into(),
        ..config("LABEL")
    };
    assert_eq!(
        s.evaluate_suggestions(
            &prefix,
            "input.children.dom:attribute(\"matched\").value",
            None
        )
        .unwrap()
        .items[0]
            .atom(),
        Some(AtomValue::Boolean(false))
    );
    let value = SuggestionsConfig {
        filter_by: Some("label value".into()),
        ..config("stored")
    };
    assert_eq!(
        s.evaluate_suggestions(
            &value,
            "input.children.dom:attribute(\"matched\").value",
            None
        )
        .unwrap()
        .items[0]
            .atom(),
        Some(AtomValue::Boolean(true))
    );
    for filter in ["external", "none"] {
        let c = SuggestionsConfig {
            filter: filter.into(),
            ..config("unmatched")
        };
        assert_eq!(
            s.evaluate_suggestions(&c, "input.children.dom:attribute(\"matched\").value", None)
                .unwrap()
                .items[0]
                .atom(),
            Some(AtomValue::Boolean(true))
        );
        assert!(s
            .evaluate_suggestions(
                &SuggestionsConfig {
                    filter_by: Some("label".into()),
                    ..c
                },
                "input",
                None
            )
            .is_err());
    }
    for c in [
        SuggestionsConfig {
            filter: "unknown".into(),
            ..config("")
        },
        SuggestionsConfig {
            filter_by: Some("label bad".into()),
            ..config("")
        },
        SuggestionsConfig {
            active: Some(0),
            ..config("")
        },
    ] {
        assert!(s.evaluate_suggestions(&c, "input", None).is_err());
    }
    let a = SuggestionsConfig {
        expanded: true,
        active: Some(0),
        ..config("")
    };
    assert_eq!(
        s.evaluate_suggestions(&a, "input.children.dom:attribute(\"active\").value", None)
            .unwrap()
            .items[0]
            .atom(),
        Some(AtomValue::Boolean(true))
    );
    assert_eq!(
        s.evaluate_suggestions(
            &config("no match"),
            "input.children.dom:attribute(\"active\").value",
            None
        )
        .unwrap()
        .items[0]
            .atom(),
        Some(AtomValue::Boolean(false))
    );
}
