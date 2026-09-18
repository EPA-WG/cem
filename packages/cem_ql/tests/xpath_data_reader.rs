//! XPATH-DEMO-READER-VIEW: explicitly selected XML owners, never inferred data.
use cem_ml::{
    lifecycle::LoadedInputAstStream,
    resolver::{ResolverPolicy, ResolverRegistry},
};
use cem_ql::{
    api::{compile, evaluate, CompileContext, EvaluationContext},
    eval::{AtomValue, Item, ItemStream},
    native::{NativeQueryFunction, NativeQueryRequest},
    render::{
        compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
        TemplateData,
    },
    xpath::functions::{CemtXPathFunctions, XPathQueryItem},
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, Weak},
};

const LIBRARY: &str = r#"@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@ns f = "urn:fruit"
@default t
{module |
    {function @name=demo.pick @visibility=public @returns=any |
        {param @name=document @type=any @required=true}
        {body | {xpath @context=document @sequence-type="node()*" |
            {expression | /f:r/f:item}
    }   }   }
    {function @name=demo.accept @visibility=public @returns=boolean |
        {param @name=candidate @type=any @required=true}
        {body | {xpath @context=candidate @sequence-type="xs:boolean" |
            {expression | exists(@qty[. >= 2])}
}   }   }   }"#;

fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

fn context(source: &str) -> EvaluationContext {
    let mut context = EvaluationContext {
        policy_bindings: BTreeMap::from([("source".into(), ItemStream::once(string(source)))]),
        ..Default::default()
    };
    CemtXPathFunctions::compile(LIBRARY, "memory:reader.cemt")
        .unwrap()
        .install(
            &mut context.native_functions,
            Arc::new(ResolverRegistry::new()),
            Arc::new(ResolverPolicy::new()),
        )
        .unwrap();
    context
}

