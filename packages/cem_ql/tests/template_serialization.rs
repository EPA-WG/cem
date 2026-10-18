use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{
    compile_template, render_compiled_template, render_plan_to_html_with_control,
    render_plan_to_xml_with_source_map, CompileTemplateOptions, TemplateData,
};

#[test]
fn html_void_serialization_ignores_constructed_children_and_retains_output_ranges() {
    for tag in ["input", "img", "br", "meta"] {
        let source = format!("{{cem:element @name={tag} @namespace=http://www.w3.org/1999/xhtml |\n    {{cem:attribute @name=id @value=field}}\n    {{$text}}\n}}");
        let artifact = compile_template(
            &source,
            &CompileTemplateOptions {
                host_bindings: vec!["text".into()],
                ..Default::default()
            },
        );
        assert!(
            artifact.diagnostics.is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        for value in ["", "ignored HTML void content"] {
            let data = TemplateData::default().with_binding(
                "text",
                ItemStream::once(Item::Atomic(AtomValue::String(value.into()))),
            );
            let plan = render_compiled_template(&artifact, &data);
            assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
            let html = render_plan_to_html_with_control(
                &plan,
                &OperationControl::default(),
                ROOT_EXECUTION_SCOPE_ID,
            )
            .unwrap();
            assert_eq!(html.rendered, format!("<{tag} id=\"field\">"));
            assert!(!html.output_spans.is_empty());
            assert!(html
                .output_spans
                .iter()
                .all(|span| span.output_range.end() <= html.rendered.len() as u64));
            let xml = render_plan_to_xml_with_source_map(&plan);
            assert!(
                xml.rendered.contains(&format!("</{tag}>")),
                "{}",
                xml.rendered
            );
            assert!(xml.rendered.contains(value));
        }
    }
}

#[test]
fn foreign_elements_named_like_html_void_elements_keep_their_content_and_end_tags() {
    let artifact = compile_template(
        "{cem:element @name=input @namespace=urn:foreign | Kept}",
        &CompileTemplateOptions::default(),
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    let plan = render_compiled_template(&artifact, &TemplateData::default());
    let html = render_plan_to_html_with_control(
        &plan,
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
    )
    .unwrap();
    assert_eq!(html.rendered, "<input>Kept</input>");
    let empty = compile_template(
        "{cem:element @name=input @namespace=urn:foreign}",
        &CompileTemplateOptions::default(),
    );
    let plan = render_compiled_template(&empty, &TemplateData::default());
    let html = render_plan_to_html_with_control(
        &plan,
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
    )
    .unwrap();
    assert_eq!(html.rendered, "<input></input>");
}
