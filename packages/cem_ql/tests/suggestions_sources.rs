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
fn query_publications_share_original_sources_but_keep_immutable_query_state() {
    let s = session("{cem-option @value=x | Original {#later}}");
    let first = s
        .publish_suggestions(
            "first",
            &SuggestionsConfig {
                query: "Original".into(),
                query_revision: 1,
                ..Default::default()
            },
        )
        .unwrap();
    let second = s
        .publish_suggestions(
            "second",
            &SuggestionsConfig {
                query: "Absent".into(),
                query_revision: 2,
                ..Default::default()
            },
        )
        .unwrap();
    let original = s.evaluate("input", None).unwrap();
    for view in [&first, &second] {
        let row = view.row(0).unwrap();
        let source = row.view().unwrap().field("source").unwrap();
        assert_eq!(
            source[0].view().unwrap().identity(),
            original.items[0].view().unwrap().identity()
        );
    }
    assert_eq!(
        first.row(0).unwrap().view().unwrap().identity(),
        second.row(0).unwrap().view().unwrap().identity()
    );
    let query = |view: &cem_ql::suggestions::SuggestionsView| {
        let mut data = TemplateData::default();
        data.bind_reserved_native_slice("suggestions", ItemStream::once(view.root()))
            .unwrap();
        render_plan_to_html(&cem_ql::render::render_compiled_template(
            &compile_template(
                "{output | {$datadom.slices.suggestions.dom:attribute(\"query\").value}}",
                &CompileTemplateOptions {
                    host_bindings: vec!["datadom".into()],
                    ..Default::default()
                },
            ),
            &data,
        ))
    };
    assert!(query(&first).contains("Original"));
    assert!(query(&second).contains("Absent"));
    assert!(s.release_suggestions("first"));
    assert!(s.suggestions_publication("second").is_ok());
    assert!(s.evaluate("input", None).is_ok());
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

#[test]
fn admitted_row_handles_prepare_only_bounded_current_commit_values() {
    let s = session("{cem-option @value=same | First}{cem-option @value=same | Second}{cem-option @value=no @disabled | Disabled}");
    s.publish_suggestions("first", &config("First")).unwrap();
    let first = s.suggestion_row_controls("first").unwrap();
    assert_eq!(first.len(), 3);
    assert_eq!(first[0].value, "same");
    assert!(first[0].eligible);
    assert!(!first[1].eligible);
    assert!(!first[2].eligible);
    assert_ne!(first[0].handle, first[1].handle);
    assert!(s.suggestion_row_control("first", "forged").is_err());
    s.publish_suggestions("second", &config("Second")).unwrap();
    let second = s.suggestion_row_controls("second").unwrap();
    assert_eq!(first[0].handle, second[0].handle);
    assert!(!second[0].eligible);
    assert!(second[1].eligible);
    assert!(s.release_suggestions("first"));
    assert!(s.suggestion_row_control("first", &first[0].handle).is_err());
    assert!(
        s.suggestion_row_control("second", &first[1].handle)
            .unwrap()
            .eligible
    );
    let replacement = session("{cem-option @value=same | First}");
    replacement
        .publish_suggestions("second", &config("First"))
        .unwrap();
    assert!(replacement
        .suggestion_row_control("second", &first[0].handle)
        .is_err());
    let bounded = session_with_limits(
        "{cem-option @value=x | Choice}",
        cem_ml::value::artifact::CemValueArtifactLimits {
            // Includes the retained default label as well as scalar controls.
            max_bytes: 1024,
            ..Default::default()
        },
    );
    bounded.publish_suggestions("ready", &config("")).unwrap();
    assert_eq!(bounded.suggestion_row_controls("ready").unwrap().len(), 1);
}

#[test]
fn native_row_placement_metadata_requires_exact_current_view_and_unique_shells() {
    use cem_ql::suggestions::project_suggestion_placements;
    let s = session("{cem-option @value=same | First}{cem-option @value=same | Second}");
    let view = s.publish_suggestions("placements", &config("")).unwrap();
    let render = |source: &str, row: Item| {
        let artifact = compile_template(
            source,
            &CompileTemplateOptions {
                host_bindings: vec!["row".into()],
                ..Default::default()
            },
        );
        assert!(
            artifact.diagnostics.is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        cem_ql::render::render_compiled_template(
            &artifact,
            &TemplateData::default().with_binding("row", ItemStream::once(row)),
        )
    };
    let plan = render(
        "{section | before{div @role=option @suggestion-row={row} | label}}",
        view.row(1).unwrap(),
    );
    let (clean, metadata) =
        project_suggestion_placements(&plan, Some(&view), &Default::default()).unwrap();
    assert_eq!(metadata.len(), 1);
    assert_eq!(metadata[0].path, vec![0, 1]);
    assert_eq!(metadata[0].handle, view.row_controls()[1].handle);
    assert!(!render_plan_to_html(&clean).contains("suggestion-row"));
    for expression in ["#row", "#(#row)"] {
        let source = format!("{{div @role=option @suggestion-row={{{expression}}} | label}}");
        let (_, refs) = project_suggestion_placements(
            &render(&source, view.row(1).unwrap()),
            Some(&view),
            &Default::default(),
        )
        .unwrap();
        assert_eq!(refs[0].handle, metadata[0].handle);
    }
    assert!(project_suggestion_placements(&plan, None, &Default::default()).is_err());
    let newer = s
        .publish_suggestions("next-placements", &config("Second"))
        .unwrap();
    assert!(project_suggestion_placements(&plan, Some(&newer), &Default::default()).is_err());
    let original = s.evaluate("input.children", None).unwrap().items[0].clone();
    assert!(project_suggestion_placements(
        &render("{div @role=option @suggestion-row={row}}", original),
        Some(&view),
        &Default::default()
    )
    .is_err());
    for source in [
        "{div @role=option @suggestion-row=forged}",
        "{div @suggestion-row={row}}",
        "{div @role=option @suggestion-row={row}}{div @role=option @suggestion-row={row}}",
    ] {
        assert!(
            project_suggestion_placements(
                &render(source, view.row(1).unwrap()),
                Some(&view),
                &Default::default()
            )
            .is_err(),
            "{source}"
        );
    }
    assert!(project_suggestion_placements(
        &plan,
        Some(&view),
        &cem_ml::value::artifact::CemValueArtifactLimits {
            max_values: 1,
            ..Default::default()
        }
    )
    .is_err());
}

#[test]
fn local_xml_options_capture_retains_namespace_owner_and_bounds() {
    let text = r#"<options xmlns="http://www.w3.org/1999/xhtml"><option value="a">Alpha</option><option value="b">Beta</option></options>"#;
    let captured = session(text);
    let plan = captured.suggestions().unwrap();
    let first = plan.view(&config("Alpha")).unwrap();
    let second = plan.view(&config("Beta")).unwrap();
    assert_eq!(plan.len(), 2);
    let selected = captured.evaluate("input", None).unwrap();
    assert_eq!(
        strings(captured.evaluate("input.namespace", None).unwrap()),
        vec!["http://www.w3.org/1999/xhtml"; 2]
    );
    let root = first.root().view().unwrap().field("children").unwrap();
    let other = second.root().view().unwrap().field("children").unwrap();
    assert_eq!(
        root[0].view().unwrap().field("source").unwrap()[0]
            .view()
            .unwrap()
            .identity(),
        selected.items[0].view().unwrap().identity()
    );
    assert_eq!(
        root[0].view().unwrap().field("source").unwrap()[0]
            .view()
            .unwrap()
            .identity(),
        other[0].view().unwrap().field("source").unwrap()[0]
            .view()
            .unwrap()
            .identity()
    );
    let limits = cem_ml::value::artifact::CemValueArtifactLimits {
        max_values: 1,
        ..Default::default()
    };
    assert!(NativeCapabilitySession::prepare(
        vec![ElementReferenceSource {
            source: RetainedReferenceSource::parse(
                text.as_bytes(),
                "application/xml",
                "memory:local-options.xml",
                ReloadLimits::default()
            )
            .unwrap(),
            context: true,
            policy: ReferenceScopePolicy::schema_defaults().unwrap(),
        }],
        0,
        &[],
        &[],
        TemplateData::default(),
        "input.children.children",
        true,
        limits
    )
    .is_err());
}

#[test]
fn native_label_output_requires_bounded_noninteractive_content() {
    let source = session("{data @value=x | Label}");
    let options = CompileTemplateOptions {
        host_bindings: vec!["suggestion".into(), "group".into()],
        ..Default::default()
    };
    let safe = compile_template(
        "{span | {strong | {$suggestion.dom:attribute(\"label\").value}}}",
        &options,
    );
    let output = source
        .render_suggestion(&config(""), &safe, Some(0))
        .unwrap();
    assert!(render_plan_to_html(&output).contains("Label"));
    for limits in [
        cem_ml::value::artifact::CemValueArtifactLimits {
            max_values: 1,
            ..Default::default()
        },
        cem_ml::value::artifact::CemValueArtifactLimits {
            max_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(cem_ql::suggestions::admit_label_output(&output, &limits).is_err());
    }
    for template in [
        "{button | Choose}",
        "{span @tabindex=-1 | Label}",
        "{span @id=own | Label}",
        "{span @role=option | Label}",
        "{span @contenteditable=false | Label}",
        "{span @autofocus | Label}",
        "{a @href=/ | Label}",
        "{input @type=hidden}",
        "{label | Label}",
        "{span @onclick=execute | Label}",
        "{foreign-widget | Label}",
        "{span @slot=editor | Label}",
        "{span @form=other | Label}",
        "{span @style=display:none | Label}",
        "{attribute @name=value | Changed}{span | Label}",
        "{style | ```span { color: red }```}{span | Label}",
    ] {
        let template = compile_template(template, &options);
        assert!(
            source
                .render_suggestion(&config(""), &template, Some(0))
                .is_err(),
            "unsafe label was admitted: {template:?}"
        );
    }
    let grouped = session("{optgroup @label=Group | {option @value=x | Label}}");
    assert!(grouped
        .render_suggestion_group(
            &config(""),
            &compile_template("{button | Group}", &options),
            0
        )
        .is_err());
    let constrained = session_with_limits(
        "{data @value=x | Label}",
        cem_ml::value::artifact::CemValueArtifactLimits {
            max_depth: 3,
            ..Default::default()
        },
    );
    assert!(constrained
        .render_suggestion(
            &config(""),
            &compile_template("{span | {span | {span | {span | Deep}}}}", &options),
            Some(0)
        )
        .is_err());
    let rich = session("{data @value=x | Label {strong | Safe}}");
    let rich_template = compile_template("{$suggestion.content}", &options);
    assert!(render_plan_to_html(
        &rich
            .render_suggestion(&config(""), &rich_template, Some(0))
            .unwrap()
    )
    .contains("Safe"));
    let svg = session(
        r#"<options xmlns="http://www.w3.org/1999/xhtml"><option value="x" label="Icon"><svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0"/></svg></option></options>"#,
    );
    let svg_output = svg
        .render_suggestion(&config(""), &rich_template, Some(0))
        .unwrap();
    assert!(
        matches!(&svg_output.nodes[0], cem_ql::render::RenderPlanNode::Element { namespace: Some(uri), .. } if uri == "http://www.w3.org/2000/svg")
    );
    let unsafe_source = session("{data @value=x | Label {button | Unsafe}}");
    let error = unsafe_source
        .render_suggestion(&config(""), &rich_template, Some(0))
        .unwrap_err();
    assert_eq!(error.code, "cem.suggestions.label_invalid");
    let original = unsafe_source.evaluate("input.children", None).unwrap();
    let button_source = original
        .items
        .iter()
        .find_map(|item| {
            let node = retained_cem_node(item)?;
            match node.node() {
                cem_ml::parser::CemAstNode::Element {
                    expanded_name,
                    source,
                    ..
                } if expanded_name.local_name == "button" => Some(source.clone()),
                _ => None,
            }
        })
        .unwrap();
    assert_eq!(error.source.origin(), button_source.origin());
    assert!(!error.source.frames.is_empty());
    let reference_source = session("{data @value=x @label=Label | {#missing}}");
    let before = reference_source
        .evaluate("input.children.expression", None)
        .unwrap();
    reference_source
        .render_suggestion(&config(""), &safe, Some(0))
        .unwrap();
    assert_eq!(strings(before), vec!["#missing"]);
    assert_eq!(
        strings(
            reference_source
                .evaluate("input.children.expression", None)
                .unwrap()
        ),
        vec!["#missing"]
    );
}

#[test]
fn captured_labels_stay_source_owned_and_publish_atomically() {
    let text = r#"<capture xmlns="http://www.w3.org/1999/xhtml" xmlns:s="http://www.w3.org/2000/svg"><options><option value="x" label="Explicit">Ignored</option></options><option-label type="text/cem-ml">{span | {$suggestion.dom:attribute("label").value}}{s:svg | {s:path @d="M0 0"}}</option-label></capture>"#;
    let owner = RetainedReferenceSource::parse(
        text.as_bytes(),
        "application/xml",
        "memory:labels.xml",
        ReloadLimits::default(),
    )
    .unwrap();
    let mut s = NativeCapabilitySession::prepare(
        vec![ElementReferenceSource {
            source: owner,
            context: true,
            policy: ReferenceScopePolicy::schema_defaults().unwrap(),
        }],
        0,
        &[],
        &[],
        TemplateData::default(),
        r#"seq:where(input.children.children, fn(n) => n.name == "options").children"#,
        true,
        Default::default(),
    )
    .unwrap();
    // Select the options' children only; the template text must never become an option.
    // The actual fixture selection below is intentionally explicit.
    s.configure_suggestion_labels(
        Some("seq:where(input.children.children, fn(n) => n.name == \"option-label\")"),
        None,
    )
    .unwrap();
    let view = s.publish_suggestions("labels", &config("")).unwrap();
    let label = view
        .row(0)
        .unwrap()
        .view()
        .unwrap()
        .field("labelContent")
        .unwrap();
    assert!(!label.is_empty());
    assert_eq!(
        label[1].view().unwrap().field("namespace").unwrap()[0].atom(),
        Some(AtomValue::String("http://www.w3.org/2000/svg".into()))
    );
    let template = compile_template(
        "{$datadom.slices.suggestions.children.labelContent}",
        &Default::default(),
    );
    let html = render_plan_to_html(
        &s.render_suggestions_frame("labels", &template, TemplateData::default())
            .unwrap(),
    );
    assert!(html.contains("Explicit"));
    assert!(!html.contains("Ignored"));
    assert!(s.configure_suggestion_labels(None, None).is_err());
}

fn captured_session(
    text: &str,
    limits: cem_ml::value::artifact::CemValueArtifactLimits,
    data: TemplateData,
) -> NativeCapabilitySession {
    let source = RetainedReferenceSource::parse(
        text.as_bytes(),
        "application/xml",
        "memory:captured-labels.xml",
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
        data,
        r#"seq:where(input.children.children, fn(n) => n.name == "options").children"#,
        true,
        limits,
    )
    .unwrap()
}
const OPTION_LABEL: &str =
    r#"seq:where(input.children.children, fn(n) => n.name == "option-label")"#;
#[test]
fn captured_labels_reject_ambient_bindings_and_invalid_publications() {
    for body in ["{span | {$private}}", "{foreign:span | Label}"] {
        let text = format!(
            r#"<capture xmlns="http://www.w3.org/1999/xhtml"><options><option value="x">Label</option></options><option-label type="text/cem-ml">{body}</option-label></capture>"#
        );
        let mut data = TemplateData::default();
        data.bindings.insert(
            "private".into(),
            ItemStream::once(Item::Atomic(AtomValue::String("Consumer secret".into()))),
        );
        let mut s = captured_session(&text, Default::default(), data);
        assert!(s
            .configure_suggestion_labels(Some(OPTION_LABEL), None)
            .is_err());
    }
    let mut s = captured_session(
        r#"<capture xmlns="http://www.w3.org/1999/xhtml"><options><option value="x">Label</option></options><option-label type="text/cem-ml">{button | {$suggestion.dom:attribute("label").value}}</option-label></capture>"#,
        Default::default(),
        TemplateData::default(),
    );
    s.configure_suggestion_labels(Some(OPTION_LABEL), None)
        .unwrap();
    assert!(s.publish_suggestions("denied", &config("")).is_err());
    assert!(s.suggestions_publication("denied").is_err());
    assert!(s.publish_suggestions("denied", &config("x")).is_err());
    assert_eq!(s.len(), 1);
}
#[test]
fn default_labels_preserve_rich_content_and_share_publication_budget() {
    let mut s = captured_session(
        r#"<capture xmlns="http://www.w3.org/1999/xhtml"><options><optgroup label="Group"><option value="x" label="Explicit">Ignored</option></optgroup></options></capture>"#,
        Default::default(),
        TemplateData::default(),
    );
    s.configure_suggestion_labels(None, None).unwrap();
    s.publish_suggestions("default", &config("")).unwrap();
    let template = compile_template("{$datadom.slices.suggestions.children.labelContent}|{$datadom.slices.suggestions.children.children.labelContent}", &Default::default());
    assert_eq!(
        render_plan_to_html(
            &s.render_suggestions_frame("default", &template, TemplateData::default())
                .unwrap()
        ),
        "Group|Explicit"
    );
    let mut rich = captured_session(
        r#"<capture xmlns="http://www.w3.org/1999/xhtml"><options><data value="x">Rich <strong>content</strong></data></options></capture>"#,
        Default::default(),
        TemplateData::default(),
    );
    rich.configure_suggestion_labels(None, None).unwrap();
    rich.publish_suggestions("rich", &config("")).unwrap();
    let template = compile_template(
        "{$datadom.slices.suggestions.children.labelContent}",
        &Default::default(),
    );
    assert!(render_plan_to_html(
        &rich
            .render_suggestions_frame("rich", &template, TemplateData::default())
            .unwrap()
    )
    .contains("<strong"));
    let label = "{span | Label}".repeat(5);
    let text = format!(
        r#"<capture xmlns="http://www.w3.org/1999/xhtml"><options><option value="a">A</option><option value="b">B</option><option value="c">C</option></options><option-label type="text/cem-ml">{label}</option-label></capture>"#
    );
    let mut bounded = captured_session(
        &text,
        cem_ml::value::artifact::CemValueArtifactLimits {
            max_values: 60,
            ..Default::default()
        },
        TemplateData::default(),
    );
    bounded
        .configure_suggestion_labels(Some(OPTION_LABEL), None)
        .unwrap();
    assert!(bounded.publish_suggestions("bounded", &config("")).is_err());
    assert!(bounded.suggestions_publication("bounded").is_err());
}
