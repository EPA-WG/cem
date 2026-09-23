//! Test-only immutable inspection registries; no production lifetime change.
use super::*;
use crate::{
    api::{compile, evaluate, evaluate_with_control, CompileContext, EvaluationContext},
    compile_profile::{measure, Span},
    eval::imported_cem_tree,
};
use cem_ml::{
    import::import_data,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    parser::tree::RetainedCemTree,
    projection::CemTreeAstStream,
    scheduler::ScopePolicy,
    validation::xpath::XPathNativeNode,
};
use std::{cell::RefCell, collections::BTreeMap, sync::OnceLock};

pub(crate) struct Registries {
    schema: SchemaRegistry,
    conversion: ConversionRegistry,
}
pub(crate) type Prepared = Arc<OnceLock<Registries>>;
thread_local! {
    static PREPARED: RefCell<Option<Prepared>> = const { RefCell::new(None) };
}

pub(crate) fn with_prepared<T>(prepared: &Prepared, run: impl FnOnce() -> T) -> T {
    struct Reset(Option<Prepared>);
    impl Drop for Reset {
        fn drop(&mut self) {
            PREPARED.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let _reset = Reset(PREPARED.with(|slot| slot.replace(Some(prepared.clone()))));
    run()
}

pub(super) fn prepared_output(
    owner: &Arc<RetainedCemTree>,
    stream: Arc<CemTreeAstStream>,
) -> Option<ConversionOutputPipelineExecution> {
    let prepared = PREPARED.with(|slot| slot.borrow().clone())?;
    let registries = {
        let _span = Span::new("inspect/prepared-registry-access");
        prepared.get_or_init(|| {
            let _span = Span::new("inspect/prepared-registry-build");
            Registries {
                schema: SchemaRegistry::with_builtin_schemas(),
                conversion: ConversionRegistry::with_builtin_converters(),
            }
        })
    };
    let _span = Span::new("inspect/prepared-writer");
    Some(write(owner, stream, registries))
}

fn write(
    owner: &Arc<RetainedCemTree>,
    stream: Arc<CemTreeAstStream>,
    registries: &Registries,
) -> ConversionOutputPipelineExecution {
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &registries.schema,
        conversion_registry: &registries.conversion,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    let mut pipeline = direct_cem_output_pipeline();
    pipeline.cemt_options.formatter_profile = Some("tabular".into());
    pipeline.cemt_insertion_context.formatter_profile = Some("tabular".into());
    pipeline.writer_insertion_context.formatter_profile = Some("tabular".into());
    execute_conversion_output_pipeline_from_cem_tree_with_environment(
        &environment,
        &pipeline,
        stream,
        owner.node(0).map(|node| node.source.clone()),
        vec![],
        "cemml:inspect",
        None,
        Some(owner.source_uri()),
    )
}

fn context(document: ItemStream) -> (crate::ir::CompiledQuery, EvaluationContext) {
    let bindings = BTreeMap::from([("doc".into(), document)]);
    let query = compile(
        "try { cemml:inspect(doc) } catch (code, message) { code }",
        &CompileContext {
            policy_bindings: bindings.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    (
        query,
        EvaluationContext {
            policy_bindings: bindings,
            ..Default::default()
        },
    )
}

#[test]
fn prepared_inspection_preserves_formats_views_provenance_and_owner_lifetime() {
    let prepared = Prepared::default();
    let mut previous = None;
    for (format, source) in [
        ("xml", "<?keep inert?><r empty=''><![CDATA[<raw>🍒]]></r>"),
        ("json", r#"{"fruit":"<raw>🍒","empty":""}"#),
        ("yaml", "fruit: '<raw>🍒'\nempty: ''"),
        ("csv", "fruit,empty\n<raw>🍒,\n"),
    ] {
        let owner =
            import_data(source, format, "cem", &format!("memory:inspect.{format}")).unwrap();
        for document in [
            imported_cem_tree(owner.clone()),
            crate::xpath::functions::XPathQueryItem::from_node(XPathNativeNode::cem_document(
                owner.clone(),
            )),
        ] {
            let (query, context) = context(ItemStream::once(document));
            let expected = evaluate(&query, &context);
            let (actual, stages) =
                measure(|| with_prepared(&prepared, || evaluate(&query, &context)));
            assert!(actual.error.is_none(), "{:?}", actual.diagnostics);
            assert_eq!(actual, expected);
            assert_eq!(actual.diagnostics, expected.diagnostics);
            assert!(!stages.contains_key("inspect/schema-registry"));
            assert!(!stages.contains_key("inspect/conversion-registry"));
            assert_eq!(
                stages.contains_key("inspect/prepared-registry-build"),
                previous.is_none()
            );
            previous = Some(());
        }
        let stream = Arc::new(cem_tree_inspection(owner.clone()));
        let expected = write(
            &owner,
            stream.clone(),
            &Registries {
                schema: SchemaRegistry::with_builtin_schemas(),
                conversion: ConversionRegistry::with_builtin_converters(),
            },
        );
        let actual = with_prepared(&prepared, || prepared_output(&owner, stream).unwrap());
        assert_eq!(actual.output, expected.output);
        assert_eq!(actual.diagnostics, expected.diagnostics);
        assert_eq!(actual.source_map, expected.source_map);
        assert_eq!(
            rmp_serde::to_vec_named(&actual.output_spans).unwrap(),
            rmp_serde::to_vec_named(&expected.output_spans).unwrap()
        );
        for artifact in [&actual.raw_cem_tree, &actual.formatted_cemt_tree] {
            assert!(Arc::ptr_eq(
                artifact.as_ref().unwrap().owner().source_owner().unwrap(),
                &owner
            ));
        }
        assert!(!actual.output_spans.is_empty());
        let weak = Arc::downgrade(&owner);
        drop(actual);
        drop(expected);
        drop(owner);
        assert!(
            weak.upgrade().is_none(),
            "registry baseline must not retain documents"
        );
    }
    assert!(PREPARED.with(|slot| slot.borrow().is_none()));
}

#[test]
fn prepared_inspection_keeps_lowered_limits_cancellation_and_empty_input_lazy() {
    let owner = import_data(
        "<r><row>one</row><row>two</row></r>",
        "xml",
        "cem",
        "memory:limits",
    )
    .unwrap();
    let (query, mut context) = context(ItemStream::once(imported_cem_tree(owner)));
    let prepared = Prepared::default();
    let success = with_prepared(&prepared, || evaluate(&query, &context));
    let [Item::Atomic(AtomValue::String(output))] = success.items.as_slice() else {
        panic!("inspection text")
    };
    for policy in [
        ScopePolicy::host_root().with_queue_size(2),
        ScopePolicy::host_root().with_memory_bytes(1),
        ScopePolicy::host_root().with_memory_bytes(output.len() as u64 - 1),
        ScopePolicy::host_root().with_memory_bytes(output.len() as u64),
    ] {
        context.scope_policy = policy;
        let expected = evaluate(&query, &context);
        let actual = with_prepared(&prepared, || evaluate(&query, &context));
        assert_eq!(actual, expected);
        assert_eq!(actual.diagnostics, expected.diagnostics);
        if policy.memory_bytes == output.len() as u64 {
            assert_eq!(actual.items, success.items);
        } else {
            assert!(actual.error.is_some() && actual.items.is_empty());
        }
    }
    context.scope_policy = ScopePolicy::host_root();
    for cancel in [false, true] {
        let control = if cancel {
            let control = OperationControl::default();
            control.cancel_root(None, None).unwrap();
            control
        } else {
            OperationControl::with_root_policy(
                Default::default(),
                ScopePolicy::host_root().with_memory_bytes(1),
            )
            .unwrap()
        };
        let empty = Prepared::default();
        let actual = with_prepared(&empty, || {
            evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID)
        });
        assert!(actual.error.is_some() && actual.items.is_empty());
        assert!(empty.get().is_none());
        assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }
    context
        .policy_bindings
        .insert("doc".into(), ItemStream::empty());
    let empty = Prepared::default();
    let actual = with_prepared(&empty, || evaluate(&query, &context));
    assert!(actual.error.is_none() && actual.items.is_empty());
    assert!(empty.get().is_none());
}

#[test]
fn prepared_inspection_initializes_once_across_concurrent_calls_and_restores_on_unwind() {
    let prepared = Prepared::default();
    let start = Arc::new(std::sync::Barrier::new(4));
    let threads: Vec<_> = (0..4)
        .map(|i| {
            let prepared = prepared.clone();
            let start = start.clone();
            std::thread::spawn(move || {
                let owner = import_data(
                    &format!("<r>document-{i}</r>"),
                    "xml",
                    "cem",
                    &format!("memory:thread-{i}"),
                )
                .unwrap();
                let (query, context) = context(ItemStream::once(imported_cem_tree(owner)));
                start.wait();
                let (result, stages) =
                    measure(|| with_prepared(&prepared, || evaluate(&query, &context)));
                let [Item::Atomic(AtomValue::String(text))] = result.items.as_slice() else {
                    panic!("inspection text")
                };
                assert!(text.contains(&format!("document-{i}")));
                assert!(text.contains(&format!("memory:thread-{i}")));
                assert!(result.error.is_none());
                usize::from(stages.contains_key("inspect/prepared-registry-build"))
            })
        })
        .collect();
    assert_eq!(
        threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .sum::<usize>(),
        1
    );
    let failed = std::panic::catch_unwind(|| with_prepared(&prepared, || panic!("fixture unwind")));
    assert!(failed.is_err());
    assert!(PREPARED.with(|slot| slot.borrow().is_none()));
}