fn run(query: &str, context: &EvaluationContext) -> ItemStream {
    let compiled = compile(
        query,
        &CompileContext {
            policy_bindings: context.policy_bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let result = evaluate(&compiled, context);
    assert!(result.error.is_none(), "{result:?}");
    result
}

fn owner(item: &Item) -> Arc<LoadedInputAstStream> {
    item.view()
        .unwrap()
        .downcast_ref::<XPathQueryItem>()
        .expect("explicit XPath view")
        .xpath_item()
        .native_node()
        .unwrap()
        .source_owner()
        .unwrap()
}

#[test]
fn explicit_view_retains_original_xml_and_namespaced_selected_nodes() {
    let context =
        context("<r xmlns='urn:fruit'><item qty='2'>🍒</item><item xmlns=''>wrong</item></r>");
    let root = run(r#"data:read(source, "xml", "xpath").root"#, &context);
    assert_eq!(root.items.len(), 1);
    let original = owner(&root.items[0]);
    let selected = run(
        r#"native:call("demo.pick", data:read(source, "xml", "xpath").root)"#,
        &context,
    );
    assert_eq!(selected.items.len(), 1);
    assert!(Arc::ptr_eq(&original, &owner(&selected.items[0])));
    assert_eq!(
        selected.items[0].atom(),
        Some(AtomValue::String("🍒".into()))
    );
    assert!(!selected.items[0].source_map().unwrap().frames.is_empty());
    let default = run(r#"data:read(source, "xml").root"#, &context);
    assert!(default.items[0]
        .view()
        .unwrap()
        .downcast_ref::<XPathQueryItem>()
        .is_none());
}

#[test]
fn changed_sources_and_contexts_do_not_alias_native_owners() {
    let mut context = context("<r/>");
    let query = r#"data:read(source, "xml", "xpath").root"#;
    let first = run(query, &context);
    let same = run(query, &context);
    assert!(Arc::ptr_eq(&owner(&first.items[0]), &owner(&same.items[0])));
    let separate = run(query, &self::context("<r/>"));
    assert!(!Arc::ptr_eq(
        &owner(&first.items[0]),
        &owner(&separate.items[0])
    ));
    context
        .policy_bindings
        .insert("source".into(), ItemStream::once(string("<changed/>")));
    let changed = run(query, &context);
    assert!(!Arc::ptr_eq(
        &owner(&first.items[0]),
        &owner(&changed.items[0])
    ));
    let shared = context.clone();
    let again = run(query, &shared);
    assert!(Arc::ptr_eq(
        &owner(&changed.items[0]),
        &owner(&again.items[0])
    ));
    shared.data_readers.clear();
    let reparsed = run(query, &context);
    assert!(!Arc::ptr_eq(
        &owner(&changed.items[0]),
        &owner(&reparsed.items[0])
    ));
    assert_eq!(
        changed.items[0].atom(),
        Some(AtomValue::String(String::new()))
    );
}

#[test]
fn invalid_inputs_return_reports_without_partial_roots() {
    for (source, format) in [
        ("<r>".to_owned(), "xml"),
        ("{}".into(), "json"),
        ("<!DOCTYPE r><r/>".into(), "xml"),
        (format!("<r>{}</r>", "x".repeat(32768)), "xml"),
        (format!("{}{}", "<r>".repeat(65), "</r>".repeat(65)), "xml"),
        (format!("<r>{}</r>", "<i/>".repeat(4096)), "xml"),
    ] {
        let context = context(&source);
        let report = run(
            &format!(r#"data:read(source, "{format}", "xpath")"#),
            &context,
        );
        let view = report.items[0].view().unwrap();
        assert!(
            !matches!(view.field("error").unwrap()[0].atom(), Some(AtomValue::String(s)) if s.is_empty())
        );
        assert!(view.field("root").unwrap().is_empty());
    }
}

#[test]
fn reader_retention_is_bounded_and_releases_owners_with_its_context() {
    let mut context = context("<r/>");
    let query = r#"data:read(source, "xml", "xpath").root"#;
    let first = run(query, &context);
    let weak = Arc::downgrade(&owner(&first.items[0]));
    drop(first);
    assert!(weak.upgrade().is_some(), "unchanged reads retain the parse");
    for n in 0..16 {
        context.policy_bindings.insert(
            "source".into(),
            ItemStream::once(string(&format!("<r n='{n}'/>"))),
        );
        run(query, &context);
    }
    assert!(
        weak.upgrade().is_none(),
        "the least recent owner is evicted"
    );
    let last = run(query, &context);
    let last_weak = Arc::downgrade(&owner(&last.items[0]));
    drop(context);
    assert!(
        last_weak.upgrade().is_some(),
        "returned nodes outlive reader disposal"
    );
    drop(last);
    assert!(last_weak.upgrade().is_none());
}

#[test]
fn declarative_reader_renders_changed_xml_and_cemt_matches() {
    let context = context("");
    let template = compile_template(
        r#"{module |
        {template @mode=fruit @match='native:call("demo.accept", node)' | {body | {b | {$node}}}}
        {template @mode=fruit @match=true @priority=-10 | {body | {i | {$node}}}}
        {body |
            {cem-data @name=document @select=source @type=xml @projection=xpath}
            {apply-templates @mode=fruit @select='native:call("demo.pick", document.root)'}
    }   }"#,
        &CompileTemplateOptions {
            host_bindings: vec!["source".into()],
            ..Default::default()
        },
    );
    assert!(
        template.diagnostics.is_empty(),
        "{:?}",
        template.diagnostics
    );
    let mut data = TemplateData {
        native_functions: context.native_functions,
        ..Default::default()
    };
    for (source, expected) in [
        (
            "<r xmlns='urn:fruit'><item qty='1'>🍋</item><item qty='2'>🍒</item></r>",
            "<i>🍋</i><b>🍒</b>",
        ),
        (
            "<r xmlns='urn:fruit'><item qty='3'>🍇</item></r>",
            "<b>🍇</b>",
        ),
    ] {
        data.bindings
            .insert("source".into(), ItemStream::once(string(source)));
        let plan = render_compiled_template(&template, &data);
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        assert_eq!(render_plan_to_html(&plan).trim(), expected);
    }
}

#[derive(Debug)]
struct ObserveOwner(Arc<Mutex<Vec<Weak<LoadedInputAstStream>>>>);

impl NativeQueryFunction for ObserveOwner {
    fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
        let value = request.arguments[0].clone();
        self.0
            .lock()
            .unwrap()
            .push(Arc::downgrade(&owner(&value.items[0])));
        value
    }
}

#[test]
fn render_turns_reuse_reader_owners_until_template_data_disposal() {
    let observed = Arc::new(Mutex::new(Vec::new()));
    let mut data = TemplateData::default();
    data.native_functions
        .register("test.observe", 1, ObserveOwner(observed.clone()))
        .unwrap();
    let template = compile_template(
        r#"{cem-data @name=document @select=source @type=xml @projection=xpath}{output | {$native:call("test.observe", document.root)}}"#,
        &CompileTemplateOptions {
            host_bindings: vec!["source".into()],
            ..Default::default()
        },
    );
    assert!(template.diagnostics.is_empty());
    for text in ["one", "one", "two"] {
        data.bindings.insert(
            "source".into(),
            ItemStream::once(string(&format!("<r>{text}</r>"))),
        );
        let plan = render_compiled_template(&template, &data);
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        assert_eq!(
            render_plan_to_html(&plan),
            format!("<output>{text}</output>")
        );
    }
    let owners = observed.lock().unwrap();
    assert_eq!(owners.len(), 3);
    assert!(owners.iter().all(|w| w.upgrade().is_some()));
    assert!(owners[0].ptr_eq(&owners[1]));
    assert!(!owners[0].ptr_eq(&owners[2]));
    drop(data);
    assert!(owners.iter().all(|w| w.upgrade().is_none()));
}
