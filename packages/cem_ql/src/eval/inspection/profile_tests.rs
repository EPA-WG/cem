//! Inspection lifetime regressions, fresh baseline and isolated cold-cache override.
use super::*;
use crate::{
    api::{compile, evaluate, evaluate_with_control, CompileContext, EvaluationContext},
    compile_profile::measure,
    eval::imported_cem_tree,
};
use cem_ml::{
    import::import_data,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    scheduler::ScopePolicy,
    validation::xpath::XPathNativeNode,
};
use std::{cell::RefCell, collections::BTreeMap, sync::OnceLock};

#[derive(Clone, Default)]
pub(crate) struct Prepared(Arc<OnceLock<InspectionRegistries>>);

#[derive(Clone)]
enum RegistryOverride {
    Fresh,
    Prepared(Prepared),
}
thread_local! {
    static OVERRIDE: RefCell<Option<RegistryOverride>> = const { RefCell::new(None) };
}

fn with_override<T>(value: RegistryOverride, run: impl FnOnce() -> T) -> T {
    struct Reset(Option<RegistryOverride>);
    impl Drop for Reset {
        fn drop(&mut self) {
            OVERRIDE.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let _reset = Reset(OVERRIDE.with(|slot| slot.replace(Some(value))));
    run()
}

pub(crate) fn with_prepared<T>(prepared: &Prepared, run: impl FnOnce() -> T) -> T {
    with_override(RegistryOverride::Prepared(prepared.clone()), run)
}

pub(crate) fn with_fresh<T>(run: impl FnOnce() -> T) -> T {
    with_override(RegistryOverride::Fresh, run)
}

pub(super) fn override_output(
    owner: &Arc<RetainedCemTree>,
    stream: Arc<CemTreeAstStream>,
) -> Option<ConversionOutputPipelineExecution> {
    let override_value = OVERRIDE.with(|slot| slot.borrow().clone())?;
    Some(match override_value {
        RegistryOverride::Fresh => {
            write_inspection(owner, stream, &InspectionRegistries::builtin())
        }
        RegistryOverride::Prepared(prepared) => {
            write_inspection(owner, stream, registry_pair(&prepared.0))
        }
    })
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
fn inspection_preserves_formats_views_provenance_and_owner_lifetime() {
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
            let expected = with_fresh(|| evaluate(&query, &context));
            let actual = evaluate(&query, &context);
            assert_eq!(actual, expected);
            assert_eq!(actual.diagnostics, expected.diagnostics);
            let (actual, stages) =
                measure(|| with_prepared(&prepared, || evaluate(&query, &context)));
            assert!(actual.error.is_none(), "{:?}", actual.diagnostics);
            assert_eq!(actual, expected);
            assert_eq!(actual.diagnostics, expected.diagnostics);
            assert_eq!(
                stages.contains_key("inspect/registry-build"),
                previous.is_none()
            );
            previous = Some(());
        }
        let stream = Arc::new(cem_tree_inspection(owner.clone()));
        let expected = write_inspection(
            &owner,
            stream.clone(),
            &InspectionRegistries {
                schema: SchemaRegistry::with_builtin_schemas(),
                conversion: ConversionRegistry::with_builtin_converters(),
            },
        );
        let actual = inspection_output(&owner, stream);
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
    assert!(OVERRIDE.with(|slot| slot.borrow().is_none()));
}

#[test]
fn inspection_keeps_lowered_limits_cancellation_and_empty_input_lazy() {
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
        let expected = with_fresh(|| evaluate(&query, &context));
        let actual = evaluate(&query, &context);
        assert_eq!(actual, expected);
        assert_eq!(actual.diagnostics, expected.diagnostics);
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
        assert!(empty.0.get().is_none());
        let (default, stages) =
            measure(|| evaluate_with_control(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID));
        assert_eq!(default, actual);
        assert_eq!(default.diagnostics, actual.diagnostics);
        assert!(!stages.contains_key("inspect/registry-access"));
        assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }
    context
        .policy_bindings
        .insert("doc".into(), ItemStream::empty());
    let empty = Prepared::default();
    let actual = with_prepared(&empty, || evaluate(&query, &context));
    assert!(actual.error.is_none() && actual.items.is_empty());
    assert!(empty.0.get().is_none());
    let (default, stages) = measure(|| evaluate(&query, &context));
    assert_eq!(default, actual);
    assert!(!stages.contains_key("inspect/registry-access"));
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
                usize::from(stages.contains_key("inspect/registry-build"))
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
    assert!(OVERRIDE.with(|slot| slot.borrow().is_none()));
}

#[test]
fn default_inspection_reuses_registries_without_retaining_documents() {
    let owner = import_data("<r>unique owner</r>", "xml", "cem", "memory:default").unwrap();
    let weak = Arc::downgrade(&owner);
    let (query, context) = context(ItemStream::once(imported_cem_tree(owner)));
    let expected = with_fresh(|| evaluate(&query, &context));
    assert_eq!(evaluate(&query, &context), expected);
    for _ in 0..2 {
        let (actual, stages) = measure(|| evaluate(&query, &context));
        assert_eq!(actual, expected);
        assert_eq!(actual.diagnostics, expected.diagnostics);
        assert!(!stages.contains_key("inspect/schema-registry"));
        assert!(!stages.contains_key("inspect/conversion-registry"));
        assert_eq!(stages["inspect/registry-access"].calls, 1);
        assert_eq!(stages["inspect/writer"].calls, 1);
    }
    drop(query);
    drop(context);
    drop(expected);
    assert!(weak.upgrade().is_none());
}

#[test]
fn default_concurrent_inspection_shares_registries_and_keeps_documents_local() {
    let start = Arc::new(std::sync::Barrier::new(4));
    let threads: Vec<_> = (0..4)
        .map(|i| {
            let start = start.clone();
            std::thread::spawn(move || {
                let owner = import_data(
                    &format!("<r>default-{i}</r>"),
                    "xml",
                    "cem",
                    &format!("memory:default-{i}"),
                )
                .unwrap();
                let weak = Arc::downgrade(&owner);
                let (query, context) = context(ItemStream::once(imported_cem_tree(owner)));
                start.wait();
                let (result, stages) = measure(|| evaluate(&query, &context));
                assert!(result.error.is_none());
                assert_eq!(stages["inspect/registry-access"].calls, 1);
                let [Item::Atomic(AtomValue::String(text))] = result.items.as_slice() else {
                    panic!("inspection text")
                };
                assert!(text.contains(&format!("default-{i}")));
                assert!(text.contains(&format!("memory:default-{i}")));
                drop(query);
                drop(context);
                drop(result);
                assert!(weak.upgrade().is_none());
                registry_pair(&INSPECTION_REGISTRIES) as *const InspectionRegistries as usize
            })
        })
        .collect();
    let identities: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert!(identities.iter().all(|id| id == &identities[0]));
}
