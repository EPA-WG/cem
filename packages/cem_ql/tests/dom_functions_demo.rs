//! The authored browser fixture must first work through native CEMT rendering.
use cem_ql::render::{render_template, TemplateData};

#[test]
fn every_authored_dom_sample_renders_natively() {
    let source = include_str!("../../cem-elements/demo/functions/dom.html");
    let expected = [
        "a, b, a", "row", "a, b", "4", "section, row", "section", "fruit",
        "ivysaur", "2", "other", "2", "b", "a, a", "a", "b", "a, b", "b, c",
        "a!, b!", "a, a, b, b", "true", "false", "true", "3", "a, b, b",
        "second, first, third", "c, b, a",
    ];
    let templates: Vec<_> = source.split("<template type=\"text/cem-ml\">").skip(1)
        .map(|part| part.split_once("</template>").unwrap().0).collect();
    assert_eq!(templates.len(), expected.len());
    for (index, (template, expected)) in templates.iter().zip(expected).enumerate() {
        // Decode the enclosing HTML text layer through CEM-ML's XML import,
        // just as the browser supplies decoded template text. No test parser.
        use cem_ql::{api::{compile, evaluate, CompileContext, EvaluationContext}, eval::{AtomValue, Item, ItemStream}};
        let mut context = EvaluationContext::default();
        context.policy_bindings.insert("source".into(), ItemStream::once(Item::Atomic(AtomValue::String(format!("<template>{template}</template>")))));
        let query = compile("dom:text(data:read(source, \"xml\").root)", &CompileContext { policy_bindings: context.policy_bindings.clone(), ..Default::default() }).unwrap();
        let decoded = evaluate(&query, &context);
        assert!(decoded.error.is_none(), "{:?}", decoded.diagnostics);
        let Some(AtomValue::String(template)) = decoded.items[0].atom() else { panic!() };
        let result = render_template(&template, &TemplateData::default());
        assert!(result.diagnostics.is_empty(), "sample {}: {:?}", index + 1, result.diagnostics);
        assert!(result.rendered.contains(&format!("<output>{expected}</output>")), "sample {}: {}", index + 1, result.rendered);
    }
}
